// Свой экран живого генератора (M10, GDD 4.27 В): слоты слева, шкала
// порции, табло и шкала заряда (`living_screen`). Хук спрашивают и клиент, и
// сервер. Вторая инъекция на `native_screen` после `screen_layout.rs` (тот
// знает только машины и на наши блоки ничего не ставит).
fn biolab_core_living_screen(block: &str, screen: &mut Option<crate::game::native_hooks::NativeScreen>) {
    use crate::game::biolab_core as m;
    if screen.is_some() {
        return;
    }
    let Some(layout) = m::living_screen(block) else {
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
