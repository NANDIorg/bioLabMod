// Слоты генераторов со слотами (M10, GDD 4.27 п. 11): вольтов столб (цинк,
// медь, кислота, пустая тара) и паровой котёл (топливо, ведро воды, пустые
// вёдра). Своё меню мода (`data/biolab/horizon/menu/<генератор>.json`),
// стороны для воронок — `early_faces` (раздел «M10: генераторы 0–2» в
// `runtime.rs`). Вторая инъекция на `native_block_entity_container` рядом с
// `machine_slots.rs`: генераторы — не машины, тот их не знает.
fn biolab_core_gen_slots(
    block: &str,
    container: &mut Option<horizon_client::chunk::block_entity::ModContainer>,
) {
    use crate::game::biolab_core as m;
    if container.is_some() {
        return;
    }
    let Some(slots) = m::early_slots(block) else {
        return;
    };
    let (faces, inputs, outputs) = m::early_faces(block);
    *container = Some(horizon_client::chunk::block_entity::ModContainer {
        slots,
        faces,
        inputs,
        outputs,
        menu: Some(block.to_string()),
    });
}
