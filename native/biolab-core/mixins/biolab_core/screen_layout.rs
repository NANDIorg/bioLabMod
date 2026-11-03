// Свой экран машины: где её слоты, где инвентарь, и что заголовок с подписью
// инвентаря рисуем сами. Этот хук спрашивают и клиент, и сервер: слот без
// позиции (топливо у всех машин на меню печки, кроме чана) выпадает и из
// серверного меню, так что shift-клик его не наполнит.
fn biolab_core_screen(
    block: &str,
    screen: &mut Option<crate::game::native_hooks::NativeScreen>,
) {
    use crate::game::biolab_core as m;
    let Some(layout) = m::machine_screen(block) else {
        return;
    };
    *screen = Some(crate::game::native_hooks::NativeScreen {
        width: layout.width,
        height: layout.height,
        slots: layout.slots,
        inventory_y: layout.inventory[1],
        inventory_x: layout.inventory[0],
        own_labels: true,
        // Фон и рамки слотов рисуем сами (`native_screen_draw`).
        own_background: true,
    });
}
