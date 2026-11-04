// Сервер сказал «журнал применён» (`research_open.rs`): открыть свой экран
// мода с картой исследований. Хук сети получает игру на запись, так что
// экран открывается прямо здесь, без отложенных просьб. Экран не откроется
// поверх другого экрана — значит, игрок уже успел открыть что-то своё.
fn biolab_core_research_net(game: &mut crate::game::Game<'_>, channel: &str) {
    use crate::game::biolab_core as m;
    if channel != m::NET_JOURNAL || !game.native_on(m::ID) {
        return;
    }
    // Повторный пакет (зажатая кнопка успела применить журнал дважды) не
    // сбрасывает уже открытую запись обратно на карту.
    let already = game.native_mod_screen() == Some(m::ID);
    if game.native_mod_screen_open(m::ID) && !already {
        m::open_research();
    }
}
