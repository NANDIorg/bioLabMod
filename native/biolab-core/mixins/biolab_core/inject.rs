// Шприц по мобу: яд травит, седативное усыпляет, пенициллин лечит.
fn biolab_core_interact_entity(
    game: &mut crate::game::Game<'_>,
    entity: i32,
    stack: Option<&horizon_client::items::ItemStack>,
    claimed: &mut bool,
) {
    use crate::game::biolab_core as m;
    /// Гены моба по меткам на нём.
    fn read_genome(game: &crate::game::Game<'_>, entity: i32) -> Vec<(&'static str, i32)> {
        use crate::game::biolab_core as m;
        use horizon_client::mobs::finalize::{Attr, has_modifier};
        let Some(mob) = game.mobs.iter().find(|mob| mob.id as i32 == entity) else {
            return Vec::new();
        };
        m::GENE_MARKERS
            .iter()
            .filter(|marker| has_modifier(mob, Attr::FollowRange, marker))
            .filter_map(|marker| m::gene_of_marker(marker))
            .collect()
    }

    /// Ген, записанный во фрагмент.
    fn gene_in(stack: &horizon_client::items::ItemStack) -> Option<String> {
        use crate::game::biolab_core as m;
        let extra = stack.extra.as_ref()?;
        let (_, data) = extra
            .other
            .iter()
            .find(|(key, _)| key == "minecraft:custom_data")?;
        match data.get(m::GENE_KEY) {
            Some(horizon_world::nbt::Tag::String(s)) => Some(s.to_string_lossy()),
            _ => None,
        }
    }

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

    if !game.native_on(m::ID) {
        return;
    }
    let Some(stack) = stack else {
        return;
    };
    // Микроскоп: читаем геном моба по меткам и показываем игроку.
    if &*stack.item == m::MICROSCOPE_ID {
        let genes = read_genome(game, entity);
        for line in m::genome_lines(&genes) {
            game.native_chat(&line);
        }
        m::remember_injection(game.ticks);
        *claimed = true;
        return;
    }

    // Фрагмент ДНК: вживляем ген мобу. Метку читает `biolab-deep` — он же
    // выдаёт прибавки и внешность.
    if &*stack.item == m::FRAGMENT_ID {
        let Some(text) = gene_in(stack) else {
            game.native_chat("Фрагмент пустой: ген не прочитан");
            return;
        };
        let Some((name, strength)) = m::gene_from_text(&text) else {
            return;
        };
        let have = read_genome(game, entity);
        let result = m::splice_mob(&have, name, strength);
        // Клик по мобу ловят оба хука: без этой отметки фрагмент достанется
        // ещё и самому игроку (проверено в игре — ген вживлялся обоим).
        m::remember_injection(game.ticks);
        if result == m::Splice::Done
            && let Some(marker) = m::marker_for(name, strength)
            && let Some(mob) = game
                .mobs
                .iter_mut()
                .find(|mob| mob.id as i32 == entity && mob.alive())
        {
            horizon_client::mobs::finalize::add_modifier(
                mob,
                horizon_client::mobs::finalize::Attr::FollowRange,
                marker,
                0.0,
                horizon_client::mobs::finalize::Op::AddValue,
            );
            let slot = game.selected as i32;
            if let Some(carried) = game.native_carried_stack(slot)
                && &*carried.item == m::FRAGMENT_ID
            {
                carried.count = carried.count.saturating_sub(1);
            }
        }
        game.native_chat(&m::splice_message(result, name, strength));
        if result == m::Splice::Done {
            let target = game
                .mobs
                .iter()
                .find(|mob| mob.id as i32 == entity)
                .map(|mob| mob.kind.id.to_string())
                .unwrap_or_default();
            let world = game.world_root.clone();
            m::record_trial(&world, &m::trial_record(m::FRAGMENT_ID, &target));
            let at = game
                .mobs
                .iter()
                .find(|mob| mob.id as i32 == entity)
                .map(|mob| [mob.position.x, mob.position.y + 0.9, mob.position.z]);
            if let Some(at) = at {
                play_at(game, m::Cue::GeneSplice, at);
            }
        }
        *claimed = true;
        return;
    }

    let Some((effects, ticks, amplifier)) = m::mob_injection(&stack.item) else {
        return;
    };
    // Мобы приходят списком, а хук даёт сетевой номер сущности.
    let Some(mob) = game
        .mobs
        .iter_mut()
        .find(|mob| mob.id as i32 == entity && mob.alive())
    else {
        return;
    };
    let name = mob.kind.id.clone();
    let at = [mob.position.x, mob.position.y + 0.9, mob.position.z];
    for effect in effects {
        mob.add_effect(effect, ticks, amplifier);
    }
    game.native_chat(&m::injection_message(&stack.item, &name));
    play_at(game, m::mob_injection_cue(&stack.item), at);
    m::remember_injection(game.ticks);
    // Опыт уходит в журнал: пульт потом покажет, кому что вводили.
    let world = game.world_root.clone();
    m::record_trial(&world, &m::trial_record(&stack.item, &name));

    // Шприц тратится, пустой остаётся в руке. Если это был последний в
    // стаке, кладём пустой прямо в слот: выдача в инвентарь из этого хука
    // не доходит (проверено в игре — шприц пропадал совсем).
    let slot = game.selected as i32;
    let leftover = m::after_use(&stack.item);
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
}
