// Карта исследований рисуется вместо ванильной книги, когда её экран открыл
// журнал лаборанта (`research_open.rs`): вкладки наук слева, тёмное поле с
// узлами и связями, подсказка у курсора, запись на пергаменте поверх поля.
fn biolab_deep_research_draw(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
) -> std::ops::ControlFlow<()> {
    use crate::game::biolab_core as m;
    use std::ops::ControlFlow;

    const PAPER: u32 = 0xFF_E9DCBC;
    const PAPER_EDGE: u32 = 0xFF_5C4026;
    const INK: u32 = 0x3A3028;
    const TITLE_INK: u32 = 0x5A2D1B;
    const NOTE: u32 = 0x8A7A66;
    const RULE: u32 = 0xFF_A0794A;

    /// Строка по ширине: ширину меряет сам шрифт.
    fn wrap(game: &crate::game::Game<'_>, text: &str, width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in text.split_whitespace() {
            let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if game.native_text_width(&candidate) <= width || line.is_empty() {
                line = candidate;
            } else {
                lines.push(std::mem::take(&mut line));
                line = word.to_string();
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }

    /// Ванильный слот: тёмный верх-лево, светлый низ-право.
    fn slot(
        game: &crate::game::Game<'_>,
        batch: &mut horizon_client::hud::HudBatch,
        at: [f32; 2],
        item: Option<&str>,
        count: u32,
    ) {
        let [x, y] = at;
        game.native_fill(batch, [x, y, 18.0, 18.0], 0xFF_FFFFFF);
        game.native_fill(batch, [x, y, 17.0, 17.0], 0xFF_373737);
        game.native_fill(batch, [x + 1.0, y + 1.0, 16.0, 16.0], 0xFF_8B8B8B);
        if let Some(item) = item {
            game.native_item(batch, item, [x + 1.0, y + 1.0], 16.0);
        }
        if count > 1 {
            let text = count.to_string();
            let w = game.native_text_width(&text);
            game.native_text(batch, &text, [x + 18.0 - w, y + 10.0], 0xFFFFFF, true);
        }
    }

    /// Линия связи толщиной 2 между двумя точками по горизонтали.
    fn hline(game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, x0: f32, x1: f32, y: f32, color: u32) {
        let (a, b) = (x0.min(x1), x0.max(x1));
        game.native_fill(batch, [a - 1.0, y - 1.0, b - a + 2.0, 2.0], color);
    }

    fn vline(game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, x: f32, y0: f32, y1: f32, color: u32) {
        let (a, b) = (y0.min(y1), y0.max(y1));
        game.native_fill(batch, [x - 1.0, a - 1.0, 2.0, b - a + 2.0], color);
    }

    /// Рамка в один пиксель вокруг прямоугольника.
    fn outline(game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, r: [f32; 4], color: u32) {
        let [x, y, w, h] = r;
        game.native_fill(batch, [x - 1.0, y - 1.0, w + 2.0, 1.0], color);
        game.native_fill(batch, [x - 1.0, y + h, w + 2.0, 1.0], color);
        game.native_fill(batch, [x - 1.0, y, 1.0, h], color);
        game.native_fill(batch, [x + w, y, 1.0, h], color);
    }

    let view = m::research_view();
    let ours = matches!(
        &game.book,
        Some(crate::game::books::BookScreen::View { pages, lectern: None, .. }) if pages.is_empty()
    );
    if !view.open || !ours {
        return ControlFlow::Continue(());
    }

    let mask = m::shown_research();
    let (width, height) = (batch.width, batch.height);
    let l = m::research_layout(width, height);
    let cursor = game.native_cursor();
    // Доступные узлы мерцают: рамка меняет оттенок раз в 0,6 с.
    let pulse = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() / 600 % 2 == 0)
        .unwrap_or(false);

    game.native_fill(batch, [0.0, 0.0, width, height], m::RESEARCH_SHADE);

    // Вкладки наук: выбранная выступает и касается обложки.
    for (i, science) in m::SCIENCES.iter().enumerate() {
        let r = m::tab_rect(&l, i);
        let on = i == view.science;
        let dst = if on { [r[0] - 4.0, r[1], r[2] + 6.0, r[3]] } else { r };
        let sprite = if on { "biolab:gui/research/tab_on" } else { "biolab:gui/research/tab" };
        if !game.native_sprite(batch, sprite, [0.0, 0.0, 22.0, 22.0], dst) {
            game.native_fill(batch, dst, m::RESEARCH_TAB_EDGE);
            game.native_fill(
                batch,
                [dst[0] + 1.0, dst[1] + 1.0, dst[2] - 2.0, dst[3] - 2.0],
                if on { m::RESEARCH_TAB_ON } else { m::RESEARCH_TAB },
            );
        }
        game.native_item(batch, science.icon, [dst[0] + 3.0, dst[1] + 3.0], 16.0);
        if m::inside(r, cursor) && !on {
            outline(game, batch, r, m::RESEARCH_GOLD);
        }
    }

    // Обложка и поле.
    let [fx, fy, fw, fh] = l.frame;
    game.native_fill(batch, [fx - 1.0, fy - 1.0, fw + 2.0, fh + 2.0], m::RESEARCH_GOLD);
    game.native_fill(batch, [fx, fy, fw, fh], m::RESEARCH_LEATHER);
    let [x, y, w, h] = l.field;
    let tile: f32 = 32.0;
    let mut ty: f32 = 0.0;
    while ty < h {
        let th = tile.min(h - ty);
        let mut tx: f32 = 0.0;
        while tx < w {
            let tw = tile.min(w - tx);
            let dst = [x + tx, y + ty, tw, th];
            if !game.native_sprite(batch, "biolab:gui/research/field", [0.0, 0.0, tw, th], dst) {
                game.native_fill(batch, dst, m::RESEARCH_FIELD);
            }
            tx += tile;
        }
        ty += tile;
    }
    outline(game, batch, l.field, m::RESEARCH_TAB_EDGE);

    let title = m::SCIENCES[view.science].title.to_uppercase();
    game.native_text(batch, &title, [x + 6.0, y + 5.0], m::RESEARCH_TEXT_GOLD, true);
    let progress = m::research_progress(mask);
    let pw = game.native_text_width(&progress);
    game.native_text(batch, &progress, [x + w - pw - 6.0, y + 5.0], m::RESEARCH_TEXT_DIM, false);

    match view.page {
        None => {
            let states: Vec<m::NodeState> = (0..m::RESEARCH.len()).map(|i| m::node_state(mask, i)).collect();
            let shown = |i: usize| m::RESEARCH[i].science == view.science && states[i] != m::NodeState::Hidden;

            // Связи: под узлами, ломаной из горизонталей и вертикали.
            for (i, r) in m::RESEARCH.iter().enumerate() {
                if !shown(i) {
                    continue;
                }
                let b = m::node_center(&l, r);
                for parent in r.after {
                    let Some(p) = m::research_index(parent) else { continue };
                    if !shown(p) {
                        continue;
                    }
                    let a = m::node_center(&l, &m::RESEARCH[p]);
                    let color = match (states[p], states[i]) {
                        (m::NodeState::Done, m::NodeState::Done) => m::RESEARCH_GOLD,
                        (m::NodeState::Done, _) => m::RESEARCH_OPEN_EDGE,
                        _ => m::RESEARCH_LINE_DIM,
                    };
                    if a[1] == b[1] {
                        hline(game, batch, a[0], b[0], a[1], color);
                    } else if a[0] == b[0] {
                        vline(game, batch, a[0], a[1], b[1], color);
                    } else {
                        let mid = ((a[0] + b[0]) / 2.0).floor();
                        hline(game, batch, a[0], mid, a[1], color);
                        vline(game, batch, mid, a[1], b[1], color);
                        hline(game, batch, mid, b[0], b[1], color);
                    }
                }
            }

            // Узлы.
            let mut hovered = None;
            for (i, r) in m::RESEARCH.iter().enumerate() {
                if !shown(i) {
                    continue;
                }
                let rect = m::node_rect(&l, r);
                let (sprite, edge, fill) = match states[i] {
                    m::NodeState::Done => ("biolab:gui/research/node_done", m::RESEARCH_DONE_EDGE, m::RESEARCH_DONE_FILL),
                    m::NodeState::Open => ("biolab:gui/research/node_open", m::RESEARCH_OPEN_EDGE, m::RESEARCH_OPEN_FILL),
                    _ => ("biolab:gui/research/node_locked", m::RESEARCH_LOCK_EDGE, m::RESEARCH_LOCK_FILL),
                };
                if !game.native_sprite(batch, sprite, [0.0, 0.0, 24.0, 24.0], rect) {
                    game.native_fill(batch, rect, edge);
                    game.native_fill(batch, [rect[0] + 2.0, rect[1] + 2.0, rect[2] - 4.0, rect[3] - 4.0], fill);
                }
                if states[i] == m::NodeState::Open && pulse {
                    outline(game, batch, rect, m::RESEARCH_OPEN_GLOW);
                }
                if states[i] == m::NodeState::Locked {
                    let qw = game.native_text_width("?");
                    game.native_text(batch, "?", [rect[0] + (rect[2] - qw) / 2.0, rect[1] + 8.0], m::RESEARCH_TEXT_LOCK, false);
                } else {
                    game.native_item(batch, r.icon, [rect[0] + 4.0, rect[1] + 4.0], 16.0);
                }
                if m::inside(rect, cursor) {
                    hovered = Some(i);
                    outline(game, batch, [rect[0] - 1.0, rect[1] - 1.0, rect[2] + 2.0, rect[3] + 2.0], 0xFF_FFFFFF);
                }
            }

            let hint = "Щелчок — открыть запись · 1–5 науки · Esc — закрыть";
            game.native_text(batch, hint, [x + 6.0, y + h - 11.0], m::RESEARCH_TEXT_DIM, false);

            // Подсказка у курсора — поверх всего.
            if let Some(i) = hovered {
                let lines = m::research_tooltip(mask, i);
                let tw = lines.iter().map(|s| game.native_text_width(s)).fold(0.0, f32::max) + 8.0;
                let th = lines.len() as f32 * 10.0 + 5.0;
                let mut tx = cursor[0] + 10.0;
                if tx + tw > width {
                    tx = cursor[0] - tw - 6.0;
                }
                let ty = (cursor[1] - 14.0).clamp(1.0, (height - th - 1.0).max(1.0));
                game.native_fill(batch, [tx - 1.0, ty - 1.0, tw + 2.0, th + 2.0], m::RESEARCH_TIP_EDGE);
                game.native_fill(batch, [tx, ty, tw, th], m::RESEARCH_TIP_FILL);
                for (k, line) in lines.iter().enumerate() {
                    let color = if k == 0 { m::RESEARCH_TEXT_WHITE } else { m::RESEARCH_TEXT_DIM };
                    game.native_text(batch, line, [tx + 4.0, ty + 3.0 + k as f32 * 10.0], color, k == 0);
                }
            }
        }
        Some((node, page)) => {
            let text = game
                .native_data(m::RESEARCH_PATH)
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .unwrap_or_default();
            let pages = m::research_pages(&text, node);
            let page = page.min(pages.len().saturating_sub(1));
            let [px, py, pw, ph] = l.page;
            game.native_fill(batch, [px - 1.0, py - 1.0, pw + 2.0, ph + 2.0], PAPER_EDGE);
            if !game.native_sprite(batch, "biolab:gui/research/parchment", [0.0, 0.0, 64.0, 64.0], l.page) {
                game.native_fill(batch, l.page, PAPER);
            }
            game.native_item(batch, m::RESEARCH[node].icon, [px + pw - 24.0, py + 6.0], 16.0);

            let left = px + 10.0;
            let text_width = pw - 20.0;
            let bottom = py + ph - 18.0;
            let mut row = py + 8.0;
            for line in m::page_lines(&pages[page]) {
                if row > bottom - 8.0 {
                    break;
                }
                match line {
                    m::Line::Title(title) => {
                        game.native_text(batch, &title, [left, row], TITLE_INK, false);
                        row += 11.0;
                        game.native_fill(batch, [left, row, text_width - 24.0, 1.0], RULE);
                        row += 4.0;
                    }
                    m::Line::Text(text) => {
                        for piece in wrap(game, &text, text_width) {
                            if row > bottom - 8.0 {
                                break;
                            }
                            game.native_text(batch, &piece, [left, row], INK, false);
                            row += 10.0;
                        }
                    }
                    m::Line::Gap => row += 5.0,
                    m::Line::Items(items) => {
                        let mut spot = left;
                        for item in items {
                            if spot + 16.0 > left + text_width {
                                break;
                            }
                            if game.native_item(batch, &item, [spot, row], 16.0) {
                                spot += 19.0;
                            }
                        }
                        row += 19.0;
                    }
                    m::Line::Craft(name) => {
                        let Some(recipe) = game
                            .native_data(&m::recipe_path(&name))
                            .and_then(|bytes| String::from_utf8(bytes).ok())
                            .and_then(|json| m::recipe_view(&json))
                        else {
                            continue;
                        };
                        match recipe {
                            m::RecipeView::Grid { cells, result, count, shapeless } => {
                                if row + 54.0 > bottom + 4.0 {
                                    break;
                                }
                                for (k, cell) in cells.iter().enumerate() {
                                    let at = [left + (k % 3) as f32 * 18.0, row + (k / 3) as f32 * 18.0];
                                    slot(game, batch, at, cell.as_deref(), 1);
                                }
                                game.native_text(batch, "→", [left + 59.0, row + 23.0], INK, false);
                                slot(game, batch, [left + 72.0, row + 18.0], Some(&result), count);
                                if shapeless {
                                    game.native_text(batch, "без формы", [left + 96.0, row + 23.0], NOTE, false);
                                }
                                row += 58.0;
                            }
                            m::RecipeView::Machine { input, machine, result, count, seconds, byproduct } => {
                                if row + 18.0 > bottom + 4.0 {
                                    break;
                                }
                                slot(game, batch, [left, row], Some(&input), 1);
                                game.native_item(batch, &machine, [left + 24.0, row + 1.0], 16.0);
                                let time = format!("{seconds} с →");
                                game.native_text(batch, &time, [left + 43.0, row + 5.0], INK, false);
                                let tw = game.native_text_width(&time);
                                slot(game, batch, [left + 47.0 + tw, row], Some(&result), count);
                                // Побочный выход центрифуги — вторым слотом через «+».
                                if let Some((extra, extra_count)) = byproduct {
                                    let plus = left + 47.0 + tw + 21.0;
                                    game.native_text(batch, "+", [plus, row + 5.0], INK, false);
                                    let plus_width = game.native_text_width("+");
                                    slot(game, batch, [plus + plus_width + 3.0, row], Some(&extra), extra_count);
                                }
                                row += 22.0;
                            }
                        }
                    }
                }
            }

            // Кнопки внизу страницы: «<», «к карте», «>» и номер страницы.
            let [previous, next, back] = m::page_buttons(&l);
            let button = |batch: &mut horizon_client::hud::HudBatch, r: [f32; 4], label: &str, on: bool| {
                if !on {
                    return;
                }
                let lw = game.native_text_width(label);
                let color = if m::inside(r, cursor) { TITLE_INK } else { NOTE };
                game.native_text(batch, label, [r[0] + (r[2] - lw) / 2.0, r[1] + 2.0], color, false);
            };
            button(batch, previous, "<", page > 0);
            button(batch, next, ">", page + 1 < pages.len());
            button(batch, back, "к карте", true);
            if pages.len() > 1 {
                let counter = format!("{}/{}", page + 1, pages.len());
                let cw = game.native_text_width(&counter);
                game.native_text(batch, &counter, [next[0] - cw - 4.0, next[1] + 2.0], NOTE, false);
            }
        }
    }
    ControlFlow::Break(())
}
