// Нажимная пластина (M10, GDD 4.27 Б1): сущность внутри клетки блока мода
// (`Block.entityInside`, хук `native_block_entity_inside`, API 73, сервер) —
// в том числе предмет, упавший на пластину. Как `piezo_step.rs`: только
// записать касание, шаги считает тик пластины.
fn biolab_core_piezo_inside(dimension: &str, entity: i32, position: [i32; 3]) {
    crate::game::biolab_core::note_piezo_contact(dimension, position, entity);
}
