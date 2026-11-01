// Слоты машины берутся из её строки в таблице: у машин на рецептах это три
// слота печки, у центрифуги — вход и четыре выхода воронки, у синтезатора —
// сетка на девять. Какие слоты входы, а какие выходы, решает `runtime.rs`:
// слот топлива печки воронки заполняют только у клонирующего чана.
// Воронки кладут сверху и сбоку, забирают снизу.
fn biolab_core_block_entity_container(
    block: &str,
    container: &mut Option<horizon_client::chunk::block_entity::ModContainer>,
) {
    use crate::game::biolab_core as m;
    let Some(machine) = m::machine(block) else {
        return;
    };
    let inputs = m::machine_inputs(machine);
    let outputs = m::machine_outputs(machine);
    *container = Some(horizon_client::chunk::block_entity::ModContainer {
        slots: machine.slots,
        faces: [
            outputs.clone(),
            inputs.clone(),
            inputs.clone(),
            inputs.clone(),
            inputs.clone(),
            inputs.clone(),
        ],
        inputs,
        outputs,
        menu: Some(machine.menu.to_string()),
    });
}
