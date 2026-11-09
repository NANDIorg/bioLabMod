// Подсказка над шкалами и табло экрана живого генератора (M10, GDD 4.27 В):
// заряд числом, что генератору нужно, сколько горит порция. Тексты —
// `living_screen_tooltip` в `runtime.rs`, под тестами.
fn biolab_core_living_tip(block: &str, data: &[i32], at: [f32; 2], lines: &mut Vec<String>) {
    use crate::game::biolab_core as m;
    lines.extend(m::living_screen_tooltip(block, data, at));
}
