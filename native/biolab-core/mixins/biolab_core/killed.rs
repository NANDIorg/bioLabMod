// Мутанта убили: нестабильный геном оставляет после себя лужу кислоты,
// некроз — заразной слизи. Ткани с убитого больше нет: её срезают
// скальпелем с живого мутанта на операционном столе (GDD 4.17, `inject.rs`).
fn biolab_core_mob_killed(game: &mut crate::game::Game<'_>, entity: u32) {
    use crate::game::biolab_core as m;
    use horizon_client::mobs::finalize::{Attr, has_modifier};

    if !game.native_on(m::ID) {
        return;
    }
    let Some(mob) = game.mobs.iter().find(|mob| mob.id == entity) else {
        return;
    };
    let genes: Vec<(&'static str, i32)> = m::GENE_MARKERS
        .iter()
        .filter(|marker| has_modifier(mob, Attr::FollowRange, marker))
        .filter_map(|marker| m::gene_of_marker(marker))
        .collect();
    if genes.is_empty() {
        return;
    }
    let spot = [
        mob.position.x.floor() as i32,
        mob.position.y.floor() as i32,
        mob.position.z.floor() as i32,
    ];
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
