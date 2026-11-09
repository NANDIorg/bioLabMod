// Живые генераторы (M10, GDD 4.27 В): микробный топливный элемент,
// фотобиореактор, глюкозный элемент, электроцитная батарея.
//
// Таблица буферов и правила — раздел «M10: живые генераторы» в `runtime.rs`.
// Выработку класть `crate::game::biolab_power::generate` — в свой буфер;
// из него тянут машины и накопители. Модуль игры
// (`crate::game::biolab_gen_living`), его тик зовёт `power_tick.rs`.
//
// Здесь — исполнение на живой игре: что лежит в слотах, сколько горит
// порция, износ культуры, водорослей и фермента (в `custom_data` предмета),
// свет и погода над фотобиореактором, слоты данных экрана и лампа `lit`.
// Ещё — общий для хуков тип урона током (`ELECTRIC`) и чтение `custom_data`.

use crate::game::biolab_core as m;
use horizon_client::chunk::block_entity::ModBlockEntity;
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

/// Урон током (`data/biolab/damage_type/electric.json`) для мобов: флаги —
/// как теги типа (из них он только в `#minecraft:panic_causes`).
pub static ELECTRIC: crate::game::biolab_effects::DamageType = crate::game::biolab_effects::DamageType {
    id: m::ELECTRIC_DAMAGE,
    bypasses_armor: false,
    no_knockback: false,
    panic_causes: true,
};

/// Сколько тиков ещё горит порция, сколько было в ней, износ с последней
/// записи в предмет, сила гена горящей ткани — в сейве блок-сущности.
const BURN_KEY: &str = "biolab_burn";
const BURN_TOTAL_KEY: &str = "biolab_burn_total";
const WEAR_KEY: &str = "biolab_wear";
const STRENGTH_KEY: &str = "biolab_shock";

/// Строка из `custom_data` стека.
pub fn text_of(stack: &ItemStack, key: &str) -> Option<String> {
    crate::game::biolab_suit::custom_text(stack, key)
}

fn int(entity: &ModBlockEntity, key: &str) -> i32 {
    match entity.nbt.get(&key.into()) {
        Some(Tag::Int(v)) => *v,
        _ => 0,
    }
}

fn set_int(entity: &mut ModBlockEntity, key: &str, value: i32) {
    if int(entity, key) != value || entity.nbt.get(&key.into()).is_none() {
        entity.nbt.insert(key.into(), Tag::Int(value));
    }
}

/// Убрать из слота одну штуку.
fn take_one(entity: &mut ModBlockEntity, slot: usize) {
    if let Some(place) = entity.items.get_mut(slot)
        && let Some(stack) = place.as_mut()
    {
        stack.count = stack.count.saturating_sub(1);
        if stack.count == 0 {
            *place = None;
        }
    }
}

/// Что генератор сделал за тик: на экран и в буфер.
struct Report {
    rate: i32,
    state: m::LivingState,
    life: i32,
    extra: i32,
}

impl Report {
    fn stop(state: m::LivingState) -> Report {
        Report { rate: 0, state, life: -1, extra: 0 }
    }
}

/// Износ того, что живёт в слоте `slot` (культура, водоросли): тик работы
/// прибавляет износ, раз в `WEAR_STEP` он списывается с ресурса в
/// `custom_data`. Ресурс кончился — предмет пропадает. Ответ — ресурс в %.
fn wear(entity: &mut ModBlockEntity, slot: usize, full: i32) -> i32 {
    let worn = int(entity, WEAR_KEY) + 1;
    let Some(Some(stack)) = entity.items.get(slot) else {
        return 0;
    };
    let mut left = m::life_left(text_of(stack, m::LIFE_KEY).as_deref(), full);
    if worn < m::WEAR_STEP {
        set_int(entity, WEAR_KEY, worn);
        return m::life_percent(left, full);
    }
    left -= worn;
    set_int(entity, WEAR_KEY, 0);
    if left <= 0 {
        entity.items[slot] = None;
        return 0;
    }
    if let Some(Some(stack)) = entity.items.get_mut(slot) {
        crate::game::biolab_suit::set_custom_text(stack, m::LIFE_KEY, Some(&left.to_string()));
    }
    m::life_percent(left, full)
}

