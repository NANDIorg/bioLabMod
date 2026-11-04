// Кнопка мыши над картой исследований (`native_screen_mouse_button_pressed_pre`,
// API 80): левая — вкладка, узел, кнопки страницы. Любое нажатие над журналом
// глотаем и запоминаем, чтобы проглотить и его отпускание
// (`research_release.rs`). Хук получает игру на чтение: звук щелчка играет
// кадровый хук (`research_frame.rs`) по просьбе отсюда.
fn biolab_core_research_click(
    game: &crate::game::Game<'_>,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    use crate::game::biolab_core as m;
    if screen.id != "mod" || game.native_mod_screen() != Some(m::ID) {
        return;
    }
    *canceled = true;
    m::with_research_input(|input| input.press(button));
    if button != 0 {
        return;
    }
    let view = m::research_view();
    // У экрана без панели `rect` — всё окно в пикселях интерфейса, те же
    // ширина и высота, что у кадра в `research_draw.rs`: раскладка совпадает.
    let layout = m::research_layout(screen.rect[2], screen.rect[3]);
    let mask = m::shown_research();
    let hit = m::research_hit(&layout, &view, mask, screen.mouse);
    let text = game
        .native_data(m::RESEARCH_PATH)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_default();
    let next = m::research_click(view, hit, mask, m::research_page_count(&view, &text));
    if next != view {
        m::set_research_view(next);
        m::with_research_input(|input| input.click = true);
    }
}
