// Предмет в руке: препарат уходит в кровь, заражённая еда поднимает заражение.
fn biolab_core_use_item(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    claimed: &mut bool,
) {
    use crate::game::biolab_core as m;
    /// Звук события мода в точке `at` всем игрокам рядом. Звук встаёт в
    /// очередь уровня (`native_play_sound`, API 25), и сервер рассылает его
    /// пакетом — в одиночной игре тоже: окно подключено к своему серверу как
    /// игрок. Без своего сервера (клиент) `native_play_sound` звук не берёт,
    /// поэтому там выходим сразу. `with_level` — внутренний API игры, не
    /// хук-поинт: при обновлении Horizon проверить.
    fn play_at(
        game: &mut crate::game::Game<'_>,
        cue: crate::game::biolab_core::Cue,
        at: [f64; 3],
    ) {
        let Some(sound) = crate::game::biolab_core::sound(cue) else {
            return;
        };
        if game.serving.is_none() && game.options.dedicated.is_none() {
            return;
        }
        let _ = game.with_level(|_, level, _| {
            level.native_play_sound(sound.event, at, sound.source, sound.volume, sound.pitch)
        });
    }
    /// Звук у головы игрока, который действует.
    fn play(game: &mut crate::game::Game<'_>, cue: crate::game::biolab_core::Cue) {
        if let Some(player) = game.native_player() {
            play_at(game, cue, [player.x, player.y + 1.0, player.z]);
        }
    }

    if !game.native_on(m::ID) {
        return;
    }
    let uuid = game.native_acting_id();
    let world = game.world_root.clone();

    // Журнал лаборанта открывает карту исследований на клиенте (`biolab-deep`):
    // серверу здесь делать нечего, лишь бы другие обработчики не трогали книгу.
    if &*stack.item == m::MANUAL_ID {
        *claimed = true;
        return;
    }

    // Бинт: перевязка останавливает кровотечение и немного лечит.
    if &*stack.item == m::BANDAGE_ID {
        game.potions
            .effects
            .list
            .retain(|e| e.id != m::BLEEDING);
        game.apply_effect_action(horizon_client::effects::EffectAction::Heal(m::BANDAGE_HEAL));
        let slot = game.selected as i32;
        if let Some(carried) = game.native_carried_stack(slot)
            && &*carried.item == m::BANDAGE_ID
        {
            carried.count = carried.count.saturating_sub(1);
        }
        game.native_chat("Повязка наложена: кровотечение остановлено");
        play(game, m::Cue::Bandage);
        *claimed = true;
        return;
    }

    // Оружие для дротиков (GDD 4.20): трубка и заряженный арбалет стреляют,
    // пустое ружьё заряжается, нечем — осечка (`weapons.rs`). Натяжение
    // арбалета и прицел ружья клик не забирают: это использование со временем
    // (`weapon_start.rs`, `weapon_tick.rs`, `weapon_release.rs`).
    if let Some(weapon) = m::dart_weapon(&stack.item) {
        if crate::game::biolab_weapons::click(game, weapon, stack) {
            *claimed = true;
        }
        return;
    }

    // Дротик здесь не забираем: клик проходит дальше, и игра бросает его
    // сама, как снежок, — у дротика свой тип сущности (`entity_type/`).
    // Вещество мобу вводит хук попадания (`dart_hit.rs`). Раньше здесь был
    // мгновенный «выстрел» в моба под прицелом; снаряд честнее: виден полёт,
    // нужно упреждение, промах ломает дротик о блок.
    if m::dart_syringe(&stack.item).is_some() {
        return;
    }

    // Урожай с ГМ-грядки: съедается и действует по геному растения.
    if &*stack.item == m::GM_CROP {
        let text = stack
            .extra
            .as_ref()
            .and_then(|extra| {
                extra
                    .other
                    .iter()
                    .find(|(key, _)| key == "minecraft:custom_data")
                    .and_then(|(_, data)| match data.get(m::PLOT_GENOME_KEY) {
                        Some(horizon_world::nbt::Tag::String(s)) => Some(s.to_string_lossy()),
                        _ => None,
                    })
            })
            .unwrap_or_default();
        let genes = m::genome_from_text(&text);
        if genes.is_empty() {
            return;
        }
        for (effect, ticks, amplifier) in m::crop_effects(&genes) {
            game.add_player_effect(horizon_client::effects::EffectInstance::new(
                effect, ticks, amplifier,
            ));
        }
        let slot = game.selected as i32;
        if let Some(carried) = game.native_carried_stack(slot)
            && &*carried.item == m::GM_CROP
        {
            carried.count = carried.count.saturating_sub(1);
        }
        game.native_chat(&m::crop_label(&genes));
        play(game, m::Cue::Eat);
        *claimed = true;
        return;
    }

    // Фрагмент ДНК в руке: генотерапия, ген уходит в собственный геном.
    if &*stack.item == m::FRAGMENT_ID {
        if m::just_injected(game.ticks) {
            *claimed = true;
            return;
        }
        let text = stack
            .extra
            .as_ref()
            .and_then(|extra| {
                extra
                    .other
                    .iter()
                    .find(|(key, _)| key == "minecraft:custom_data")
                    .and_then(|(_, data)| match data.get(m::GENE_KEY) {
                        Some(horizon_world::nbt::Tag::String(s)) => Some(s.to_string_lossy()),
                        _ => None,
                    })
            })
            .unwrap_or_default();
        let Some((name, strength)) = m::gene_from_text(&text) else {
            game.native_chat("Фрагмент пустой: ген не прочитан");
            *claimed = true;
            return;
        };
        let Some(result) = m::splice_world_player(&world, uuid, name, strength) else {
            return;
        };
        game.native_chat(&m::splice_message(result, name, strength));
        if result == m::Splice::Done {
            play(game, m::Cue::GeneTherapy);
            let slot = game.selected as i32;
            if let Some(carried) = game.native_carried_stack(slot)
                && &*carried.item == m::FRAGMENT_ID
            {
                carried.count = carried.count.saturating_sub(1);
            }
        }
        *claimed = true;
        return;
    }

    // Препарат: доза уходит в кровь, дальше ею занимается тик кинетики.
    // Если этим же кликом шприц уже достался мобу, внутрь ничего не идёт.
    if let Some(med) = m::medicine(&stack.item) {
        if m::just_injected(game.ticks) {
            *claimed = true;
            return;
        }
        let Some(state) = m::take_medicine(&world, uuid, med) else {
            return;
        };
        game.native_chat(&m::dose_message(med, &state));
        // Укол, таблетка, мазь или аэрозоль — по форме препарата.
        play(game, m::dose_cue(med.form));
        // Съеденную таблетку и вколотую ампулу убираем сами: клик мы забрали,
        // ванильный расход предмета уже не сработает. От шприца остаётся
        // пустой: последний в стаке подменяем прямо в слоте, потому что
        // выдача в инвентарь отсюда не всегда доходит (проверено в игре —
        // шприц от вакцины пропадал совсем).
        let leftover = m::after_use(&stack.item);
        let slot = game.selected as i32;
        let mut give_back = false;
        if let Some(carried) = game.native_carried_stack(slot)
            && &*carried.item == &*stack.item
        {
            carried.count = carried.count.saturating_sub(1);
            match (carried.count, leftover) {
                (0, Some(item)) => {
                    carried.item = item.into();
                    carried.count = 1;
                    carried.damage = 0;
                    carried.extra = None;
                }
                (_, Some(_)) => give_back = true,
                _ => {}
            }
        }
        if give_back && let Some(item) = leftover {
            let _ = game.native_give(item, 1);
        }
        *claimed = true;
        return;
    }

    // Чашка с культурой: заразить себя своим же штаммом — так проверяют
    // выведенную линию, когда подопытных не осталось.
    if &*stack.item == m::CULTURE_ID {
        let text = stack
            .extra
            .as_ref()
            .and_then(|extra| {
                extra
                    .other
                    .iter()
                    .find(|(key, _)| key == "minecraft:custom_data")
                    .and_then(|(_, data)| match data.get(m::STRAIN_KEY) {
                        Some(horizon_world::nbt::Tag::String(s)) => Some(s.to_string_lossy()),
                        _ => None,
                    })
            })
            .unwrap_or_default();
        let Some(strain) = m::strain_from_text(&text) else {
            game.native_chat(&format!("Штамм не прочитан: «{text}»"));
            *claimed = true;
            return;
        };
        if let Some(level) = m::infect_world_player(&world, uuid, &strain, m::CULTURE_DOSE) {
            game.native_chat(&format!("Заражение штаммом: {level}%"));
        }
        let slot = game.selected as i32;
        if let Some(carried) = game.native_carried_stack(slot)
            && &*carried.item == m::CULTURE_ID
        {
            carried.count = carried.count.saturating_sub(1);
        }
        *claimed = true;
        return;
    }

    if &*stack.item == m::REMEDY_ID {
        let Some(state) = m::player_tick(&world, uuid) else {
            return;
        };
        let relief = m::strain_relief(&m::strain_or_wild(&state));
        let next = state.infection.saturating_sub(relief);
        m::set_infection(&world, uuid, next);
        game.native_chat(&m::remedy_message(state.infection, next));
        play(game, m::Cue::Remedy);
        // Сыворотка одноразовая: без этого её можно было пить бесконечно.
        let slot = game.selected as i32;
        if let Some(carried) = game.native_carried_stack(slot)
            && &*carried.item == m::REMEDY_ID
        {
            carried.count = carried.count.saturating_sub(1);
        }
        *claimed = true;
        return;
    }

    // Заражённую еду не перехватываем: игрок всё равно её ест, просто платит
    // за это уровнем заражения — если не привит.
    if let Some(amount) = m::tainted_food(&stack.item)
        && let Some(state) = m::player_tick(&world, uuid)
    {
        let next = m::after_exposure_with(state.infection, amount, state.immunity > 0);
        m::set_infection(&world, uuid, next);
    }
}
