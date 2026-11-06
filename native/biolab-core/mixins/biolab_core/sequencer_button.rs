// Кнопка режима секвенатора пришла на сервер (GDD 4.18, п. 3):
// `native_menu_button` (API 20) — игрок нажал кнопку меню блок-сущности мода
// (`clickMenuButton`). Номер кнопки — номер режима (`SeqMode::id`), его шлёт
// окно из `sequencer_click.rs`. Режим — в сейв консоли; прогресс начатой
// работы сбрасывается, число режима на экране меняется сразу, не дожидаясь
// тика. Зовётся из `lan_station_button` → `mod_menu_button` (`lan_stations.rs`
// пина): одиночная игра — гость своего же сервера, путь один, без условия
// `mod_api::mods()`. Блок-сущность в это время вынута из мира, как в тике.
fn biolab_core_sequencer_button(
    entity: &mut horizon_client::chunk::block_entity::ModBlockEntity,
    id: i32,
) {
    use crate::game::biolab_core as m;
    use horizon_world::nbt::Tag;
    if entity.id != m::SEQUENCER {
        return;
    }
    let current = match entity.nbt.get(&m::SEQ_MODE_KEY.into()) {
        Some(Tag::Int(v)) => *v,
        _ => 0,
    };
    let Some(mode) = m::seq_press(current, id) else {
        return;
    };
    entity.nbt.insert(m::SEQ_MODE_KEY.into(), Tag::Int(mode.id()));
    // Число слотов считаем заранее: `resize` берёт `entity.data` изменяемо.
    let slots = m::machine_data_slots(&entity.id);
    entity.data.resize(slots, 0);
    entity.data[m::DATA_PROGRESS] = 0;
    entity.data[m::DATA_TOTAL] = 0;
    entity.data[m::DATA_SEQ_MODE] = mode.id();
}
