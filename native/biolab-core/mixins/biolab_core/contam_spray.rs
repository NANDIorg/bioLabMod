// Распылитель (M10, GDD 4.21 «Обратимость»): ПКМ — пшик хлором по чанку под
// ногами, пустой — заправка хлором из инвентаря. Ещё один перехват
// `native_use_item` (стабильный API 13), как `suit_use.rs`.
fn biolab_core_contam_spray(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    claimed: &mut bool,
) {
    use crate::game::biolab_core as m;
    if *claimed || !game.native_on(m::ID) || &*stack.item != m::SPRAYER {
        return;
    }
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    *claimed = true;
    crate::game::biolab_world_contam::spray(game);
}
