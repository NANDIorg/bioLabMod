// Заражённый (GDD 4.26 А): удары из очереди и болезнь в стаде.
//
// Хук урона (`infected_hurt.rs`) только запоминает удары ближнего боя. Здесь,
// на тике игрока (сервер видит мобов и состояние игрока), решаем:
// * скот, которого ударил заражённый, заболевает (эффект `biolab:infection`
//   на десять минут — его видит пульт);
// * игрок, которого ранил заражённый, получает его штамм через ту же болезнь,
//   что и близость к зомби (`infect_world_player`). Кто ударил игрока, хук в
//   пине не говорит — берём ближайшего охотящегося на игрока в пределах удара,
//   как гадает сама игра (`hurt_player_from`);
// * раз в шаг болезни больной скот заражает соседей вплотную, с шансом по
//   заразности штамма.
// Мобов и предметы меняем только там, где мир настоящий (свой сервер или
// выделенный), как остальные хуки мода.
fn biolab_core_infected_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    use horizon_client_core::who::Who;

    if !game.native_on(m::ID) {
        return;
    }
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }

    // Удары из очереди. Удар по другому игроку (гость LAN) ждёт его тика.
    let hits = m::take_mob_hits();
    if !hits.is_empty() {
        let me = game.player.as_ref().map(|p| p.entity_id);
        let mut later = Vec::new();
        let mut sick = Vec::new();
        let mut player_hit = None;
        for hit in hits {
            if hit.player {
                if Some(hit.victim) == me {
                    player_hit = Some(hit.causing);
                } else {
                    later.push(hit);
                }
                continue;
            }
            let by_infected = game
                .mobs
                .iter()
                .any(|mob| mob.id as i32 == hit.causing && m::is_infected(&mob.kind.id));
            if by_infected {
                sick.push(hit.victim);
            }
        }
        m::requeue_mob_hits(later);
        // В держащей карантинной камере (GDD 4.18 п. 4) не заражаются.
        let dimension = game.dim.id();
        for victim in sick {
            if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id as i32 == victim && mob.alive())
                && !m::in_quarantine_at(dimension, [mob.position.x, mob.position.y, mob.position.z])
            {
                mob.add_effect(m::INFECTION_EFFECT, m::INFECTED_MOB_TICKS, 0);
            }
        }
        if let Some(causing) = player_hit
            && let Some(player) = game.native_player()
            && !m::in_quarantine_at(player.dimension, [player.x, player.y, player.z])
        {
            let near: Vec<(u32, f64)> = game
                .mobs
                .iter()
                .filter(|mob| mob.alive() && mob.ai.target == Some(Who::Player))
                .map(|mob| {
                    let (dx, dy, dz) =
                        (mob.position.x - player.x, mob.position.y - player.y, mob.position.z - player.z);
                    (mob.id, dx * dx + dy * dy + dz * dz)
                })
                .collect();
            let attacker = m::guess_attacker(causing, &near);
            let by_infected = attacker.is_some_and(|id| {
                game.mobs.iter().any(|mob| mob.id == id && m::is_infected(&mob.kind.id))
            });
            if by_infected {
                let uuid = game.native_acting_id();
                let world = game.world_root.clone();
                // Штамм записывается, только если игрок был здоров: с нуля
                // уровень после удара — ровно шаг удара.
                if m::infect_world_player(&world, uuid, &m::INFECTED_STRAIN, m::INFECTED_HIT_STEP)
                    == Some(m::INFECTED_HIT_STEP)
                {
                    crate::game::biolab_say::say(game, m::INFECTED_HIT_MESSAGE);
                }
            }
        }
    }

    // Стадо: больной заражает здорового соседа вплотную.
    if !m::herd_check_due(game.ticks) {
        return;
    }
    let has_infection = |mob: &horizon_client::mobs::Mob| {
        mob.extra
            .effects
            .iter()
            .any(|(id, ticks, _)| &**id == m::INFECTION_EFFECT && *ticks != 0)
    };
    let herd: Vec<(u32, [f64; 3], bool)> = game
        .mobs
        .iter()
        .filter(|mob| mob.alive() && m::catches_infection(&mob.kind.id))
        .map(|mob| (mob.id, [mob.position.x, mob.position.y, mob.position.z], has_infection(mob)))
        .collect();
    let ticks = game.ticks;
    let dimension = game.dim.id();
    let caught: Vec<u32> = herd
        .iter()
        .filter(|(_, _, ill)| !*ill)
        // Здоровый в держащей карантинной камере не заражается.
        .filter(|(_, at, _)| !m::in_quarantine_at(dimension, *at))
        .filter(|(id, at, _)| {
            herd.iter().any(|(other, there, ill)| {
                *ill
                    && (there[0] - at[0]).abs() < m::CONTACT_RANGE
                    && (there[1] - at[1]).abs() < m::CONTACT_RANGE
                    && (there[2] - at[2]).abs() < m::CONTACT_RANGE
                    && m::spreads(&m::INFECTED_STRAIN, m::herd_seed(*other, *id, ticks))
            })
        })
        .map(|(id, _, _)| *id)
        .collect();
    for id in caught {
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
            mob.add_effect(m::INFECTION_EFFECT, m::INFECTED_MOB_TICKS, 0);
        }
    }
}
