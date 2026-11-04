// Слоты машины берутся из её строки в таблице: у машин на рецептах это три
// слота печки, у центрифуги — вход и четыре выхода воронки, у синтезатора —
// сетка на девять. Роли слотов, стороны для воронок и меню решает
// `runtime.rs` (раздел «Автоматизация машин»): снизу воронки забирают выход,
// сверху кладут сырьё, сбоку — раствор чану (у остальных тоже сырьё).
// Выходы в `inputs` не попадают, иначе воронка доливала бы готовое.
fn biolab_core_block_entity_container(
    block: &str,
    container: &mut Option<horizon_client::chunk::block_entity::ModContainer>,
) {
    use crate::game::biolab_core as m;
    let Some(machine) = m::machine(block) else {
        return;
    };
    *container = Some(horizon_client::chunk::block_entity::ModContainer {
        slots: machine.slots,
        faces: m::machine_faces(machine),
        inputs: m::machine_inputs(machine),
        outputs: m::machine_outputs(machine),
        // Своё меню мода (`data/biolab/horizon/menu/`): у него выходы
        // закрыты для игрока и работают правила слотов.
        menu: Some(m::machine_menu(machine).to_string()),
    });
}
