// Урожай заражённой земли (M10, GDD 4.21, ступень 4): всё, что выпало из
// блока в чанке ступени 4+, загрязнено (M9, `biolab_contam`) штаммом чанка.
// Ещё одна инъекция на хук выпадения (API 75), рядом с `contam_drops.rs`.
// Урожай ГМ-грядки собирается ПКМ, без выпадения, — его пачкает `harvest.rs`.
fn biolab_core_contam_loot(
    game: &mut crate::game::Game<'_>,
    context: &horizon_client::mod_api::NativeLootDrops<'_>,
    drops: &mut Vec<horizon_client::items::ItemStack>,
) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_suit as suit;
    if !game.native_on(m::ID) || drops.is_empty() {
        return;
    }
    let Some((pos, _)) = context.block else {
        return;
    };
    let (stage, strain) = crate::game::biolab_world_contam::stage_at(context.dimension, pos);
    let Some(dirt) = m::blight_harvest_dirt(stage, strain) else {
        return;
    };
    for stack in drops.iter_mut().filter(|stack| &*stack.item != m::CULTURE_ID) {
        let next = m::dirtier(suit::contamination(stack), Some(dirt));
        suit::set_contamination(stack, next);
    }
}
