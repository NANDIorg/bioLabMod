// Аэрозоль снотворного газа (GDD 4.20, M5): ПКМ — облачко эфира или
// хлороформа в паре блоков перед лицом. Себе такой аэрозоль не принимают:
// его нет в `MEDICINES`, поэтому `remedy.rs` на тот же клик ничего не
// делает. Граната здесь не забирается: игра сама бросает её, как снежок
// (свой тип сущности), облако даёт попадание (`grenade_hit.rs`). Второй
// перехват на `native_use_item` (стабильный API 13), как `research_open.rs`.
fn biolab_core_gas_use(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    claimed: &mut bool,
) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) || *claimed {
        return;
    }
    let Some((sleeper, m::GasForm::Spray)) = m::gas_of_item(&stack.item) else {
        return;
    };
    let Some(player) = game.native_player() else {
        return;
    };
    *claimed = true;
    let at = m::spray_point([player.x, player.y, player.z], player.yaw, player.pitch);
    let dimension = player.dimension.to_string();
    crate::game::biolab_sleep::release(game, &dimension, sleeper, at, m::SPRAY_RADIUS, m::SPRAY_TICKS, m::Cue::Spray);
    // Баллончик одноразовый: клик забран, ванильный расход не сработает.
    let main = game.selected as i32;
    let slot = if game.native_carried_stack(main).is_some_and(|held| &*held.item == &*stack.item) {
        main
    } else {
        -106
    };
    if let Some(held) = game.native_carried_stack(slot)
        && &*held.item == &*stack.item
    {
        held.count = held.count.saturating_sub(1);
    }
}
