// Игрок выпустил дротик: запоминаем, откуда и чем. Хук попадания знает
// только, где дротик встретил цель, а дальность действия у броска рукой и у
// оружия своя (GDD 4.20, `DART_THROW_RANGE`, `DART_WEAPONS`). Хук зовётся
// сразу после запуска, изнутри `add_projectile`, пока состояние бросавшего на
// месте (`throw_item` или выстрел оружия, `weapons.rs`), поэтому
// `native_player` — это он, и гость LAN тоже. Из чего выпущен, хук не знает:
// оружие взводит свою дальность перед `add_projectile`, и `dart_launch_from`
// её забирает; бросок рукой не взводит ничего. Хук сервера, стабильный API 293.
fn biolab_core_projectile_launch(
    game: &crate::game::Game<'_>,
    player: Option<u128>,
    entity: u32,
    kind: &str,
) {
    use crate::game::biolab_core as m;
    if player.is_none() || !game.native_on(m::ID) || m::dart_projectile(kind).is_none() {
        return;
    }
    let Some(thrower) = game.native_player() else {
        return;
    };
    // Снаряд вылетает от глаз (рост глаз игрока — 1,62).
    let launch = m::dart_launch_from([thrower.x, thrower.y + 1.62, thrower.z]);
    m::remember_dart_launch(entity, launch, game.ticks);
}
