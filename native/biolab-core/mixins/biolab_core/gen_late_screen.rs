// Свой экран двигателя-генератора и изотопной батареи (M10, GDD 4.27 п. 11):
// слот топлива, табло и шкалы. Раскладка — `late_screen` в `runtime.rs`,
// рисует `gen_late_draw.rs`. Вторая инъекция на `native_screen` рядом со
// `screen_layout.rs` (тот знает только машины); спрашивают клиент и сервер.
fn biolab_core_gen_late_screen(
    block: &str,
    screen: &mut Option<crate::game::native_hooks::NativeScreen>,
) {
    use crate::game::biolab_core as m;
    if screen.is_some() {
        return;
    }
    let Some(layout) = m::late_screen(block) else {
        return;
    };
    *screen = Some(crate::game::native_hooks::NativeScreen {
        width: layout.width,
        height: layout.height,
        slots: layout.slots,
        inventory_y: layout.inventory[1],
        inventory_x: layout.inventory[0],
        own_labels: true,
        own_background: true,
    });
}
