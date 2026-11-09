// Слоты живых генераторов (M10, GDD 4.27 В): МТЭ и глюкозный — по два
// (живое и корм), фотобиореактор и батарея — по одному. Воронки кладут с
// любой стороны, кроме низа, и ничего не забирают: выхода у генераторов нет,
// топливо выгорает внутри. Что берёт слот — `living_slot_accepts`
// (`gen_living_accepts.rs` для воронок, `gen_living_slot_rules.rs` для
// меню). Своё меню — `data/biolab/horizon/menu/<блок>.json`.
fn biolab_core_living_slots(
    block: &str,
    container: &mut Option<horizon_client::chunk::block_entity::ModContainer>,
) {
    use crate::game::biolab_core as m;
    if container.is_some() || m::living_kind(block).is_none() {
        return;
    }
    let all: Vec<usize> = (0..m::living_slots(block)).collect();
    let mut faces: [Vec<usize>; 6] = std::array::from_fn(|_| all.clone());
    faces[0] = Vec::new();
    *container = Some(horizon_client::chunk::block_entity::ModContainer {
        slots: all.len(),
        faces,
        inputs: all,
        outputs: Vec::new(),
        menu: Some(block.to_string()),
    });
}
