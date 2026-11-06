// Кнопки режимов на экране консоли секвенатора (GDD 4.18, п. 3): окно ловит
// нажатие над кнопкой (`native_screen_click`, API 20) и называет её номер —
// Horizon сам шлёт его серверу пакетом `ContainerButtonClick`, где режим
// меняет `sequencer_button.rs`. `at` — точка от левого верхнего угла панели,
// те же координаты, что у раскладки экрана (`seq_buttons`). Зовётся из
// `native_screen_press` (`native_hooks.rs` пина) при нажатии над экраном мода
// мимо слотов, без условия `mod_api::mods()`. Правая кнопка — тоже выбор:
// так нажатие не уходит дальше.
fn biolab_core_sequencer_click(block: &str, data: &[i32], at: [f32; 2], right: bool, button: &mut Option<i32>) {
    use crate::game::biolab_core as m;
    let _ = (data, right);
    if block != m::SEQUENCER || button.is_some() {
        return;
    }
    if let Some(mode) = m::seq_button_at(at) {
        *button = Some(mode.id());
    }
}