/// Есть ли место в буфере под тик работы на `rate`.
fn room(entity: &ModBlockEntity, rate: i32) -> bool {
    let cap = m::energy_spec(&entity.id).map_or(0, |s| s.capacity);
    crate::game::biolab_power::stored(entity) + rate <= cap
}

/// МТЭ: культура в слоте 0, органика в слоте 1.
fn microbial(entity: &mut ModBlockEntity) -> Report {
    let strain = match entity.items.first() {
        Some(Some(stack)) if &*stack.item == m::CULTURE_ID => {
            text_of(stack, m::STRAIN_KEY).and_then(|t| m::strain_from_text(&t)).unwrap_or(m::WILD)
        }
        _ => return Report::stop(m::LivingState::NoCulture),
    };
    let rate = m::mfc_output(&strain);
    let full = m::mfc_life(&strain);
    let life = entity.items.first().and_then(|s| s.as_ref()).map_or(full, |s| {
        m::life_left(text_of(s, m::LIFE_KEY).as_deref(), full)
    });
    let percent = m::life_percent(life, full);
    if !room(entity, rate) {
        return Report { rate: 0, state: m::LivingState::Full, life: percent, extra: rate };
    }
    let mut burn = int(entity, BURN_KEY);
    if burn <= 0 {
        let feed = entity.items.get(1).and_then(|s| s.as_ref()).and_then(|s| m::mfc_feed_ticks(&s.item));
        let Some(ticks) = feed else {
            return Report { rate: 0, state: m::LivingState::NoFeed, life: percent, extra: rate };
        };
        take_one(entity, 1);
        burn = ticks;
        set_int(entity, BURN_TOTAL_KEY, ticks);
    }
    set_int(entity, BURN_KEY, burn - 1);
    let percent = wear(entity, 0, full);
    Report { rate, state: m::LivingState::Working, life: percent, extra: rate }
}

/// Фотобиореактор: колба с водорослями, свет неба над блоком.
fn photo(game: &crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) -> Report {
    let (_, sky) = game.native_light([pos[0], pos[1] + 1, pos[2]]);
    let has = matches!(entity.items.first(), Some(Some(s)) if &*s.item == m::ALGAE_FLASK);
    if !has {
        return Report { extra: i32::from(sky), ..Report::stop(m::LivingState::NoAlgae) };
    }
    let life = entity.items.first().and_then(|s| s.as_ref()).map_or(m::ALGAE_LIFE, |s| {
        m::life_left(text_of(s, m::LIFE_KEY).as_deref(), m::ALGAE_LIFE)
    });
    let percent = m::life_percent(life, m::ALGAE_LIFE);
    let day = m::daylight(game.native_day_time());
    let (rain, _) = game.native_weather();
    let rate = m::photo_output(day, sky, rain);
    if rate <= 0 {
        return Report { rate: 0, state: m::LivingState::Dark, life: percent, extra: i32::from(sky) };
    }
    if !room(entity, rate) {
        return Report { rate: 0, state: m::LivingState::Full, life: percent, extra: i32::from(sky) };
    }
    let percent = wear(entity, 0, m::ALGAE_LIFE);
    Report { rate, state: m::LivingState::Working, life: percent, extra: i32::from(sky) }
}

