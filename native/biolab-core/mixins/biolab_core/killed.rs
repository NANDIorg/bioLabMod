// Мутанта убили: нестабильный геном оставляет после себя лужу кислоты,
// некроз — заразной слизи. Ткани с убитого больше нет: её срезают
// скальпелем с живого мутанта на операционном столе (GDD 4.17, `inject.rs`).
fn biolab_core_mob_killed(game: &mut crate::game::Game<'_>, entity: u32) {
    use crate::game::biolab_core as m;

    if !game.native_on(m::ID) {
        return;
    }
    let Some(spot) = game.mobs.iter().find(|mob| mob.id == entity).map(|mob| {
        [
            mob.position.x.floor() as i32,
            mob.position.y.floor() as i32,
            mob.position.z.floor() as i32,
        ]
    }) else {
        return;
    };
    // Геном — данные на мобе (M11). Убитый ещё в списке мобов, и данные при
    // нём, пока игра его не убрала.
    let genes = crate::game::biolab_core_data::genes(game, entity);
    if genes.is_empty() {
        return;
    }
    let volatile = m::behaviour_strength(&genes, "volatile") > 0;
    let necrosis = m::behaviour_strength(&genes, "necrosis") > 0;
    if !volatile && !necrosis {
        return;
    }

    // Нестабильный геном оставляет кислоту, некроз — заразную слизь.
    let fluid = if volatile { m::FLUIDS[0].block } else { m::FLUIDS[2].block };
    let dimension = game.dim.id().to_string();
    if let Some(state) = game.pack.assets.registry.default_state(fluid) {
        game.native_set_block(
            &dimension,
            spot,
            state,
            horizon_world::level::flags::UPDATE_ALL,
        );
        game.native_set_block_property(spot, "layers", &m::FLUID_LAYERS.to_string());
    }
}
