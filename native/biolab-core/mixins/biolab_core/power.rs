// Энергосеть на живой игре (M10, GDD 4.22, 4.27).
//
// Правила и таблицы — разделы «M10: контракт» и «M10: энергосеть» в
// `runtime.rs`, там же тесты. Здесь — исполнение, общее для нескольких
// хуков: запас блок-сущности, выработка генераторов, расход машин, добор из
// сети. Модуль игры (`crate::game::biolab_power`), как `biolab_suit`.
//
// Запас лежит в сейве блок-сущности целым `energy` — там же его читают
// хелперы платформы `native_energy*`. Тикающая блок-сущность вынута из мира,
// хелперы её не видят: свой запас блок читает и пишет здесь, напрямую.
//
// Сигнатуры `stored`, `set_stored`, `generate`, `consume` — контракт для
// генераторов и машин, не менять. Тела `pull` и `tick` — агента «Сеть».

use crate::game::biolab_core as m;
use horizon_client::chunk::block_entity::ModBlockEntity;
use horizon_world::nbt::Tag;

/// Запас энергии блок-сущности, Вт·с.
pub fn stored(entity: &ModBlockEntity) -> i32 {
    match entity.nbt.get(&m::ENERGY_KEY.into()) {
        Some(Tag::Int(energy)) => (*energy).max(0),
        _ => 0,
    }
}

/// Записать запас (в пределах ёмкости блока; блок не из сети — как есть).
pub fn set_stored(entity: &mut ModBlockEntity, energy: i32) {
    let cap = m::energy_spec(&entity.id).map_or(i32::MAX, |s| s.capacity);
    entity.nbt.insert(m::ENERGY_KEY.into(), Tag::Int(energy.clamp(0, cap)));
}

/// Генератор выработал `amount`: в свой буфер, сколько влезет. Сколько вошло.
pub fn generate(entity: &mut ModBlockEntity, amount: i32) -> i32 {
    let Some(spec) = m::energy_spec(&entity.id) else {
        return 0;
    };
    let (next, put) = m::add_energy(stored(entity), amount, spec.capacity);
    if put > 0 {
        set_stored(entity, next);
    }
    put
}

/// Машина берёт `amount` на этот тик работы: если своего запаса мало —
/// добирает из сети, потом списывает. `true` — питание есть, тик идёт.
pub fn consume(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
    amount: i32,
) -> bool {
    if amount <= 0 {
        return true;
    }
    if stored(entity) < amount {
        pull(game, pos, entity);
    }
    let have = stored(entity);
    if have < amount {
        return false;
    }
    set_stored(entity, have - amount);
    true
}

/// Добрать буфер блока `entity` в `pos` из сети: обойти кабели от него и
/// забрать из генераторов, потом из накопителей (4.27 п. 5). Тело — агента
/// «Сеть».
pub fn pull(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    pull_for(game, pos, entity, 1);
}

/// Тик блоков сети, которые не машины: накопители (тянут из генераторов),
/// повторитель. Тело — агента «Сеть».
pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if !m::is_storage(&entity.id) || !serving(game) {
        return;
    }
    let now = game.ticks.wrapping_add(stagger(pos));
    // Аккумулятор: износ от полных разрядов и ёмкость по износу (её буфер
    // платформы не знает — лишнее, если кто-то налил, срезаем).
    if entity.id == m::LEAD_BATTERY {
        let wear = int(entity, m::BATTERY_WEAR_KEY);
        let capacity = m::storage_capacity(&entity.id, wear);
        let have = stored(entity);
        if have > capacity {
            set_stored(entity, capacity);
        }
        let charged = int(entity, m::BATTERY_CHARGED_KEY) > 0;
        let (charged_now, worn) = m::battery_cycle(stored(entity), capacity, charged);
        if worn {
            entity.nbt.insert(m::BATTERY_WEAR_KEY.into(), Tag::Int(wear + 1));
        }
        if charged_now != charged {
            entity.nbt.insert(m::BATTERY_CHARGED_KEY.into(), Tag::Int(i32::from(charged_now)));
        }
    }
    // Лейденская банка течёт сама.
    if entity.id == m::LEYDEN_JAR && now % m::LEYDEN_LEAK_EVERY == 0 {
        let have = stored(entity);
        set_stored(entity, have - m::leyden_leak(have));
    }
    if now % m::STORAGE_PULL_EVERY == 0 {
        pull_for(game, pos, entity, m::STORAGE_PULL_EVERY as i32);
    }
    if now % m::SHORT_CHECK_EVERY == 0 {
        hazards(game, pos, entity);
    }
}

