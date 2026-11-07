// Съел загрязнённое (M9, GDD 4.23 п. 13): доза по каналу «съеденное» — её
// копит тик дозы (`dose_tick.rs`), костюм тут не помогает. Еда не исчезает и
// не портится — она становится риском. Своя доза гнилой плоти и паучьего
// глаза уже приходит из `remedy.rs` (`tainted_food_dose`) — здесь только
// сверх неё (`contaminated_food_dose`).
//
// Хук `native_living_entity_use_item_finish` (стабильный API 90): игрок
// доел/допил (`entity` — None), `stack` — то, что было в руке до последнего
// укуса, с его `custom_data`. Только сервер.
fn biolab_core_contam_eat(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    entity: Option<i32>,
) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) || entity.is_some() {
        return;
    }
    let Some(c) = crate::game::biolab_suit::contamination(stack) else {
        return;
    };
    let uuid = game.native_acting_id();
    if let Some(dose) = m::contaminated_food_dose(&stack.item, &c) {
        m::queue_dose(uuid, m::DoseChannel::Ingest, dose, c.strain);
    }
    crate::game::biolab_say::say(game, &m::eaten_message(&c));
}
