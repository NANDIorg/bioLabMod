// Батарея-предмет (M10, GDD 4.27 п. 9): буфер `Capabilities.Energy.ITEM`
// (API 307). Заряд — в `custom_data.horizon_energy` стека; читают и пишут
// его хелперы платформы `native_item_energy*` (`biolab_power::click`).
// Ввод и вывод за раз — вся ёмкость: один клик заряжает и отдаёт целиком.
fn biolab_core_battery_buffer(
    game: &crate::game::Game<'_>,
    item: &str,
    buffer: &mut Option<crate::game::ItemEnergyBuffer>,
) {
    use crate::game::biolab_core as m;
    if buffer.is_some() || item != m::BATTERY || !game.native_on(m::ID) {
        return;
    }
    *buffer = Some(crate::game::ItemEnergyBuffer {
        capacity: m::BATTERY_CAPACITY,
        max_insert: m::BATTERY_CAPACITY,
        max_extract: m::BATTERY_CAPACITY,
    });
}
