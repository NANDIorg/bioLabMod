// Правила слотов своего меню машины (GDD 4.15): в выход руками не положить,
// в слот раствора чана — только раствор, во вход — только то, с чем машина
// работает. Хук — свободная функция в lib-крейте (`menu.rs`), куда модуль
// `crate::game::biolab_core` не виден, поэтому `runtime.rs` добавлен и сюда,
// вторым модулем `crate::biolab_core`. Зовётся по нескольку раз за клик, без
// уровня и игрока: решение — по id предмета, из таблиц мода.
// Оговорка платформы: шифт-клик доливает слот с тем же предметом, не
// спрашивая `may_place`. Нам это не мешает — такой предмет слот уже принял,
// а выходы (`results`) игра не доливает вовсе.
fn biolab_core_menu_slot_rules(menu: &str, slot: usize, stack: &crate::items::ItemStack, may_place: &mut bool) {
    use crate::biolab_core as m;
    if let Some(place) = m::menu_slot_rule(menu, slot, &stack.item) {
        // Ответ типа меню (false на `results`) не перекрываем, только сужаем.
        *may_place = *may_place && place;
    }
}
