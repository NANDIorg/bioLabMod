// Порча ползёт на случайном тике (M10, GDD 4.21 «Производительность»):
// игра сама тикает траву и лужи, обхода мира нет. Лужа мутагена и заразной
// слизи — живой источник (вклад в очередь); трава в заражённом чанке —
// кандидат на порчу (блок меняет тик заражения: отсюда уровень даёт менять
// только свойства). Со ступени 4 трава не растёт: ванильный тик отменяем.
// Стабильный хук `random_tick` (контракт cancel), только у своего сервера.
fn biolab_core_contam_spread(
    level: &crate::game::GameLevel<'_>,
    dimension: horizon_world::level::Dimension,
    pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId,
    handled: &mut bool,
) {
    use crate::game::biolab_core as m;
    if *handled {
        return;
    }
    let Some(block) = level.registry.state(state) else {
        return;
    };
    // Имена измерений — как у `Game::dim.id()`, которым их пишет тик.
    let dimension = match dimension {
        horizon_world::level::Dimension::Nether => "minecraft:the_nether",
        horizon_world::level::Dimension::End => "minecraft:the_end",
        _ => "minecraft:overworld",
    };
    if m::blight_random_tick(dimension, [pos.x, pos.y, pos.z], &block.name) {
        *handled = true;
    }
}
