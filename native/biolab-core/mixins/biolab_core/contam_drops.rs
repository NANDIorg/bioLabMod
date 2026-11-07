// Грязная добыча (M9, GDD 4.23 п. 13): всё, что выпало с заражённого, —
// 60 % его штамма; с больного скота (эффект `biolab:infection`) — 40 %;
// всё, что выпало у живой утечки тяжёлого штамма (урожай, блоки, добыча), —
// 30 % её штамма. Культура не пачкается: штамм в ней — её содержимое
// (`infected_drops.rs`). Ещё одна инъекция на хук выпадения (API 75), как
// `plant_drops.rs` и `infected_drops.rs`.
fn biolab_core_contam_drops(
    game: &mut crate::game::Game<'_>,
    context: &horizon_client::mod_api::NativeLootDrops<'_>,
    drops: &mut Vec<horizon_client::items::ItemStack>,
) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_suit as suit;
    if !game.native_on(m::ID) || drops.is_empty() {
        return;
    }
    let (infected, sick) = match context.entity {
        Some((id, kind)) => (
            m::is_infected(kind),
            game.mobs.iter().any(|mob| {
                mob.id == id
                    && mob
                        .extra
                        .effects
                        .iter()
                        .any(|(effect, ticks, _)| &**effect == m::INFECTION_EFFECT && *ticks != 0)
            }),
        ),
        None => (false, false),
    };
    let leak = m::leak_near(context.dimension, context.origin);
    let Some(dirt) = m::drop_contamination(infected, sick, leak) else {
        return;
    };
    for stack in drops.iter_mut().filter(|stack| &*stack.item != m::CULTURE_ID) {
        let next = m::dirtier(suit::contamination(stack), Some(dirt));
        suit::set_contamination(stack, next);
    }
}
