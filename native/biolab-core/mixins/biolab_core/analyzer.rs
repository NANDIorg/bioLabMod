// Анализатор крови в руке: показывает заражение в углу экрана.
// Состояние ведёт серверная сторона, а HUD рисует клиентская; в одиночной
// игре это один процесс, поэтому читаем тот же файл мира. Файл перечитываем
// раз в секунду, а не каждый кадр.
fn biolab_core_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    use crate::game::biolab_core as m;
    use horizon_client::extras::Corner;
    if !game.native_on(m::ID) || !game.native_corner_shown(Corner::TopRight) {
        return;
    }
    // Анализатор должен быть в руке. `selected` — выбранный слот хотбара,
    // поле игры: при обновлении Horizon проверить.
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
    let mut lines = m::analyzer_lines_with(
        infection,
        m::shown_journal(),
        &m::shown_blood(),
        &m::shown_strain(),
    );
    // Сколько пропускает надетая спецодежда (M9, GDD 4.25): броня у клиента
    // та же, что прислал сервер.
    lines.extend(m::suit_lines(&crate::game::biolab_suit::worn(game)));
    lines.extend(m::blight_lines_shown()); // M10-Г: заражённость чанка
    let w = lines
        .iter()
        .map(|l| game.native_text_width(l))
        .fold(0.0, f32::max)
        + 6.0;
    let h = 10.0 * lines.len() as f32 + 4.0;
    let (x, y) = corners.place(Corner::TopRight, w, h);
    game.native_fill(batch, [x, y, w, h], m::PANEL_BACKGROUND);
    for (i, line) in lines.iter().enumerate() {
        let colour = if i == 0 { m::PANEL_TITLE } else { m::PANEL_INK };
        game.native_text(
            batch,
            line,
            [x + 3.0, y + 3.0 + 10.0 * i as f32],
            colour,
            true,
        );
    }
}
