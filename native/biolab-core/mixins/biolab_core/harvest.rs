// Правый клик по блоку мода: пульт даёт отчёт, спелая грядка — урожай. Этот хук ловит и пустую
// руку, в отличие от `native_use_item_on`, которому нужен предмет.
fn biolab_core_right_click_block(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    canceled: &mut bool,
) {
    use crate::game::biolab_core as m;
    use horizon_world::nbt::Tag;
    if !game.native_on(m::ID) {
        return;
    }

    // Пульт наблюдения: кто в вольерах, чем болен и что ему вводили.
    if game.native_block(pos) == m::CONSOLE {
        use horizon_client::mobs::finalize::{Attr, has_modifier};
        let centre = [
            f64::from(pos[0]) + 0.5,
            f64::from(pos[1]) + 0.5,
            f64::from(pos[2]) + 0.5,
        ];
        let subjects: Vec<(String, f32, f32, Vec<(&'static str, i32)>, bool)> = game
            .mobs
            .iter()
            .filter(|mob| {
                mob.alive()
                    && (mob.position.x - centre[0]).abs() < m::CONSOLE_RANGE
                    && (mob.position.y - centre[1]).abs() < m::CONSOLE_RANGE
                    && (mob.position.z - centre[2]).abs() < m::CONSOLE_RANGE
            })
            .map(|mob| {
                let genes = m::GENE_MARKERS
                    .iter()
                    .filter(|marker| has_modifier(mob, Attr::FollowRange, marker))
                    .filter_map(|marker| m::gene_of_marker(marker))
                    .collect();
                let sick = mob
                    .extra
                    .effects
                    .iter()
                    .any(|(id, ticks, _)| &**id == m::INFECTION_EFFECT && *ticks != 0);
                (
                    mob.kind.id.to_string(),
                    mob.health,
                    mob.attrs.max_health,
                    genes,
                    sick,
                )
            })
            .collect();
        let world = game.world_root.clone();
        let trials = m::trial_lines(&world, m::TRIAL_LINES);
        for line in m::console_lines(&subjects, &trials) {
            game.native_chat(&line);
        }
        *canceled = true;
        return;
    }

    if game.native_block(pos) != m::GM_PLOT {
        return;
    }
    let ripe = game
        .native_block_property(pos, "layers")
        .and_then(|v| v.parse::<i32>().ok())
        .is_some_and(m::plot_ripe);
    if !ripe {
        return;
    }
    let planted = game
        .native_with_mod_block_entity(pos, |entity| {
            match entity.nbt.get(&m::PLOT_GENOME_KEY.into()) {
                Some(Tag::String(s)) => s.to_string_lossy(),
                _ => String::new(),
            }
        })
        .unwrap_or_default();
    let genes = m::genome_from_text(&planted);
    if genes.is_empty() {
        return;
    }

    // Урожай несёт геном растения: его можно съесть или посадить снова.
    // Стопка урожая — один плод (меню сливает стопки по id, и разные геномы
    // слиплись бы), поэтому каждый плод ложится в своё свободное место.
    let mut crop = horizon_client::items::ItemStack {
        item: m::GM_CROP.into(),
        count: 1,
        damage: 0,
        extra: None,
    };
    let data = crop.extra.get_or_insert_with(Default::default);
    data.other.push((
        "minecraft:custom_data".to_string(),
        Tag::compound([(
            m::PLOT_GENOME_KEY,
            Tag::String(planted.as_str().into()),
        )]),
    ));
    data.custom_name = Some(Tag::String(m::crop_label(&genes).as_str().into()));

    // Мест меньше, чем плодов, — не собираем вовсе: иначе часть урожая пропала бы.
    let free = (0..36)
        .filter(|&slot| {
            game.native_carried_slot(slot)
                .is_some_and(|place| place.is_none())
        })
        .count();
    if free < m::PLOT_YIELD as usize {
        game.native_chat(&format!(
            "Урожай некуда положить: нужно свободных мест — {}",
            m::PLOT_YIELD
        ));
        return;
    }
    let mut left = m::PLOT_YIELD;
    for slot in 0..36 {
        if left == 0 {
            break;
        }
        let Some(place) = game.native_carried_slot(slot) else {
            continue;
        };
        if place.is_none() {
            *place = Some(crop.clone());
            left -= 1;
        }
    }

    game.native_with_mod_block_entity(pos, |entity| {
        entity.nbt.remove(&m::PLOT_GENOME_KEY.into());
        entity.data.clear();
    });
    game.native_set_block_property(pos, "layers", "1");
    game.native_chat(&format!("Собрано: {}", m::crop_label(&genes)));
    *canceled = true;
}
