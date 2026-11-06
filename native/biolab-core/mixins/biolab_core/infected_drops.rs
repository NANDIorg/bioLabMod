// Заражённый (GDD 4.26 А): культура из его добычи — его штамма.
//
// Таблица `biolab:entities/infected` изредка роняет чашку с культурой; чашка
// без данных — дикая линия (`strain_data`). Функции `set_custom_data` в пине
// нет, а через `set_components` штамм жил бы вторым экземпляром в JSON —
// поэтому хук выпадения (API 75) дописывает штамм заражённого
// (`INFECTED_STRAIN`) и подпись в каждую такую чашку.
fn biolab_core_infected_drops(
    game: &mut crate::game::Game<'_>,
    context: &horizon_client::mod_api::NativeLootDrops<'_>,
    drops: &mut Vec<horizon_client::items::ItemStack>,
) {
    use crate::game::biolab_core as m;
    use horizon_world::nbt::Tag;
    if !game.native_on(m::ID) || context.used {
        return;
    }
    let Some((_, kind)) = context.entity else {
        return;
    };
    if !m::is_infected(kind) {
        return;
    }
    let text = m::strain_to_text(&m::INFECTED_STRAIN);
    for stack in drops.iter_mut().filter(|stack| &*stack.item == m::CULTURE_ID) {
        let data = stack.extra.get_or_insert_with(Default::default);
        data.other.retain(|(key, _)| key != "minecraft:custom_data");
        data.other.push((
            "minecraft:custom_data".to_string(),
            Tag::compound([(m::STRAIN_KEY, Tag::String(text.as_str().into()))]),
        ));
        data.other.sort_by(|a, b| a.0.cmp(&b.0));
        data.custom_name = Some(Tag::String(m::culture_label(&m::INFECTED_STRAIN).as_str().into()));
    }
}
