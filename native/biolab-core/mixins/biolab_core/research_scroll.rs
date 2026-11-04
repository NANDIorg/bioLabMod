// Колесо над картой исследований (`native_screen_mouse_scrolled_pre`, API 93):
// на записи листает страницы, на карте — науки. Доли щелчка от тачпада
// копятся в `ResearchInput::scroll`, чтобы запись не листалась от касания.
fn biolab_core_research_scroll(
    game: &crate::game::Game<'_>,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    scroll: [f64; 2],
    canceled: &mut bool,
) {
    use crate::game::biolab_core as m;
    if screen.id != "mod" || game.native_mod_screen() != Some(m::ID) {
        return;
    }
    *canceled = true;
    let steps = m::with_research_input(|input| input.scroll(scroll[1])).unwrap_or(0);
    if steps == 0 {
        return;
    }
    let view = m::research_view();
    let text = game
        .native_data(m::RESEARCH_PATH)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_default();
    let next = m::research_scroll(view, steps, m::research_page_count(&view, &text));
    if next != view {
        m::set_research_view(next);
        m::with_research_input(|input| input.click = true);
    }
}
