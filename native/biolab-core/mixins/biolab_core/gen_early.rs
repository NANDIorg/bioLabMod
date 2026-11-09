// Генераторы тиров 0–2 (M10, GDD 4.27 Б1): электрофорная машина, нажимная
// пластина, вольтов столб, термопара, паровой котёл с динамо, водяное колесо,
// ветрогенератор.
//
// Таблица буферов и правила — раздел «M10: генераторы 0–2» в `runtime.rs`.
// Выработку класть `crate::game::biolab_power::generate` — в свой буфер;
// из него тянут машины и накопители. Модуль игры
// (`crate::game::biolab_gen_early`), его тик зовёт `power_tick.rs`, клик
// пустой рукой — `gen_click.rs`.
//
// Тикающая блок-сущность вынута из мира: свой запас — только через
// `biolab_power::stored/generate`, мир вокруг — хелперами `native_block`,
// `native_block_property`, `native_light`, `native_weather` (API 20, 97).
// Опрос мира — раз в `EARLY_POLL` тиков, итог кэшируется в сейве блока.

use crate::game::biolab_core as m;
use horizon_client::chunk::block_entity::ModBlockEntity;
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

/// Кэш опроса: выработка за тик (у термопары — разница °C) и что тик узнал
/// о месте (`early_status_line`).
const RATE_KEY: &str = "biolab_gen_rate";
const INFO_KEY: &str = "biolab_gen_info";
/// Остаток сотых у термопары.
const CARRY_KEY: &str = "biolab_gen_carry";
/// Время прошлого разряда электрофорной машины.
const LAST_KEY: &str = "biolab_gen_last";
/// Столб: тиков осталось паре пластин и кислоте, полный срок кислоты.
/// Котёл: тиков горения, полный срок, выработка этого топлива, вода в мл.
const FUEL_KEY: &str = "biolab_gen_fuel";
const FUEL_TOTAL_KEY: &str = "biolab_gen_fuel_total";
const TANK_KEY: &str = "biolab_gen_tank";
const TANK_TOTAL_KEY: &str = "biolab_gen_tank_total";
const BURN_RATE_KEY: &str = "biolab_gen_burn_rate";

fn int(entity: &ModBlockEntity, key: &str) -> Option<i32> {
    match entity.nbt.get(&key.into()) {
        Some(Tag::Int(v)) => Some(*v),
        _ => None,
    }
}

fn set_int(entity: &mut ModBlockEntity, key: &str, value: i32) {
    if int(entity, key) != Some(value) {
        entity.nbt.insert(key.into(), Tag::Int(value));
    }
}

fn serving(game: &crate::game::Game<'_>) -> bool {
    game.serving.is_some() || game.options.dedicated.is_some()
}

/// Пора ли опросить мир: раз в `EARLY_POLL` тиков, у соседних блоков —
/// в разные тики.
fn poll_due(now: i64, pos: [i32; 3], entity: &ModBlockEntity) -> bool {
    let shift = i64::from(pos[0].wrapping_mul(7) ^ pos[1].wrapping_mul(3) ^ pos[2].wrapping_mul(13));
    int(entity, RATE_KEY).is_none() || (now + shift).rem_euclid(m::EARLY_POLL) == 0
}

/// Шесть соседей: низ, верх, север, юг, запад, восток.
fn neighbours(pos: [i32; 3]) -> [[i32; 3]; 6] {
    let [x, y, z] = pos;
    [[x, y - 1, z], [x, y + 1, z], [x, y, z - 1], [x, y, z + 1], [x - 1, y, z], [x + 1, y, z]]
}

fn centre(pos: [i32; 3]) -> [f64; 3] {
    [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5]
}

/// Звук в точке (`native_play_sound`, API 25): только у сервера.
fn play(game: &mut crate::game::Game<'_>, sound: &str, at: [f64; 3], volume: f32, pitch: f32) {
    if !serving(game) {
        return;
    }
    let _ = game.with_level(|_, level, _| level.native_play_sound(sound, at, m::SOURCE_BLOCK, volume, pitch));
}

/// Ванильные частицы (`native_send_particles`, API 103): только у сервера.
fn puff(
    game: &mut crate::game::Game<'_>,
    kind: horizon_client::particles::ParticleType,
    at: [f64; 3],
    spread: [f64; 3],
    count: i32,
) {
    if !serving(game) {
        return;
    }
    let options = horizon_client::particles::ParticleOptions::simple(kind);
    let _ = game.with_level(|_, level, _| level.native_send_particles(&options, None, at, spread, 0.02, count));
}

