// Снотворные в игре (GDD 4.20, M5): доза мобу по силе против массы, сон
// после задержки, вялость, газовые облака и вспышка эфира.
//
// Правила и числа — в `runtime.rs` (раздел «Снотворные»), там же тесты. Здесь
// — исполнение на живой игре, общее для нескольких хуков: шприц, отвар и
// тряпка (`inject.rs`), дротик (`dart_hit.rs`), граната (`grenade_hit.rs`),
// аэрозоль (`gas_use.rs`), тик (`sleep_tick.rs`), стол (`surgery.rs`). Поэтому
// это модуль игры (`crate::game::biolab_sleep`), как `biolab_core_data`, а
// не вложенные функции хука: у хелпера их не позвать из другого хелпера.
//
// Масса читает гены из данных моба (`biolab_core_data::genes`) — для этого
// нужна вся игра (`&mut Game`), поэтому моба ищем по id каждый раз заново, а
// не держим ссылку на него через вызовы.

use crate::game::biolab_core as m;

/// Звук события мода в точке `at` всем рядом — как в `inject.rs`: очередь
/// уровня (`native_play_sound`, API 25), пакет от сервера. Без своего
/// сервера (клиент чужого мира) звука нет. `with_level` — внутренний API
/// игры, не хук-поинт: при обновлении Horizon проверить.
pub fn play_at(game: &mut crate::game::Game<'_>, cue: m::Cue, at: [f64; 3]) {
    let Some(sound) = m::sound(cue) else {
        return;
    };
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let _ = game.with_level(|_, level, _| level.native_play_sound(sound.event, at, sound.source, sound.volume, sound.pitch));
}

/// Своя частица мода (`native_send_particles`, API 103) — как `puff_at` в
/// `inject.rs`.
pub fn puff_at(game: &mut crate::game::Game<'_>, puff: crate::game::biolab_particles::Puff, at: [f64; 3], seed: u64) {
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let options = horizon_client::particles::ParticleOptions::named(puff.name);
    let _ = game.with_level(|_, level, _| {
        for s in crate::game::biolab_particles::sends(&puff, at, seed) {
            let _ = level.native_send_particles(&options, None, s.at, s.spread, s.speed, s.count);
        }
    });
}

/// Вид и масса живого моба `id`; None — моба нет или он мёртв.
pub fn mass_of(game: &mut crate::game::Game<'_>, id: u32) -> Option<(String, u8)> {
    let (kind, width, height) = game
        .mobs
        .iter()
        .find(|mob| mob.id == id && mob.alive() && !mob.removed)
        .map(|mob| (m::full_kind(&mob.kind.id), mob.kind.width, mob.kind.height))?;
    let genes = crate::game::biolab_core_data::genes(game, id);
    let mass = m::mob_mass(&kind, width, height, &genes);
    Some((kind, mass))
}

/// Середина тела моба — для облака и «вплотную».
fn body_of(game: &crate::game::Game<'_>, id: u32) -> Option<[f64; 3]> {
    game.mobs
        .iter()
        .find(|mob| mob.id == id && mob.alive() && !mob.removed)
        .map(|mob| [mob.position.x, mob.position.y + f64::from(mob.height()) / 2.0, mob.position.z])
}