// ---- Исполнение сети (агент «Сеть», GDD 4.27 А) ----
//
// Обход сети — `scan_grid` модуля мода, кэш на `GRID_CACHE_TICKS` с
// ревизией (`bump_grid_revision`: блок сети поставили, сломали, кабель
// сгорел). Чужой запас — хелперами платформы (`native_energy*`), свой —
// здесь, в `entity.nbt`. Всё — только на своём сервере.

use horizon_core::geometry::Direction;
use std::sync::{Arc, Mutex, OnceLock};

/// Мир настоящий только у своего сервера или выделенного.
fn serving(game: &crate::game::Game<'_>) -> bool {
    game.serving.is_some() || game.options.dedicated.is_some()
}

/// Разнести периодические дела блоков по тикам.
fn stagger(pos: [i32; 3]) -> u64 {
    (pos[0].wrapping_mul(31) ^ pos[1].wrapping_mul(7) ^ pos[2].wrapping_mul(17)) as u32 as u64
}

fn int(entity: &ModBlockEntity, key: &str) -> i32 {
    match entity.nbt.get(&key.into()) {
        Some(Tag::Int(v)) => *v,
        _ => 0,
    }
}

/// Сторона блока по номеру в `GRID_FACES` (порядок `Direction` платформы).
fn face(index: usize) -> Direction {
    Direction::ALL[index % 6]
}

/// Ёмкость блока: у аккумулятора — с износом.
pub fn capacity_of(entity: &ModBlockEntity) -> i32 {
    match m::energy_spec(&entity.id) {
        Some(spec) if spec.role == m::PowerRole::Storage => {
            m::storage_capacity(&entity.id, int(entity, m::BATTERY_WEAR_KEY))
        }
        Some(spec) => spec.capacity,
        None => 0,
    }
}

fn centre(pos: [i32; 3]) -> [f64; 3] {
    [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5]
}

/// Обход сети в кэше: когда построен, при какой ревизии, сам обход и
/// основной питатель (откуда брали в последний раз) — по его пути идёт
/// нагрузка машины.
struct Cached {
    built: u64,
    revision: u64,
    scan: Arc<m::GridScan>,
    primary: Option<usize>,
}

type GridCache = std::collections::HashMap<(String, [i32; 3]), Cached>;

fn cache() -> &'static Mutex<GridCache> {
    static CACHE: OnceLock<Mutex<GridCache>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Обход сети от `pos` (из кэша, если свежий).
fn scan(game: &crate::game::Game<'_>, pos: [i32; 3]) -> Arc<m::GridScan> {
    let key = (game.dim.id().to_string(), pos);
    let now = game.ticks;
    let revision = m::grid_revision();
    if let Ok(held) = cache().lock()
        && let Some(entry) = held.get(&key)
        && entry.revision == revision
        && now.saturating_sub(entry.built) < m::GRID_CACHE_TICKS
    {
        return Arc::clone(&entry.scan);
    }
    let fresh = Arc::new(m::scan_grid(pos, |p| game.native_block(p)));
    if let Ok(mut held) = cache().lock() {
        if held.len() > 2048 {
            held.retain(|_, e| now.saturating_sub(e.built) < m::GRID_CACHE_TICKS * 4);
        }
        let primary = held.get(&key).and_then(|e| e.primary).filter(|i| *i < fresh.nodes.len());
        held.insert(key, Cached { built: now, revision, scan: Arc::clone(&fresh), primary });
    }
    fresh
}

fn remember_primary(game: &crate::game::Game<'_>, pos: [i32; 3], primary: usize) {
    if let Ok(mut held) = cache().lock()
        && let Some(entry) = held.get_mut(&(game.dim.id().to_string(), pos))
    {
        entry.primary = Some(primary);
    }
}

fn primary_of(game: &crate::game::Game<'_>, pos: [i32; 3]) -> Option<usize> {
    cache().lock().ok()?.get(&(game.dim.id().to_string(), pos)).and_then(|e| e.primary)
}

