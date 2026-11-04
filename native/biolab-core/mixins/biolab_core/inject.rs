// Шприц по мобу: яд травит, седативное усыпляет, пенициллин лечит.
// Операционная (GDD 4.17): носилки грузят усыплённого моба, скальпель
// срезает плоть с мутанта на операционном столе.
fn biolab_core_interact_entity(
    game: &mut crate::game::Game<'_>,
    entity: i32,
    stack: Option<&horizon_client::items::ItemStack>,
    claimed: &mut bool,
) {
    use crate::game::biolab_core as m;
    /// Всплеск своих частиц в точке `at` (`native_send_particles`, API 103).
    /// Шлёт сервер, как и звук: у окна без своего сервера очереди нет.
    fn puff_at(
        game: &mut crate::game::Game<'_>,
        puff: crate::game::biolab_particles::Puff,
        at: [f64; 3],
        seed: u64,
    ) {
        if game.serving.is_none() && game.options.dedicated.is_none() {
            return;
        }
        let options = horizon_client::particles::ParticleOptions::named(puff.name);
        let _ = game.with_level(|_, level, _| {
            for s in crate::game::biolab_particles::sends(&puff, at, seed) {
                let _ = level.native_send_particles(&options, None, s.at, s.spread, s.speed, s.count);
            }
        });
    }
    /// Гены моба — из данных на нём (`biolab:genome`, M11).
    fn read_genome(game: &mut crate::game::Game<'_>, entity: i32) -> Vec<(&'static str, i32)> {
        let Ok(id) = u32::try_from(entity) else {
            return Vec::new();
        };
        if !game.mobs.iter().any(|mob| mob.id == id) {
            return Vec::new();
        }
        crate::game::biolab_core_data::genes(game, id)
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

    /// Пациент, записанный в носилки; None — носилки пусты.
    fn patient_in(stack: &horizon_client::items::ItemStack) -> Option<String> {
        use crate::game::biolab_core as m;
        let extra = stack.extra.as_ref()?;
        let (_, data) = extra
            .other
            .iter()
            .find(|(key, _)| key == "minecraft:custom_data")?;
        match data.get(m::PATIENT_KEY) {
            Some(horizon_world::nbt::Tag::String(s)) => Some(s.to_string_lossy()),
            _ => None,
        }
    }

    /// Слот руки, в которой лежит `item`: основная, если в ней он, иначе
    /// левая (-106). Хук спрашивает каждую руку по очереди, и менять надо
    /// ту, что кликнула.
    fn hand_with(game: &mut crate::game::Game<'_>, item: &str) -> i32 {
        let main = game.selected as i32;
        if game
            .native_carried_stack(main)
            .is_some_and(|held| &*held.item == item)
        {
            main
        } else {
            -106
        }
    }

    /// Кусок мутантной ткани падает на землю у пациента, как обычный лут, а
    /// не в рюкзак: с полным рюкзаком срез раньше просто не делался.
    ///
    /// Срез скальпелем — не поломка блока и не смерть моба, поэтому
    /// `native_loot_drops` его не слышит (`mod_api.rs`, `NativeLootDrops`), а
    /// стабильного помощника «создать предметную сущность» в пине нет. Берём
    /// ту же функцию, которой игра роняет лут умершего моба после
    /// `native_loot_drops` (`Game::spawn_item`, `creatures.rs`: позиция моба,
    /// толчок `dispenser::new_item_motion`, задержка подбора 10 тиков). Это
    /// внутренний API, не хук-поинт: при обновлении Horizon проверить.
    ///
    /// Хук серверный (`native_interact_entity`), так что мир здесь свой.
    /// Страховка на случай, если его позовут у клиента чужого сервера: там
    /// предмет на земле был бы призраком, которого сервер не знает.
    fn drop_tissue(game: &mut crate::game::Game<'_>, at: [f64; 3]) {
        use crate::game::biolab_core as m;
        if game.remote.is_some() {
            return;
        }
        let tissue = horizon_client::items::ItemStack {
            item: m::TISSUE_ID.into(),
            count: 1,
            damage: 0,
            extra: None,
        };
        game.spawn_item(tissue, at, crate::game::dispenser::new_item_motion(), 10);
    }

    if !game.native_on(m::ID) {
        return;
    }
    let Some(stack) = stack else {
        return;
    };

    // Носилки. Пустые грузят усыплённого моба — и снимают его со стола: там
    // он тоже спит. Занятые укладывают пациента на стол (`place_fluid.rs`).
    if &*stack.item == m::STRETCHER_ID {
        // Клик забираем всегда: носилки в лицо жителю не открывают торговлю.
        *claimed = true;
        if patient_in(stack).is_some() {
            game.native_chat("Носилки заняты: ПКМ по операционному столу — уложить");
            return;
        }
        let Some(mob) = game
            .mobs
            .iter()
            .find(|mob| mob.id as i32 == entity && mob.alive() && !mob.removed)
        else {
            return;
        };
        let kind = m::full_kind(&mob.kind.id);
        let asleep = mob
            .extra
            .effects
            .iter()
            .any(|(id, ticks, _)| *id == m::SEDATION_EFFECT && *ticks != 0);
        let (id, health, name) = (mob.id, mob.health, mob.custom_name.clone());
        let at = [mob.position.x, mob.position.y + 0.5, mob.position.z];
        if !m::can_carry(&kind) {
            game.native_chat("Такого на носилках не унести");
            return;
        }
        if !asleep {
            game.native_chat("Сначала усыпите: шприц или дротик со снотворным");
            return;
        }
        // Геном и срезы — из данных моба (M11): с ними пациент и ляжет на стол.
        let genes = crate::game::biolab_core_data::genes(game, id);
        let flayed = crate::game::biolab_core_data::surgery(game, id) & m::FLAYED_BITS;
        let patient = m::Patient { kind, genes, health, flayed, name };
        // Моб уходит из мира без смерти и добычи: теперь он в носилках.
        // Вольер, державший его, сам освободит место, не найдя моба.
        let dimension = game.dim.id().to_string();
        if !game.native_remove_mob(&dimension, id) {
            return;
        }
        let text = m::patient_to_text(&patient);
        let slot = hand_with(game, m::STRETCHER_ID);
        if let Some(held) = game.native_carried_stack(slot)
            && &*held.item == m::STRETCHER_ID
        {
            use horizon_world::nbt::Tag;
            let data = held.extra.get_or_insert_with(Default::default);
            data.other.retain(|(key, _)| key != "minecraft:custom_data");
            data.other.push((
                "minecraft:custom_data".to_string(),
                Tag::compound([(m::PATIENT_KEY, Tag::String(text.as_str().into()))]),
            ));
            data.other.sort_by(|a, b| a.0.cmp(&b.0));
            // Подпись сразу, не дожидаясь тика инвентаря.
            data.custom_name = m::stretcher_label(&text).map(|label| Tag::String(label.as_str().into()));
        }
        play_at(game, m::Cue::Stretcher, at);
        game.native_chat(&format!("На носилках: {}", m::patient_title(&patient)));
        return;
    }

    // Скальпель: с мутанта на операционном столе — по конечности за клик.
    if &*stack.item == m::SCALPEL_ID {
        let Some((id, kind, health, at)) = game
            .mobs
            .iter()
            .find(|mob| mob.id as i32 == entity && mob.alive())
            .map(|mob| {
                (
                    mob.id,
                    m::full_kind(&mob.kind.id),
                    mob.health,
                    [mob.position.x, mob.position.y + 0.5, mob.position.z],
                )
            })
        else {
            return;
        };
        // Геном и операционная — данные моба (M11).
        let genes = crate::game::biolab_core_data::genes(game, id);
        let bits = crate::game::biolab_core_data::surgery(game, id);
        if bits & m::TABLE_BIT == 0 {
            // Обычный моб не на столе — клик ванилле: житель торгует и со
            // скальпелем в руке. Мутанту подсказываем, как быть.
            if !genes.is_empty() {
                *claimed = true;
                game.native_chat("Срезать можно только на операционном столе: усыпите и погрузите на носилки");
            }
            return;
        }
        *claimed = true;
        // Ткань — мутантная: из обычной плоти экстрактору нечего достать.
        if genes.is_empty() {
            game.native_chat("Мутаций нет: из обычной плоти ген не достать");
            return;
        }
        let mask = bits & m::FLAYED_BITS;
        let Some(cut) = m::next_cut(mask, m::cuts_of(&kind)) else {
            game.native_chat("Плоти не осталось");
            return;
        };
        // Срез — бит в данных операционной; окно получит его само и
        // нарисует кость.
        crate::game::biolab_core_data::set_surgery(game, id, true, mask | (1 << cut));
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
            mob.add_effect(m::BLEEDING, m::BLEEDING_TICKS, 0);
        }
        // Срез — всплеск крови сразу; дальше капает кровотечение (`ambient.rs`).
        let seed = u64::from(id) ^ game.ticks;
        puff_at(game, crate::game::biolab_particles::CUT, at, seed);
        // Урон — через помощник API 37: он сам отправит здоровье клиентам.
        let dimension = game.dim.id().to_string();
        game.native_set_mob_health(&dimension, id, m::after_cut(health));
        drop_tissue(game, at);
        game.native_chat(&m::cut_message(&kind, cut, 1, &genes));
        play_at(game, m::Cue::Tissue, at);
        return;
    }

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

    // Фрагмент ДНК: вживляем ген мобу — в данные `biolab:genome`. По ним
    // `biolab-deep` выдаёт прибавки, а окно рисует внешность.
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
        let target = game
            .mobs
            .iter()
            .find(|mob| mob.id as i32 == entity && mob.alive())
            .map(|mob| mob.id);
        if result == m::Splice::Done
            && let Some(id) = target
        {
            // Усиление того же гена заменяет его силу: строка данных держит
            // по гену одну, сильнейшую (`genome_data`).
            let mut genes = have.clone();
            genes.push((name, strength));
            crate::game::biolab_core_data::set_genes(game, id, &genes);
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