/// Доза снотворного `s` мобу `id` путём `via`: исход по силе против массы,
/// отложенный сон, вялость, передозировка. `chat` — сказать игроку, что
/// вышло (газу молчим: облако задевает многих разом). None — моба нет.
pub fn dose(
    game: &mut crate::game::Game<'_>,
    id: u32,
    s: &'static m::Sleeper,
    via: m::Via,
    chat: bool,
) -> Option<m::SleepOutcome> {
    let (kind, mass) = mass_of(game, id)?;
    let now = game.ticks;
    let dimension = game.dim.id().to_string();
    let (outcome, repeated) = {
        let mut book = m::sleep_book().lock().ok()?;
        let repeated = book.repeated(id, s.id, now);
        let outcome = m::sleep_outcome(s, mass, repeated);
        match outcome {
            m::SleepOutcome::Takes | m::SleepOutcome::Overdose => {
                book.note_dose(id, s.id, now);
                book.schedule(id, &dimension, s.id, now + m::sleep_delay(s, mass));
            }
            m::SleepOutcome::Drowsy => {
                book.note_dose(id, s.id, now);
                book.make_drowsy(id, now + (s.ticks / 2).max(1) as u64);
            }
            m::SleepOutcome::Woken | m::SleepOutcome::Fatal => book.wake(id),
            m::SleepOutcome::TooWeak => {}
        }
        (outcome, repeated)
    };
    if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
        match outcome {
            // Пробуждающее снимает сон, паралич и замедление вялости.
            m::SleepOutcome::Woken => mob.extra.effects.retain(|(effect, _, _)| {
                *effect != m::SEDATION_EFFECT && *effect != m::PARALYSIS_EFFECT && *effect != "minecraft:slowness"
            }),
            m::SleepOutcome::Drowsy => mob.add_effect("slowness", (s.ticks / 2).max(1), m::DROWSY_SLOWNESS),
            // Передозировка травит сразу; уснёт моб всё равно после задержки.
            m::SleepOutcome::Overdose => {
                let (ticks, amplifier) = m::OVERDOSE_TOXICOSIS;
                mob.add_effect(m::TOXICOSIS_EFFECT, ticks, amplifier);
            }
            // Сильный газ — смерть от яда (тип токсикоза, GDD 4.26, Б): броня
            // не спасает, тотем — да.
            m::SleepOutcome::Fatal => {
                let toxin = crate::game::biolab_effects::damage_of(m::TOXICOSIS_EFFECT);
                mob.damage(1000.0, crate::game::biolab_damage::mob(toxin));
            }
            m::SleepOutcome::Takes | m::SleepOutcome::TooWeak => {}
        }
    }
    if chat {
        crate::game::biolab_say::say(game, &m::sleep_message(s, via, &kind, mass, outcome, repeated));
    }
    Some(outcome)
}

/// Отложенный сон настал: эффект сна (или паралича) и забыть цель.
pub fn fall_asleep(game: &mut crate::game::Game<'_>, id: u32, s: &'static m::Sleeper) {
    let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id && mob.alive() && !mob.removed) else {
        return;
    };
    let effect = match s.kind {
        m::SleepKind::Paralysis => m::PARALYSIS_EFFECT,
        _ => m::SEDATION_EFFECT,
    };
    mob.add_effect(effect, s.ticks, 0);
    mob.ai.target = None;
    if let Ok(mut book) = m::sleep_book().lock() {
        book.fell_asleep(id, s.id);
    }
}

/// Дротик со снотворным попал (`dart_hit.rs`): доза, звук, журнал опытов.
/// false — в шприце дротика не снотворное, пусть хук делает своё.
pub fn dart(game: &mut crate::game::Game<'_>, target: u32, syringe: &str, at: [f64; 3]) -> bool {
    let Some(s) = m::sleeper_of_item(syringe) else {
        return false;
    };
    let kind = game
        .mobs
        .iter()
        .find(|mob| mob.id == target)
        .map(|mob| mob.kind.id.to_string())
        .unwrap_or_default();
    dose(game, target, s, m::Via::Dart, true);
    play_at(game, m::Cue::DartHit, at);
    let world = game.world_root.clone();
    m::record_trial(&world, &m::trial_record(syringe, &kind));
    true
}

/// Достаёт ли действующий игрок до моба `id`: от глаз до середины тела.
pub fn within_reach(game: &crate::game::Game<'_>, id: u32, reach: f64) -> bool {
    let (Some(player), Some(body)) = (game.native_player(), body_of(game, id)) else {
        return false;
    };
    let eye = [player.x, player.y + m::EYE_HEIGHT, player.z];
    let d2: f64 = (0..3).map(|i| (body[i] - eye[i]).powi(2)).sum();
    d2 <= reach * reach
}

