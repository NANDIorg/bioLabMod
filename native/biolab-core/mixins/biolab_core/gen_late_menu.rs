// Правила слота своего меню двигателя и изотопной батареи (M10): руками в
// слот топлива — только их топливо. Хук — свободная функция lib-крейта
// (`menu.rs`), там виден `crate::biolab_core` (модуль `runtime-menu`), как у
// `menu_slot_rules.rs`. Вторая инъекция на этот хук; ответ только сужаем.
fn biolab_core_gen_late_menu(menu: &str, slot: usize, stack: &crate::items::ItemStack, may_place: &mut bool) {
    use crate::biolab_core as m;
    if let Some(place) = m::late_slot_accepts(menu, slot, &stack.item) {
        *may_place = *may_place && place;
    }
}
