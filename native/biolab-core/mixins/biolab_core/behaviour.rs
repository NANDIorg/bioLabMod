// Тик мира: мутанты ведут себя так, как велит геном. Этот хук видит всех
// мобов сразу, поэтому здесь живёт то, чего не сделать из тика одного моба:
// охота, лояльность, стая и ауры.
fn biolab_core_level_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    use horizon_client::mobs::finalize::{Attr, has_modifier};
    use horizon_client::mobs::species::Species;
    use horizon_client_core::who::Who;

    /// Гены моба по меткам на нём.
    fn genes_of(mob: &horizon_client::mobs::Mob) -> Vec<(&'static str, i32)> {
        use crate::game::biolab_core as m;
        use horizon_client::mobs::finalize::{Attr, has_modifier};
        m::GENE_MARKERS
            .iter()
            .filter(|marker| has_modifier(mob, Attr::FollowRange, marker))
            .filter_map(|marker| m::gene_of_marker(marker))
            .collect()
    }

    if !game.native_on(m::ID) {
        return;
    }
    if game.ticks % m::BEHAVIOUR_EVERY != 0 {
        return;
    }

    // Кто есть кто: собираем один раз, дальше работаем с числами.
    let player = game.native_player().map(|p| [p.x, p.y, p.z]);
    let mobs: Vec<(u32, [f64; 3], Species, bool, Vec<(&'static str, i32)>)> = game
        .mobs
        .iter()
        .filter(|mob| mob.alive())
        .map(|mob| {
            (
                mob.id,
                [mob.position.x, mob.position.y, mob.position.z],
                mob.species(),
                m::GENE_MARKERS
                    .iter()
                    .any(|marker| has_modifier(mob, Attr::FollowRange, marker)),
                genes_of(mob),
            )
        })
        .collect();

    let near = |a: [f64; 3], b: [f64; 3], radius: f64| {
        (a[0] - b[0]).abs() < radius && (a[1] - b[1]).abs() < radius && (a[2] - b[2]).abs() < radius
    };
    // Мирная живность — добыча хищника.
    let prey = |species: Species| {
        matches!(
            species,
            Species::Cow
                | Species::Pig
                | Species::Sheep
                | Species::Chicken
                | Species::Rabbit
                | Species::Goat
                | Species::Mooshroom
        )
    };

    let mut orders: Vec<(u32, Option<Who>, Vec<(&'static str, i32, i32)>, f32)> = Vec::new();
    for (id, at, _, _, genes) in &mobs {
        if genes.is_empty() {
            continue;
        }
        let mut target: Option<Who> = None;
        let mut effects: Vec<(&'static str, i32, i32)> = Vec::new();
        let mut heal = 0.0;

        // Хищник: ближайшее мирное животное становится целью.
        if m::behaviour_strength(genes, "predator") > 0 {
            let mut best: Option<(u32, f64)> = None;
            for (other, spot, species, _, _) in &mobs {
                if other == id || !prey(*species) {
                    continue;
                }
                let away = (spot[0] - at[0]).powi(2)
                    + (spot[1] - at[1]).powi(2)
                    + (spot[2] - at[2]).powi(2);
                if away < m::HUNT_RANGE * m::HUNT_RANGE
                    && best.is_none_or(|(_, far)| away < far)
                {
                    best = Some((*other, away));
                }
            }
            if let Some((victim, _)) = best {
                target = Some(Who::Mob(victim));
            }
        }

        // Стая: рядом свои — все злее и быстрее.
        if m::behaviour_strength(genes, "pack") > 0 {
            let friends = mobs
                .iter()
                .filter(|(other, spot, _, marked, _)| {
                    other != id && *marked && near(*at, *spot, m::PACK_RANGE)
                })
                .count();
            if friends >= m::PACK_NEIGHBOURS {
                effects.push(("speed", 60, 0));
                effects.push(("strength", 60, 0));
            }
        }

        // Светобоязнь: днём мутант не охотится, а мечется.
        if m::behaviour_strength(genes, "photophobia") > 0 && m::daylight(game.world.day_time) {
            target = None;
            effects.push(("speed", 40, 1));
            effects.push(("biolab:sedation", 40, 0));
        }

        // Лечащая аура: мутанты вокруг затягивают раны.
        if m::aura_of(m::behaviour_strength(genes, "heal_aura")) > 0.0 {
            heal = 1.0;
        }
        orders.push((*id, target, effects, heal));
    }

    // Вольеры ищем вокруг самих мобов, а не вокруг игрока: на серверной
    // стороне игрока может не быть вовсе, и тогда клетки «исчезали».
    for (id, at, _, _, _) in &mobs {
        let base = [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32];
        let reach = m::PEN_RANGE as i32;
        let mut found = None;
        for dx in -reach..=reach {
            for dy in -2..=1 {
                for dz in -reach..=reach {
                    let spot = [base[0] + dx, base[1] + dy, base[2] + dz];
                    if game.native_block(spot) == m::PEN {
                        found = Some(spot);
                    }
                }
            }
        }
        if let Some(pen) = found
            && m::subject_of(pen).is_none()
        {
            m::hold_subject(pen, *id);
            game.native_chat("Вольер принял подопытного");
        }
    }

    // Записанный подопытный не уходит: ушедшего возвращает телепорт, потому
    // что скорость ванильный тик моба перезаписывает своей.
    let dimension = game.dim.id().to_string();
    for (pen, id) in m::all_subjects() {
        if game.native_block(pen) != m::PEN {
            m::release_subject(pen);
            continue;
        }
        let centre = m::pen_centre(pen);
        match mobs.iter().find(|(other, _, _, _, _)| *other == id) {
            Some((_, at, _, _, _)) if !near(centre, *at, m::PEN_RANGE) => {
                game.native_teleport_mob(&dimension, id, centre);
            }
            None => m::release_subject(pen),
            _ => {}
        }
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
            mob.ai.target = None;
        }
    }

    // Ауры на игрока: кислота жжёт, холод сковывает.
    if let Some(spot) = player {
        let mut acid = false;
        let mut chill = false;
        for (_, at, _, _, genes) in &mobs {
            let radius = m::aura_of(m::behaviour_strength(genes, "acid_aura"));
            if radius > 0.0 && near(*at, spot, radius) {
                acid = true;
            }
            let radius = m::aura_of(m::behaviour_strength(genes, "chill_aura"));
            if radius > 0.0 && near(*at, spot, radius) {
                chill = true;
            }
        }
        if acid {
            game.add_player_effect(horizon_client::effects::EffectInstance::new(
                m::FLUIDS[0].effect,
                40,
                0,
            ));
        }
        if chill {
            game.add_player_effect(horizon_client::effects::EffectInstance::new(
                "slowness", 40, 0,
            ));
        }
    }

    // Лечащие ауры собираем отдельно: лечится не носитель, а соседи-мутанты.
    let healers: Vec<[f64; 3]> = mobs
        .iter()
        .filter(|(_, _, _, _, genes)| {
            m::aura_of(m::behaviour_strength(genes, "heal_aura")) > 0.0
        })
        .map(|(_, at, _, _, _)| *at)
        .collect();

    for (id, target, effects, _) in orders {
        let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) else {
            continue;
        };
        let genes = genes_of(mob);
        // Лояльный мутант не трогает людей: цель-игрок просто забывается.
        if m::behaviour_strength(&genes, "loyal") > 0 && mob.ai.target == Some(Who::Player) {
            mob.ai.target = None;
        }
        if m::behaviour_strength(&genes, "photophobia") > 0 && m::daylight(game.world.day_time) {
            mob.ai.target = None;
        } else if let Some(who) = target
            && mob.ai.target.is_none()
        {
            mob.ai.target = Some(who);
        }
        for (effect, ticks, amplifier) in effects {
            mob.add_effect(effect, ticks, amplifier);
        }
        let spot = [mob.position.x, mob.position.y, mob.position.z];
        if !genes.is_empty()
            && healers
                .iter()
                .any(|healer| near(*healer, spot, 6.0) && *healer != spot)
        {
            mob.health = (mob.health + 0.5).min(mob.attrs.max_health);
        }
    }
}
