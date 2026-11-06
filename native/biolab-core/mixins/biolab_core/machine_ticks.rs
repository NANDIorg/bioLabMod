// Блок-сущность машины тикает. Порт установки — тоже: в первом тике он
// заводит свои слоты (`machine.rs`), иначе воронка и `game_container` видят
// контейнер без единого слота («полон») — его слоты выделял только перенос
// контроллера, и то лишь у выходного порта.
fn biolab_core_block_entity_ticks(block: &str, ticks: &mut bool) {
    use crate::game::biolab_core as m;
    if m::is_machine(block) || block == m::GM_PLOT || m::port(block).is_some() {
        *ticks = true;
    }
}