/// Добор за `span` тиков: накопитель тянет раз в несколько тиков, и за раз
/// берёт столько, сколько взял бы за все. Каждый питатель отдаёт не больше
/// класса самого слабого кабеля на пути за тик.
fn pull_for(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity, span: i32) {
    let Some(spec) = m::energy_spec(&entity.id) else {
        return;
    };
    if spec.role == m::PowerRole::Generator || spec.max_in <= 0 || !serving(game) {
        return;
    }
    let have = stored(entity);
    let want = (capacity_of(entity) - have).min(spec.max_in.saturating_mul(span));
    if want <= 0 {
        return;
    }
    let grid = scan(game, pos);
    let dimension = game.dim.id().to_string();
    let mut got = 0;
    let mut primary = None;
    for index in grid.feeders(spec.role) {
        if got >= want {
            break;
        }
        let node = &grid.nodes[index];
        let ask = (want - got).min(node.class.saturating_mul(span));
        let took = game.native_energy_extract(&dimension, node.pos, face(node.face), ask, false);
        if took > 0 {
            got += took;
            primary.get_or_insert(index);
        }
    }
    if got > 0 {
        set_stored(entity, have + got);
    }
    if let Some(primary) = primary {
        remember_primary(game, pos, primary);
    }
}

/// Тик машины мода, на своём сервере, до её работы (`machine.rs`): числа
/// запаса на экран и долив буфера из сети, пока машина стоит. Не
/// потребитель — сразу выход.
pub fn machine(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if m::machine_draw(&entity.id) <= 0 {
        return;
    }
    let slots = m::machine_data_slots(&entity.id);
    if entity.data.len() < slots {
        entity.data.resize(slots, 0);
    }
    if game.ticks.wrapping_add(stagger(pos)) % m::TOP_UP_EVERY == 0 && stored(entity) < capacity_of(entity) {
        pull_for(game, pos, entity, m::TOP_UP_EVERY as i32);
    }
    let have = stored(entity);
    let id = entity.id.clone();
    m::write_power(&id, &mut entity.data, have, false);
}

/// Тик работы машины-потребителя: списать `machine_draw` (`consume`),
/// записать нагрузку на кабели (перегрузка плавит), раз в 5 с — проверить
/// сеть на замыкание. `false` — заряда нет: прогресс стоит, лампа гаснет
/// (это делает машина), экран пишет «Нет питания».
pub fn work(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) -> bool {
    let draw = m::machine_draw(&entity.id);
    if draw <= 0 {
        return true;
    }
    let slots = m::machine_data_slots(&entity.id);
    if entity.data.len() < slots {
        entity.data.resize(slots, 0);
    }
    let powered = consume(game, pos, entity, draw);
    if powered {
        load(game, pos, draw);
    }
    let have = stored(entity);
    let id = entity.id.clone();
    m::write_power(&id, &mut entity.data, have, !powered);
    if game.ticks.wrapping_add(stagger(pos)) % m::SHORT_CHECK_EVERY == 0 {
        hazards(game, pos, entity);
    }
    // Карантин без питания перестаёт держать (4.27 п. 10) — сказать один раз.
    if id == m::QUARANTINE {
        let was = int(entity, "biolab_unpowered") > 0;
        if was == powered {
            entity.nbt.insert("biolab_unpowered".into(), Tag::Int(i32::from(!powered)));
            if !powered {
                let dimension = game.dim.id().to_string();
                m::note_grid_event(&dimension, centre(pos), (0.0, 0.0, ""), (16.0, m::QUARANTINE_UNPOWERED_LINE));
            }
        }
    }
    powered
}

/// Машина без питания в этом тике: лампа гаснет, прогресс стоит. Шаг для
/// звука — простой.
pub fn stall(entity: &mut ModBlockEntity) -> m::Step {
    if let Some(lit) = entity.data.get_mut(m::DATA_LIT) {
        *lit = 0;
    }
    m::Step::Idle
}

