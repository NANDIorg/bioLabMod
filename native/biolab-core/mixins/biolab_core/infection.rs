// Тик игрока: болезнь идёт своим чередом, симптомы держатся сами.
fn biolab_core_player_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    use horizon_client::effects::EffectInstance;
    if !game.native_on(m::ID) {
        return;
    }
    let uuid = game.native_acting_id();
    let world = game.world_root.clone();
    let Some(state) = m::player_tick(&world, uuid) else {
        return;
    };
    // Первый вход в мир: игрок получает «Журнал лаборанта». Страницы берём
    // из данных мода, чтобы правка текста не требовала пересборки.
    if state.ticks <= 2 && m::take_manual(&world, uuid) {
        let text = game
            .native_data(m::MANUAL_PATH)
            .and_then(|bytes| String::from_utf8(bytes).ok());
        let pages = match text.as_deref() {
            Some(text) => m::manual_pages(text),
            None => m::manual_fallback(),
        };
        let mut book = horizon_client::items::ItemStack {
            item: m::MANUAL_ID.into(),
            count: 1,
            damage: 0,
            extra: None,
        };
        let data = book.extra.get_or_insert_with(Default::default);
        data.written = Some(horizon_client::item_data::WrittenBook {
            title: m::MANUAL_TITLE.to_string(),
            author: m::MANUAL_AUTHOR.to_string(),
            generation: 0,
            pages,
            page_tags: Vec::new(),
        });
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
            game.native_chat("Выдан журнал лаборанта: прочтите, с чего начать");
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

    // Кислотные железы мутанта жгут всех, кто подошёл близко.
    if state.ticks % m::FLUID_EVERY == 0
        && let Some(player) = game.native_player()
    {
        use horizon_client::mobs::finalize::{Attr, has_modifier};
        let burned = game.mobs.iter().any(|mob| {
            if !mob.alive() {
                return false;
            }
            m::GENE_MARKERS.iter().any(|marker| {
                let Some(radius) = m::aura_radius(marker) else {
                    return false;
                };
                has_modifier(mob, Attr::FollowRange, marker)
                    && (mob.position.x - player.x).abs() < radius
                    && (mob.position.y - player.y).abs() < radius
                    && (mob.position.z - player.z).abs() < radius
            })
        });
        if burned {
            game.add_player_effect(EffectInstance::new(m::FLUIDS[0].effect, 60, 0));
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
