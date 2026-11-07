// Кнопка панели M9 пришла на сервер (GDD 4.23 п. 3, 4): `native_menu_button`
// (API 20) — игрок нажал кнопку меню блок-сущности мода (`clickMenuButton`).
// Номер кнопки назвало окно (`panel_click.rs`). Ручка режима — в сейв
// машины, заказ проверки — тоже: саму работу делает тик машины, у которого
// есть мир и папка мира. Зовётся из `lan_station_button` → `mod_menu_button`
// (`lan_stations.rs` пина): одиночная игра — гость своего же сервера, путь
// один, без условия `mod_api::mods()`. Блок-сущность в это время вынута из
// мира, как в тике. Вторая инъекция на этот хук рядом с
// `sequencer_button.rs`.
fn biolab_core_panel_button(
    entity: &mut horizon_client::chunk::block_entity::ModBlockEntity,
    id: i32,
) {
    use crate::game::biolab_core as m;
    use horizon_world::nbt::Tag;
    if m::has_protocol(&entity.id)
        && let Some((knob, up)) = m::proto_press(id)
    {
        let (temperature, hold) = crate::game::biolab_protocol::knobs(entity);
        let (temperature, hold) = match knob {
            m::Knob::Temperature => (m::knob_turn(knob, temperature, up), hold),
            m::Knob::Hold => (temperature, m::knob_turn(knob, hold, up)),
        };
        crate::game::biolab_protocol::set_knobs(entity, temperature, hold);
        return;
    }
    if entity.id == m::SPECTROMETER
        && let Some(probe) = m::probe_press(id)
    {
        // Заказ проверки: прогресс начатой сбрасывается, иначе новая
        // доделалась бы по чужому ходу.
        entity.nbt.insert(m::PROBE_KEY.into(), Tag::Int(probe.id() as i32 + 1));
        let slots = m::machine_data_slots(&entity.id);
        entity.data.resize(slots, 0);
        entity.data[m::DATA_PROGRESS] = 0;
        entity.data[m::DATA_TOTAL] = probe.ticks();
        entity.data[m::DATA_PROBE_RUN] = probe.id() as i32 + 1;
    }
}
