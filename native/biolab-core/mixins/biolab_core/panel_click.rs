// Кнопки панелей M9 на экранах машин (GDD 4.23 п. 3, 4): ручки режима у
// ферментера и экстрактора и проверки у спектрометра. Окно ловит нажатие над
// кнопкой (`native_screen_click`, API 20) и называет её номер — Horizon сам
// шлёт его серверу пакетом `ContainerButtonClick`, где его разбирает
// `panel_button.rs`. Вторая инъекция на этот хук рядом с `sequencer_click.rs`:
// номера кнопок разведены (`PROTO_BUTTON_BASE`, `PROBE_BUTTON_BASE`), и у
// каждой машины свои. `at` — точка от левого верхнего угла панели, те же
// координаты, что у раскладки экрана. Зовётся из `native_screen_press`
// (`native_hooks.rs` пина) при нажатии над экраном мода мимо слотов, без
// условия `mod_api::mods()`. Правая кнопка — то же, что левая: так нажатие
// не уходит дальше.
fn biolab_core_panel_click(block: &str, data: &[i32], at: [f32; 2], right: bool, button: &mut Option<i32>) {
    use crate::game::biolab_core as m;
    let _ = right;
    if button.is_some() {
        return;
    }
    if m::has_protocol(block)
        && let Some((knob, up)) = m::proto_button_at(at)
    {
        *button = Some(m::proto_button_id(knob, up));
        return;
    }
    // Проверки — только у пробы: у обычной колбы прибор рисует спектр, и
    // кнопок там нет.
    if block == m::SPECTROMETER
        && m::probe_shown(data).is_some()
        && let Some(probe) = m::probe_button_at(at)
    {
        *button = Some(m::probe_button_id(probe));
    }
}