/// Нагрузка машины на кабели её основного пути; перегруженный кабель
/// плавится.
fn load(game: &mut crate::game::Game<'_>, pos: [i32; 3], draw: i32) {
    let grid = scan(game, pos);
    let index = primary_of(game, pos)
        .filter(|i| *i < grid.nodes.len())
        .or_else(|| grid.feeders(m::PowerRole::Consumer).first().copied());
    let Some(index) = index else {
        return;
    };
    let path = &grid.nodes[index].path;
    if path.is_empty() {
        return;
    }
    let dimension = game.dim.id().to_string();
    if let Some(cable) = m::grid_load_add(&dimension, path, draw, game.ticks) {
        melt(game, cable);
    }
}

/// Перегрузка (4.27 п. 10): кабель пропадает, дым и шипение, цепь рвётся,
/// игрокам рядом — строка.
fn melt(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    let dimension = game.dim.id().to_string();
    if let Some(air) = game.pack.assets.registry.default_state("minecraft:air") {
        game.native_set_block(&dimension, pos, air, horizon_world::level::flags::UPDATE_ALL);
    }
    m::grid_load_forget(&dimension, pos);
    m::bump_grid_revision();
    let at = centre(pos);
    puff(game, at, horizon_client::particles::ParticleType::LargeSmoke, 12);
    sound(game, at, m::MELT_SOUND);
    let line = format!("{} ({} {} {})", m::MELT_LINE, pos[0], pos[1], pos[2]);
    m::note_grid_event(&dimension, at, (0.0, 0.0, ""), (16.0, &line));
}

fn puff(game: &mut crate::game::Game<'_>, at: [f64; 3], kind: horizon_client::particles::ParticleType, count: i32) {
    if !serving(game) {
        return;
    }
    let options = horizon_client::particles::ParticleOptions::simple(kind);
    let _ = game.with_level(|_, level, _| level.native_send_particles(&options, None, at, [0.25, 0.25, 0.25], 0.02, count));
}

fn sound(game: &mut crate::game::Game<'_>, at: [f64; 3], s: m::Sound) {
    if !serving(game) {
        return;
    }
    let _ = game.with_level(|_, level, _| level.native_play_sound(s.event, at, s.source, s.volume, s.pitch));
}

/// Мокнет ли блок в `pos`: вода рядом или дождь над открытым небом.
fn exposed(game: &crate::game::Game<'_>, pos: [i32; 3], rain: f32) -> bool {
    let neighbours: Vec<String> = m::GRID_FACES
        .iter()
        .map(|d| game.native_block([pos[0] + d[0], pos[1] + d[1], pos[2] + d[2]]))
        .collect();
    let sky = if rain >= m::RAIN_WET { game.native_light([pos[0], pos[1] + 1, pos[2]]).1 } else { 0 };
    m::exposed(&neighbours, rain, sky)
}

/// Когда место замыкало последний раз (тик).
fn shorted() -> &'static Mutex<std::collections::HashMap<(String, [i32; 3]), u64>> {
    static AT: OnceLock<Mutex<std::collections::HashMap<(String, [i32; 3]), u64>>> = OnceLock::new();
    AT.get_or_init(Default::default)
}

