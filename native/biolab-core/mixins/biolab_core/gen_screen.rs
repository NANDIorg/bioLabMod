// Экран столба и котла (M10): где слоты, где инвентарь, заголовок и подписи
// рисуем сами. Хук спрашивают клиент и сервер. Раскладка — `early_screen`
// (раздел «M10: генераторы 0–2» в `runtime.rs`); вторая инъекция на
// `native_screen` рядом с `screen_layout.rs` (тот знает только машины).
fn biolab_core_gen_screen(
    block: &str,
    screen: &mut Option<crate::game::native_hooks::NativeScreen>,
) {
    use crate::game::biolab_core as m;
    if screen.is_some() {
        return;
    }
    let Some(layout) = m::early_screen(block) else {
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