/// Лампа работы: свойство блока (`native_set_block_property`, API 18; сам
/// ничего не пишет, если значение то же).
fn lamp(game: &mut crate::game::Game<'_>, pos: [i32; 3], key: &str, on: bool) {
    game.native_set_block_property(pos, key, if on { "true" } else { "false" });
}

/// Строка из `custom_data` предмета (вещество колбы).
fn custom_text(stack: &ItemStack, key: &str) -> Option<String> {
    let extra = stack.extra.as_ref()?;
    let (_, data) = extra.other.iter().find(|(name, _)| name == "minecraft:custom_data")?;
    match data.get(key) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    }
}

fn item_in(entity: &ModBlockEntity, slot: usize) -> Option<&ItemStack> {
    entity.items.get(slot).and_then(Option::as_ref)
}

/// Убрать из слота один предмет.
fn take_one(entity: &mut ModBlockEntity, slot: usize) {
    if let Some(place) = entity.items.get_mut(slot) {
        if let Some(stack) = place.as_mut() {
            stack.count = stack.count.saturating_sub(1);
            if stack.count == 0 {
                *place = None;
            }
        }
    }
}

/// Влезет ли в слот ещё один `item` (пустая тара без данных).
fn fits(game: &crate::game::Game<'_>, entity: &ModBlockEntity, slot: usize, item: &str) -> bool {
    match entity.items.get(slot) {
        Some(None) => true,
        Some(Some(stack)) => {
            let max = game.native_max_stack_size(item).unwrap_or(1);
            &*stack.item == item && stack.extra.is_none() && stack.count < max
        }
        None => false,
    }
}

/// Положить в слот один `item`; проверять `fits` заранее.
fn put_one(entity: &mut ModBlockEntity, slot: usize, item: &str) {
    if let Some(place) = entity.items.get_mut(slot) {
        match place {
            Some(stack) => stack.count += 1,
            None => *place = Some(ItemStack { item: item.into(), count: 1, damage: 0, extra: None }),
        }
    }
}

/// Тик блок-сущности. Не свой блок — сразу выход.
pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if !m::is_early_generator(&entity.id) || !serving(game) {
        return;
    }
    let now = game.world.game_time;
    let id = entity.id.clone();
    match id.as_str() {
        m::ELECTROPHORE => {
            // Разряд — по клику (`click`); здесь только лампа: искрит
            // полсекунды после разряда.
            let last = match entity.nbt.get(&LAST_KEY.into()) {
                Some(Tag::Long(v)) => Some(*v),
                _ => None,
            };
            let sparking = last.is_some_and(|last| (0..m::ELECTROPHORE_COOLDOWN).contains(&(now - last)));
            lamp(game, pos, "lit", sparking);
        }
        m::PIEZO_PLATE => piezo_tick(game, pos, entity),
        m::VOLTAIC_PILE => pile_tick(game, pos, entity),
        m::THERMOCOUPLE => thermo_tick(game, pos, entity, now),
        m::STEAM_BOILER => boiler_tick(game, pos, entity, now),
        m::WATER_WHEEL => wheel_tick(game, pos, entity, now),
        m::WIND_TURBINE => wind_tick(game, pos, entity, now),
        _ => {}
    }
}

/// Нажимная пластина: новые шаги из книги касаний (`piezo_step.rs`,
/// `piezo_inside.rs`) — по 1 Вт·с; пока кто-то стоит, пластина нажата.
fn piezo_tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let dimension = game.dim.id().to_string();
    let (steps, occupied) = m::take_piezo_steps(&dimension, pos);
    if steps > 0 {
        crate::game::biolab_power::generate(entity, steps as i32 * m::PIEZO_PER_STEP);
        let total = int(entity, INFO_KEY).unwrap_or(0).saturating_add(steps as i32);
        set_int(entity, INFO_KEY, total);
        play(game, "minecraft:block.stone_pressure_plate.click_on", centre(pos), 0.3, 1.2);
    }
    set_int(entity, RATE_KEY, m::PIEZO_PER_STEP);
    lamp(game, pos, "powered", occupied);
}

