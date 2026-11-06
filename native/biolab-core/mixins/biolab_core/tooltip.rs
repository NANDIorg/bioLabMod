// Подсказка на предмете мода. Сначала строки данных — что в колбе, какой ген
// во фрагменте, какой штамм в культуре, что даст урожай, — под ними одна-две
// строки «что делает / как применить» из таблицы `ITEM_HINTS` (GDD 4.12).
fn biolab_core_tooltip(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    lines: &mut Vec<(horizon_client::text::Text, u32)>,
) {
    use crate::game::biolab_core as m;

    /// Строки данных предмета: то, что лежит в его `custom_data`.
    fn data_lines(
        game: &crate::game::Game<'_>,
        stack: &horizon_client::items::ItemStack,
        lines: &mut Vec<(horizon_client::text::Text, u32)>,
    ) {
        use crate::game::biolab_core as m;
        use horizon_world::nbt::Tag;
        // Оружие для дротиков (GDD 4.20): чем заряжен арбалет, магазин и газ ружья.
        if m::dart_weapon(&stack.item).is_some() {
            for line in m::weapon_tooltip(&stack.item, &crate::game::biolab_weapons::state_of(stack)) {
                lines.push((horizon_client::text::Text::literal(&line), 0xAA_AAAA));
            }
            return;
        }
        // Носилки: кто на них, здоровье, геном и срезы. Пустые — «Пусто».
        if &*stack.item == m::STRETCHER_ID {
            let text = stack
                .extra
                .as_ref()
                .and_then(|extra| {
                    extra
                        .other
                        .iter()
                        .find(|(key, _)| key == "minecraft:custom_data")
                        .and_then(|(_, data)| match data.get(m::PATIENT_KEY) {
                            Some(Tag::String(s)) => Some(s.to_string_lossy()),
                            _ => None,
                        })
                })
                .unwrap_or_default();
            for line in m::stretcher_tooltip(&text) {
                lines.push((horizon_client::text::Text::literal(&line), 0xAA_AAAA));
            }
            return;
        }
        // Фрагмент ДНК: что за ген внутри и чего он стоит.
        if &*stack.item == m::FRAGMENT_ID {
            let text = stack
                .extra
                .as_ref()
                .and_then(|extra| {
                    extra
                        .other
                        .iter()
                        .find(|(key, _)| key == "minecraft:custom_data")
                        .and_then(|(_, data)| match data.get(m::GENE_KEY) {
                            Some(Tag::String(s)) => Some(s.to_string_lossy()),
                            _ => None,
                        })
                })
                .unwrap_or_default();
            for line in m::fragment_tooltip(&text) {
                lines.push((horizon_client::text::Text::literal(&line), 0xAA_AAAA));
            }
            return;
        }
        // Векторы секвенатора: гены и цена, или какой ген удалит.
        if &*stack.item == m::GENE_VECTOR || &*stack.item == m::ANTISENSE_VECTOR {
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
                            Some(Tag::String(s)) => Some(s.to_string_lossy()),
                            _ => None,
                        })
                })
                .unwrap_or_default();
            let found = if antisense { m::antisense_tooltip(&text) } else { m::vector_tooltip(&text) };
            for line in found {
                lines.push((horizon_client::text::Text::literal(&line), 0xAA_AAAA));
            }
            return;
        }
        // Культура: характеристики штамма.
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
                            Some(Tag::String(s)) => Some(s.to_string_lossy()),
                            _ => None,
                        })
                })
                .unwrap_or_default();
            match m::strain_from_text(&text) {
                Some(strain) => {
                    for line in m::strain_lines(&strain) {
                        lines.push((horizon_client::text::Text::literal(&line), 0xAA_AAAA));
                    }
                }
                None => lines.push((
                    horizon_client::text::Text::literal("Штамм не определён"),
                    0xAA_AAAA,
                )),
            }
            return;
        }

        // Урожай: что даст его геном.
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
                            Some(Tag::String(s)) => Some(s.to_string_lossy()),
                            _ => None,
                        })
                })
                .unwrap_or_default();
            for line in m::crop_tooltip(&text) {
                lines.push((horizon_client::text::Text::literal(&line), 0xAA_AAAA));
            }
            return;
        }
        if &*stack.item != m::FLASK_ID {
            return;
        }
        let Some(extra) = stack.extra.as_ref() else {
            return;
        };
        let Some((_, data)) = extra
            .other
            .iter()
            .find(|(key, _)| key == "minecraft:custom_data")
        else {
            return;
        };
        let contents = match data.get(m::CONTENTS_KEY) {
            Some(Tag::String(s)) => s.to_string_lossy(),
            _ => return,
        };
        let ml = match data.get(m::ML_KEY) {
            Some(Tag::Int(v)) => *v,
            _ => 0,
        };
        // Имя, которое вещество получило в журнале этого мира, главнее формулы.
        let world = game.world_root.clone();
        if let Some(name) = m::compound_display(&world, &contents) {
            lines.push((horizon_client::text::Text::literal(&name), 0xAA_AAAA));
        }
        for line in m::flask_tooltip(&contents, ml) {
            lines.push((horizon_client::text::Text::literal(&line), 0xAA_AAAA));
        }
    }

    if !game.native_on(m::ID) {
        return;
    }
    data_lines(game, stack, lines);
    for line in m::item_hint(&*stack.item) {
        lines.push((horizon_client::text::Text::literal(&line), m::HINT_INK));
    }
}
