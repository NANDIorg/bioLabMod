// Операционная (GDD 4.17), каждый тик: пациент лежит над своим столом и
// спит, срезанные конечности замедляют моба, ген регенерации их отращивает.
// Висит на тике игрока, как вольер (`behaviour.rs`): `native_level_tick_post`
// в одиночной игре не вызывается. Игроков бывает несколько — тик мира
// обрабатывается один раз (`surgery_pass_due`).
fn biolab_core_surgery_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    use horizon_client::mobs::finalize::{
        Attr, Op, add_modifier, has_modifier, modifier_amount, remove_modifier,
    };

    if !game.native_on(m::ID) {
        return;
    }
    let ticks = game.ticks;
    if !m::surgery_pass_due(ticks) {
        return;
    }

    // Только мобы операционной: на столе, со срезами или с замедлением,
    // которое пора снять. Остальных пропускаем по дешёвой проверке меток.
    let patients: Vec<(u32, [f64; 3], bool, u8, bool)> = game
        .mobs
        .iter()
        .filter(|mob| mob.alive() && !mob.removed)
        .filter_map(|mob| {
            let on_table = has_modifier(mob, Attr::FollowRange, m::SURGERY_TABLE);
            let mask = m::flayed_mask(|marker| has_modifier(mob, Attr::FollowRange, marker));
            let slowed = has_modifier(mob, Attr::MovementSpeed, m::SURGERY_SLOW);
            if !on_table && mask == 0 && !slowed {
                return None;
            }
            let genes: Vec<(&'static str, i32)> = m::GENE_MARKERS
                .iter()
                .filter(|marker| has_modifier(mob, Attr::FollowRange, marker))
                .filter_map(|marker| m::gene_of_marker(marker))
                .collect();
            let regen = m::behaviour_strength(&genes, "regen") > 0;
            Some((
                mob.id,
                [mob.position.x, mob.position.y, mob.position.z],
                on_table,
                mask,
                regen,
            ))
        })
        .collect();

    let dimension = game.dim.id().to_string();
    for (id, at, on_table, mut mask, regen) in patients {
        // Регенерация: раз в минуту отрастает последняя срезанная конечность.
        if regen
            && m::regrow_due(ticks, id)
            && let Some(limb) = m::regrown(mask)
            && let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id)
        {
            remove_modifier(mob, Attr::FollowRange, m::SURGERY_FLAYED[usize::from(limb)]);
            mask &= !(1 << limb);
        }

        // Стол держит пациента телепортом, как вольер: скорость ванильный
        // тик моба перезаписывает своей. Снотворное подновляем, чтобы он не
        // проснулся посреди операции. Стола под ним нет — свободен.
        let mut held = false;
        if on_table {
            match m::nearest_table(at, |spot| game.native_block(spot) == m::OPERATING_TABLE) {
                Some(table) => {
                    held = true;
                    let spot = m::table_spot(table);
                    if m::off_table(at, spot) {
                        game.native_teleport_mob(&dimension, id, spot);
                    }
                    if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
                        mob.add_effect(m::SEDATION_EFFECT, m::TABLE_SEDATION_TICKS, 0);
                        mob.ai.target = None;
                    }
                }
                None => {
                    if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
                        remove_modifier(mob, Attr::FollowRange, m::SURGERY_TABLE);
                    }
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
