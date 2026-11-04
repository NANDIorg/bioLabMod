// Сигнал компаратора у машины — прогресс работы, 0–15 (GDD 4.15,
// `comparator_signal`). Тик машины кладёт его в таблицу мода по месту и будит
// соседей, только когда ступень сменилась; здесь мы его отдаём. Из
// блок-сущности не читаем: пока машина тикает, она вынута из мира, а соседей
// мы будим как раз из тика.
fn biolab_core_analog_output(
    level: &crate::game::GameLevel<'_>,
    dimension: horizon_world::level::Dimension,
    pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId,
    output: &mut i32,
) {
    use crate::game::biolab_core as m;
    if !level.registry.state(state).is_some_and(|s| m::is_machine(&s.name)) {
        return;
    }
    // Имена измерений — как у `Game::dim.id()`, которым их пишет тик.
    let dimension = match dimension {
        horizon_world::level::Dimension::Nether => "minecraft:the_nether",
        horizon_world::level::Dimension::End => "minecraft:the_end",
        _ => "minecraft:overworld",
    };
    *output = m::known_signal(dimension, [pos.x, pos.y, pos.z]);
}
