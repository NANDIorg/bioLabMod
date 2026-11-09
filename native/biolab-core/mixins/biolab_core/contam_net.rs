// Сервер прислал заражённость своего чанка и зоны 3×3 (M10,
// `biolab_core:blight`, раз в секунду): для анализатора и перекраски
// (`contam_view.rs`). Стабильный хук API 40.
fn biolab_core_contam_net(channel: &str, data: &[u8]) {
    use crate::game::biolab_core as m;
    if channel != m::NET_BLIGHT {
        return;
    }
    if let Some((chunk, zone)) = m::blight_from_packet(data) {
        m::remember_blight(chunk, zone);
    }
}
