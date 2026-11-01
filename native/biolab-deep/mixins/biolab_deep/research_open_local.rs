// То же, что `research_open.rs`, для игры без сервера в отдельном потоке:
// предмет применяется прямо в окне через `use_book`.
fn biolab_deep_research_open_local(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
) -> std::ops::ControlFlow<bool> {
    use crate::game::biolab_core as m;
    use std::ops::ControlFlow;
    if !game.native_on(m::ID) || &*stack.item != m::MANUAL_ID {
        return ControlFlow::Continue(());
    }
    m::open_research();
    game.book = Some(crate::game::books::BookScreen::View {
        pages: Vec::new(),
        page: 0,
        lectern: None,
    });
    game.keys.clear();
    game.right = false;
    game.capture(false);
    ControlFlow::Break(true)
}
