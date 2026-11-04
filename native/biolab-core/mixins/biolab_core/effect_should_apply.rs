// Срабатывает ли наш эффект в этот тик (`MobEffect.shouldApplyEffectTickThisTick`,
// стабильный хук API 56) — у игрока и у моба одинаково. Кто носит эффект, хук
// не говорит, поэтому ритм — от часов игры (`game.ticks`), а не от носителя и
// не от остатка длительности (GDD 3.7). Таблица — `biolab_effects`.
fn biolab_core_effect_should_apply(
    game: &crate::game::Game<'_>,
    effect: &str,
    amplifier: i32,
    apply: &mut bool,
) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_effects as fx;
    if !game.native_on(m::ID) {
        return;
    }
    // Только «да»: «нет» — ответ по умолчанию, и чужой мод мог уже сказать
    // своё про свой эффект.
    if fx::acts(effect, game.ticks, amplifier) {
        *apply = true;
    }
}
