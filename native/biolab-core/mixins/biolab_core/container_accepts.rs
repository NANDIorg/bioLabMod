// Что слот машины берёт от воронки, выбрасывателя и `native_insert_item`
// (GDD 4.15, 4.26 Г; NeoForge `IItemHandler.isItemValid`). Игра спрашивает
// про каждый слот, который ей открывает `ModContainer` машины (стороны —
// `machine_slots.rs`); отказ — предмет идёт в следующий слот или остаётся в
// воронке, вход не забивается чужим. Ответ — тот же, что у правил слотов
// меню (`container_slot_accepts` → `slot_accepts` в `runtime.rs`).
//
// Заменил `hopper_filter.rs` на `native_inventory_move_item`: тот спрашивал
// про машину целиком и при наших бандлах-миксинах не звался вовсе (почему —
// в `runtime.rs`, раздел «M11-Г»). Из машины воронка забирает как раньше:
// этот хук спрашивают только при вкладывании. Зовётся только на сервере.
fn biolab_core_container_accepts(
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
    if m::container_slot_accepts(&entity.id, slot, &stack.item) == Some(false) {
        *cancel = true;
    }
}
