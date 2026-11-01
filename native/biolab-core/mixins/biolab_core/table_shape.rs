// Секции операционного стола соединяются в один стол (GDD 4.17). Шаблон —
// `minecraft:mushroom_stem`: сторона `false`, когда за ней соседняя секция,
// `true` — открыта, и по этим свойствам модель рисует ножки, борта и лампу.
// Ванильный `updateShape` гриба ставит `false` при появлении соседа, но
// обратно в `true` сторону не возвращает — сломали соседнюю секцию, а модель
// осталась «склеенной» с пустотой. Поэтому на свои секции отвечаем сами, в
// обе стороны. Хук зовётся часто (шесть раз на каждую запись блока рядом):
// чужой блок отсекается первым же сравнением имени.
fn biolab_core_table_shape(
    level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId,
    direction: horizon_core::geometry::Direction,
    neighbour: horizon_world::registry::BlockStateId,
    new: &mut Option<horizon_world::registry::BlockStateId>,
) {
    use crate::game::biolab_core as m;
    let registry = level.registry;
    let Some(own) = registry.state(state) else {
        return;
    };
    if own.name != m::OPERATING_TABLE {
        return;
    }
    let Some(key) = m::table_side_key(direction.normal()) else {
        return;
    };
    let joined = registry
        .state(neighbour)
        .is_some_and(|n| n.name == m::OPERATING_TABLE);
    let mut next = own.clone();
    next.properties
        .insert(key.to_string(), m::table_side_value(joined).to_string());
    // Не нашлось такого состояния (свойств шаблона не оказалось) — пусть
    // решает ванилла гриба.
    if let Some(id) = registry.id(&next) {
        *new = Some(id);
    }
}