/// Вольтов столб: пара пластин и кислота горят тик в тик, пока буфер не
/// полон; кончилось — берёт следующие из слотов, пустую тару — в выход.
fn pile_tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if entity.items.len() < m::PILE_SLOTS {
        entity.items.resize(m::PILE_SLOTS, None);
    }
    entity.data.resize(m::EARLY_DATA_SLOTS, 0);
    let capacity = m::energy_spec(&entity.id).map_or(0, |s| s.capacity);
    let mut plates = int(entity, FUEL_KEY).unwrap_or(0);
    let mut acid = int(entity, TANK_KEY).unwrap_or(0);
    let mut acid_total = int(entity, TANK_TOTAL_KEY).unwrap_or(m::PILE_ACID_FLASK_TICKS);
    let mut status = m::EARLY_WORKING;
    let mut rate = 0;
    if crate::game::biolab_power::stored(entity) >= capacity {
        status = m::EARLY_BUFFER_FULL;
    } else {
        if plates <= 0 {
            let zinc = item_in(entity, m::PILE_ZINC).is_some_and(|s| &*s.item == m::ZINC_PLATE);
            let copper = item_in(entity, m::PILE_COPPER).is_some_and(|s| &*s.item == m::COPPER_PLATE);
            if zinc && copper {
                take_one(entity, m::PILE_ZINC);
                take_one(entity, m::PILE_COPPER);
                plates = m::PILE_PLATE_TICKS;
            }
        }
        if acid <= 0 {
            let held = item_in(entity, m::PILE_ACID).map(|s| (String::from(&*s.item), custom_text(s, m::CONTENTS_KEY)));
            match held {
                None => status = m::EARLY_NO_ACID,
                Some((item, contents)) => match m::pile_acid(&item, contents.as_deref()) {
                    None => status = m::EARLY_NOT_ACID,
                    Some((ticks, empty)) if fits(game, entity, m::PILE_EMPTY, empty) => {
                        take_one(entity, m::PILE_ACID);
                        put_one(entity, m::PILE_EMPTY, empty);
                        acid = ticks;
                        acid_total = ticks;
                    }
                    Some(_) => status = m::EARLY_EMPTY_FULL,
                },
            }
        }
        if plates <= 0 {
            status = m::EARLY_NO_PLATES;
        }
        if plates > 0 && acid > 0 {
            crate::game::biolab_power::generate(entity, m::PILE_RATE);
            plates -= 1;
            acid -= 1;
            rate = m::PILE_RATE;
            status = m::EARLY_WORKING;
        }
    }
    set_int(entity, FUEL_KEY, plates);
    set_int(entity, TANK_KEY, acid);
    set_int(entity, TANK_TOTAL_KEY, acid_total);
    let stored = crate::game::biolab_power::stored(entity);
    let data = &mut entity.data;
    data[m::EARLY_DATA_LIT] = i32::from(rate > 0);
    data[m::EARLY_DATA_FUEL] = plates;
    data[m::EARLY_DATA_FUEL_TOTAL] = m::PILE_PLATE_TICKS;
    data[m::EARLY_DATA_TANK] = acid;
    data[m::EARLY_DATA_TANK_TOTAL] = acid_total;
    data[m::EARLY_DATA_ENERGY] = stored;
    data[m::EARLY_DATA_CAPACITY] = capacity;
    data[m::EARLY_DATA_RATE] = rate;
    data[m::EARLY_DATA_STATUS] = status;
    lamp(game, pos, "lit", rate > 0);
}

/// Термопара: раз в секунду — температуры шести соседей; каждый тик —
/// разница в сотых, целые сотни в буфер.
fn thermo_tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity, now: i64) {
    if poll_due(now, pos, entity) {
        let temperatures: Vec<i32> = neighbours(pos).iter().map(|at| m::block_temperature(&game.native_block(*at))).collect();
        let delta = m::thermo_delta(&temperatures);
        set_int(entity, RATE_KEY, delta / m::THERMO_STEP);
        set_int(entity, INFO_KEY, delta);
    }
    let delta = int(entity, INFO_KEY).unwrap_or(0);
    let capacity = m::energy_spec(&entity.id).map_or(0, |s| s.capacity);
    if crate::game::biolab_power::stored(entity) < capacity {
        let (gain, carry) = m::thermo_gain(int(entity, CARRY_KEY).unwrap_or(0), delta);
        if gain > 0 {
            crate::game::biolab_power::generate(entity, gain);
        }
        set_int(entity, CARRY_KEY, carry);
    }
    lamp(game, pos, "lit", delta >= m::THERMO_STEP);
}

