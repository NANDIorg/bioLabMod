// Клавиши на карте исследований: Esc закрывает запись, потом журнал;
// стрелки листают запись, цифры 1–5 переключают науки. Остальное глотаем,
// чтобы ванильная книга не листала пустые страницы.
fn biolab_deep_research_key(
    game: &mut crate::game::Game<'_>,
    key: Option<winit::keyboard::KeyCode>,
) -> std::ops::ControlFlow<()> {
    use crate::game::biolab_core as m;
    use std::ops::ControlFlow;
    use winit::keyboard::KeyCode;
    let view = m::research_view();
    let ours = matches!(
        &game.book,
        Some(crate::game::books::BookScreen::View { pages, lectern: None, .. }) if pages.is_empty()
    );
    if !view.open || !ours {
        return ControlFlow::Continue(());
    }
    let name = match key {
        Some(KeyCode::Escape) => "escape",
        Some(KeyCode::Backspace) => "backspace",
        Some(KeyCode::KeyE) => "e",
        Some(KeyCode::ArrowLeft) => "left",
        Some(KeyCode::KeyA) => "a",
        Some(KeyCode::ArrowRight) => "right",
        Some(KeyCode::KeyD) => "d",
        Some(KeyCode::Digit1) => "1",
        Some(KeyCode::Digit2) => "2",
        Some(KeyCode::Digit3) => "3",
        Some(KeyCode::Digit4) => "4",
        Some(KeyCode::Digit5) => "5",
        _ => return ControlFlow::Break(()),
    };
    let pages = match view.page {
        Some((node, _)) => {
            let text = game
                .native_data(m::RESEARCH_PATH)
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .unwrap_or_default();
            m::research_pages(&text, node).len()
        }
        None => 1,
    };
    let (next, close) = m::research_key(view, name, pages);
    m::set_research_view(next);
    if close {
        game.close_book();
    }
    ControlFlow::Break(())
}
