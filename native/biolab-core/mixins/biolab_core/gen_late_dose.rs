// Облучение у изотопной батареи (M10, GDD 4.22, 4.27 Б2): тик игрока раз в
// секунду кладёт дозу по воздуху, если рядом (3 блока) работает батарея.
// Режет спецодежда (`suit_multiplier`), копит тик дозы. Тик игрока, а не
// уровня: `native_level_tick_post` в одиночной игре не вызывается (GDD 3.8).
// Ещё одна инъекция на `native_player_tick` (API 31). Работа —
// `crate::game::biolab_gen_late::player_tick`.
fn biolab_core_gen_late_dose(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) {
        return;
    }
    crate::game::biolab_gen_late::player_tick(game);
}
