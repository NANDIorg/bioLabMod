// Компаратор читает машины мода (GDD 4.15). Отвечаем по одному состоянию
// блока, как просит хук: у лабораторного стола шаблон верстака, у которого
// своего сигнала нет, у остальных — печь, но и ей ответ нужен наш.
fn biolab_core_has_analog_output(
    level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId,
    output: &mut bool,
) {
    use crate::game::biolab_core as m;
    if level.registry.state(state).is_some_and(|s| m::is_machine(&s.name)) {
        *output = true;
    }
}
