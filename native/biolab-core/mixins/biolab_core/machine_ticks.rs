// Блок-сущность машины тикает.
fn biolab_core_block_entity_ticks(block: &str, ticks: &mut bool) {
    use crate::game::biolab_core as m;
    if m::is_machine(block) || block == m::GM_PLOT {
        *ticks = true;
    }
}
