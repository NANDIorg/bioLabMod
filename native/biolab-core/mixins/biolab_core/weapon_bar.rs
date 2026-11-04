// Полоска под иконкой оружия (GDD 4.20): у ружья — сколько дротиков в
// магазине (красная, когда кончился газ), у заряженного арбалета — полная
// (`weapon_bar`). Данные — со стека, их клиент получает от сервера. Хук
// спрашивают каждый кадр для каждого нарисованного стека: сначала дешёвая
// проверка id. Клиент, стабильный API 60.
fn biolab_core_item_bar(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    bar: &mut Option<(i32, u32)>,
) {
    use crate::game::biolab_core as m;
    if m::dart_weapon(&stack.item).is_none() || !game.native_on(m::ID) {
        return;
    }
    if let Some(shown) = m::weapon_bar(&stack.item, &crate::game::biolab_weapons::state_of(stack)) {
        *bar = Some(shown);
    }
}
