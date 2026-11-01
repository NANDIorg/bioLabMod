// Поведение своих эффектов на игроке: кислота, токсикоз и кровотечение бьют
// каждый в своём ритме. Часы для ритма — счётчик тиков самой игры, а не
// остаток длительности эффекта и не собственный счётчик вызовов: остаток
// сбрасывает фармакология (она подновляет эффекты каждую секунду), а эта
// функция вызывается чаще раза в тик — в игре из-за этого токсикоз бил
// впятеро чаще расчётного и убивал за пять секунд.
fn biolab_deep_effect_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_deep as m;
    if !game.native_on(m::ID) {
        return;
    }
    let clock = game.ticks as i64;
    let hits: Vec<f32> = game
        .potions
        .effects
        .list
        .iter()
        .filter(|e| e.duration != 0)
        .filter_map(|e| m::effect_damage(e.id, clock, e.amplifier))
        .collect();
    for amount in hits {
        game.apply_effect_action(horizon_client::effects::EffectAction::Hurt(
            m::DAMAGE_TYPE,
            amount,
        ));
    }
}
