// Клавиши, пока открыта книга: стрелки листают, цифры прыгают по разделам,
// Esc закрывает. Книга — оверлей поверх игры, так что экран у игрока
// остаётся игровым и ванильные клавиши продолжают работать.
fn biolab_core_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) || !pressed || !m::book_open() {
        return;
    }
    let text = game
        .native_data(m::MANUAL_PATH)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_default();
    let total = m::manual_pages(&text).len();
    match m::book_key(key) {
        m::BookKey::Next => m::turn_page(1, total),
        m::BookKey::Previous => m::turn_page(-1, total),
        m::BookKey::Jump(page) => m::go_to_page(page, total),
        m::BookKey::Close => m::close_book(),
        m::BookKey::Ignore => {}
    }
}