/// Паровой котёл: вода из ведра или от источника рядом, топливо печи, динамо
/// сверху. Новое топливо не разжигается, пока буфер полон; начатое догорает
/// (без воды или динамо — впустую).
fn boiler_tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity, now: i64) {
    if entity.items.len() < m::BOILER_SLOTS {
        entity.items.resize(m::BOILER_SLOTS, None);
    }
    entity.data.resize(m::EARLY_DATA_SLOTS, 0);
    let capacity = m::energy_spec(&entity.id).map_or(0, |s| s.capacity);
    let mut water = int(entity, TANK_KEY).unwrap_or(0);
    let mut burn = int(entity, FUEL_KEY).unwrap_or(0);
    let mut burn_total = int(entity, FUEL_TOTAL_KEY).unwrap_or(0);
    let mut burn_rate = int(entity, BURN_RATE_KEY).unwrap_or(m::BOILER_RATE);

    // Раз в секунду: динамо сверху и источник воды вплотную (сбоку или снизу).
    if poll_due(now, pos, entity) {
        let dynamo = game.native_block([pos[0], pos[1] + 1, pos[2]]) == m::DYNAMO;
        let source = neighbours(pos).iter().filter(|at| at[1] <= pos[1]).any(|at| {
            game.native_block(*at) == "minecraft:water"
                && game.native_block_property(*at, "level").as_deref() == Some("0")
        });
        set_int(entity, RATE_KEY, i32::from(dynamo));
        set_int(entity, INFO_KEY, i32::from(source));
    }
    let dynamo = int(entity, RATE_KEY).unwrap_or(0) > 0;
    if int(entity, INFO_KEY).unwrap_or(0) > 0 {
        water = m::BOILER_TANK;
    }
    // Ведро воды — в бак, пустое ведро — в выход.
    let bucket = item_in(entity, m::BOILER_WATER).is_some_and(|s| &*s.item == "minecraft:water_bucket");
    if bucket && water <= m::BOILER_TANK - m::BOILER_BUCKET && fits(game, entity, m::BOILER_EMPTY, "minecraft:bucket") {
        take_one(entity, m::BOILER_WATER);
        put_one(entity, m::BOILER_EMPTY, "minecraft:bucket");
        water += m::BOILER_BUCKET;
    }

    let full = crate::game::biolab_power::stored(entity) >= capacity;
    let mut status = m::EARLY_WORKING;
    if burn <= 0 {
        let fuel = item_in(entity, m::BOILER_FUEL).map(|s| String::from(&*s.item));
        let ticks = fuel.as_deref().map_or(0, m::boiler_fuel_ticks);
        let rest = fuel.as_deref().and_then(m::boiler_fuel_remainder);
        if !dynamo {
            status = m::EARLY_NO_DYNAMO;
        } else if water <= 0 {
            status = m::EARLY_NO_WATER;
        } else if full {
            status = m::EARLY_BUFFER_FULL;
        } else if ticks <= 0 {
            status = m::EARLY_NO_FUEL;
        } else if rest.is_some_and(|rest| !fits(game, entity, m::BOILER_EMPTY, rest)) {
            status = m::EARLY_EMPTY_FULL;
        } else if let Some(fuel) = fuel {
            take_one(entity, m::BOILER_FUEL);
            if let Some(rest) = rest {
                put_one(entity, m::BOILER_EMPTY, rest);
            }
            burn = ticks;
            burn_total = ticks;
            burn_rate = m::boiler_rate(&fuel);
        }
    }
    let mut rate = 0;
    if burn > 0 {
        burn -= 1;
        if dynamo && water > 0 {
            water = (water - m::BOILER_STEAM_PER_TICK).max(0);
            crate::game::biolab_power::generate(entity, burn_rate);
            rate = burn_rate;
            status = m::EARLY_WORKING;
        } else {
            status = if dynamo { m::EARLY_NO_WATER } else { m::EARLY_NO_DYNAMO };
        }
    }
    set_int(entity, TANK_KEY, water);
    set_int(entity, FUEL_KEY, burn);
    set_int(entity, FUEL_TOTAL_KEY, burn_total);
    set_int(entity, BURN_RATE_KEY, burn_rate);
    let stored = crate::game::biolab_power::stored(entity);
    let data = &mut entity.data;
    data[m::EARLY_DATA_LIT] = i32::from(burn > 0);
    data[m::EARLY_DATA_FUEL] = burn;
    data[m::EARLY_DATA_FUEL_TOTAL] = burn_total;
    data[m::EARLY_DATA_TANK] = water;
    data[m::EARLY_DATA_TANK_TOTAL] = m::BOILER_TANK;
    data[m::EARLY_DATA_ENERGY] = stored;
    data[m::EARLY_DATA_CAPACITY] = capacity;
    data[m::EARLY_DATA_RATE] = rate;
    data[m::EARLY_DATA_STATUS] = status;
    lamp(game, pos, "lit", burn > 0);
    // Пар из трубы над динамо: раз в полсекунды, пока крутит.
    if rate > 0 && now % 10 == 0 {
        let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 2.1, f64::from(pos[2]) + 0.5];
        puff(game, horizon_client::particles::ParticleType::WhiteSmoke, at, [0.15, 0.1, 0.15], 3);
    }
}

