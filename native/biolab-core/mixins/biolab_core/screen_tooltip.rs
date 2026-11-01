// Подсказка над прибором машины: числа из слотов данных словами. Тексты
// собирает модуль мода — там они под тестами.
fn biolab_core_screen_tooltip(block: &str, data: &[i32], at: [f32; 2], lines: &mut Vec<String>) {
    use crate::game::biolab_core as m;
    lines.extend(m::screen_tooltip(block, data, at));
}
