// Кнопка мыши отпущена над картой исследований
// (`native_screen_mouse_button_released_pre`, API 80). Глотаем только
// отпускание того нажатия, что проглотил `research_click.rs`: правый щелчок,
// открывший журнал, начался до экрана, и его отпускание должно дойти до игры,
// иначе она сочтёт кнопку зажатой.
fn biolab_core_research_release(
    game: &crate::game::Game<'_>,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    use crate::game::biolab_core as m;
    if screen.id != "mod" || game.native_mod_screen() != Some(m::ID) {
        return;
    }
    if m::with_research_input(|input| input.release(button)).unwrap_or(false) {
        *canceled = true;
    }
}
