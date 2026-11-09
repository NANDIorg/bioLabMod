// Игрок поставил блок энергосети (M10, GDD 4.27 А): обходы сетей в кэше
// устарели — новый кабель или генератор подхватится со следующего тика, а
// не через `GRID_CACHE_TICKS`. Хук постановки (API 14, сервер) только
// слушает.
fn biolab_core_power_placed(game: &crate::game::Game<'_>, state: horizon_world::registry::BlockStateId) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) {
        return;
    }
    if game
        .pack
        .assets
        .registry
        .state(state)
        .is_some_and(|s| m::is_conductor(&s.name) || m::energy_spec(&s.name).is_some())
    {
        m::bump_grid_revision();
    }
}
