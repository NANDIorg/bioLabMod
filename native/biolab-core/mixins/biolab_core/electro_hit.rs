// M10: ген электрических органов у игрока (GDD 4.22, 4.27 В). Удар по мобу
// — ещё и разряд: цели урон током `biolab:electric` (через 11 тиков, после
// кадров неуязвимости от самого удара) и замедление, мобам в 1,5–2,5 блоках
// от неё — половина сразу. Но если игрок стоит в воде или под
// дождём под открытым небом, ток уходит в него самого: урон по себе тем же
// типом, а цели ничего. Не чаще раза в секунду (`PLAYER_SHOCK_EVERY`).
// Ген игрока — из состояния мира (`world_player_gene`, как рога в
// `thorns.rs`). Вторая инъекция на `native_hurt_enemy` (API 39), после
// `thorns.rs`. Числа — раздел «M10: живые генераторы» в `runtime.rs`.
fn biolab_core_electro_hit(game: &mut crate::game::Game<'_>, entity: u32) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) {
        return;
    }
    let world = game.world_root.clone();
    let uuid = game.native_acting_id();
    let strength = m::world_player_gene(&world, uuid, m::ELECTROCYTES);
    if strength <= 0 {
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };
    if !m::player_shock_ready(uuid, game.ticks) {
        return;
    }
    let feet = [player.x.floor() as i32, player.y.floor() as i32, player.z.floor() as i32];
    let head = [feet[0], (player.y + 1.6).floor() as i32, feet[2]];
    let (_, sky) = game.native_light(head);
    let (rain, _) = game.native_weather();
    let (feet_block, head_block) = (game.native_block(feet), game.native_block(head));
    let amount = m::player_shock_damage(strength);
    if m::shock_backfires(&feet_block, &head_block, rain, sky) {
        crate::game::biolab_damage::player(game, &crate::game::biolab_gen_living::ELECTRIC, amount);
        crate::game::biolab_say::say(game, m::SHOCK_BACKFIRE);
        return;
    }
    let Some(target) = game
        .mobs
        .iter()
        .find(|mob| mob.id == entity && mob.alive())
        .map(|mob| [mob.position.x, mob.position.y, mob.position.z])
    else {
        return;
    };
    let radius = m::player_shock_radius(strength);
    let game_ticks = game.ticks;
    let shock = || crate::game::biolab_damage::mob(&crate::game::biolab_gen_living::ELECTRIC);
    for mob in game.mobs.iter_mut() {
        if !mob.alive() {
            continue;
        }
        let p = [mob.position.x, mob.position.y, mob.position.z];
        let d2 = (p[0] - target[0]).powi(2) + (p[1] - target[1]).powi(2) + (p[2] - target[2]).powi(2);
        if mob.id == entity {
            // Цель только что ранена ударом: ток дойдёт после кадров
            // неуязвимости (`electro_tick.rs` разбирает очередь).
            m::queue_shock(entity, amount, game_ticks + m::PLAYER_SHOCK_DELAY);
            mob.add_effect("slowness", 40, strength - 1);
        } else if d2 < radius * radius {
            mob.damage(amount * 0.5, shock());
            mob.add_effect("slowness", 30, 0);
        }
    }
    if game.serving.is_some() || game.options.dedicated.is_some() {
        let options = horizon_client::particles::ParticleOptions::simple(horizon_client::particles::ParticleType::ElectricSpark);
        let at = [target[0], target[1] + 1.0, target[2]];
        let _ = game.with_level(|_, level, _| level.native_send_particles(&options, None, at, [0.4, 0.5, 0.4], 0.15, 12));
    }
}
