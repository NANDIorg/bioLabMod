// Перекраска зоны (M10, GDD 4.27 п. 1): окно по уровню зоны от сервера
// решает, пора ли перекрасить траву, листву и воду (`tint_step`: с запасом у
// границы ступени и не чаще раза в 30 с), и зовёт
// `native_refresh_biome_colors` (API 107) — тот переспрашивает
// `native_biome_colors` (`contam_colors.rs`) и перестраивает секции.
// Стабильный хук `native_tick`; работает только окно.
fn biolab_core_contam_view(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) || !game.native_window() {
        return;
    }
    if game.ticks % 10 != 0 {
        return;
    }
    if m::blight_view_tick().is_some() {
        game.native_refresh_biome_colors();
    }
}
