// Правый клик по блоку: ведро разливает лужу, ГМ-грядку сажают и собирают,
// а носилки укладывают пациента на операционный стол (GDD 4.17).
fn biolab_core_use_item_on(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face: [i32; 3],
    stack: &horizon_client::items::ItemStack,
    claimed: &mut bool,
) {
    use crate::game::biolab_core as m;
    /// Звук события мода в точке `at` всем игрокам рядом — как в `inject.rs`:
    /// очередь уровня (`native_play_sound`, API 25), без своего сервера звука
    /// нет. `with_level` — внутренний API игры: при обновлении Horizon
    /// проверить.
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

    // Операционный стол: носилки с пациентом укладывают его на стол. Моб
    // появляется над столом заново, как клон из чана (`machine.rs`): геном,
    // срезы и стол пишем в его данные (`biolab_core_data`, M11), прибавки по
    // генам выставит `biolab-deep`.
    if &*stack.item == m::STRETCHER_ID && game.native_block(pos) == m::OPERATING_TABLE {
        use horizon_world::nbt::Tag;
        *claimed = true;
        let text = stack.extra.as_ref().and_then(|extra| {
            extra
                .other
                .iter()
                .find(|(key, _)| key == "minecraft:custom_data")
                .and_then(|(_, data)| match data.get(m::PATIENT_KEY) {
                    Some(Tag::String(s)) => Some(s.to_string_lossy()),
                    _ => None,
                })
        });
        let Some(patient) = text.as_deref().and_then(m::patient_from_text) else {
            game.native_chat("Носилки пусты: ПКМ по усыплённому мобу — погрузить");
            return;
        };
        // Стол — весь ряд секций, в который входит та, по которой кликнули.
        let row = m::table_row(pos, |at| game.native_block(at) == m::OPERATING_TABLE);
        // Стороны секций — по соседям: ванилла гриба не открывает сторону
        // обратно, когда соседа сломали (основную работу делает хук
        // `table_shape.rs`, здесь — страховка). Запись без изменения
        // `native_set_block_property` пропускает сама.
        for section in row.sections() {
            let sides = m::table_sides(section, |at| game.native_block(at) == m::OPERATING_TABLE);
            for (key, value) in sides {
                game.native_set_block_property(section, key, value);
            }
        }
        // Длина пациента — по хитбоксу вида, как его знает Horizon
        // (`MobType` из `entity-types.json`), и по генам. Незнакомый вид не
        // появился бы и сам (`native_spawn_mob`), стол его не меряет.
        let need = game
            .res
            .mob_types()
            .get(&patient.kind)
            .map_or(1, |t| m::table_sections(&patient.kind, t.width, t.height, &patient.genes));
        if row.len < need {
            // Носилки остаются занятыми: можно доставить секции и повторить.
            game.native_chat(&m::table_too_short(need, row.len));
            return;
        }
        if row
            .sections()
            .into_iter()
            .any(|s| !m::is_air(&game.native_block([s[0], s[1] + 1, s[2]])))
        {
            game.native_chat("Над столом нет места");
            return;
        }
        // Пациент ложится по центру ряда, вдоль него, головой к изголовью.
        let spot = row.center();
        let yaw = row.yaw();
        // Один стол — один пациент: на любой секции ряда. Кто на столе —
        // данные моба; читаем их у тех, кто над рядом.
        let over: Vec<u32> = game
            .mobs
            .iter()
            .filter(|mob| mob.alive() && row.covers([mob.position.x, mob.position.y, mob.position.z]))
            .map(|mob| mob.id)
            .collect();
        let busy = over
            .into_iter()
            .any(|id| crate::game::biolab_core_data::surgery(game, id) & m::TABLE_BIT != 0);
        if busy {
            game.native_chat("Стол занят: сначала снимите пациента");
            return;
        }
        let dimension = game.dim.id().to_string();
        let options = crate::game::native_mobs::NativeSpawn {
            name: patient.name.clone(),
            health: Some(patient.health),
            ..Default::default()
        };
        // Не вышло — пациент остаётся на носилках. Чаще всего это мирная
        // сложность: враждебный моб на ней сразу исчез бы.
        let Some(id) = game.native_spawn_mob(&dimension, &patient.kind, spot, &options) else {
            game.native_chat("Не удалось уложить: на мирной сложности враждебные не появляются");
            return;
        };
        // Свежему мобу игра успевает бросить случайный геном (`joined.rs` в
        // biolab-deep) — данные пациента его просто перезаписывают. Прибавок
        // бросок не ставит (их выставит проход мутантов по итоговому
        // геному), так что снимать нечего.
        crate::game::biolab_core_data::set_genes(game, id, &patient.genes);
        crate::game::biolab_core_data::set_surgery(game, id, true, patient.flayed);
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
            // Пристёгнут и спит с первого тика: иначе зомби успел бы ударить.
            mob.add_effect(m::SEDATION_EFFECT, m::TABLE_SEDATION_TICKS, 0);
            mob.ai.target = None;
            // Вдоль ряда: отрисовка «лёжа» кладёт моба по направлению взгляда
            // (тело — по `body_yaw`, голова — по `head_yaw`), а клиенту поворот
            // уходит пакетами сущности. Прошлые значения — те же, чтобы с
            // первого кадра не было разворота.
            mob.yaw = yaw;
            mob.previous_yaw = yaw;
            mob.body_yaw = yaw;
            mob.previous_body_yaw = yaw;
            mob.head_yaw = yaw;
            mob.previous_head_yaw = yaw;
        }
        // Максимум здоровья по генам вырастет в тике моба, и `biolab-deep`
        // дольёт прибавку: пару тиков держим здоровье тем, что было.
        m::settle_health(id, patient.health, game.ticks + m::HEALTH_SETTLE_TICKS);
        // Носилки пустеют: данные и подпись уходят вместе.
        let main = game.selected as i32;
        let slot = if game
            .native_carried_stack(main)
            .is_some_and(|held| &*held.item == m::STRETCHER_ID)
        {
            main
        } else {
            -106
        };
        if let Some(held) = game.native_carried_stack(slot)
            && &*held.item == m::STRETCHER_ID
        {
            held.extra = None;
        }
        play_at(game, m::Cue::Strap, spot);
        game.native_chat(&format!(
            "На столе: {}. Скальпель — ПКМ по пациенту",
            m::patient_title(&patient)
        ));
        return;
    }
    // Грядка: фрагментом ДНК или урожаем сажают, пустой рукой собирают.
    if game.native_block(pos) == m::GM_PLOT {
        let planted = game.native_with_mod_block_entity(pos, |entity| {
            match entity.nbt.get(&m::PLOT_GENOME_KEY.into()) {
                Some(horizon_world::nbt::Tag::String(s)) => s.to_string_lossy(),
                _ => String::new(),
            }
        });
        let planted = planted.unwrap_or_default();

        // Пустая грядка принимает ген из фрагмента или из урожая.
        if planted.is_empty() {
            let carried = stack.extra.as_ref().and_then(|extra| {
                extra
                    .other
                    .iter()
                    .find(|(key, _)| key == "minecraft:custom_data")
                    .and_then(|(_, data)| {
                        let key = if &*stack.item == m::FRAGMENT_ID {
                            m::GENE_KEY
                        } else {
                            m::PLOT_GENOME_KEY
                        };
                        match data.get(key) {
                            Some(horizon_world::nbt::Tag::String(s)) => Some(s.to_string_lossy()),
                            _ => None,
                        }
                    })
            });
            let Some(text) = carried.filter(|t| !m::genome_from_text(t).is_empty()) else {
                return;
            };
            game.native_with_mod_block_entity(pos, |entity| {
                entity.nbt.insert(
                    m::PLOT_GENOME_KEY.into(),
                    horizon_world::nbt::Tag::String(text.as_str().into()),
                );
                entity.data.clear();
            });
            game.native_set_block_property(pos, "layers", "1");
            let slot = game.selected as i32;
            if let Some(held) = game.native_carried_stack(slot)
                && &*held.item == &*stack.item
            {
                held.count = held.count.saturating_sub(1);
            }
            game.native_chat(&format!("Посажено: {}", m::crop_label(&m::genome_from_text(&text))));
            *claimed = true;
        }
        return;
    }

    let Some(fluid) = m::fluid_of_bucket(&stack.item) else {
        return;
    };
    let at = [pos[0] + face[0], pos[1] + face[1], pos[2] + face[2]];
    if game.native_block(at) != "minecraft:air" {
        return;
    }
    let Some(state) = game.pack.assets.registry.default_state(fluid.block) else {
        return;
    };
    let Some(player) = game.native_player() else {
        return;
    };
    let dimension = player.dimension.clone();
    if game.native_set_block(&dimension, at, state, horizon_world::level::flags::UPDATE_ALL) {
        game.native_set_block_property(at, "layers", &m::FLUID_LAYERS.to_string());
        *claimed = true;
        // Вылитое ведро пустеет, как ванильное: в выживании в руке остаётся
        // `minecraft:bucket`, в творческом — то же ведро. Режим — флаг
        // `creative` игры: по нему ванилла не тратит предметы (`consume_held`)
        // и не меняет ведро (`SolidBucketItem.useOn`); у гостя LAN в его ход
        // флаг подменён на гостевой. Руку ищем как соседний код грядки:
        // основная, если в ней это ведро, иначе левая (-106).
        if let Some(empty) = m::emptied_bucket(&stack.item, game.creative) {
            let main = game.selected as i32;
            let slot = if game
                .native_carried_stack(main)
                .is_some_and(|held| &*held.item == &*stack.item)
            {
                main
            } else {
                -106
            };
            if let Some(held) = game.native_carried_stack(slot)
                && &*held.item == &*stack.item
            {
                held.item = empty.into();
                held.count = 1;
                held.damage = 0;
                held.extra = None;
            }
        }
    }
}