/// Газ вышел наружу: облако вещества `s` в точке `at`, частицы и звук.
pub fn release(
    game: &mut crate::game::Game<'_>,
    dimension: &str,
    s: &'static m::Sleeper,
    at: [f64; 3],
    radius: f64,
    ticks: u64,
    cue: m::Cue,
) {
    let until = game.ticks + ticks;
    if let Ok(mut book) = m::sleep_book().lock() {
        book.add_cloud(m::Cloud {
            dimension: dimension.to_string(),
            at,
            radius,
            until,
            sleeper: s.id,
            dosed: Vec::new(),
            players: Vec::new(),
        });
    }
    puff_at(game, crate::game::biolab_particles::BURST, at, until);
    play_at(game, cue, at);
}

/// Есть ли огонь в облаке или у его края (эфир, GDD 4.20): огненный блок
/// или горящий моб. Кубом по радиусу: у гранаты это 7×7×7 блоков раз в
/// `CLOUD_EVERY` тиков.
fn fire_near(game: &crate::game::Game<'_>, cloud: &m::Cloud) -> bool {
    let r = cloud.radius.ceil() as i32;
    let base = cloud.at.map(|v| v.floor() as i32);
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                let pos = [base[0] + dx, base[1] + dy, base[2] + dz];
                let block = game.native_block(pos);
                if block == "minecraft:air" {
                    continue;
                }
                let lit = if block.ends_with("campfire") { game.native_block_property(pos, "lit") } else { None };
                if m::ignites(&block, lit.as_deref()) {
                    return true;
                }
            }
        }
    }
    game.mobs.iter().any(|mob| {
        mob.alive()
            && mob.fire_ticks > 0
            && m::in_cloud(cloud, [mob.position.x, mob.position.y + f64::from(mob.height()) / 2.0, mob.position.z])
    })
}

/// Вспышка эфира: всем в облаке урон огнём и поджог, облако выгорает.
fn flash(game: &mut crate::game::Game<'_>, cloud: &m::Cloud) {
    let inside: Vec<u32> = game
        .mobs
        .iter()
        .filter(|mob| {
            mob.alive()
                && !mob.removed
                && m::in_cloud(cloud, [mob.position.x, mob.position.y + f64::from(mob.height()) / 2.0, mob.position.z])
        })
        .map(|mob| mob.id)
        .collect();
    for id in inside {
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
            mob.damage(m::FLASH_DAMAGE, horizon_client::mobs::Damage::of(horizon_client::mobs::DamageKind::InFire));
            if !mob.kind.flags.fire_immune {
                mob.fire_ticks = mob.fire_ticks.max(m::FLASH_FIRE_TICKS);
            }
        }
    }
    // Игрок, на чьём тике идёт проход, — тем же путём, что урон эффектов.
    if let Some(player) = game.native_player()
        && m::in_cloud(cloud, [player.x, player.y + 0.9, player.z])
    {
        game.apply_effect_action(horizon_client::effects::EffectAction::Hurt("minecraft:in_fire", m::FLASH_DAMAGE));
    }
    if game.serving.is_some() || game.options.dedicated.is_some() {
        let options = horizon_client::particles::ParticleOptions::simple(horizon_client::particles::ParticleType::Flame);
        let spread = [cloud.radius * 0.5, cloud.radius * 0.4, cloud.radius * 0.5];
        let at = cloud.at;
        let _ = game.with_level(|_, level, _| level.native_send_particles(&options, None, at, spread, 0.05, 40));
    }
    play_at(game, m::Cue::GasFlash, cloud.at);
    if let Ok(mut book) = m::sleep_book().lock() {
        book.remove_cloud(cloud.at, cloud.until);
    }
}

/// Тик снотворных на тике игрока (`sleep_tick.rs`). Часть уровня — раз за
/// тик на уровень: отложенный сон, вялость, облака по мобам. Часть игрока —
/// на тике каждого: вдох газа без противогаза.
pub fn tick(game: &mut crate::game::Game<'_>) {
    let now = game.ticks;
    let dimension = game.dim.id().to_string();
    if m::sleep_pass_due(now, &dimension) {
        level_tick(game, now, &dimension);
    }
    if now % m::CLOUD_EVERY == 0 {
        player_tick(game, now, &dimension);
    }
}

