// Доза и болезнь (M9, GDD 4.23 п. 1, 2, 12): тик дозы игрока на сервере.
//
// * Очередь доз (`take_queued_doses`): лужи, зомби вплотную и утечка
//   (`infection.rs`), удар заражённого (`infected_tick.rs`), съеденное
//   (`remedy.rs` и загрязнённая еда) — каждая порция режется спецодеждой по
//   своему каналу (`suit_multiplier`) и копится (`Exposure`). Раз в секунду
//   проверка: дошла до порога — заражение штаммом источника, иначе доза тает.
// * Приступы: расписание по `game.ticks`, перед приступом — стук сердца.
//   Симптомы держит тик болезни (`infection.rs`, `seizure_now`).
// * Звук опасности: анализатор в руке щёлкает дозиметром, в маске слышно
//   дыхание. Тревога утечки — в `quarantine.rs` (`leak`).
// * Доза — клиенту (`NET_DOSE`), для панели под анализатором (`dose_hud.rs`).
//
// Правила — раздел «M9: доза и болезнь» в `runtime.rs`, под тестами. Только
// свой сервер или выделенный: очередь, мобы и звуки — там.
fn biolab_core_dose_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;

    /// Звук у головы игрока — как `play_at` в `infection.rs`: в очередь
    /// уровня (`native_play_sound`, API 25), сервер рассылает его пакетом.
    fn play(game: &mut crate::game::Game<'_>, cue: crate::game::biolab_core::Cue, at: [f64; 3]) {
        let Some(sound) = crate::game::biolab_core::sound(cue) else {
            return;
        };
        let _ = game.with_level(|_, level, _| {
            level.native_play_sound(sound.event, at, sound.source, sound.volume, sound.pitch)
        });
    }

    /// Что надето: слоты брони `native_carried_slot` — 103 голова, 102 грудь,
    /// 101 ноги, 100 ступни; в `Worn` тот же порядок сверху вниз. Вторая
    /// строка — `custom_data` стака в SNBT (`Tag.toString`), пусто — нет.
    fn worn(game: &mut crate::game::Game<'_>) -> crate::game::biolab_core::Worn {
        let mut worn = crate::game::biolab_core::Worn::default();
        for (i, slot) in [103, 102, 101, 100].into_iter().enumerate() {
            let Some(stack) = game.native_carried_stack(slot) else {
                continue;
            };
            if stack.count == 0 {
                continue;
            }
            let data = stack
                .extra
                .as_ref()
                .and_then(|extra| extra.other.iter().find(|(key, _)| key == "minecraft:custom_data"))
                .map(|(_, data)| horizon_world::snbt_text::to_snbt(data))
                .unwrap_or_default();
            worn.slots[i] = (stack.item.to_string(), data);
        }
        worn
    }

    if !game.native_on(m::ID) {
        return;
    }
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };
    let uuid = game.native_acting_id();
    let world = game.world_root.clone();
    let now = game.ticks;
    let head = [player.x, player.y + 1.0, player.z];
    let worn = worn(game);

    // Порции из очереди — сквозь спецодежду.
    let doses = m::take_queued_doses(uuid);
    if !doses.is_empty() {
        m::with_exposure(uuid, |exposure| {
            for dose in &doses {
                exposure.add(dose.channel, dose.amount, m::suit_multiplier(&worn, dose.channel), dose.strain);
            }
        });
    }

    // Проверка раз в секунду. В держащей карантинной камере (GDD 4.18 п. 4)
    // доза не заражает, а только тает.
    if m::dose_check_due(now) {
        let clean = m::in_quarantine_at(player.dimension, [player.x, player.y, player.z]);
        let (source, exposure) = m::with_exposure(uuid, |e| (e.check(!clean), *e)).unwrap_or_default();
        if let Some((channel, strain)) = source {
            let strain = strain.unwrap_or(m::WILD);
            let before = m::world_player_state(&world, uuid).map_or(0, |s| s.infection);
            // Привитого `infect_world_player` не заражает: уровень не меняется.
            let after = m::infect_world_player(&world, uuid, &strain, m::DOSE_INFECTION);
            if before == 0 && after.is_some_and(|level| level > 0) {
                crate::game::biolab_say::say(game, &m::dose_infected_message(channel, Some(&strain)));
            }
        }
        let _ = game.native_net_send_to_player(uuid, m::NET_DOSE, &m::dose_packet(&exposure));

        // Опасность для дозиметра: доза, свежий поток и источники рядом —
        // зомби и заражённые, больной скот, живая утечка.
        let range = m::DOSIMETER_RANGE;
        let mut near: f32 = game
            .mobs
            .iter()
            .filter(|mob| {
                mob.alive()
                    && (mob.species() == horizon_client::mobs::species::Species::Zombie
                        || m::is_infected(&mob.kind.id)
                        || mob.extra.effects.iter().any(|(id, ticks, _)| &**id == m::INFECTION_EFFECT && *ticks != 0))
            })
            .map(|mob| {
                let (dx, dy, dz) = (mob.position.x - player.x, mob.position.y - player.y, mob.position.z - player.z);
                (dx * dx + dy * dy + dz * dz).sqrt()
            })
            .filter(|distance| *distance < range)
            .map(m::source_weight)
            .sum();
        if m::leak_near(player.dimension, [player.x, player.y, player.z]).is_some() {
            near += 1.0;
        }
        near += m::contamination_danger(m::contamination_level_at(player.dimension, [player.x.floor() as i32, player.y.floor() as i32, player.z.floor() as i32])); // M10-Г
        m::set_danger(uuid, m::danger_level(&exposure, near));
    }

    // Дозиметр: анализатор в руке (основной или второй) щёлкает в темпе
    // опасности.
    let selected = game.selected as i32;
    let holding = [selected, -106]
        .into_iter()
        .any(|slot| game.native_carried_stack(slot).is_some_and(|s| &*s.item == m::ANALYZER_ID));
    if holding && m::dosimeter_tick(uuid, now) {
        play(game, m::Cue::DosimeterClick, head);
    }

    // В маске или костюме слышно своё дыхание.
    if m::breathes(m::suit_multiplier(&worn, m::DoseChannel::Air)) && m::breath_due(now, m::seizure_seed(uuid)) {
        play(game, m::Cue::Breath, head);
    }

    // Приступы: перед каждым — стук сердца.
    if let Some(state) = m::world_player_state(&world, uuid) {
        let strain = m::strain_or_wild(&state);
        if m::seizure_tick(uuid, now, state.infection, &strain) == m::SeizureEvent::Warn {
            play(game, m::Cue::SeizureWarning, head);
        }
    }
}
