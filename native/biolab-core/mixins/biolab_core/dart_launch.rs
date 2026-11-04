// Игрок бросил дротик: запоминаем, откуда. Хук попадания знает только, где
// дротик встретил цель, а дальность броска рукой ограничена (GDD 4.20,
// `DART_THROW_RANGE`). Хук зовётся сразу после броска, пока состояние
// бросавшего на месте (`throw_item` → `add_projectile`), поэтому
// `native_player` — это он, и гость LAN тоже. Хук сервера, стабильный API 293.
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
    m::remember_dart_launch(entity, [thrower.x, thrower.y + 1.62, thrower.z], game.ticks);
}
