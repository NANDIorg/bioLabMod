// Мазок с моба (GDD 4.10, 4.26 А): ватной палочкой по заражённому, больному
// скоту или зомби — палочка уносит штамм (`swab_strain`), и инкубатор
// вырастит из неё культуру этого штамма как есть (`incubated_strain`).
// Чистая палочка, как и прежде, даёт дикую линию.
//
// Вторая инъекция на `native_interact_entity` рядом с `inject.rs`: тот
// палочку не трогает (её нет ни в шприцах, ни в инструментах).
fn biolab_core_infected_swab(
    game: &mut crate::game::Game<'_>,
    entity: i32,
    stack: Option<&horizon_client::items::ItemStack>,
    claimed: &mut bool,
) {
    use crate::game::biolab_core as m;
    use horizon_world::nbt::Tag;

    /// Штамм, уже записанный на палочку.
    fn strain_on(stack: &horizon_client::items::ItemStack) -> bool {
        use crate::game::biolab_core as m;
        stack.extra.as_ref().is_some_and(|extra| {
            extra.other.iter().any(|(key, data)| {
                key == "minecraft:custom_data" && data.get(m::STRAIN_KEY).is_some()
            })
        })
    }

    if *claimed || !game.native_on(m::ID) {
        return;
    }
    let Some(stack) = stack else {
        return;
    };
    if &*stack.item != m::SWAB_ID {
        return;
    }
    let used = strain_on(stack);
    let Some((kind, sick)) = game
        .mobs
        .iter()
        .find(|mob| mob.id as i32 == entity && mob.alive())
        .map(|mob| {
            let sick = mob
                .extra
                .effects
                .iter()
                .any(|(id, ticks, _)| &**id == m::INFECTION_EFFECT && *ticks != 0);
            (m::full_kind(&mob.kind.id), sick)
        })
    else {
        return;
    };
    // Клик наш: палочка в лицо жителю не открывает торговлю.
    *claimed = true;
    if used {
        crate::game::biolab_say::say(game, m::SWAB_USED);
        return;
    }
    let Some(strain) = m::swab_strain(&kind, sick) else {
        crate::game::biolab_say::say(game, m::SWAB_CLEAN);
        return;
    };

    // Палочка с мазком — отдельный предмет: штамм в `custom_data`, подпись
    // в имени. Последнюю палочку меняем на месте, из стопки — забираем одну
    // и кладём мазок в свободный слот (выдача через `native_give` данные не
    // переносит).
    let text = m::strain_to_text(&strain);
    let stamp = |swab: &mut horizon_client::items::ItemStack| {
        let data = swab.extra.get_or_insert_with(Default::default);
        data.other.retain(|(key, _)| key != "minecraft:custom_data");
        data.other.push((
            "minecraft:custom_data".to_string(),
            Tag::compound([(m::STRAIN_KEY, Tag::String(text.as_str().into()))]),
        ));
        data.other.sort_by(|a, b| a.0.cmp(&b.0));
        data.custom_name = Some(Tag::String(m::swab_label(&strain).as_str().into()));
    };
    // Рука, которой кликнули: основная, если палочка в ней, иначе левая.
    let main = game.selected as i32;
    let hand = if game.native_carried_stack(main).is_some_and(|held| &*held.item == m::SWAB_ID) {
        main
    } else {
        -106
    };
    let count = match game.native_carried_stack(hand) {
        Some(held) if &*held.item == m::SWAB_ID => held.count,
        _ => return,
    };
    if count <= 1 {
        if let Some(held) = game.native_carried_stack(hand) {
            stamp(held);
        }
    } else {
        let free = (0..36).find(|slot| game.native_carried_slot(*slot).is_some_and(|place| place.is_none()));
        let Some(free) = free else {
            crate::game::biolab_say::say(game, m::SWAB_NO_ROOM);
            return;
        };
        let mut swab = horizon_client::items::ItemStack {
            item: m::SWAB_ID.into(),
            count: 1,
            damage: 0,
            extra: None,
        };
        stamp(&mut swab);
        if let Some(held) = game.native_carried_stack(hand) {
            held.count -= 1;
        }
        if let Some(place) = game.native_carried_slot(free) {
            *place = Some(swab);
        }
    }
    m::remember_injection(game.ticks);
    crate::game::biolab_say::say(game, &m::swab_message(&kind, &strain));
}
