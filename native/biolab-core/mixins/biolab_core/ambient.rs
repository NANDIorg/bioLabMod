// Фон из своих частиц (GDD 3.9, «Частицы»): над лужами, работающими
// машинами, усыплёнными, обожжёнными и кровоточащими, вокруг мутантов с
// некрозом и кислотными железами. Что и как — таблицы `biolab_particles`,
// здесь только поиск источников и отправка.
//
// Висит на тике игрока, как вольеры и операционная: `native_level_tick_post`
// в одиночной игре не вызывается. Источники ищем вокруг игрока — частицы и
// так доходят только до игроков в 32 блоках, а машине или луже вдали от
// всех пыхать незачем. Раз в `EVERY` тиков.
fn biolab_core_ambient(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_particles as fx;
    use horizon_client::mobs::finalize::{Attr, has_modifier};
    use horizon_world::nbt::Tag;

    if !game.native_on(m::ID) || game.ticks % fx::EVERY != 0 {
        return;
    }
    // Частицы шлёт сервер (`native_send_particles`, API 103), в одиночной
    // игре тоже: окно подключено к своему серверу как игрок. У окна своей
    // очереди нет — выходим сразу и не ищем впустую.
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };
    let cycle = game.ticks / fx::EVERY;
    let me = [player.x, player.y, player.z];
    let mut puffs: Vec<(fx::Puff, [f64; 3], u64)> = Vec::new();

    // Лужи и машины вокруг игрока.
    let base = [me[0].floor() as i32, me[1].floor() as i32, me[2].floor() as i32];
    for dx in -fx::SCAN..=fx::SCAN {
        for dy in -fx::SCAN_Y..=fx::SCAN_Y {
            for dz in -fx::SCAN..=fx::SCAN {
                let pos = [base[0] + dx, base[1] + dy, base[2] + dz];
                let seed = fx::block_seed(pos);
                let block = game.native_block(pos);
                if let Some(puff) = fx::of_puddle(&block) {
                    if !fx::due(seed, cycle, puff.one_in) {
                        continue;
                    }
                    let layers = game
                        .native_block_property(pos, "layers")
                        .and_then(|v| v.parse::<i32>().ok())
                        .unwrap_or(1);
                    puffs.push((puff, fx::over_block(&puff, pos, fx::puddle_surface(layers)), seed));
                } else if let Some(puff) = fx::of_machine(&block) {
                    if !fx::due(seed, cycle, puff.one_in) {
                        continue;
                    }
                    // Отметку «занята» ставит тик машины (`machine_sound`).
                    // Читаем напрямую, а не `native_with_mod_block_entity`:
                    // тот помечает чанк изменённым, и он сохранялся бы
                    // каждые полсекунды.
                    let busy = matches!(
                        game.world.block_entities.get(&(pos[0], pos[1], pos[2])),
                        Some(horizon_client::chunk::block_entity::BlockEntity::Mod(e))
                            if matches!(e.nbt.get(&m::BUSY_KEY.into()), Some(Tag::Int(1)))
                    );
                    if busy {
                        puffs.push((puff, fx::over_block(&puff, pos, 1.0), seed));
                    }
                }
            }
        }
    }

    // Мобы рядом: эффекты и гены. Завитки эффектов, которые показывают наши
    // частицы, прячем (`hidden_effects`, как у `hideParticles`) — иначе над
    // усыплённым крутились бы ещё и ванильные спирали седации. Метаданные
    // моба уходят клиентам каждый снимок, так что спрятанное доходит и до
    // окна. Свежий эффект успевает покрутиться до ближайшего цикла.
    let near = |at: [f64; 3]| {
        (at[0] - me[0]).abs() < fx::REACH
            && (at[1] - me[1]).abs() < fx::REACH
            && (at[2] - me[2]).abs() < fx::REACH
    };
    for mob in game.mobs.iter_mut() {
        let feet = [mob.position.x, mob.position.y, mob.position.z];
        if !mob.alive() || mob.removed || !near(feet) {
            continue;
        }
        let seed = u64::from(mob.id);
        let height = f64::from(mob.height());
        let shown: Vec<&'static str> = mob
            .extra
            .effects
            .iter()
            .filter(|(id, ticks, _)| *ticks != 0 && fx::hides_swirls(id))
            .map(|(id, _, _)| *id)
            .collect();
        for id in shown {
            if !mob.extra.hidden_effects.contains(&id) {
                mob.extra.hidden_effects.push(id);
            }
            if let Some(puff) = fx::of_effect(id)
                && fx::due(seed, cycle, puff.one_in)
            {
                puffs.push((puff, fx::over_body(&puff, feet, height), seed ^ cycle));
            }
        }
        let genes: Vec<(&'static str, i32)> = m::GENE_MARKERS
            .iter()
            .filter(|marker| has_modifier(mob, Attr::FollowRange, marker))
            .filter_map(|marker| m::gene_of_marker(marker))
            .collect();
        if genes.is_empty() {
            continue;
        }
        for (flag, puff) in fx::BEHAVIOURS {
            if m::behaviour_strength(&genes, flag) > 0 && fx::due(seed, cycle, puff.one_in) {
                puffs.push((puff, fx::over_body(&puff, feet, height), seed ^ cycle));
            }
        }
    }

    // Сам игрок: его частицы видят другие, а у себя — разве что краем глаза.
    // Завитки игроку не прячем: флаг видимости уходит клиенту только вместе с
    // выдачей эффекта, а эффект луж и аур выдаётся заново каждую секунду.
    let seed = game.native_acting_id() as u64;
    let effects: Vec<&'static str> = game.potions.effects.list.iter().map(|e| e.id).collect();
    for id in effects {
        if let Some(puff) = fx::of_effect(id)
            && fx::due(seed, cycle, puff.one_in)
        {
            puffs.push((puff, fx::over_body(&puff, me, fx::PLAYER_HEIGHT), seed ^ cycle));
        }
    }

    if puffs.is_empty() {
        return;
    }
    // Один заход в уровень на весь цикл: `with_level` собирает коробки
    // игрока, вагонеток и стрел — внутренний API игры, не хук-поинт, при
    // обновлении Horizon проверить.
    let _ = game.with_level(|_, level, _| {
        for (puff, at, seed) in &puffs {
            let options = horizon_client::particles::ParticleOptions::named(puff.name);
            for s in fx::sends(puff, *at, *seed) {
                let _ = level.native_send_particles(&options, None, s.at, s.spread, s.speed, s.count);
            }
        }
    });
}
