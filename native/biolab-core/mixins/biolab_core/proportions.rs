// M9: пропорции тела (GDD 4.24) — что мутант с такими генами делает с игроком
// рядом. Хитбокс моба от генов не меняется, поэтому всё — числом, поверх
// ванильного ИИ, как уродства в `behaviour.rs`:
// * долгорукость — охотник достаёт игрока дальше своего удара (до 3 / 3,5 /
//   4 блоков), но замахивается раз в 2 с;
// * большеголовость — враждебный моб замечает игрока в 12 / 16 / 20 блоках
//   и сам светится сквозь стены: прятаться большой голове некуда.
// Числа (здоровье, скорость, урон, броня, взгляд, прыжок) ставит
// `biolab-deep` (`biolab_proportions`), косую походку — тоже он.
// Висит на тике игрока, как и `behaviour.rs`: тик уровня в одиночной игре не
// зовётся. Геном — данные на мобе (`biolab_core_data`).
// Атрибутов у помощника быть не может (так требует инструмент миксинов).
fn biolab_core_proportions_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_core_data as data;
    use horizon_client_core::who::Who;

    if !game.native_on(m::ID) {
        return;
    }
    if game.ticks % m::BEHAVIOUR_EVERY != 0 {
        return;
    }
    let Some(spot) = game.native_player().map(|p| [p.x, p.y, p.z]) else {
        return;
    };
    if game.creative || game.spectator() {
        return;
    }
    let ids: Vec<u32> = game.mobs.iter().filter(|mob| mob.alive()).map(|mob| mob.id).collect();
    let genomes = data::genes_by_id(game, &ids);
    let day = m::daylight(game.world.day_time);
    // Удары длинной руки: откуда (моб стоит там) и сколько.
    let mut strikes: Vec<([f64; 3], f32)> = Vec::new();
    // Кто заметил игрока, кто светится.
    let mut spotted: Vec<u32> = Vec::new();
    let mut glowing: Vec<u32> = Vec::new();
    for mob in game.mobs.iter() {
        let Some(genes) = genomes.get(&mob.id) else {
            continue;
        };
        if genes.is_empty() || !mob.alive() {
            continue;
        }
        let reach = m::behaviour_strength(genes, "long_reach");
        let head = m::behaviour_strength(genes, "big_head");
        if reach == 0 && head == 0 {
            continue;
        }
        // Лояльный не трогает людей, светобоязливый днём не охотится.
        let calm = m::behaviour_strength(genes, "loyal") > 0
            || (m::behaviour_strength(genes, "photophobia") > 0 && day);
        let at = [mob.position.x, mob.position.y, mob.position.z];
        let flat = ((at[0] - spot[0]).powi(2) + (at[2] - spot[2]).powi(2)).sqrt();
        let full = (flat * flat + (at[1] - spot[1]).powi(2)).sqrt();
        let level = (at[1] - spot[1]).abs() < 2.0;
        let hunting = mob.ai.target == Some(Who::Player);
        if head > 0 && full < m::big_head_radius(head) {
            glowing.push(mob.id);
            if !calm && mob.hostile() && !hunting {
                spotted.push(mob.id);
            }
        }
        if reach > 0
            && !calm
            && hunting
            && level
            && flat > m::LONG_ARM_FROM
            && flat < m::long_arm_reach(reach)
            && m::kind_has_arms(&mob.kind.id)
            && game.ticks % m::LONG_ARM_EVERY == 0
        {
            strikes.push((at, mob.attrs.attack_damage));
        }
    }
    for mob in game.mobs.iter_mut() {
        if glowing.contains(&mob.id) {
            mob.add_effect("glowing", m::BIG_HEAD_GLOW_TICKS, 0);
        }
        if spotted.contains(&mob.id) && mob.ai.target.is_none() {
            mob.ai.target = Some(Who::Player);
        }
    }
    // Ванильный путь урона игрока: щит, броня, неуязвимость, толчок от моба.
    for (from, amount) in strikes {
        if amount > 0.0 {
            let _ = game.mob_hits_player(amount, from, "minecraft:mob_attack");
        }
    }
}
