// Слот топлива двигателя-генератора и изотопной батареи (M10, GDD 4.27
// п. 11): один слот, воронка кладёт с любой стороны, забрать нечего. Меню —
// своё (`data/biolab/horizon/menu/<блок>.json`), его правила — в
// `gen_late_menu.rs`, правила воронки — в `gen_late_accepts.rs`. Вторая
// инъекция на `native_block_entity_container` рядом с `machine_slots.rs`:
// чужой контейнер не трогаем. Генераторы — не машины (`is_machine`), иначе
// их не тикал бы `power_tick.rs`.
fn biolab_core_gen_late_slots(
    block: &str,
    container: &mut Option<horizon_client::chunk::block_entity::ModContainer>,
) {
    use crate::game::biolab_core as m;
    if container.is_some() {
        return;
    }
    let slots = m::late_slots(block);
    if slots == 0 {
        return;
    }
    let all: Vec<usize> = (0..slots).collect();
    *container = Some(horizon_client::chunk::block_entity::ModContainer {
        slots,
        faces: std::array::from_fn(|_| all.clone()),
        inputs: all,
        outputs: Vec::new(),
        menu: Some(block.to_string()),
    });
}
