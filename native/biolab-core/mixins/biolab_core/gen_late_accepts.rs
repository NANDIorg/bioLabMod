// Что слот топлива двигателя и изотопной батареи берёт от воронки (M10):
// двигатель — спирт и растительное масло, батарея — изотоп. Правило то же,
// что у меню (`late_slot_accepts` в `runtime.rs`). Вторая инъекция на
// `native_container_accepts` рядом с `container_accepts.rs` (тот отвечает за
// машины и криобанк, на генераторах молчит). Только сервер.
fn biolab_core_gen_late_accepts(
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
    if m::late_slot_accepts(&entity.id, slot, &stack.item) == Some(false) {
        *cancel = true;
    }
}