/// Короткое замыкание (4.27 п. 10): блок сети или кабель его сети мокнет —
/// удар током всем в 2 блоках, половина заряда уходит в землю (свой буфер и
/// буферы питателей). Зовёт тик блока сети раз в `SHORT_CHECK_EVERY`.
fn hazards(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let rain = game.native_weather().0;
    let grid = scan(game, pos);
    let mut spot = exposed(game, pos, rain).then_some(pos);
    if spot.is_none() {
        spot = grid.cables.iter().take(m::SHORT_PROBE).map(|(p, _)| *p).find(|p| exposed(game, *p, rain));
    }
    let Some(spot) = spot else {
        return;
    };
    let dimension = game.dim.id().to_string();
    let now = game.ticks;
    if let Ok(mut map) = shorted().lock() {
        let key = (dimension.clone(), spot);
        if map.get(&key).is_some_and(|t| now.saturating_sub(*t) < m::SHORT_COOLDOWN) {
            return;
        }
        if map.len() > 1024 {
            map.retain(|_, t| now.saturating_sub(*t) < m::SHORT_COOLDOWN);
        }
        map.insert(key, now);
    }
    // Заряд — в землю.
    let have = stored(entity);
    set_stored(entity, have - m::short_drain(have));
    for node in grid.nodes.iter().filter(|n| n.role != m::PowerRole::Consumer).take(16) {
        let held = game.native_energy(&dimension, node.pos, face(node.face)).map_or(0, |e| e.amount);
        let lost = m::short_drain(held);
        if lost > 0 {
            game.native_energy_extract(&dimension, node.pos, face(node.face), lost, false);
        }
    }
    // Удар током: мобам сразу, игрокам — на их тике (`player_tick`).
    let at = centre(spot);
    let hit = horizon_client::mobs::Damage::of(horizon_client::mobs::DamageKind::Named(
        horizon_client::mobs::NamedDamage::new(m::ELECTRIC_DAMAGE, horizon_client::mobs::NamedDamage::PANIC_CAUSES),
    ));
    for mob in game.mobs.iter_mut() {
        if mob.alive() && !mob.removed && m::within(at, [mob.position.x, mob.position.y, mob.position.z], m::SHORT_RADIUS) {
            let _ = mob.damage(m::SHORT_DAMAGE, hit);
        }
    }
    m::note_grid_event(&dimension, at, (m::SHORT_RADIUS, m::SHORT_DAMAGE, m::SHORT_HURT_LINE), (12.0, m::SHORT_LINE));
    puff(game, at, horizon_client::particles::ParticleType::ElectricSpark, 24);
    sound(game, at, m::SHORT_SOUND);
}

/// Тик игрока (`power_player.rs`): удары током и строки сети рядом с ним.
pub fn player_tick(game: &mut crate::game::Game<'_>) {
    if !serving(game) {
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };
    let dimension = player.dimension.to_string();
    let at = [player.x, player.y, player.z];
    let uuid = game.native_acting_id();
    let (damage, lines) = m::take_grid_events(uuid, &dimension, at);
    if damage > 0.0 {
        game.apply_effect_action(horizon_client::effects::EffectAction::Hurt(m::ELECTRIC_DAMAGE, damage));
    }
    for line in lines {
        crate::game::biolab_say::say(game, &line);
    }
}

/// Блок сети сломал игрок (`power_broken.rs`, до того, как блок исчез):
/// заряженный конденсатор (и банка, слабее) разряжается в него.
pub fn broken(game: &mut crate::game::Game<'_>, pos: [i32; 3], block: &str) {
    m::bump_grid_revision();
    let dimension = game.dim.id().to_string();
    m::grid_load_forget(&dimension, pos);
    if !m::is_storage(block) {
        return;
    }
    let held = game.native_with_mod_block_entity(pos, |e| stored(e)).unwrap_or(0);
    let damage = m::discharge_damage(block, held);
    if damage <= 0.0 {
        return;
    }
    let _ = game.native_with_mod_block_entity(pos, |e| set_stored(e, 0));
    game.apply_effect_action(horizon_client::effects::EffectAction::Hurt(m::ELECTRIC_DAMAGE, damage));
    let at = centre(pos);
    puff(game, at, horizon_client::particles::ParticleType::ElectricSpark, 30);
    sound(game, at, m::SHORT_SOUND);
    crate::game::biolab_say::say(game, &format!("Разряд: {} Вт·с ушли в вас", m::spaced(held)));
}

/// ПКМ по блоку (`power_click.rs`): батареей — зарядить её от генератора
/// или накопителя либо отдать заряд машине; пустой рукой по кабелю —
/// сведения о сети, по накопителю — его заряд. `true` — клик наш.
pub fn click(game: &mut crate::game::Game<'_>, pos: [i32; 3]) -> bool {
    if !serving(game) {
        return false;
    }
    // Один клик по кабелю доходил дважды с разницей в тик — сводка сети
    // печаталась два раза (проверено в игре 09.11.2026). Тот же блок в
    // ближайшие 4 тика — тот же клик: забираем, но молчим.
    static LAST: std::sync::Mutex<Option<([i32; 3], u64)>> = std::sync::Mutex::new(None);
    let now = game.ticks as u64;
    if let Ok(mut last) = LAST.lock() {
        if let Some((at, tick)) = *last
            && at == pos
            && now.saturating_sub(tick) <= 4
        {
            return m::is_conductor(&game.native_block(pos)) || m::is_storage(&game.native_block(pos));
        }
        *last = Some((pos, now));
    }
    let block = game.native_block(pos);
    let main = game.selected as i32;
    let held = game.native_carried_stack(main).map(|s| s.clone());
    match held {
        Some(stack) if &*stack.item == m::BATTERY => battery_click(game, pos, &block, main, &stack),
        Some(_) => false,
        None if m::is_conductor(&block) => {
            report(game, pos, &block);
            true
        }
        None if m::is_storage(&block) => {
            let (have, wear) = game
                .native_with_mod_block_entity(pos, |e| (stored(e), int(e, m::BATTERY_WEAR_KEY)))
                .unwrap_or((0, 0));
            crate::game::biolab_say::say(game, &m::storage_line(&block, have, wear));
            true
        }
        None => false,
    }
}

