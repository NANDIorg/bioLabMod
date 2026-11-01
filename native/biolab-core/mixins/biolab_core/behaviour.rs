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
    for (id, at, own, _, genes) in &mobs {
        if genes.is_empty() {
            continue;
        }
        let mut target: Option<Who> = None;
        let mut effects: Vec<(&'static str, i32, i32)> = Vec::new();
        let mut heal = 0.0;

        // Хищник: ближайшее мирное животное становится целью. Мутантом
        // теперь бывает любой вид, и ген могут вколоть корове — сама добыча
        // не охотится (атаки у неё нет, цель её ИИ всё равно бросит), а
        // хищник не охотится на свой вид.
        if m::behaviour_strength(genes, "predator") > 0 && !prey(*own) {
            let mut best: Option<(u32, f64)> = None;
            for (other, spot, species, _, _) in &mobs {
                if other == id || !prey(*species) || species == own {
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

        // Стая: рядом свои — все злее и быстрее. Свои — мутанты того же
        // вида: когда мутировали одни зомби, это выходило само, а теперь
        // мутант-корова зомби в стаю не годится.
        if m::behaviour_strength(genes, "pack") > 0 {
            let friends = mobs
                .iter()
                .filter(|(other, spot, species, marked, _)| {
                    other != id && *marked && species == own && near(*at, *spot, m::PACK_RANGE)
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
    // Пациента операционного стола вольер не берёт: иначе стол и вольер по
    // соседству перетягивали бы его телепортами (GDD 4.17).
    fn on_table(game: &crate::game::Game<'_>, id: u32) -> bool {
        use crate::game::biolab_core as m;
        use horizon_client::mobs::finalize::{Attr, has_modifier};
        game.mobs
            .iter()
            .any(|mob| mob.id == id && has_modifier(mob, Attr::FollowRange, m::SURGERY_TABLE))
    }
    for (id, at, _, _, _) in &mobs {
        if on_table(game, *id) {
            continue;
        }
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
        if game.native_block(pen) != m::PEN || on_table(game, id) {
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

    // Лужи жгут не только игрока: моб, стоящий в луже, получает своё.
    // Раньше лужи действовали лишь на того, кто в них встал ногами игрока.
    for (id, at, _, _, _) in &mobs {
        let under = [
            at[0].floor() as i32,
            at[1].floor() as i32,
            at[2].floor() as i32,
        ];
        let Some(fluid) = m::fluid_of_block(&game.native_block(under)) else {
            continue;
        };
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == *id) {
            mob.add_effect(fluid.effect, 60, 0);
        }
    }

    // Уродства (GDD 4.19): что мутант делает с игроком рядом. Цели ближнего
    // боя закрыты внутри селектора целей игры, поэтому всё — поверх
    // ванильного ИИ: отдельный удар, толчок, ускорение, выдача цели.
    // * дицефалия, многоглазие — враждебный моб замечает игрока в радиусе,
    //   откуда бы тот ни подходил (сзади, крадучись, невидимым);
    // * полимелия — третья рука бьёт сама, вплотную, раз в 2/1,5/1 с;
    // * рогатость — издали разбег (ускорение), вплотную удар рогами с
    //   отбрасыванием;
    // * хвост — хлещет игрока, который стоит у него за спиной;
    // * щупальца — дотягиваются и подтягивают игрока к себе.
    // Удар идёт через ванильный путь урона игрока (`mob_hits_player`): щит,
    // броня, кадры неуязвимости и толчок от источника — как у обычного удара.
    // Откуда бьют (моб стоит там), урон, добавочный толчок прочь. Кто ударил,
    // игра сама находит по точке: ближайший к ней моб.
    let mut strikes: Vec<([f64; 3], f32, f64)> = Vec::new();
    // Откуда тянут и с какой силой.
    let mut pulls: Vec<([f64; 3], f64)> = Vec::new();
    // Кто разгоняется (сила рогов) и кто заметил игрока.
    let mut runs: Vec<(u32, i32)> = Vec::new();
    let mut spotted: Vec<u32> = Vec::new();
    if let Some(spot) = player
        && !game.creative
        && !game.spectator()
    {
        let day = m::daylight(game.world.day_time);
        for (id, at, _, _, genes) in &mobs {
            if genes.is_empty() {
                continue;
            }
            let Some(mob) = game.mobs.iter().find(|mob| mob.id == *id) else {
                continue;
            };
            let flat = ((at[0] - spot[0]).powi(2) + (at[2] - spot[2]).powi(2)).sqrt();
            let full = (flat * flat + (at[1] - spot[1]).powi(2)).sqrt();
            let level = (at[1] - spot[1]).abs() < 2.0;
            let hunting = mob.ai.target == Some(Who::Player);
            // Лояльный не трогает людей, светобоязливый днём не охотится:
            // уродства этого не отменяют.
            let calm = m::behaviour_strength(genes, "loyal") > 0
                || (m::behaviour_strength(genes, "photophobia") > 0 && day);
            if calm {
                continue;
            }
            let watch = m::behaviour_strength(genes, "watchful");
            let sight = m::behaviour_strength(genes, "far_sight");
            if mob.hostile()
                && !hunting
                && ((watch > 0 && full < m::watch_radius(watch))
                    || (sight > 0 && full < m::far_sight_radius(sight)))
            {
                spotted.push(*id);
            }
            let arm = m::behaviour_strength(genes, "extra_arm");
            if arm > 0
                && hunting
                && level
                && flat < m::MELEE_REACH
                && game.ticks % m::extra_arm_period(arm) == 0
            {
                strikes.push((*at, m::extra_arm_damage(mob.attrs.attack_damage), 0.0));
            }
            let horns = m::behaviour_strength(genes, "ram");
            if horns > 0 && hunting {
                if flat > m::RAM_RUN.0 && flat < m::RAM_RUN.1 {
                    runs.push((*id, horns));
                } else if level && flat < m::MELEE_REACH && game.ticks % m::RAM_EVERY == 0 {
                    strikes.push((*at, m::ram_damage(horns), m::ram_push(horns)));
                }
            }
            // Хвост бьёт и не глядя: игрок за спиной враждебного моба или
            // того, кто уже на него охотится.
            let tail = m::behaviour_strength(genes, "tail");
            if tail > 0
                && (hunting || mob.hostile())
                && level
                && flat < m::MELEE_REACH
                && m::behind(mob.body_yaw, [at[0], at[2]], [spot[0], spot[2]])
                && game.ticks % m::TAIL_EVERY == 0
            {
                strikes.push((*at, m::tail_damage(tail), m::tail_push(tail)));
            }
            let grab = m::behaviour_strength(genes, "grab");
            if grab > 0
                && hunting
                && flat > 1.5
                && full < m::grab_reach(grab)
                && game.ticks % m::GRAB_EVERY == 0
            {
                pulls.push((*at, m::grab_pull(grab)));
            }
        }
    }
    for (id, horns) in runs {
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
            mob.add_effect("speed", 20, horns - 1);
        }
    }
    if let Some(spot) = player {
        for (from, amount, push) in strikes {
            let hurt = game.mob_hits_player(amount, from, "minecraft:mob_attack");
            // Толчок прочь от моба: `knock_back_player` толкает против
            // направления (`xd`, `zd`), а оно — от игрока к мобу.
            if hurt && push > 0.0 {
                game.knock_back_player(push, from[0] - spot[0], from[2] - spot[2]);
            }
        }
        for (from, power) in pulls {
            // Направление от моба к игроку: толчок против него — к мобу.
            game.knock_back_player(power, spot[0] - from[0], spot[2] - from[2]);
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
        // Рвущие когти: пока мутант рядом, свежие раны игрока кровоточат.
        // Свежая рана: игрок только что получил урон.
        let wounded = game.player.as_ref().is_some_and(|p| p.health.hurt.hurt_time > 0);
        let rending = wounded
            && mobs.iter().any(|(_, at, _, _, genes)| {
                m::aura_of(m::behaviour_strength(genes, "rend")) > 0.0
                    && near(*at, spot, m::aura_of(m::behaviour_strength(genes, "rend")))
            });
        if rending {
            game.add_player_effect(horizon_client::effects::EffectInstance::new(
                m::BLEEDING,
                m::BLEEDING_TICKS,
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
        } else if spotted.contains(&id) && mob.ai.target.is_none() {
            // Дицефалия, многоглазие: заметил игрока сам.
            mob.ai.target = Some(Who::Player);
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
