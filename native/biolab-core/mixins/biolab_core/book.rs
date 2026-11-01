// Книга-гид рисуется поверх игры: тёмная подложка, разворот, заголовок,
// строки текста и иконки предметов. Листается клавишами — курсор в игре
// захвачен прицелом, мышью по странице не потыкаешь.
fn biolab_core_book_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
) {
    use crate::game::biolab_core as m;

    /// Разбить строку по ширине страницы: ширину меряет сам шрифт, поэтому
    /// перенос живёт здесь, а не в чистой логике.
    fn wrap(game: &crate::game::Game<'_>, text: &str, width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in text.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
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
        if lines.is_empty() {
            lines.push(String::new());
        }
        lines
    }

    if !game.native_on(m::ID) || !m::book_open() || !game.native_playing() {
        return;
    }

    // Страницы берём из данных мода — те же, что у текстовой книги.
    let text = game
        .native_data(m::MANUAL_PATH)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_default();
    let pages = m::manual_pages(&text);
    if pages.is_empty() {
        return;
    }
    let page = m::book_page().min(pages.len() - 1);

    // Разворот по центру экрана.
    let (width, height) = (batch.width, batch.height);
    let x = ((width - m::BOOK_WIDTH) / 2.0).max(0.0).floor();
    let y = ((height - m::BOOK_HEIGHT) / 2.0).max(0.0).floor();

    // Подложка притемняет мир, чтобы текст читался.
    game.native_fill(batch, [0.0, 0.0, width, height], m::BOOK_SHADE);
    game.native_fill(
        batch,
        [x - 2.0, y - 2.0, m::BOOK_WIDTH + 4.0, m::BOOK_HEIGHT + 4.0],
        m::BOOK_EDGE,
    );
    game.native_fill(batch, [x, y, m::BOOK_WIDTH, m::BOOK_HEIGHT], m::BOOK_PAPER);

    let left = x + m::BOOK_PAD;
    let bottom = y + m::BOOK_HEIGHT - m::BOOK_PAD - m::BOOK_LINE;
    let mut row = y + m::BOOK_PAD;
    for line in m::page_lines(&pages[page]) {
        if row > bottom {
            break;
        }
        match line {
            m::Line::Title(title) => {
                // Заголовок крупнее остального только цветом: свой шрифт нам
                // недоступен, зато чертой под ним страница читается как глава.
                game.native_text(batch, &title, [left, row], m::BOOK_TITLE_INK, false);
                row += m::BOOK_LINE + 2.0;
                game.native_fill(
                    batch,
                    [left, row, m::BOOK_WIDTH - m::BOOK_PAD * 2.0, 1.0],
                    m::BOOK_EDGE,
                );
                row += 4.0;
            }
            m::Line::Text(text) => {
                for piece in wrap(game, &text, m::BOOK_WIDTH - m::BOOK_PAD * 2.0) {
                    if row > bottom {
                        break;
                    }
                    game.native_text(batch, &piece, [left, row], m::BOOK_INK, false);
                    row += m::BOOK_LINE;
                }
            }
            m::Line::Gap => row += m::BOOK_LINE / 2.0,
            m::Line::Items(items) => {
                let mut spot = left;
                for item in items {
                    if game.native_item(batch, &item, [spot, row], m::BOOK_ICON) {
                        spot += m::BOOK_ICON + 4.0;
                    }
                }
                row += m::BOOK_ICON + 3.0;
            }
            m::Line::Craft(item) => {
                // Результат крафта с подписью: сетка ингредиентов — следующий шаг.
                game.native_item(batch, &item, [left, row], m::BOOK_ICON);
                game.native_text(
                    batch,
                    "— крафт",
                    [left + m::BOOK_ICON + 4.0, row + 4.0],
                    m::BOOK_NOTE,
                    false,
                );
                row += m::BOOK_ICON + 3.0;
            }
        }
    }

    let footer = m::book_footer(page, pages.len());
    let footer_width = game.native_text_width(&footer);
    game.native_text(
        batch,
        &footer,
        [
            x + (m::BOOK_WIDTH - footer_width) / 2.0,
            y + m::BOOK_HEIGHT - m::BOOK_PAD,
        ],
        m::BOOK_NOTE,
        false,
    );
}
