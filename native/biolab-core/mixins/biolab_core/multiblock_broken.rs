// Контроллер многоблочной установки сломал игрок (GDD 4.18, M8): части,
// которые он склеил (`formed=true`), расклеиваются — иначе без контроллера
// они так и стояли бы «собранными». Сломанную часть расклеивать не нужно:
// контроллер сам заметит нехватку на ближайшей проверке (раз в секунду).
// Хук ломания блока игроком (API 14), на сервере, до того, как блок исчез:
// блок-сущность ещё на месте, её сейв (`biolab_mb_parts`) читается. Ломание
// не отменяем. Зовётся и без трейт-модов (`game.rs` пина,
// `break_block_as_player`: вызов не под `mod_api::mods()`), как и
// `mandrake.rs` на том же хуке. Взрыв и поршень
// его не зовут — тогда части останутся склеенными (известное ограничение).
fn biolab_core_multiblock_broken(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) {
        return;
    }
    // Чужой блок отсекается первым же сравнением имени.
    let controller = game
        .pack
        .assets
        .registry
        .state(state)
        .is_some_and(|s| m::multiblock(&s.name).is_some());
    if controller {
        crate::game::biolab_multiblock::broken(game, pos);
        // Фильтр карантинной камеры: камера больше не держит.
        crate::game::biolab_quarantine::broken(game, pos);
    }
}