fn level_tick(game: &mut crate::game::Game<'_>, now: u64, dimension: &str) {
    // Сон, чья задержка вышла.
    let (due, drowsy, clouds) = {
        let Ok(mut book) = m::sleep_book().lock() else {
            return;
        };
        let due = book.take_due(now, dimension);
        let drowsy = book.drowsy_now(now);
        let clouds: Vec<m::Cloud> = book.clouds_now(now).iter().filter(|c| c.dimension == dimension).cloned().collect();
        (due, drowsy, clouds)
    };
    for p in due {
        if let Some(s) = m::sleeper(p.sleeper) {
            fall_asleep(game, p.mob, s);
        }
    }

    // Вялый не нападает издалека: цель-игрока дальше `DROWSY_REACH` бросает.
    if !drowsy.is_empty()
        && let Some(player) = game.native_player()
    {
        let me = [player.x, player.y, player.z];
        for mob in game.mobs.iter_mut().filter(|mob| drowsy.contains(&mob.id)) {
            let far = (0..3)
                .map(|i| ([mob.position.x, mob.position.y, mob.position.z][i] - me[i]).powi(2))
                .sum::<f64>()
                > m::DROWSY_REACH * m::DROWSY_REACH;
            if far && matches!(mob.ai.target, Some(horizon_client::mobs::Who::Player)) {
                mob.ai.target = None;
            }
        }
    }

    if now % m::CLOUD_EVERY != 0 {
        return;
    }
    for cloud in clouds {
        let Some(s) = m::sleeper(cloud.sleeper) else {
            continue;
        };
        // Клубы по всему шару — пока облако висит.
        let puff = crate::game::biolab_particles::cloud_puff(cloud.radius);
        puff_at(game, puff, cloud.at, now ^ cloud.until);
        // Эфир у огня — вспышка, и облака больше нет.
        if m::flammable(s) && fire_near(game, &cloud) {
            flash(game, &cloud);
            continue;
        }
        // Мобы в облаке, кому оно ещё не давало дозу.
        let inside: Vec<u32> = game
            .mobs
            .iter()
            .filter(|mob| {
                mob.alive()
                    && !mob.removed
                    && !cloud.dosed.contains(&mob.id)
                    && m::in_cloud(&cloud, [mob.position.x, mob.position.y + f64::from(mob.height()) / 2.0, mob.position.z])
            })
            .map(|mob| mob.id)
            .collect();
        for id in inside {
            dose(game, id, s, m::Via::Gas, false);
            if let Ok(mut book) = m::sleep_book().lock() {
                book.mark_cloud(cloud.at, cloud.until, Some(id), None);
            }
        }
    }
}

/// Игрок в облаке без противогаза вдыхает дозу — раз за облако.
fn player_tick(game: &mut crate::game::Game<'_>, now: u64, dimension: &str) {
    let Some(player) = game.native_player() else {
        return;
    };
    let body = [player.x, player.y + 0.9, player.z];
    let uuid = game.native_acting_id();
    let clouds: Vec<m::Cloud> = match m::sleep_book().lock() {
        Ok(mut book) => book
            .clouds_now(now)
            .iter()
            .filter(|c| c.dimension == dimension && !c.players.contains(&uuid) && m::in_cloud(c, body))
            .cloned()
            .collect(),
        Err(_) => return,
    };
    if clouds.is_empty() {
        return;
    }
    // Голова — слот 103 (`native_carried_slot`: 100–103 — броня, 103 — шлем).
    let head = game.native_carried_stack(103).map(|stack| stack.item.to_string());
    let protected = m::gas_protected(head.as_deref());
    let world = game.world_root.clone();
    let resistance = m::world_player_gene(&world, uuid, m::RESISTANCE_GENE);
    for cloud in clouds {
        if let Ok(mut book) = m::sleep_book().lock() {
            book.mark_cloud(cloud.at, cloud.until, None, Some(uuid));
        }
        let Some(s) = m::sleeper(cloud.sleeper) else {
            continue;
        };
        if protected {
            continue;
        }
        let dose = m::gas_dose(s, resistance);
        if dose == 0 {
            continue;
        }
        if let Some(state) = m::take_gas(&world, uuid, s.id, dose) {
            crate::game::biolab_say::say(game, &m::gas_message(s, &state));
        }
    }
}
