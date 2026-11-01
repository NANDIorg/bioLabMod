// Слоты машины берутся из её строки в таблице: у машин на рецептах это три
// слота печки, у синтезатора — сетка на девять. Последний слот всегда выход:
// воронки кладут сверху и сбоку, забирают снизу.
fn biolab_core_block_entity_container(
    block: &str,
    container: &mut Option<horizon_client::chunk::block_entity::ModContainer>,
) {
    use crate::game::biolab_core as m;
    let Some(machine) = m::machine(block) else {
        return;
    };
    let output = machine.slots - 1;
    let inputs: Vec<usize> = (0..output).collect();
    *container = Some(horizon_client::chunk::block_entity::ModContainer {
        slots: machine.slots,
        faces: [
            vec![output],
            inputs.clone(),
            inputs.clone(),
            inputs.clone(),
            inputs.clone(),
            inputs.clone(),
        ],
        inputs,
        outputs: vec![output],
        menu: Some(machine.menu.to_string()),
    });
}
