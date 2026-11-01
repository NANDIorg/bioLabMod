// Журнал лаборанта в руке открывает карту исследований: экран ванильной
// книги, но пустой — его рисует и слушает `research_draw.rs`. Ванильный
// экран нужен ради того, что он даёт даром: свободный курсор, мышь и
// клавиши уходят ему, а мир не реагирует. На сервер запрос не идёт.
fn biolab_deep_research_open(
    game: &mut crate::game::Game<'_>,
    hand: horizon_client::first_person::Hand,
) -> std::ops::ControlFlow<bool> {
    use crate::game::biolab_core as m;
    use std::ops::ControlFlow;
    if !game.native_on(m::ID) {
        return ControlFlow::Continue(());
    }
    let manual = game.with_hand(hand, |g| g.held().is_some_and(|s| &*s.item == m::MANUAL_ID));
    if !manual {
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
