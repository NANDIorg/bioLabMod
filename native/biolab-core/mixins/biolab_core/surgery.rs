// Операционная (GDD 4.17), каждый тик: пациент лежит над своим столом и
// спит, срезанные конечности замедляют моба, ген регенерации их отращивает.
// Висит на тике игрока, как вольер (`behaviour.rs`): `native_level_tick_post`
// в одиночной игре не вызывается. Игроков бывает несколько — тик мира
// обрабатывается один раз (`surgery_pass_due`).
fn biolab_core_surgery_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_core_data as data;
    use horizon_client::mobs::finalize::{Attr, Op, add_modifier, has_modifier, modifier_amount, remove_modifier};

    if !game.native_on(m::ID) {
        return;
    }
    let ticks = game.ticks;
    if !m::surgery_pass_due(ticks) {
        return;
    }

    // Только мобы операционной: на столе, со срезами или с замедлением,
    // которое пора снять. Стол и срезы — данные на мобе (`biolab:surgery`,
    // M11): их читаем после обхода мобов, по собранным id.
    let alive: Vec<(u32, [f64; 3], bool)> = game
        .mobs
        .iter()
        .filter(|mob| mob.alive() && !mob.removed)
        .map(|mob| {
            (
                mob.id,
                [mob.position.x, mob.position.y, mob.position.z],
                has_modifier(mob, Attr::MovementSpeed, m::SURGERY_SLOW),
            )
        })
        .collect();
    let mut patients: Vec<(u32, [f64; 3], bool, u8, bool)> = Vec::new();
    for (id, at, slowed) in alive {
        let bits = data::surgery(game, id);
        let on_table = bits & m::TABLE_BIT != 0;
        let mask = bits & m::FLAYED_BITS;
        if !on_table && mask == 0 && !slowed {
            continue;
        }
        let regen = m::behaviour_strength(&data::genes(game, id), "regen") > 0;
        patients.push((id, at, on_table, mask, regen));
    }

    let dimension = game.dim.id().to_string();
    for (id, at, mut on_table, mut mask, regen) in patients {
        // Регенерация: раз в минуту отрастает последняя срезанная конечность.
        if regen
            && m::regrow_due(ticks, id)
            && let Some(limb) = m::regrown(mask)
        {
            mask &= !(1 << limb);
            data::set_surgery(game, id, on_table, mask);
        }

        // Стол держит пациента телепортом, как вольер: скорость ванильный
        // тик моба перезаписывает своей. Снотворное подновляем, чтобы он не
        // проснулся посреди операции. Стола под ним нет — свободен.
        let mut held = false;
        if on_table {
            match m::nearest_table(at, |spot| game.native_block(spot) == m::OPERATING_TABLE) {
                Some(table) => {
                    held = true;
                    // Свой стол — весь ряд секций под пациентом: держим над его
                    // центром. Ряд мог измениться (секцию добавили или
                    // сломали) — тогда центр сдвинулся, и пациент переедет.
                    let row = m::table_row(table, |at| game.native_block(at) == m::OPERATING_TABLE);
                    let spot = row.center();
                    if m::off_table(at, spot) {
                        game.native_teleport_mob(&dimension, id, spot);
                    }
                    // Раз в секунду — стороны секций по соседям (страховка к
                    // хуку `table_shape.rs`: ванилла гриба сама сторону не
                    // откроет, когда соседа сломали).
                    if m::table_sides_due(ticks) {
                        for section in row.sections() {
                            let sides = m::table_sides(section, |at| game.native_block(at) == m::OPERATING_TABLE);
                            for (key, value) in sides {
                                game.native_set_block_property(section, key, value);
                            }
                        }
                    }
                    let yaw = row.yaw();
                    if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
                        mob.add_effect(m::SEDATION_EFFECT, m::TABLE_SEDATION_TICKS, 0);
                        mob.ai.target = None;
                        // Лежит вдоль ряда, головой к изголовью: взгляд, тело и
                        // голова — по ряду каждый тик, иначе цели «посмотреть
                        // на игрока» развернули бы его поперёк стола.
                        mob.yaw = yaw;
                        mob.body_yaw = yaw;
                        mob.head_yaw = yaw;
                    }
                }
                None => {
                    on_table = false;
                    data::set_surgery(game, id, on_table, mask);
                }
            }
        }

        // Замедление за срезанное: на столе его нет, без срезов — снимаем.
        // Ставим, только когда число изменилось: модификатор пересчитывает
        // атрибут.
        let want = if held { 0.0 } else { m::surgery_slow(mask) };
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
            let have = modifier_amount(mob, Attr::MovementSpeed, m::SURGERY_SLOW);
            if want == 0.0 {
                if have.is_some() {
                    remove_modifier(mob, Attr::MovementSpeed, m::SURGERY_SLOW);
                }
            } else if have != Some(want) {
                add_modifier(mob, Attr::MovementSpeed, m::SURGERY_SLOW, want, Op::AddMultipliedBase);
            }
        }
    }

    // Только что уложенным — здоровье с носилок, пока геном поднимает им
    // максимум (`place_fluid.rs`).
    for (id, health) in m::due_settles(ticks) {
        game.native_set_mob_health(&dimension, id, health);
    }
}
