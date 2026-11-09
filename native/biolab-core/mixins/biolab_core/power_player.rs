// Аварии сети у игрока (M10, GDD 4.27 п. 10): удар током от короткого
// замыкания и строки о сгоревшем кабеле, обесточенном карантине. Урон своего
// типа (`biolab:electric`) даётся только игроку, на чьём тике идёт хук, —
// поэтому сеть кладёт события в очередь, а бьёт и говорит тик игрока. Тик
// игрока, а не уровня: `native_level_tick_post` в одиночной игре не
// вызывается (GDD 3.8).
fn biolab_core_power_player(game: &mut crate::game::Game<'_>) {
    if !game.native_on(crate::game::biolab_core::ID) {
        return;
    }
    crate::game::biolab_power::player_tick(game);
}
