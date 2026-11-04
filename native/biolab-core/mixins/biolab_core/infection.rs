// Тик игрока: болезнь идёт своим чередом, симптомы держатся сами.
fn biolab_core_player_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    use horizon_client::effects::EffectInstance;
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
    let Some(state) = m::player_tick(&world, uuid) else {
        return;
    };
    // Первый вход в мир: игрок получает «Журнал лаборанта» — карту
    // исследований. Что изучено, хранится у игрока, а не в книге.
    if state.ticks <= 2 && m::take_manual(&world, uuid) {
        let book = horizon_client::items::ItemStack::new(m::MANUAL_ID, 1);
        let mut placed = false;
        for slot in 0..36 {
            if let Some(place) = game.native_carried_slot(slot)
                && place.is_none()
            {
                *place = Some(book.clone());
                placed = true;
                break;
            }
        }
        if placed {
            game.native_chat("Выдан журнал лаборанта: откройте его, там карта исследований");
        }
    }

    // Стоим в луже: она действует и подсыхает.
    if state.ticks % m::FLUID_EVERY == 0
        && let Some(player) = game.native_player()
    {
        let at = [
            player.x.floor() as i32,
            player.y.floor() as i32,
            player.z.floor() as i32,
        ];
        let block = game.native_block(at);
        if let Some(fluid) = m::fluid_of_block(&block) {
            game.add_player_effect(EffectInstance::new(fluid.effect, 60, 0));
            // Лужа мутагена переписывает геном того, кто в ней стоит.
            if fluid.block == m::MUTAGEN_PUDDLE
                && let Some(name) =
                    m::mutagen_world_tick(&world, uuid, m::FLUID_EVERY as u32, state.ticks)
            {
                game.native_chat(&m::mutagen_message(name));
                play(game, m::Cue::Mutagen);
            }
            if fluid.infects > 0 {
                m::infect_world_player(&world, uuid, &m::WILD, fluid.infects);
            }
            // Лужа тратится: слой за срабатывание, последний высыхает.
            let layers = game
                .native_block_property(at, "layers")
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(1);
            match m::next_layers(layers) {
                0 => {
                    if let Some(air) = game.pack.assets.registry.default_state("minecraft:air") {
                        let dimension = player.dimension.clone();
                        game.native_set_block(
                            &dimension,
                            at,
                            air,
                            horizon_world::level::flags::UPDATE_ALL,
                        );
                    }
                }
                left => {
                    game.native_set_block_property(at, "layers", &left.to_string());
                }
            }
        }
    }

    // Кислотные железы мутанта жгут всех, кто подошёл близко. Геном — данные
    // на мобе (`biolab_core_data`): внутри обхода мобов их не прочитать,
    // поэтому сначала берём тех, кто достаточно близко для самой сильной
    // ауры, а гены спрашиваем уже у них.
    if state.ticks % m::FLUID_EVERY == 0
        && let Some(player) = game.native_player()
    {
        let reach = m::acid_radius(&[("acid_glands", 3)]).unwrap_or(0.0);
        let near: Vec<(u32, [f64; 3])> = game
            .mobs
            .iter()
            .filter(|mob| {
                mob.alive()
                    && (mob.position.x - player.x).abs() < reach
                    && (mob.position.y - player.y).abs() < reach
                    && (mob.position.z - player.z).abs() < reach
            })
            .map(|mob| (mob.id, [mob.position.x, mob.position.y, mob.position.z]))
            .collect();
        let burned = near.into_iter().any(|(id, at)| {
            let Some(radius) = m::acid_radius(&crate::game::biolab_core_data::genes(game, id)) else {
                return false;
            };
            (at[0] - player.x).abs() < radius
                && (at[1] - player.y).abs() < radius
                && (at[2] - player.z).abs() < radius
        });
        if burned {
            game.add_player_effect(EffectInstance::new(m::FLUIDS[0].effect, 60, 0));
        }
    }

    // Уродства игрока (GDD 4.19): гены без ванильного эффекта. Рогатость —
    // в ударе по мобу (`thorns.rs`), остальные — здесь. Мобы и предметы
    // меняем только там, где мир настоящий (свой сервер или выделенный), как
    // и помощники Horizon (`native_item_entities`): у клиента чужого сервера
    // это копии, и правка дала бы только дёрганье.
    let serving = game.serving.is_some() || game.options.dedicated.is_some();
    // Хвост: новый удар видно по скачку `hurt_time` (урон сбрасывает его на
    // 10, дальше тает). Кто именно ударил, хук не знает, поэтому хвост
    // хлещет всех враждебных или охотящихся на игрока, кто вплотную за спиной.
    let hurt_time = game.player.as_ref().map_or(0, |p| p.health.hurt.hurt_time);
    let fresh_hit = m::note_hurt_time(uuid, hurt_time);
    let tail = m::player_gene(&state, "tail");
    if serving
        && tail > 0
        && fresh_hit
        && let Some(player) = game.native_player()
    {
        use horizon_client_core::who::Who;
        let from = [player.x, player.y, player.z];
        for mob in game.mobs.iter_mut() {
            let (dx, dz) = (mob.position.x - player.x, mob.position.z - player.z);
            if !mob.alive()
                || (dx * dx + dz * dz).sqrt() > m::PLAYER_TAIL_REACH
                || (mob.position.y - player.y).abs() > 2.0
                || !(mob.hostile() || mob.ai.target == Some(Who::Player))
                || !m::behind(player.yaw, [player.x, player.z], [mob.position.x, mob.position.z])
            {
                continue;
            }
            mob.damage(
                m::player_tail_damage(tail),
                horizon_client::mobs::Damage::player(from),
            );
        }
    }
    // Щупальца: предметы с земли сами ползут к игроку. Только те, что уже
    // можно подобрать: брошенный игроком предмет (задержка подбора) не
    // возвращается ему в руки.
    let tentacles = m::player_gene(&state, "tentacles");
    if serving
        && tentacles > 0
        && let Some(player) = game.native_player()
    {
        let at = [player.x, player.y, player.z];
        for item in game.dropped.iter_mut() {
            if item.pickup_delay > 0 || item.stack.count == 0 {
                continue;
            }
            let spot = [item.position.x, item.position.y, item.position.z];
            if let Some(v) = m::magnet_velocity(spot, at, tentacles) {
                item.velocity.x = v[0];
                item.velocity.y = v[1];
                item.velocity.z = v[2];
            }
        }
    }
    // Дицефалия: вторая голова замечает мобов вокруг — они подсвечиваются.
    let heads = m::player_gene(&state, "dicephaly");
    if serving
        && heads > 0
        && state.ticks % 20 == 0
        && let Some(player) = game.native_player()
    {
        let radius = m::glow_radius(heads);
        for mob in game.mobs.iter_mut() {
            if mob.alive()
                && (mob.position.x - player.x).abs() < radius
                && (mob.position.y - player.y).abs() < radius
                && (mob.position.z - player.z).abs() < radius
            {
                mob.add_effect("glowing", 30, 0);
            }
        }
    }

    // Близкий контакт с заражённым мобом поднимает уровень.
    if state.ticks % m::INFECTION_EVERY == 0
        && let Some(player) = game.native_player()
    {
        let near = game.mobs.iter().any(|mob| {
            mob.species() == horizon_client::mobs::species::Species::Zombie
                && mob.alive()
                && (mob.position.x - player.x).abs() < m::CONTACT_RANGE
                && (mob.position.y - player.y).abs() < m::CONTACT_RANGE
                && (mob.position.z - player.z).abs() < m::CONTACT_RANGE
        });
        if near {
            m::infect_world_player(&world, uuid, &m::WILD, m::CONTACT_STEP);
        }
    }

    // Болезнь растёт не каждый тик, иначе сто процентов набегают за пять
    // секунд. Скорость задаёт штамм: заразный разгоняется вдвое быстрее.
    let strain = m::strain_or_wild(&state);
    if state.ticks % m::INFECTION_EVERY == 0 && state.infection > 0 {
        let next = (state.infection + m::strain_step(&strain)).min(100);
        m::set_infection(&world, uuid, next);
    }
    // Фармакокинетика: раз в секунду кровь всасывает дозу из депо, выводит
    // лишнее и решает, действует ли препарат. Этот же шаг сообщает клиенту
    // состояние — анализатору больше неоткуда его взять.
    if state.ticks % m::PHARMA_EVERY == 0
        && let Some((pharma, after)) = m::pharma_tick(&world, uuid)
    {
        for (name, amplifier) in &pharma.effects {
            game.add_player_effect(EffectInstance::new(
                name,
                m::PHARMA_EFFECT_TICKS,
                *amplifier,
            ));
        }
        if pharma.hardened {
            game.native_chat("Курс прерван: штамм стал устойчивее");
        }
        if after.infection != state.infection {
            // Лечение пенициллином: записываем новый уровень на диск.
            m::set_infection(&world, uuid, after.infection);
        }
        // Гены игрока: что они дают и чем он за это платит.
        for (name, amplifier) in m::gene_effects(&after) {
            game.add_player_effect(EffectInstance::new(
                name,
                m::PHARMA_EFFECT_TICKS,
                amplifier,
            ));
        }
        let journal = m::journal_count(&world);
        let _ = game.native_net_send_to_player(
            uuid,
            m::NET_INFECTION,
            &m::pharma_packet(&after, journal),
        );
        // Гены — в данные игрока (`biolab:player_genes`, M11), для наростов
        // на его модели (GDD 4.19). Данные с `sync` сами уходят ему и всем,
        // кто его видит; рисует `biolab-deep`. Своего пакета больше нет.
        crate::game::biolab_core_data::set_player_genes(game, uuid, &m::player_genes(&after));

        // Карта исследований: что игрок держит и что записано в журнале
        // зажигает узлы. Новое — в чат, вся маска — клиенту для экрана.
        let mut items = Vec::new();
        for slot in (0..36).chain([-106]) {
            if let Some(Some(stack)) = game.native_carried_slot(slot).map(|s| s.as_ref()) {
                items.push(stack.item.to_string());
            }
        }
        let entries = m::discoveries(&world);
        let facts = m::ResearchFacts {
            items: &items,
            discovered: entries.len() as u32,
            identified: entries.iter().filter(|e| e.identified).count() as u32,
        };
        let (mask, opened) = m::research_tick(&world, uuid, &facts);
        let mut fresh = false;
        for index in opened {
            if index != 0 {
                game.native_chat(&m::research_message(index));
                fresh = true;
            }
        }
        // Новый узел — короткий приятный звук, один на все открытые разом.
        // Играем его здесь, на сервере, где узел и открылся: клиент об этом
        // узнаёт только по маске и сравнивать её ему не с чем.
        if fresh {
            play(game, m::Cue::Research);
        }
        let _ = game.native_net_send_to_player(uuid, m::NET_RESEARCH, &m::research_packet(mask));
    }

    // Симптомы: подновляем короткими эффектами, чтобы они гасли сами, когда
    // игрок выздоровел.
    let stage = m::stage_with(state.infection, &strain);
    if stage != m::Stage::Clear {
        game.add_player_effect(EffectInstance::new(m::INFECTION_EFFECT, 40, 0));
    }
    for name in m::symptoms_with(stage, &strain) {
        game.add_player_effect(EffectInstance::new(name, 40, 0));
    }
}