/// Водяное колесо: текущая вода хоть с одной стороны — 5 Вт·с за тик.
fn wheel_tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity, now: i64) {
    if poll_due(now, pos, entity) {
        let flowing = neighbours(pos)
            .iter()
            .filter(|at| m::is_flowing_water(&game.native_block(**at), game.native_block_property(**at, "level").as_deref()))
            .count() as u32;
        set_int(entity, RATE_KEY, m::wheel_rate(flowing));
        set_int(entity, INFO_KEY, flowing as i32);
    }
    let rate = int(entity, RATE_KEY).unwrap_or(0);
    if rate > 0 {
        crate::game::biolab_power::generate(entity, rate);
    }
    lamp(game, pos, "lit", rate > 0);
}

/// Ветряк: раз в секунду — небо над ним, простор вокруг, высота, гроза и
/// новый порыв.
fn wind_tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity, now: i64) {
    if poll_due(now, pos, entity) {
        let (_, sky) = game.native_light([pos[0], pos[1] + 1, pos[2]]);
        let sky = sky >= 15;
        let probe = m::wind_probe(pos);
        let total = probe.len() as u32;
        let open = probe.iter().filter(|at| m::is_open_air(&game.native_block(**at))).count() as u32;
        let (_, thunder) = game.native_weather();
        let thunder = thunder > 0.5;
        let gust = m::wind_gust(pos, now / m::EARLY_POLL);
        let rate = m::wind_rate(pos[1], open, total, sky, thunder, gust);
        let note = m::wind_note(pos[1], open, total, sky) + if thunder && rate > 0 { m::WIND_THUNDER } else { 0 };
        set_int(entity, RATE_KEY, rate);
        set_int(entity, INFO_KEY, note);
    }
    let rate = int(entity, RATE_KEY).unwrap_or(0);
    if rate > 0 {
        crate::game::biolab_power::generate(entity, rate);
    }
    lamp(game, pos, "lit", rate > 0);
}

/// ПКМ пустой рукой по генератору без слотов (`gen_click.rs`): разряд
/// электрофорной машины или строка состояния по кэшу тика.
pub fn click(game: &mut crate::game::Game<'_>, pos: [i32; 3], block: &str) {
    let now = game.world.game_time;
    if block == m::ELECTROPHORE {
        let discharge = game
            .native_with_mod_block_entity(pos, |entity| {
                let last = match entity.nbt.get(&LAST_KEY.into()) {
                    Some(Tag::Long(v)) => Some(*v),
                    _ => None,
                };
                if !m::electrophore_ready(now, last) {
                    return None;
                }
                entity.nbt.insert(LAST_KEY.into(), Tag::Long(now));
                let put = crate::game::biolab_power::generate(entity, m::ELECTROPHORE_CHARGE);
                Some((put, crate::game::biolab_power::stored(entity)))
            })
            .flatten();
        let Some((put, stored)) = discharge else {
            return;
        };
        let at = centre(pos);
        play(game, "minecraft:entity.firework_rocket.twinkle", at, 0.6, 1.6);
        puff(game, horizon_client::particles::ParticleType::ElectricSpark, [at[0], at[1] + 0.3, at[2]], [0.25, 0.2, 0.25], 12);
        game.native_set_block_property(pos, "lit", "true");
        crate::game::biolab_say::say(game, &m::electrophore_line(put, stored));
        return;
    }
    let state = game.native_with_mod_block_entity(pos, |entity| {
        (crate::game::biolab_power::stored(entity), int(entity, RATE_KEY).unwrap_or(0), int(entity, INFO_KEY).unwrap_or(0))
    });
    let Some((stored, rate, info)) = state else {
        return;
    };
    let line = m::early_status_line(block, stored, rate, info);
    if !line.is_empty() {
        crate::game::biolab_say::say(game, &line);
    }
}
