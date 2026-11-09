// Заряд батареи-предмета полоской под иконкой (M10, GDD 4.27 п. 9), как
// магазин ружья (`weapon_bar.rs`). Клиент, API 60; хук спрашивают каждый
// кадр для каждого стека — сначала дешёвая проверка id.
fn biolab_core_battery_bar(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    bar: &mut Option<(i32, u32)>,
) {
    use crate::game::biolab_core as m;
    if &*stack.item != m::BATTERY || !game.native_on(m::ID) {
        return;
    }
    let stored = game.native_item_energy(stack).map_or(0, |e| e.amount);
    *bar = Some(m::battery_bar(stored));
}
