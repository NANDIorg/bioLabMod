// Блок энергосети сломал игрок (M10, GDD 4.27 п. 10): обходы сетей в кэше
// устарели, а заряженный конденсатор (и банка, слабее) разряжается в
// ломающего. Хук ломания (API 14, сервер, до того, как блок исчез: запас в
// блок-сущности ещё читается), как `multiblock_broken.rs`. Ломание не
// отменяем. Взрыв и поршень этот хук не зовут — кэш тогда переживёт блок
// до `GRID_CACHE_TICKS`.
fn biolab_core_power_broken(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) {
        return;
    }
    // Чужой блок отсекается первым же сравнением имени.
    let name = match game.pack.assets.registry.state(state) {
        Some(s) if m::is_conductor(&s.name) || m::energy_spec(&s.name).is_some() => s.name.to_string(),
        _ => return,
    };
    crate::game::biolab_power::broken(game, pos, &name);
}
