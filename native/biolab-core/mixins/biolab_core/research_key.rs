// Клавиша над картой исследований (`native_screen_key_pressed_pre`, API 104):
// Esc закрывает запись, потом журнал; E и Backspace — то же; стрелки и A/D
// листают запись, цифры 1–5 переключают науки. На своём экране мода клавиши
// наконец доходят — и от игрока, и от агентского `game_key` (ванильная книга
// их обходила). Чужие клавиши не глотаем. Хук на чтение: закрывает экран
// кадровый хук (`research_frame.rs`), а Escape на карте — сама игра.
fn biolab_core_research_key(
    game: &crate::game::Game<'_>,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    key: &str,
    canceled: &mut bool,
) {
    use crate::game::biolab_core as m;
    if screen.id != "mod" || game.native_mod_screen() != Some(m::ID) {
        return;
    }
    let view = m::research_view();
    let text = game
        .native_data(m::RESEARCH_PATH)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_default();
    let press = m::research_key_press(view, key, m::research_page_count(&view, &text));
    if !press.cancel {
        return;
    }
    *canceled = true;
    m::set_research_view(press.view);
    if press.close {
        m::with_research_input(|input| input.close = true);
    }
}
