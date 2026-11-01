// Сервер прислал состояние для анализатора: заражение, журнал и кровь.
fn biolab_core_net_from_server(channel: &str, data: &[u8]) {
    use crate::game::biolab_core as m;
    if channel == m::NET_RESEARCH {
        if let Some(mask) = m::research_from_packet(data) {
            m::remember_research(mask);
        }
        return;
    }
    if channel != m::NET_INFECTION {
        return;
    }
    if let Some(level) = data.first() {
        m::remember_infection(u32::from(*level));
    }
    if let Some(count) = data.get(1) {
        m::remember_journal(u32::from(*count));
    }
    m::remember_blood(m::pharma_from_packet(data));
    m::remember_strain(m::strain_from_packet(data));
}
