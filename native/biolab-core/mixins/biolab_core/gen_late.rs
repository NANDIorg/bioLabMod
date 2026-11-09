// Генераторы тиров 3–5 (M10, GDD 4.27 Б2): солнечная панель,
// двигатель-генератор, геотермальный съёмник, изотопная батарея.
//
// Таблица буферов и правила — раздел «M10: генераторы 3–5» в `runtime.rs`.
// Выработку класть `crate::game::biolab_power::generate` — в свой буфер;
// из него тянут машины и накопители. Модуль игры
// (`crate::game::biolab_gen_late`), его тик зовёт `power_tick.rs`.
//
// Здесь — исполнение на живой игре: тик блок-сущности (выработка, топливо,
// износ, лампа `lit`, отметка «горячо» `BUSY_KEY`, звук, заражение округи),
// состояние панели и съёмника по ПКМ пустой рукой (`gen_late_click.rs`) и
// доза у изотопной батареи на тике игрока (`gen_late_dose.rs`). Слоты, экран
// и правила слотов двигателя и батареи — `gen_late_slots.rs`,
// `gen_late_screen.rs`, `gen_late_draw.rs`, `gen_late_accepts.rs`,
// `gen_late_menu.rs`.

use crate::game::biolab_core as m;
use crate::game::biolab_power as power;
use horizon_client::chunk::block_entity::ModBlockEntity;
use horizon_world::nbt::Tag;

fn int(entity: &ModBlockEntity, key: &str) -> i32 {
    match entity.nbt.get(&key.into()) {
        Some(Tag::Int(v)) => *v,
        _ => 0,
    }
}

/// Записать число в сейв, только если оно поменялось.
fn set_int(entity: &mut ModBlockEntity, key: &str, value: i32) {
    if !matches!(entity.nbt.get(&key.into()), Some(Tag::Int(v)) if *v == value) {
        entity.nbt.insert(key.into(), Tag::Int(value));
    }
}

/// Ведёт ли эта сторона мир (сервер или окно одиночной игры со своим
/// сервером): только она шлёт звук, кладёт дозу и заражение.
fn serving(game: &crate::game::Game<'_>) -> bool {
    game.serving.is_some() || game.options.dedicated.is_some()
}

/// Звук у блока — как `play` в `bioreactor.rs`: очередь уровня
/// (`native_play_sound`, API 25), сервер рассылает пакетом.
fn play(game: &mut crate::game::Game<'_>, sound: Option<m::Sound>, pos: [i32; 3]) {
    let Some(sound) = sound else {
        return;
    };
    if !serving(game) {
        return;
    }
    let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5];
    let _ = game.with_level(|_, level, _| {
        level.native_play_sound(sound.event, at, sound.source, sound.volume, sound.pitch)
    });
}

/// Отметка «работает» (`BUSY_KEY`: по ней жарко в костюме у двигателя и
/// батареи) и звук смены состояния или такта работы.
fn beat(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity, running: bool, clock: i32) {
    let was = matches!(entity.nbt.get(&m::BUSY_KEY.into()), Some(Tag::Int(1)));
    if was != running {
        entity.nbt.insert(m::BUSY_KEY.into(), Tag::Int(i32::from(running)));
    }
    let id = entity.id.to_string();
    if let Some(phase) = m::gen_beat(&id, was, running, clock) {
        play(game, m::gen_sound(&id, phase), pos);
    }
}

/// Лампа работы — свойство `lit` блока (шаблон печки, модель `_on`).
/// Помощник ничего не пишет, если значение уже такое.
fn lamp(game: &mut crate::game::Game<'_>, pos: [i32; 3], on: bool) {
    game.native_set_block_property(pos, "lit", if on { "true" } else { "false" });
}

