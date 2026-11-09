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
    fn drop_tissue(game: &mut crate::game::Game<'_>, at: [f64; 3], genes: &[(&'static str, i32)]) {
        use crate::game::biolab_core as m;
        if game.remote.is_some() {
            return;
        }
        let mut tissue = horizon_client::items::ItemStack {
            item: m::TISSUE_ID.into(),
            count: 1,
            damage: 0,
            extra: None,
        };
        // M10 (GDD 4.27 В): ткань несёт гены пациента — по ним её берёт
        // электроцитная батарея, их называет подсказка.
        if !genes.is_empty() {
            crate::game::biolab_suit::set_custom_text(&mut tissue, m::TISSUE_GENOME_KEY, Some(&m::genome_to_text(genes)));
        }
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
            crate::game::biolab_say::say(game, "Носилки заняты: ПКМ по операционному столу — уложить");
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
        let has = |effect: &str| mob.extra.effects.iter().any(|(id, ticks, _)| *id == effect && *ticks != 0);
        let sleeping = has(m::SEDATION_EFFECT);
        // Парализованного кураре грузить можно — на стол нельзя (GDD 4.20).
        let paralysed = !sleeping && has(m::PARALYSIS_EFFECT);
        let asleep = sleeping || paralysed;
        let (id, health, name) = (mob.id, mob.health, mob.custom_name.clone());
        let at = [mob.position.x, mob.position.y + 0.5, mob.position.z];
        if !m::can_carry(&kind) {
            crate::game::biolab_say::say(game, "Такого на носилках не унести");
            return;
        }
        if !asleep {
            crate::game::biolab_say::say(game, "Сначала усыпите: шприц или дротик со снотворным");
            return;
        }
        // Геном и срезы — из данных моба (M11): с ними пациент и ляжет на стол.
        let genes = crate::game::biolab_core_data::genes(game, id);
        let flayed = crate::game::biolab_core_data::surgery(game, id) & m::FLAYED_BITS;
        // Чем усыплён — едет с пациентом: стол подновит это вещество.
        let sleeper = m::sleep_book().lock().ok().and_then(|book| book.slept_with(id));
        let patient = m::Patient { kind, genes, health, flayed, name, sleeper, paralysed };
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
        crate::game::biolab_say::say(game, &format!("На носилках: {}", m::patient_title(&patient)));
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
                crate::game::biolab_say::say(game, "Срезать можно только на операционном столе: усыпите и погрузите на носилки");
            }
            return;
        }
        *claimed = true;
        // Ткань — мутантная: из обычной плоти экстрактору нечего достать.
        if genes.is_empty() {
            crate::game::biolab_say::say(game, "Мутаций нет: из обычной плоти ген не достать");
            return;
        }
        let mask = bits & m::FLAYED_BITS;
        let Some(cut) = m::next_cut(mask, m::cuts_of(&kind)) else {
            crate::game::biolab_say::say(game, "Плоти не осталось");
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
        drop_tissue(game, at, &genes);
        crate::game::biolab_say::say(game, &m::cut_message(&kind, cut, 1, &genes));
        play_at(game, m::Cue::Tissue, at);
        return;
    }

    // Микроскоп: читаем геном моба по меткам и показываем игроку.
    if &*stack.item == m::MICROSCOPE_ID {
        let genes = read_genome(game, entity);
        for line in m::genome_lines(&genes) {
            crate::game::biolab_say::say(game, &line);
        }
        // Известные пары в этом геноме (GDD 4.23 п. 5) и срок предохранителей.
        let world = game.world_root.clone();
        let known = m::known_pairs(&world);
        for line in m::genome_pair_lines(&genes, known) {
            crate::game::biolab_say::say(game, &line);
        }
        if let Ok(id) = u32::try_from(entity) {
            let clock = crate::game::biolab_core_data::clock(game, id);
            if let Some(left) = m::inductor_left(&genes, &clock, game.world.game_time) {
                crate::game::biolab_say::say(
                    game,
                    &format!("  Генный выключатель: до дозы {left:.1} дн."),
                );
            }
        }
        m::remember_injection(game.ticks);
        *claimed = true;
        return;
    }

    // Фрагмент ДНК: вживляем ген мобу — в данные `biolab:genome`. По ним
    // `biolab-deep` выдаёт прибавки, а окно рисует внешность.
    if &*stack.item == m::FRAGMENT_ID {
        let Some(text) = gene_in(stack) else {
            crate::game::biolab_say::say(game, "Фрагмент пустой: ген не прочитан");
            return;
        };
        let Some((name, strength)) = m::gene_from_text(&text) else {
            return;
        };
        let have = read_genome(game, entity);
        // Сочетания генов (GDD 4.23 п. 5): пары складывает `combine`, и она
        // же решает, выживет ли образец.
        let seed = (entity as u64) ^ game.ticks;
        let (results, combined) = m::splice_mob_paired(&have, &[(name, strength)], seed);
        let result = results.first().copied().unwrap_or(m::Splice::Already);
        // Клик по мобу ловят оба хука: без этой отметки фрагмент достанется
        // ещё и самому игроку (проверено в игре — ген вживлялся обоим).
        m::remember_injection(game.ticks);
        let target = game
            .mobs
            .iter()
            .find(|mob| mob.id as i32 == entity && mob.alive())
            .map(|mob| mob.id);
        if result == m::Splice::Done
            && combined.lethal.is_none()
            && let Some(id) = target
        {
            // Геном пишется целиком: пары могли ослабить ген или дорастить
            // лишний (`combine`), а строка данных держит по гену одну силу.
            crate::game::biolab_core_data::set_genes(game, id, &combined.genes);
            let slot = game.selected as i32;
            if let Some(carried) = game.native_carried_stack(slot)
                && &*carried.item == m::FRAGMENT_ID
            {
                carried.count = carried.count.saturating_sub(1);
            }
        }
        crate::game::biolab_say::say(game, &m::splice_message(result, name, strength));
        // Что сделали пары и какие из них стали известны лаборатории.
        let world = game.world_root.clone();
        for line in m::learn_pairs(&world, &combined) {
            crate::game::biolab_say::say(game, &line);
        }
        // Несовместимость: образец не выживает. Фрагмент всё равно потрачен —
        // опыт сделан, и он идёт в очки знания (GDD 4.23 п. 7).
        if let Some(pair) = combined.lethal {
            crate::game::biolab_say::say(game, &m::lethal_mob_message(pair));
            if let Some(id) = target {
                let dimension = game.dim.id().to_string();
                let at = game
                    .mobs
                    .iter()
                    .find(|mob| mob.id == id)
                    .map(|mob| [mob.position.x, mob.position.y + 0.5, mob.position.z])
                    .unwrap_or_default();
                game.native_kill_mob(&dimension, id);
                m::note_lab_event(&dimension, at, "mutant_died", &m::pair_title(pair));
            }
            let slot = game.selected as i32;
            if let Some(carried) = game.native_carried_stack(slot)
                && &*carried.item == m::FRAGMENT_ID
            {
                carried.count = carried.count.saturating_sub(1);
            }
            *claimed = true;
            return;
        }
        // M9 (GDD 4.24): ген рук корове или ног слизню прижился, но спит.
        if result == m::Splice::Done
            && let Some(note) = target
                .and_then(|id| game.mobs.iter().find(|mob| mob.id == id))
                .and_then(|mob| m::proportion_note(&mob.kind.id, name))
        {
            crate::game::biolab_say::say(game, note);
        }
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

    // Векторы секвенатора (GDD 4.18, п. 3): генный — все гены одним уколом,
    // антисмысловой — удаляет ген. Пишут те же данные `biolab:genome`, что и
    // фрагмент: прибавки по новому геному (и снятие лишних) ставит проход
    // мутантов `biolab-deep` (`apply_genome`).
    if &*stack.item == m::GENE_VECTOR || &*stack.item == m::ANTISENSE_VECTOR {
        *claimed = true;
        // Клик по мобу ловят оба хука: без отметки вектор достался бы ещё и
        // самому игроку (`remedy.rs`).
        m::remember_injection(game.ticks);
        let antisense = &*stack.item == m::ANTISENSE_VECTOR;
        let key = if antisense { m::ANTISENSE_KEY } else { m::VECTOR_KEY };
        let text = stack
            .extra
            .as_ref()
            .and_then(|extra| {
                extra
                    .other
                    .iter()
                    .find(|(k, _)| k == "minecraft:custom_data")
                    .and_then(|(_, data)| match data.get(key) {
                        Some(horizon_world::nbt::Tag::String(s)) => Some(s.to_string_lossy()),
                        _ => None,
                    })
            })
            .unwrap_or_default();
        let Some((id, kind, at)) = game
            .mobs
            .iter()
            .find(|mob| mob.id as i32 == entity && mob.alive())
            .map(|mob| (mob.id, mob.kind.id.to_string(), [mob.position.x, mob.position.y + 0.9, mob.position.z]))
        else {
            return;
        };
        let have = crate::game::biolab_core_data::genes(game, id);
        // Сочетания генов (GDD 4.23 п. 5): вектор кладётся теми же парами,
        // что и фрагмент, — все его гены приходят разом.
        let mut pairs: Option<m::Combined> = None;
        let (genome, message) = if antisense {
            match m::gene(&text) {
                Some(g) => {
                    let after = m::remove_gene(&have, g.name);
                    (after.clone(), m::antisense_message(g.name, after.is_some()))
                }
                None => (None, "Вектор пустой: ген не прочитан".to_string()),
            }
        } else {
            let genes = m::vector_genes(&text);
            if genes.is_empty() {
                (None, "Вектор пустой: генов нет".to_string())
            } else {
                let seed = u64::from(id) ^ game.ticks;
                let (results, combined) = m::splice_mob_paired(&have, &genes, seed);
                let done = results.contains(&m::Splice::Done) && combined.lethal.is_none();
                let after = combined.genes.clone();
                pairs = Some(combined);
                (done.then_some(after), m::vector_message(&genes, &results))
            }
        };
        crate::game::biolab_say::say(game, &message);
        let world = game.world_root.clone();
        if let Some(combined) = &pairs {
            for line in m::learn_pairs(&world, combined) {
                crate::game::biolab_say::say(game, &line);
            }
            if let Some(pair) = combined.lethal {
                crate::game::biolab_say::say(game, &m::lethal_mob_message(pair));
                let dimension = game.dim.id().to_string();
                game.native_kill_mob(&dimension, id);
                m::note_lab_event(&dimension, at, "mutant_died", &m::pair_title(pair));
                let slot = game.selected as i32;
                if let Some(carried) = game.native_carried_stack(slot)
                    && &*carried.item == &*stack.item
                {
                    carried.count = carried.count.saturating_sub(1);
                }
                return;
            }
        }
        let Some(genome) = genome else {
            return;
        };
        crate::game::biolab_core_data::set_genes(game, id, &genome);
        let slot = game.selected as i32;
        if let Some(carried) = game.native_carried_stack(slot)
            && &*carried.item == &*stack.item
        {
            carried.count = carried.count.saturating_sub(1);
        }
        let world = game.world_root.clone();
        m::record_trial(&world, &m::trial_record(&stack.item, &kind));
        play_at(game, m::Cue::GeneSplice, at);
        return;
    }

    /// Препарат в руке потрачен: от шприца остаётся пустой, от отвара —
    /// бутылка. Если это был последний в стаке, кладём остаток прямо в слот:
    /// выдача в инвентарь из этого хука не доходит (проверено в игре —
    /// шприц пропадал совсем).
    fn use_up(game: &mut crate::game::Game<'_>, item: &str) {
        use crate::game::biolab_core as m;
        let slot = game.selected as i32;
        let leftover = m::after_use(item);
        let mut give_back = false;
        if let Some(carried) = game.native_carried_stack(slot)
            && &*carried.item == item
        {
            carried.count = carried.count.saturating_sub(1);
            match (carried.count, leftover) {
                (0, Some(left)) => {
                    carried.item = left.into();
                    carried.count = 1;
                    carried.damage = 0;
                    carried.extra = None;
                }
                (_, Some(_)) => give_back = true,
                _ => {}
            }
        }
        if give_back && let Some(left) = leftover {
            let _ = game.native_give(left, 1);
        }
    }

    // Индуктор (GDD 4.23 п. 10): доза для генного выключателя. Срок идёт от
    // последней дозы, и укол пускает его заново (`safeguard_verdict`).
    if &*stack.item == m::INDUCTOR_SYRINGE {
        let Some((id, kind, at)) = game
            .mobs
            .iter()
            .find(|mob| mob.id as i32 == entity && mob.alive())
            .map(|mob| (mob.id, mob.kind.id.to_string(), [mob.position.x, mob.position.y + 0.9, mob.position.z]))
        else {
            return;
        };
        *claimed = true;
        // Клик по мобу ловят оба хука: без отметки шприц достался бы ещё и
        // самому игроку (`remedy.rs`).
        m::remember_injection(game.ticks);
        let genes = crate::game::biolab_core_data::genes(game, id);
        let (message, dosed) = m::inductor_message(&genes);
        crate::game::biolab_say::say(game, &message);
        if dosed {
            let now = game.world.game_time;
            let mut clock = crate::game::biolab_core_data::clock(game, id);
            if clock.set == 0 {
                clock.set = now;
            }
            if clock.host.is_empty() {
                clock.host = m::full_kind(&kind);
            }
            clock.dose = now;
            crate::game::biolab_core_data::set_clock(game, id, &clock);
            play_at(game, m::Cue::SyringeMob, at);
            let world = game.world_root.clone();
            m::record_trial(&world, &m::trial_record(&stack.item, &kind));
            use_up(game, &stack.item);
        }
        return;
    }

    // Снотворное (GDD 4.20, M5): шприц, отвар и тряпка с хлороформом —
    // исход по силе вещества против массы моба (`biolab_sleep::dose`).
    if let Some(sleeper) = m::sleeper_of_item(&stack.item) {
        let Some((id, kind, at)) = game
            .mobs
            .iter()
            .find(|mob| mob.id as i32 == entity && mob.alive())
            .map(|mob| (mob.id, mob.kind.id.to_string(), [mob.position.x, mob.position.y + 0.9, mob.position.z]))
        else {
            return;
        };
        *claimed = true;
        // Клик по мобу ловят оба хука: без отметки отвар достался бы ещё и
        // самому игроку (`remedy.rs`).
        m::remember_injection(game.ticks);
        if &*stack.item == m::CHLOROFORM_RAG && !crate::game::biolab_sleep::within_reach(game, id, m::RAG_REACH) {
            crate::game::biolab_say::say(game, "Тряпку прижимают вплотную: подойдите ближе");
            return;
        }
        crate::game::biolab_sleep::dose(game, id, sleeper, m::via_of_item(&stack.item), true);
        play_at(game, m::Cue::SyringeMob, at);
        let world = game.world_root.clone();
        m::record_trial(&world, &m::trial_record(&stack.item, &kind));
        use_up(game, &stack.item);
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
    crate::game::biolab_say::say(game, &m::injection_message(&stack.item, &name));
    play_at(game, m::mob_injection_cue(&stack.item), at);
    m::remember_injection(game.ticks);
    // Опыт уходит в журнал: пульт потом покажет, кому что вводили.
    let world = game.world_root.clone();
    m::record_trial(&world, &m::trial_record(&stack.item, &name));

    // Шприц тратится, пустой остаётся в руке (`use_up`).
    use_up(game, &stack.item);
    *claimed = true;
}
