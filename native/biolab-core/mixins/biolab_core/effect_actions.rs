// Тик игрока выполняет то, что наши эффекты записали в очередь
// (`effect_apply.rs`): там игра была только для чтения, здесь — изменяема,
// моб на месте, а игрок — тот, кто сейчас тикает. Урон и действия — те же,
// что раньше считали миксины `biolab-deep` (`tick.rs`, `mob_tick.rs`).
//
// Почему тик игрока: `native_level_tick_post` в одиночной игре не вызывается
// (GDD 3.8), а тик игрока идёт на сервере каждый тик, на уровне этого
// игрока, сразу после тика его эффектов (`tick_guests`: `guest_server_tick`,
// потом `native_player_tick`). Урон игроку приходит в тот же тик, мобам — не
// позже следующего; ритм от этого не меняется.
fn biolab_core_effect_actions(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_effects as fx;
    use horizon_client::mobs::finalize::{Attr, Op, add_modifier, has_modifier, remove_modifier};
    if !game.native_on(m::ID) || game.remote.is_some() {
        return;
    }
    let due = match fx::state().lock() {
        Ok(mut state) => state.take(game.ticks, game.lan_acting_uuid, game.dim.id()),
        Err(_) => return,
    };
    // Урон игроку — путём урона эффекта, как раньше в `tick.rs`: броня и
    // кадры неуязвимости работают как у ванильного яда.
    for amount in due.hurts {
        game.apply_effect_action(horizon_client::effects::EffectAction::Hurt(
            fx::DAMAGE_TYPE,
            amount,
        ));
    }
    for (id, act) in due.mobs {
        let Some(mob) = game
            .mobs
            .iter_mut()
            .find(|mob| mob.id == id && !mob.removed && mob.alive())
        else {
            continue;
        };
        match act {
            fx::Act::Hurt(amount) => {
                mob.damage(
                    amount,
                    horizon_client::mobs::Damage::of(horizon_client::mobs::DamageKind::Magic),
                );
            }
            // Седативное: моб забывает, кого преследовал. Стоять на месте его
            // заставляет модификатор скорости — он в `biolab-deep`
            // (`mob_tick.rs`): у прибавок к атрибутам стабильного API нет.
            fx::Act::Calm => mob.ai.target = None,
            // Мутаген набрал нагрузку: добавляется ген. Метку вешаем здесь,
            // а прибавки к атрибутам выставит `biolab-deep` (`spawn.rs`) — он
            // замечает, что метки моба изменились. Метки — по три на ген в
            // порядке `GENES` (как в `marker_for`).
            fx::Act::Mutate => {
                let mut strengths = vec![0u8; m::GENES.len()];
                for (i, marker) in m::GENE_MARKERS.iter().enumerate() {
                    if has_modifier(mob, Attr::FollowRange, marker)
                        && let Some(slot) = strengths.get_mut(i / 3)
                    {
                        *slot = (*slot).max((i % 3 + 1) as u8);
                    }
                }
                let seed = fx::mutation_seed(mob.id, u64::from(mob.age));
                let Some((index, strength)) = fx::mutation(&strengths, seed) else {
                    continue;
                };
                let Some(marker) = m::GENE_MARKERS.get(index * 3 + (strength - 1) as usize).copied() else {
                    continue;
                };
                // Усиление: слабая метка того же гена уходит, чтобы ген не
                // числился дважды.
                for weaker in 1..strength {
                    if let Some(old) = m::GENE_MARKERS.get(index * 3 + (weaker - 1) as usize).copied() {
                        remove_modifier(mob, Attr::FollowRange, old);
                    }
                }
                add_modifier(mob, Attr::FollowRange, marker, 0.0, Op::AddValue);
            }
        }
    }
}
