// Блок машины получает блок-сущность: id и пустой сейв.
fn biolab_core_block_entity(
    block: &str,
    entity: &mut Option<(String, horizon_world::nbt::Compound)>,
) {
    use crate::game::biolab_core as m;
    // Грядка — не машина, но ей тоже нужна блок-сущность: в ней живёт геном
    // растения.
    if (m::is_machine(block) || block == m::GM_PLOT) && entity.is_none() {
        *entity = Some((block.to_string(), Default::default()));
    }
}
