// Что слот живого генератора берёт от воронки и выбрасывателя (M10, GDD
// 4.27 В): культуру — только в слот культуры, органику — в слот корма,
// колбу — только с глюкозой, ткань — только с электрическими органами.
// Правило то же, что у меню (`living_slot_accepts`); здесь стек целиком, с
// его `custom_data`. Вторая инъекция на `native_container_accepts` после
// `container_accepts.rs` (тот про чужие блоки молчит). Только сервер.
fn biolab_core_living_accepts(
    game: &crate::game::Game<'_>,
    entity: &horizon_client::chunk::block_entity::ModBlockEntity,
    slot: usize,
    stack: &horizon_client::items::ItemStack,
    cancel: &mut bool,
) {
    use crate::game::biolab_core as m;
    if *cancel || !game.native_on(m::ID) {
        return;
    }
    let data = |key: &str| crate::game::biolab_gen_living::text_of(stack, key);
    if m::living_slot_accepts(&entity.id, slot, &stack.item, &data) == Some(false) {
        *cancel = true;
    }
}
