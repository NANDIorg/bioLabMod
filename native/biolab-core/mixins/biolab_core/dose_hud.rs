// Панель дозы и болезни под анализатором крови (M9, GDD 4.23 п. 1, 8): доза
// в процентах порога, две полосы — сила болезни против иммунитета — и короткий
// прогноз. Рисуется вслед за панелью `analyzer.rs` в том же углу и при том же
// условии (анализатор в руке), поэтому встаёт сразу под ней. Данные — те же,
// что у анализатора (заражение, штамм, кровь из `NET_INFECTION`), и доза из
// `NET_DOSE`; строки и доли считает `dose_rows` в `runtime.rs`.
fn biolab_core_dose_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    use crate::game::biolab_core as m;
    use horizon_client::extras::Corner;
    if !game.native_on(m::ID) || !game.native_corner_shown(Corner::TopRight) {
        return;
    }
    let slot = game.selected as i32;
    let holding = game
        .native_carried_stack(slot)
        .map(|s| s.item.clone())
        .is_some_and(|item| &*item == m::ANALYZER_ID);
    if !holding {
        return;
    }
    let Some(infection) = m::shown_infection() else {
        return;
    };
    let dose = m::shown_dose().unwrap_or(0);
    let rows = m::dose_rows(dose, infection, &m::shown_strain(), &m::shown_blood());
    // Полосы начинаются после самой длинной подписи полосы.
    let label = rows
        .iter()
        .filter(|row| row.bar.is_some())
        .map(|row| game.native_text_width(&row.text))
        .fold(0.0, f32::max);
    let w = rows
        .iter()
        .map(|row| match row.bar {
            Some(_) => label + 4.0 + m::BAR_WIDTH,
            None => game.native_text_width(&row.text),
        })
        .fold(0.0, f32::max)
        + 6.0;
    let h = 10.0 * rows.len() as f32 + 4.0;
    let (x, y) = corners.place(Corner::TopRight, w, h);
    game.native_fill(batch, [x, y, w, h], m::PANEL_BACKGROUND);
    for (i, row) in rows.iter().enumerate() {
        let top = y + 3.0 + 10.0 * i as f32;
        game.native_text(batch, &row.text, [x + 3.0, top], m::PANEL_INK, true);
        if let Some((fraction, colour)) = row.bar {
            let left = x + 3.0 + label + 4.0;
            game.native_fill(batch, [left, top + 1.0, m::BAR_WIDTH, 6.0], m::BAR_TRACK);
            let fill = m::BAR_WIDTH * fraction.clamp(0.0, 1.0);
            if fill > 0.0 {
                game.native_fill(batch, [left, top + 1.0, fill, 6.0], colour);
            }
        }
    }
}
