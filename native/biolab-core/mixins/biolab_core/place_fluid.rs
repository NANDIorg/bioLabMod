// Правый клик по блоку: ведро разливает лужу, а ГМ-грядку сажают и собирают.
fn biolab_core_use_item_on(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face: [i32; 3],
    stack: &horizon_client::items::ItemStack,
    claimed: &mut bool,
) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) {
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
    }
}
