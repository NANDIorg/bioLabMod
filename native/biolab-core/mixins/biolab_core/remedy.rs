// Предмет в руке: препарат уходит в кровь, заражённая еда поднимает заражение.
fn biolab_core_use_item(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    claimed: &mut bool,
) {
    use crate::game::biolab_core as m;
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
        *claimed = true;
        return;
    }

    // Дротик: то же вещество, что в шприце, но по цели под прицелом.
    if let Some(syringe) = m::dart_syringe(&stack.item) {
        let Some((effects, ticks, amplifier)) = m::mob_injection(syringe) else {
            return;
        };
        let Some(player) = game.native_player() else {
            return;
        };
        // Куда смотрит игрок: вектор взгляда из его углов.
        let pitch = f64::from(player.pitch).to_radians();
        let yaw = f64::from(player.yaw).to_radians();
        let look = [
            -yaw.sin() * pitch.cos(),
            -pitch.sin(),
            yaw.cos() * pitch.cos(),
        ];
        let eye = [player.x, player.y + 1.62, player.z];
        let mut best: Option<(u32, f64)> = None;
        for mob in game.mobs.iter().filter(|mob| mob.alive()) {
            let to = [
                mob.position.x - eye[0],
                mob.position.y + 0.9 - eye[1],
                mob.position.z - eye[2],
            ];
            let far = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt();
            if far < 1.0 || far > m::DART_RANGE {
                continue;
            }
            let aim = (to[0] * look[0] + to[1] * look[1] + to[2] * look[2]) / far;
            if aim < m::DART_AIM {
                continue;
            }
            if best.is_none_or(|(_, nearest)| far < nearest) {
                best = Some((mob.id, far));
            }
        }
        let Some((target, _)) = best else {
            game.native_chat("Дротик ушёл в пустоту: цели на прицеле нет");
            *claimed = true;
            return;
        };
        let name = game
            .mobs
            .iter()
            .find(|mob| mob.id == target)
            .map(|mob| mob.kind.id.to_string())
            .unwrap_or_default();
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == target) {
            for effect in effects {
                mob.add_effect(effect, ticks, amplifier);
            }
        }
        game.native_chat(&m::injection_message(syringe, &name));
        let world = game.world_root.clone();
        m::record_trial(&world, &m::trial_record(syringe, &name));
        let slot = game.selected as i32;
        if let Some(carried) = game.native_carried_stack(slot)
            && &*carried.item == &*stack.item
        {
            carried.count = carried.count.saturating_sub(1);
        }
        *claimed = true;
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
