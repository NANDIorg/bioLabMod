// Посадка своих семян на пашню (GDD 4.20). Семя — копия семян свёклы, но игра
// его сама не сажает: какой блок ставит предмет, она узнаёт по блоку, чей это
// предмет, а у наших культур предмета нет (см. `SEEDS_TEMPLATE` в `plants.rs`).
// Поэтому ставим культуру сами — так же, как ставит ванильный предмет-блок:
// в клетку за гранью клика, если она пуста и под ней пашня. Дальше растёт,
// удобряется и вытаптывается шаблон свёклы сам.
fn biolab_core_plant_seed(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face: [i32; 3],
    stack: &horizon_client::items::ItemStack,
    claimed: &mut bool,
) {
    use crate::game::biolab_plants as p;
    // Клик уже забрал соседний перехват (`place_fluid.rs`) — не трогаем.
    if *claimed || !game.native_on(crate::game::biolab_core::ID) {
        return;
    }
    let Some(plant) = p::plant_of_seeds(&stack.item) else {
        return;
    };
    let at = p::plant_target(pos, face);
    let below = [at[0], at[1] - 1, at[2]];
    if !p::can_plant(&game.native_block(at), &game.native_block(below)) {
        return;
    }
    // Возраст 0 — состояние блока по умолчанию (как у шаблона).
    let Some(state) = game.pack.assets.registry.default_state(plant.crop) else {
        return;
    };
    let Some(player) = game.native_player() else {
        return;
    };
    let dimension = player.dimension.clone();
    if !game.native_set_block(&dimension, at, state, horizon_world::level::flags::UPDATE_ALL) {
        return;
    }
    *claimed = true;
    // В выживании семя тратится, в творческом — нет (`consume_held` ваниллы).
    // Рука — основная, если в ней эти семена, иначе левая (-106), как у
    // соседних хелперов.
    if !game.creative {
        let main = game.selected as i32;
        let slot = if game
            .native_carried_stack(main)
            .is_some_and(|held| &*held.item == plant.seeds)
        {
            main
        } else {
            -106
        };
        if let Some(held) = game.native_carried_stack(slot)
            && &*held.item == plant.seeds
        {
            held.count = held.count.saturating_sub(1);
        }
    }
    // Звук посадки — в очередь уровня (API 61). Без своего сервера (чистый
    // клиент) звук не берётся, там выходим. `with_level` — внутренний API
    // игры, как в `place_fluid.rs`: при обновлении Horizon проверить.
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let centre = [f64::from(at[0]) + 0.5, f64::from(at[1]), f64::from(at[2]) + 0.5];
    let _ = game.with_level(|_, level, _| {
        level.native_play_sound(p::PLANT_SOUND, centre, p::SOURCE_BLOCK, 1.0, 1.0)
    });
}
