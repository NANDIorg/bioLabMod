// Заражённый (GDD 4.26 А): удар моба по игроку или скоту — в очередь.
//
// Хук урона (`native_living_incoming_damage`, API 50) зовётся посреди удара,
// без `self`: игра занята, ни мобов, ни игрока здесь не прочитать. Поэтому
// только запоминаем, кого и кто ударил (`note_mob_hit`), а решает, заразил ли
// удар, тик игрока (`infected_tick.rs`). Ближний удар моба —
// `minecraft:mob_attack`; по скоту хук называет ударившего (`causing`), по
// игроку в пине — нет (0), его угадывает тик игрока.
fn biolab_core_infected_hurt(entity: i32, kind: &str, damage_type: &str, causing: i32) {
    crate::game::biolab_core::note_mob_hit(entity, kind, damage_type, causing);
}
