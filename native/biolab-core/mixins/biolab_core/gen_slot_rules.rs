// Правила слотов меню столба и котла (M10): руками в слот кладётся только
// своё, в выход — ничего. Свободная функция lib-крейта: модуль виден как
// `crate::biolab_core` (вторая копия `runtime.rs`, как у `menu_slot_rules.rs`).
fn biolab_core_gen_slot_rules(menu: &str, slot: usize, stack: &crate::items::ItemStack, may_place: &mut bool) {
    use crate::biolab_core as m;
    if let Some(place) = m::early_slot_rule(menu, slot, &stack.item) {
        *may_place = *may_place && place;
    }
}
