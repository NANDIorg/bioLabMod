// Печь доготовила (M9, GDD 4.23 п. 13): остаток термостойкого штамма,
// запомненный при старте (`cook_start.rs`), готов лечь на выход — его кладёт
// тик костюма (`biolab_suit::tick`). Хук `native_furnace_smelt` (API 229).
fn biolab_core_cook_done(game: &crate::game::Game<'_>, pos: [i32; 3]) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) {
        return;
    }
    if let Ok(mut book) = m::cook_book().lock() {
        book.done(game.dim.id(), pos);
    }
}
