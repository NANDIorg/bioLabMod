// M10: ген электрических органов у мутанта (GDD 4.22, 4.27 В).
// * Разряд: мутант, который охотится на игрока и стоит вплотную, раз в
//   3 / 2,5 / 2 с (сила 1 / 2 / 3) бьёт током всех в радиусе 2 / 2,5 / 3
//   блока — игрока (ванильным путём урона игрока: щит, броня, неуязвимость,
//   тип `biolab:electric`) и мобов, кроме таких же электрических. Задетых
//   оглушает: замедление и слабость.
// * Искры: изредка над телом мутанта — ванильные частицы `electric_spark`
//   (шлёт сервер, как кровь скальпеля в `thorns.rs`).
// Цели ближнего боя закрыты в селекторе целей Horizon, поэтому разряд —
// поверх ванильного ИИ, как уродства в `behaviour.rs`. Висит на тике
// игрока: тик уровня в одиночной игре не зовётся. Числа — раздел «M10:
// живые генераторы» в `runtime.rs` (`shock_*`), под тестами.
// Атрибутов у помощника быть не может (так требует инструмент миксинов).
fn biolab_core_electro_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_core_data as data;
    use horizon_client_core::who::Who;

    if !game.native_on(m::ID) || (game.serving.is_none() && game.options.dedicated.is_none()) {
        return;
    }
    // Отложенный разряд игрока с этим геном (`electro_hit.rs`): каждый тик.
    let due = m::take_due_shocks(game.ticks);
    for (id, amount) in due {
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id && mob.alive()) {
            mob.damage(amount, crate::game::biolab_damage::mob(&crate::game::biolab_gen_living::ELECTRIC));
        }
    }
    if game.ticks % m::BEHAVIOUR_EVERY != 0 {
        return;
    }
    let ids: Vec<u32> = game.mobs.iter().filter(|mob| mob.alive()).map(|mob| mob.id).collect();
    let genomes = data::genes_by_id(game, &ids);
    // Электрические мутанты: id, где стоит, рост, сила гена.
    let shockers: Vec<(u32, [f64; 3], f64, i32)> = game
        .mobs
        .iter()
        .filter(|mob| mob.alive())
        .filter_map(|mob| {
            let genes = genomes.get(&mob.id)?;
            let s = m::behaviour_strength(genes, "shock");
            (s > 0).then(|| (mob.id, [mob.position.x, mob.position.y, mob.position.z], f64::from(mob.height()), s))
        })
        .collect();
    if shockers.is_empty() {
        return;
    }
    let pass = game.ticks / m::BEHAVIOUR_EVERY;

    // Искры: у каждого своя фаза, сильнее — чаще.
    let sparks: Vec<[f64; 3]> = shockers
        .iter()
        .filter(|(id, _, _, s)| (pass + u64::from(*id)) % m::spark_one_in(*s) == 0)
        .map(|(_, at, height, _)| [at[0], at[1] + height * 0.6, at[2]])
        .collect();
    if !sparks.is_empty() {
        let options = horizon_client::particles::ParticleOptions::simple(horizon_client::particles::ParticleType::ElectricSpark);
        let _ = game.with_level(|_, level, _| {
            for at in &sparks {
                let _ = level.native_send_particles(&options, None, *at, [0.35, 0.4, 0.35], 0.05, 4);
            }
        });
    }

    // Разряды: охотник вплотную к игроку, по своему ритму.
    let player = game.native_player().map(|p| [p.x, p.y, p.z]);
    let Some(spot) = player else {
        return;
    };
    if game.creative || game.spectator() {
        return;
    }
    let day = m::daylight(game.world.day_time);
    let mut blasts: Vec<(u32, [f64; 3], i32)> = Vec::new();
    for (id, at, _, s) in &shockers {
        let Some(mob) = game.mobs.iter().find(|mob| mob.id == *id) else {
            continue;
        };
        let genes = genomes.get(id).map(Vec::as_slice).unwrap_or(&[]);
        // Лояльный не трогает людей, светобоязливый днём не охотится.
        let calm = m::behaviour_strength(genes, "loyal") > 0
            || (m::behaviour_strength(genes, "photophobia") > 0 && day);
        let hunting = mob.ai.target == Some(Who::Player);
        let flat = ((at[0] - spot[0]).powi(2) + (at[2] - spot[2]).powi(2)).sqrt();
        let level = (at[1] - spot[1]).abs() < 2.0;
        if !calm && hunting && level && flat < m::MELEE_REACH && game.ticks % m::shock_period(*s) == 0 {
            blasts.push((*id, *at, *s));
        }
    }
    if blasts.is_empty() {
        return;
    }
    let near = |a: [f64; 3], b: [f64; 3], r: f64| {
        (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2) < r * r
    };
    let electric: Vec<u32> = shockers.iter().map(|(id, _, _, _)| *id).collect();
    for (id, at, s) in &blasts {
        let radius = m::shock_radius(*s);
        let amount = m::shock_damage(*s);
        let stun = m::shock_stun(*s);
        // Мобы вокруг: свои электрические держат разряд, сам себя не бьёт.
        for mob in game.mobs.iter_mut() {
            if mob.id == *id || electric.contains(&mob.id) || !mob.alive() {
                continue;
            }
            let p = [mob.position.x, mob.position.y, mob.position.z];
            if near(*at, p, radius) {
                mob.damage(amount, crate::game::biolab_damage::mob(&crate::game::biolab_gen_living::ELECTRIC));
                for (effect, ticks, amp) in stun {
                    mob.add_effect(effect, ticks, amp);
                }
            }
        }
        // Игрок — ванильным путём удара моба (щит, броня), своим типом урона.
        if near(*at, spot, radius) && game.mob_hits_player(amount, *at, m::ELECTRIC_DAMAGE) {
            for (effect, ticks, amp) in stun {
                game.add_player_effect(horizon_client::effects::EffectInstance::new(effect, ticks, amp));
            }
        }
        let flash = [at[0], at[1] + 1.0, at[2]];
        let options = horizon_client::particles::ParticleOptions::simple(horizon_client::particles::ParticleType::ElectricSpark);
        let _ = game.with_level(|_, level, _| {
            let _ = level.native_send_particles(&options, None, flash, [radius * 0.4, 0.5, radius * 0.4], 0.2, 24);
            level.native_play_sound("minecraft:block.redstone_torch.burnout", flash, 5, 0.8, 1.6)
        });
    }
}
