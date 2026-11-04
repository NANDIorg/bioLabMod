// Использование со временем у арбалета и ружья (GDD 4.20): предмет мода,
// которому хук отвечает больше 0, игра держит по зажатому ПКМ до отпускания
// (`item_use.rs` пина, `mod_use`). На этом держатся натяжение арбалета и
// прицел ружья; у трубки своего нет — она стреляет кликом (`remedy.rs`).
// Хук спрашивают каждый кадр и без игрока: отвечаем по одному id. Обе
// стороны, стабильный API 90.
fn biolab_core_use_duration(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    duration: &mut i32,
) {
    use crate::game::biolab_core as m;
    if let Some(ticks) = m::weapon_use_duration(&stack.item)
        && game.native_on(m::ID)
    {
        *duration = ticks;
    }
}
