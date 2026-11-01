// Предметы с данными в инвентаре: данные лежат в `minecraft:custom_data`, а
// подпись предмета всегда выводится из них — у колбы вещество и объём, у
// фрагмента ген, у культуры штамм, у урожая геном (`item_label`). Так и старые
// предметы получают подпись по нынешней схеме. Пустая колба получает
// содержимое по умолчанию. Хук срабатывает для каждого стека каждый тик,
// поэтому сначала дешёвая проверка id, а запись — только когда что-то
// поменялось.
fn biolab_core_inventory_tick(game: &mut crate::game::Game<'_>, slot: i32) {
    use crate::game::biolab_core as m;
    use horizon_world::nbt::Tag;
    if !game.native_on(m::ID) {
        return;
    }
    let Some(stack) = game.native_carried_stack(slot) else {
        return;
    };
    if &*stack.item != m::FLASK_ID {
        let item: &str = &stack.item;
        if item != m::FRAGMENT_ID && item != m::CULTURE_ID && item != m::GM_CROP {
            return;
        }
        let Some(data) = stack.extra.as_mut() else {
            return;
        };
        let custom = data
            .other
            .iter()
            .find(|(key, _)| key == "minecraft:custom_data")
            .map(|(_, tag)| tag);
        let label = m::item_label(item, |key| match custom?.get(key) {
            Some(Tag::String(s)) => Some(s.to_string_lossy()),
            _ => None,
        });
        let Some(label) = label else {
            return;
        };
        let current = match &data.custom_name {
            Some(Tag::String(s)) => Some(s.to_string_lossy()),
            _ => None,
        };
        if current.as_deref() != Some(label.as_str()) {
            data.custom_name = Some(Tag::String(label.as_str().into()));
        }
        return;
    }
    let data = stack.extra.get_or_insert_with(Default::default);
    let saved = data
        .other
        .iter()
        .find(|(key, _)| key == "minecraft:custom_data")
        .map(|(_, tag)| {
            let contents = match tag.get(m::CONTENTS_KEY) {
                Some(Tag::String(s)) => Some(s.to_string_lossy()),
                _ => None,
            };
            let ml = match tag.get(m::ML_KEY) {
                Some(Tag::Int(v)) => Some(*v),
                _ => None,
            };
            (contents, ml)
        });
    let (contents, ml) = match saved {
        Some((Some(contents), Some(ml))) => (contents, ml),
        // Нет данных или они неполные: колба считается пустой и заполняется.
        _ => {
            let (contents, ml) = m::DEFAULT_FILL;
            data.other.retain(|(key, _)| key != "minecraft:custom_data");
            data.other.push((
                "minecraft:custom_data".to_string(),
                Tag::compound([
                    (m::CONTENTS_KEY, Tag::String(contents.into())),
                    (m::ML_KEY, Tag::Int(ml)),
                ]),
            ));
            data.other.sort_by(|a, b| a.0.cmp(&b.0));
            (contents.to_string(), ml)
        }
    };
    let label = m::flask_label(&contents, ml);
    let current = match &data.custom_name {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    };
    if current.as_deref() != Some(label.as_str()) {
        data.custom_name = Some(Tag::String(label.as_str().into()));
    }
}
