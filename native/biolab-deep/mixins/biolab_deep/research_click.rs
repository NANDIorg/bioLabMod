// Щелчок на карте исследований: вкладка, узел, кнопки страницы. Ванильная
// книга его не получает.
fn biolab_deep_research_click(
    game: &mut crate::game::Game<'_>,
    size: [u32; 2],
) -> std::ops::ControlFlow<()> {
    use crate::game::biolab_core as m;
    use std::ops::ControlFlow;
    let view = m::research_view();
    let ours = matches!(
        &game.book,
        Some(crate::game::books::BookScreen::View { pages, lectern: None, .. }) if pages.is_empty()
    );
    if !view.open || !ours {
        return ControlFlow::Continue(());
    }
    let batch = horizon_client::hud::HudBatch::new(size);
    let layout = m::research_layout(batch.width, batch.height);
    let mask = m::shown_research();
    let hit = m::research_hit(&layout, &view, mask, game.native_cursor());
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
    let next = m::research_click(view, hit, mask, pages);
    if next != view {
        m::set_research_view(next);
        let _ = game.audio.play("minecraft:ui.button.click", None, 0.25, 1.0);
    }
    ControlFlow::Break(())
}