/// Глюкозный элемент: колба глюкозы в слоте 0, фермент в слоте 1. Фермент
/// списывает колбу, когда та начинает гореть.
fn glucose(entity: &mut ModBlockEntity) -> Report {
    let enzyme = entity.items.get(1).and_then(|s| s.as_ref()).filter(|s| &*s.item == m::ENZYME_ID);
    let uses = enzyme.map_or(0, |s| m::life_left(text_of(s, m::LIFE_KEY).as_deref(), m::ENZYME_USES));
    let life = if enzyme.is_some() { m::life_percent(uses, m::ENZYME_USES) } else { -1 };
    let rate = m::GLUCOSE_RATE;
    if !room(entity, rate) {
        return Report { rate: 0, state: m::LivingState::Full, life, extra: uses };
    }
    let mut burn = int(entity, BURN_KEY);
    if burn <= 0 {
        let flask = entity.items.first().and_then(|s| s.as_ref());
        let Some(flask) = flask else {
            return Report { rate: 0, state: m::LivingState::Idle, life, extra: uses };
        };
        if &*flask.item != m::FLASK_ID || text_of(flask, m::CONTENTS_KEY).as_deref() != Some(m::GLUCOSE) {
            return Report { rate: 0, state: m::LivingState::NotGlucose, life, extra: uses };
        }
        if enzyme.is_none() {
            return Report { rate: 0, state: m::LivingState::NoEnzyme, life, extra: uses };
        }
        take_one(entity, 0);
        let left = uses - 1;
        if left <= 0 {
            entity.items[1] = None;
        } else if let Some(Some(stack)) = entity.items.get_mut(1) {
            crate::game::biolab_suit::set_custom_text(stack, m::LIFE_KEY, Some(&left.to_string()));
        }
        burn = m::GLUCOSE_TICKS;
        set_int(entity, BURN_TOTAL_KEY, burn);
    }
    set_int(entity, BURN_KEY, burn - 1);
    let uses = entity
        .items
        .get(1)
        .and_then(|s| s.as_ref())
        .map_or(0, |s| m::life_left(text_of(s, m::LIFE_KEY).as_deref(), m::ENZYME_USES));
    let life = if uses > 0 { m::life_percent(uses, m::ENZYME_USES) } else { -1 };
    Report { rate, state: m::LivingState::Working, life, extra: uses }
}

/// Электроцитная батарея: ткань с геном в слоте 0, горит по силе гена.
fn electrocyte(entity: &mut ModBlockEntity) -> Report {
    let rate = m::ELECTROCYTE_RATE;
    let mut burn = int(entity, BURN_KEY);
    let mut strength = int(entity, STRENGTH_KEY);
    if !room(entity, rate) {
        return Report { rate: 0, state: m::LivingState::Full, life: -1, extra: strength };
    }
    if burn <= 0 {
        let tissue = entity.items.first().and_then(|s| s.as_ref());
        let Some(tissue) = tissue else {
            return Report::stop(m::LivingState::Idle);
        };
        let shock = if &*tissue.item == m::TISSUE_ID {
            m::tissue_electrocytes(text_of(tissue, m::TISSUE_GENOME_KEY).as_deref())
        } else {
            0
        };
        if shock <= 0 {
            return Report::stop(m::LivingState::NoGene);
        }
        take_one(entity, 0);
        burn = m::electrocyte_burn(shock);
        strength = shock;
        set_int(entity, BURN_TOTAL_KEY, burn);
        set_int(entity, STRENGTH_KEY, shock);
    }
    set_int(entity, BURN_KEY, burn - 1);
    Report { rate, state: m::LivingState::Working, life: -1, extra: strength }
}

/// Тик блок-сущности. Не свой блок — сразу выход.
pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let Some(kind) = m::living_kind(&entity.id) else {
        return;
    };
    let slots = m::living_slots(&entity.id);
    if entity.items.len() < slots {
        entity.items.resize(slots, None);
    }
    if entity.data.len() < m::LIVING_DATA_SLOTS {
        entity.data.resize(m::LIVING_DATA_SLOTS, 0);
    }
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let report = match kind {
        m::Living::Microbial => microbial(entity),
        m::Living::Photo => photo(game, pos, entity),
        m::Living::Glucose => glucose(entity),
        m::Living::Electrocyte => electrocyte(entity),
    };
    if report.rate > 0 {
        crate::game::biolab_power::generate(entity, report.rate);
    }
    let working = report.state == m::LivingState::Working;
    let burn = int(entity, BURN_KEY).max(0);
    let reading = m::LivingReading {
        stored: crate::game::biolab_power::stored(entity),
        rate: report.rate,
        state: report.state,
        burn,
        burn_total: if burn > 0 { int(entity, BURN_TOTAL_KEY) } else { 0 },
        life: report.life,
        extra: report.extra,
    };
    // Числа меню не сохраняются: пишем каждый тик, но только изменившиеся —
    // каждая запись в блок-сущность её «меняет».
    let mut data = entity.data.clone();
    m::write_living_reading(&mut data, &reading);
    if data != entity.data {
        entity.data = data;
    }
    // Лампа (модель `_on`): помощник сам ничего не пишет, если уже так.
    game.native_set_block_property(pos, "lit", if working { "true" } else { "false" });
}
