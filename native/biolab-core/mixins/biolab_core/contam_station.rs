// Станция дезактивации (M10, GDD 4.21): ПКМ хлором — заправить, пустой
// рукой — состояние. Работает сама: импульс по чанкам вокруг раз в 10 с
// (тик заражения). Хук ПКМ по блоку (API 21), как `harvest.rs`.
fn biolab_core_contam_station(game: &mut crate::game::Game<'_>, pos: [i32; 3], canceled: &mut bool) {
    use crate::game::biolab_core as m;
    if *canceled || !game.native_on(m::ID) {
        return;
    }
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    if crate::game::biolab_world_contam::station_click(game, pos) {
        *canceled = true;
    }
}
