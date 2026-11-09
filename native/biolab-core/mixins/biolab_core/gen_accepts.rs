// Что слот столба или котла берёт от воронки, выбрасывателя и
// `native_insert_item` (`native_container_accepts`, сервер): то же правило,
// что у меню, — `early_slot_rule` (раздел «M10: генераторы 0–2» в
// `runtime.rs`). Вторая инъекция на хук рядом с `container_accepts.rs`.
fn biolab_core_gen_accepts(
    game: &crate::game::Game<'_>,
    entity: &horizon_client::chunk::block_entity::ModBlockEntity,
    slot: usize,
    stack: &horizon_client::items::ItemStack,
    cancel: &mut bool,
) {
    use crate::game::biolab_core as m;
    if *cancel || !game.native_on(m::ID) {
        return;
    }
    if m::early_slot_rule(&entity.id, slot, &stack.item) == Some(false) {
        *cancel = true;
    }
}
