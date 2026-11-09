// Заряд батареи-предмета в подсказке (M10, GDD 4.27 п. 9): «Батарея: 4 200 /
// 10 000 Вт·с». Ещё одна инъекция на `native_tooltip` (API 12), рядом с
// `tooltip.rs` и `contam_tooltip.rs`.
fn biolab_core_battery_tooltip(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    lines: &mut Vec<(horizon_client::text::Text, u32)>,
) {
    use crate::game::biolab_core as m;
    if &*stack.item != m::BATTERY || !game.native_on(m::ID) {
        return;
    }
    let stored = game.native_item_energy(stack).map_or(0, |e| e.amount);
    let at = lines.len().min(1);
    lines.insert(at, (horizon_client::text::Text::literal(&m::battery_line(stored)), 0x3F_D8C4));
}