/// Тик блок-сущности. Не свой блок — сразу выход.
pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if !m::is_late_generator(&entity.id) {
        return;
    }
    let clock = (int(entity, m::GEN_CLOCK_KEY) + 1).rem_euclid(m::GEN_CLOCK_WRAP);
    entity.nbt.insert(m::GEN_CLOCK_KEY.into(), Tag::Int(clock));
    let id = entity.id.to_string();
    let running = match id.as_str() {
        m::SOLAR_PANEL => Some(solar(game, pos, entity, clock)),
        m::ENGINE_GENERATOR => Some(burner(game, pos, entity, clock, false)),
        m::RTG => Some(burner(game, pos, entity, clock, true)),
        m::GEOTHERMAL_TAP => geothermal(game, pos, entity, clock),
        _ => None,
    };
    // None — съёмник прогорел и блока больше нет.
    if let Some(running) = running {
        lamp(game, pos, running);
    }
}

/// Солнечная панель: раз в `SOLAR_POLL` тиков смотрит на небо над собой,
/// время суток и погоду; до следующего опроса вырабатывает по прошлому.
fn solar(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity, clock: i32) -> bool {
    if clock % m::SOLAR_POLL == 0 || entity.nbt.get(&m::GEN_SKY_KEY.into()).is_none() {
        let (_, sky) = game.native_light([pos[0], pos[1] + 1, pos[2]]);
        let open = m::sky_open(sky);
        let (rain, _) = game.native_weather();
        let rate = m::solar_output(open, game.native_day_time(), rain);
        set_int(entity, m::GEN_SKY_KEY, i32::from(open));
        set_int(entity, m::GEN_RATE_KEY, rate);
    }
    let rate = int(entity, m::GEN_RATE_KEY);
    if rate > 0 {
        power::generate(entity, rate);
    }
    beat(game, pos, entity, rate > 0, clock);
    rate > 0
}

/// Двигатель (`rtg` = false) и изотопная батарея: единица топлива из слота
/// горит своё число тиков. Двигатель глохнет на полном буфере и заводится,
/// когда он опустел до трёх четвертей; батарея распадается всегда.
fn burner(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity, clock: i32, rtg: bool) -> bool {
    if entity.items.len() < m::GEN_SLOTS {
        entity.items.resize(m::GEN_SLOTS, None);
    }
    let id = entity.id.to_string();
    let Some(spec) = m::energy_spec(&id) else {
        return false;
    };
    let was = matches!(entity.nbt.get(&m::BUSY_KEY.into()), Some(Tag::Int(1)));
    let want = rtg || m::engine_wants(was, power::stored(entity), spec.capacity);
    let fuel = entity.items[m::GEN_SLOT_FUEL]
        .as_ref()
        .filter(|s| s.count > 0)
        .and_then(|s| m::late_fuel_ticks(&id, &s.item));
    let burn = m::burn_step(int(entity, m::GEN_FUEL_KEY), int(entity, m::GEN_FUEL_TOTAL_KEY), want, fuel);
    if burn.take
        && let Some(place) = entity.items.get_mut(m::GEN_SLOT_FUEL)
        && let Some(stack) = place.as_mut()
    {
        stack.count = stack.count.saturating_sub(1);
        if stack.count == 0 {
            *place = None;
        }
    }
    set_int(entity, m::GEN_FUEL_KEY, burn.left);
    set_int(entity, m::GEN_FUEL_TOTAL_KEY, burn.total);
    if burn.burning {
        power::generate(entity, spec.rate);
    }
    let rate = if burn.burning { spec.rate } else { 0 };
    set_int(entity, m::GEN_RATE_KEY, rate);
    beat(game, pos, entity, burn.burning, clock);

    // Батарея светит и пачкает округу, пока распадается (4.21).
    if rtg && burn.burning && serving(game) {
        let dimension = game.dim.id();
        if clock % m::RTG_NOTE_EVERY == 0 {
            m::note_rtg(dimension, pos);
        }
        if clock % m::RTG_CONTAMINATE_EVERY == 0 {
            for at in m::rtg_chunk_points(pos) {
                m::queue_world_contamination(dimension, at, m::RTG_CONTAMINATION, None);
            }
        }
    }

    // Числа экрана: меню их не сохраняет, пишем каждый тик.
    let spare = entity.items[m::GEN_SLOT_FUEL].as_ref().map_or(0, |s| s.count);
    let data = m::gen_data(power::stored(entity), spec.capacity, rate, burn.left, burn.total, spare);
    entity.data.resize(m::GEN_DATA_SLOTS, 0);
    entity.data.copy_from_slice(&data);
    burn.burning
}

