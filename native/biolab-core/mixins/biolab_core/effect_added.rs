// Мобу дали наш эффект (`MobEffectEvent.Added`, стабильный хук API 56).
// Седативное снимает цель сразу, не дожидаясь первого тика эффекта: моба,
// которого усыпили зельем или командой, это останавливает на месте.
// (Шприц, дротик и стол дают эффект мимо хука — `mob.add_effect` — и цель
// снимают сами.)
fn biolab_core_effect_added(
    game: &mut crate::game::Game<'_>,
    effect: &str,
    entity: Option<i32>,
) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_effects as fx;
    if !game.native_on(m::ID) || game.remote.is_some() || !fx::calms(effect) {
        return;
    }
    let Some(id) = entity.and_then(|id| u32::try_from(id).ok()) else {
        return;
    };
    if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
        mob.ai.target = None;
    }
}