fn battery_click(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    block: &str,
    slot: i32,
    stack: &horizon_client::items::ItemStack,
) -> bool {
    let Some(spec) = m::energy_spec(block) else {
        return false;
    };
    let Some((have, cap)) = game.native_with_mod_block_entity(pos, |e| (stored(e), capacity_of(e))) else {
        return false;
    };
    let (moved, item, line) = match spec.role {
        // Зарядить батарею от генератора или накопителя.
        m::PowerRole::Generator | m::PowerRole::Storage => {
            let Some((took, item)) = game.native_item_energy_insert(stack, have) else {
                return false;
            };
            if took > 0 {
                let _ = game.native_with_mod_block_entity(pos, |e| {
                    let left = stored(e) - took;
                    set_stored(e, left);
                });
            }
            let line = if took > 0 { format!("Батарея заряжена на {} Вт·с", m::spaced(took)) } else { "Батарея: заряжать нечем или она полна".to_string() };
            (took, item, line)
        }
        // Отдать машине: аварийный запуск без сети.
        m::PowerRole::Consumer => {
            let Some((gave, item)) = game.native_item_energy_extract(stack, (cap - have).max(0)) else {
                return false;
            };
            if gave > 0 {
                let _ = game.native_with_mod_block_entity(pos, |e| {
                    let now = stored(e) + gave;
                    set_stored(e, now);
                });
            }
            let line = if gave > 0 {
                format!("Машине отдано {} Вт·с: в буфере {} / {}", m::spaced(gave), m::spaced(have + gave), m::spaced(cap))
            } else {
                "Батарея пуста или буфер машины полон".to_string()
            };
            (gave, item, line)
        }
    };
    let left = game.native_item_energy(&item).map_or(0, |e| e.amount);
    if moved > 0
        && let Some(hand) = game.native_carried_stack(slot)
    {
        *hand = item;
    }
    crate::game::biolab_say::say(game, &format!("{line} · {}", m::battery_line(left)));
    if moved > 0 {
        sound(game, centre(pos), m::BATTERY_SOUND);
    }
    true
}

/// Сведения о сети по ПКМ пустой рукой по кабелю (4.27 А п. 8).
fn report(game: &mut crate::game::Game<'_>, pos: [i32; 3], block: &str) {
    let grid = m::scan_grid(pos, |p| game.native_block(p));
    let dimension = game.dim.id().to_string();
    let mut r = m::GridReport {
        cable: block.to_string(),
        class: m::cable_class(block).unwrap_or(0),
        load: m::grid_load_shown(&dimension, pos, game.ticks),
        cables: grid.cables.len(),
        truncated: grid.truncated,
        ..Default::default()
    };
    for node in &grid.nodes {
        let Some(spec) = m::energy_spec(&node.block) else {
            continue;
        };
        let energy = game.native_energy(&dimension, node.pos, face(node.face));
        let (amount, capacity) = energy.map_or((0, spec.capacity), |e| (e.amount, e.capacity));
        match spec.role {
            m::PowerRole::Generator => {
                r.generators += 1;
                r.generation += spec.rate;
                r.generator_charge += amount;
            }
            m::PowerRole::Storage => {
                r.storages += 1;
                r.stored += amount;
                r.capacity += capacity;
            }
            m::PowerRole::Consumer => {
                r.consumers += 1;
                r.draw += spec.rate;
            }
        }
    }
    for line in m::grid_report_lines(&r) {
        crate::game::biolab_say::say(game, &line);
    }
}
