// Генераторы тиров 0–2 без слотов (M10, GDD 4.27 п. 11): ПКМ пустой рукой —
// строка состояния («Ветряк: 7 Вт·с/тик · заряд 1 200 / 2 400 · высота
// хорошая»), у электрофорной машины — ещё и разряд 200 Вт·с. Клик забираем
// себе (`canceled`): он тогда не уходит ни блоку, ни второй руке.
//
// Хук — `native_right_click_block` (API 21, сервер), отдельная инъекция рядом
// с `harvest.rs` и `protocol_card.rs`. Работа — `crate::game::biolab_gen_early`.
fn biolab_core_gen_click(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    canceled: &mut bool,
) {
    use crate::game::biolab_core as m;
    if *canceled || !game.native_on(m::ID) {
        return;
    }
    let block = game.native_block(pos);
    if !m::early_clickable(&block) {
        return;
    }
    // Только пустой рукой: с предметом в руке клик идёт как обычно
    // (поставить блок рядом, применить предмет).
    let main = game.selected as i32;
    if game.native_carried_stack(main).is_some() {
        return;
    }
    crate::game::biolab_gen_early::click(game, pos, &block);
    *canceled = true;
}
