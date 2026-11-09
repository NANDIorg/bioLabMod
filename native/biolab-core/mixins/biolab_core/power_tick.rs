// Тик блоков энергосети (M10, GDD 4.27): накопители и генераторы. Машины
// тикают своим путём (`machine.rs`) и тратят через `biolab_power::consume`.
// Каждый модуль сам узнаёт свои блоки и выходит на чужих.
fn biolab_core_power_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut horizon_client::chunk::block_entity::ModBlockEntity,
) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) || m::energy_spec(&entity.id).is_none() || m::is_machine(&entity.id) {
        return;
    }
    crate::game::biolab_power::tick(game, pos, entity);
    crate::game::biolab_gen_early::tick(game, pos, entity);
    crate::game::biolab_gen_late::tick(game, pos, entity);
    crate::game::biolab_gen_living::tick(game, pos, entity);
}
