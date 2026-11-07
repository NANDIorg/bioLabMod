// Сервер прислал дозу (M9, GDD 4.23 п. 1): процент порога для панели под
// анализатором (`dose_hud.rs`). Шлёт тик дозы (`dose_tick.rs`) раз в секунду.
fn biolab_core_dose_net(channel: &str, data: &[u8]) {
    use crate::game::biolab_core as m;
    if channel != m::NET_DOSE {
        return;
    }
    if let Some(percent) = m::dose_from_packet(data) {
        m::remember_dose(percent);
    }
}
