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
    // появляется над столом заново, как клон из чана (`machine.rs`): метки
    // генов, срезов и стола вешаем сами, прибавки по генам выставит
    // `biolab-deep`.
    if &*stack.item == m::STRETCHER_ID && game.native_block(pos) == m::OPERATING_TABLE {
        use horizon_client::mobs::finalize::{Attr, Op, add_modifier, has_modifier, remove_modifier};
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
        if !m::is_air(&game.native_block([pos[0], pos[1] + 1, pos[2]])) {
            game.native_chat("Над столом нет места");
            return;
        }
        let spot = m::table_spot(pos);
        // Один стол — один пациент.
        let busy = game.mobs.iter().any(|mob| {
            mob.alive()
                && has_modifier(mob, Attr::FollowRange, m::SURGERY_TABLE)
                && (mob.position.x - spot[0]).abs() < 1.0
                && (mob.position.y - spot[1]).abs() < 1.0
                && (mob.position.z - spot[2]).abs() < 1.0
        });
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
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
            // Свежему мобу игра успевает бросить случайный геном (`joined.rs`
            // в biolab-deep) — снимаем его и его прибавки, как чан.
            for old in m::GENE_MARKERS {
                remove_modifier(mob, Attr::FollowRange, old);
            }
            for (attr, name) in [
                (Attr::MaxHealth, m::STAT_MODIFIERS[0]),
                (Attr::MovementSpeed, m::STAT_MODIFIERS[1]),
                (Attr::AttackDamage, m::STAT_MODIFIERS[2]),
                (Attr::Armor, m::STAT_MODIFIERS[3]),
                (Attr::KnockbackResistance, m::STAT_MODIFIERS[4]),
            ] {
                remove_modifier(mob, attr, name);
            }
            for (name, strength) in &patient.genes {
                if let Some(marker) = m::marker_for(name, *strength) {
                    add_modifier(mob, Attr::FollowRange, marker, 0.0, Op::AddValue);
                }
            }
            // `into_iter`: метке нужна `&'static str`, а не ссылка на неё.
            for (i, marker) in m::SURGERY_FLAYED.into_iter().enumerate() {
                if patient.flayed & (1 << i) != 0 {
                    add_modifier(mob, Attr::FollowRange, marker, 0.0, Op::AddValue);
                }
            }
            add_modifier(mob, Attr::FollowRange, m::SURGERY_TABLE, 0.0, Op::AddValue);
            // Пристёгнут и спит с первого тика: иначе зомби успел бы ударить.
            mob.add_effect(m::SEDATION_EFFECT, m::TABLE_SEDATION_TICKS, 0);
            mob.ai.target = None;
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