/// Геотермальный съёмник: лава под ним — паспортная выработка; изнашивается
/// только на тиках, когда заряд вошёл в буфер. Прогорел — блок пропадает:
/// None.
fn geothermal(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity, clock: i32) -> Option<bool> {
    let below = game.native_block([pos[0], pos[1] - 1, pos[2]]);
    let rate = m::geothermal_output(&below);
    set_int(entity, m::GEN_RATE_KEY, rate);
    let put = if rate > 0 { power::generate(entity, rate) } else { 0 };
    if put > 0 {
        let worked = int(entity, m::GEN_WEAR_KEY) + 1;
        set_int(entity, m::GEN_WEAR_KEY, worked);
        match m::geothermal_wear(worked) {
            m::Wear::Worn if int(entity, m::GEN_WARNED_KEY) == 0 => {
                set_int(entity, m::GEN_WARNED_KEY, 1);
                crate::game::biolab_say::say(game, m::GEOTHERMAL_WARN_TEXT);
            }
            m::Wear::Broken if serving(game) => {
                play(game, m::gen_sound(m::GEOTHERMAL_TAP, m::Phase::Done), pos);
                crate::game::biolab_say::say(game, m::GEOTHERMAL_BROKEN_TEXT);
                // Блок пропадает: игра увидит, что тик убрал свою блок-сущность,
                // и выбросит её (`put_back_mod_block_entity`).
                if let Some(air) = game.pack.assets.registry.default_state("minecraft:air") {
                    let dimension = game.dim.id();
                    if game.native_set_block(dimension, pos, air, horizon_world::level::flags::UPDATE_ALL) {
                        return None;
                    }
                }
            }
            _ => {}
        }
    }
    beat(game, pos, entity, rate > 0, clock);
    Some(rate > 0)
}

/// ПКМ пустой рукой по панели или съёмнику — состояние сообщением; клик
/// забираем. Остальные блоки и клик с предметом — мимо.
pub fn click(game: &mut crate::game::Game<'_>, pos: [i32; 3], canceled: &mut bool) {
    let block = game.native_block(pos);
    if block != m::SOLAR_PANEL && block != m::GEOTHERMAL_TAP {
        return;
    }
    let main = game.selected as i32;
    if game.native_carried_stack(main).is_some() {
        return;
    }
    // Читаем блок-сущность напрямую, как `suit.rs`: `native_with_mod_block_entity`
    // пометил бы чанк изменённым.
    let Some(horizon_client::chunk::block_entity::BlockEntity::Mod(e)) =
        game.world.block_entities.get(&(pos[0], pos[1], pos[2]))
    else {
        return;
    };
    let stored = power::stored(e);
    let rate = int(e, m::GEN_RATE_KEY);
    let sky = int(e, m::GEN_SKY_KEY) == 1;
    let worked = int(e, m::GEN_WEAR_KEY);
    let capacity = m::energy_spec(&block).map_or(0, |s| s.capacity);
    let text = if block == m::SOLAR_PANEL {
        m::solar_status(rate, stored, capacity, sky, game.native_day_time())
    } else {
        let below = game.native_block([pos[0], pos[1] - 1, pos[2]]);
        m::geothermal_status(rate, stored, capacity, &below, worked)
    };
    *canceled = true;
    crate::game::biolab_say::say(game, &text);
}

/// Тик игрока: у работающей изотопной батареи — доза по воздуху раз в
/// секунду (режет спецодежда, копит тик дозы).
pub fn player_tick(game: &mut crate::game::Game<'_>) {
    if !serving(game) {
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };
    let uuid = game.native_acting_id();
    if uuid == 0 {
        return;
    }
    if let Some(dose) = m::rtg_dose(uuid, player.dimension, [player.x, player.y, player.z]) {
        m::queue_dose(uuid, m::DoseChannel::Air, dose, None);
    }
}
