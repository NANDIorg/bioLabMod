// Дротик-снаряд попал (GDD 4.20). Летит он копией снежка — тип сущности мода
// `data/biolab/horizon/entity_type/<дротик>.json`, — и игра сама его бросает,
// ведёт и ломает о блок. Здесь — только то, чего у снежка нет: вещество
// мобу. Что делает попадание, решает `dart_impact` (`runtime.rs`), здесь —
// исполнение. Попадание не отменяем: снежок с уроном 0 толкает моба и
// разбивается, этого дротику и надо. Хук сервера, стабильный API 136.
fn biolab_core_projectile_impact(
    game: &mut crate::game::Game<'_>,
    projectile: u32,
    kind: &str,
    entity: Option<u32>,
    at: [f64; 3],
    canceled: &mut bool,
) {
    use crate::game::biolab_core as m;
    /// Звук в точке `at` всем рядом — как в `remedy.rs`: очередь уровня
    /// (`native_play_sound`, API 25) и пакет от сервера. `with_level` —
    /// внутренний API игры, не хук-поинт: при обновлении Horizon проверить.
    fn play_at(game: &mut crate::game::Game<'_>, cue: crate::game::biolab_core::Cue, at: [f64; 3]) {
        let Some(sound) = crate::game::biolab_core::sound(cue) else {
            return;
        };
        if game.serving.is_none() && game.options.dedicated.is_none() {
            return;
        }
        let _ = game.with_level(|_, level, _| {
            level.native_play_sound(sound.event, at, sound.source, sound.volume, sound.pitch)
        });
    }

    // Чужой мод отменил попадание — снаряд летит дальше, и решать за него
    // нечего: дождёмся следующего.
    if *canceled || !game.native_on(m::ID) || m::dart_projectile(kind).is_none() {
        return;
    }
    // Точку броска забираем при любом попадании, и в блок тоже: снаряд
    // после него пропадает, а запись не должна висеть.
    let from = m::take_dart_launch(projectile);
    // Вещество идёт только живому мобу. Игрок-гость (`minecraft:player`),
    // вагонетка и прочее — не мобы: дротик просто разобьётся о них.
    let target = entity.filter(|id| game.mobs.iter().any(|mob| mob.id == *id && mob.alive()));
    match m::dart_impact(kind, target.is_some(), from, at) {
        m::DartImpact::Nothing => {}
        m::DartImpact::Spent(range) => game.native_chat(&m::dart_spent_message(range)),
        m::DartImpact::Inject(syringe) => {
            let (Some(target), Some((effects, ticks, amplifier))) = (target, m::mob_injection(syringe)) else {
                return;
            };
            let mut name = String::new();
            if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == target) {
                for effect in effects {
                    mob.add_effect(effect, ticks, amplifier);
                }
                name = mob.kind.id.to_string();
            }
            play_at(game, m::Cue::DartHit, at);
            game.native_chat(&m::injection_message(syringe, &name));
            let world = game.world_root.clone();
            m::record_trial(&world, &m::trial_record(syringe, &name));
        }
    }
}
