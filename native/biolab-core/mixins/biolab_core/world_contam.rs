// Порча ландшафта на живой игре (M10, GDD 4.21, 4.27 Г): данные чанка,
// тик заражения, порча блоков, чистка. Модуль игры
// (`crate::game::biolab_world_contam`), как `biolab_suit`: его зовут
// несколько хуков (`contam_*.rs`) и точки-источники в чужих хелперах.
//
// Правила, числа и память (зеркало `ContamWorld`) — раздел «M10: порча
// ландшафта» в `runtime.rs`, под тестами. Здесь — только то, чему нужна игра:
// данные чанка (`Host`), блоки, мобы, предметы, частицы, пакет игроку.
//
// Данные чанка — свои типы `biolab:contamination` и `biolab:blight_log`
// (носитель `Holder::Chunk`), по образцу работающего генома
// (`data.rs`, `GENOME`): значение — `String`, кодек — одно строковое поле
// компаунда, без `sync` (клиенту заражённость едет своим пакетом). Пишутся
// через `Host` (`attachment_map` → `world_attachments`), только загруженный
// чанк текущего измерения; выгруженного чанка в мире уже нет
// (`streaming.update` вынимает его до `native_chunk_unload`), поэтому
// состояние пишется заранее — раз в 5 с, журнал — сразу.
//
// Читаются на `native_chunk_load` — не через `Host`: тот при каждом
// обращении помечает чанк изменённым (`world.modified`) и копирует его
// (`Arc::make_mut`), а загрузка зовётся на каждый чанк. Поэтому читаем
// копию карты данных прямо из `chunk.saved` (поле игры, как в
// `attachment_sync.rs` пина; при обновлении Horizon проверить).

use crate::game::biolab_core as m;
use horizon_client::items::ItemStack;
use horizon_client::mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Host, Tag};
use horizon_world::nbt::JavaString;

fn encode_state(text: &String) -> Result<Compound, AttachmentError> {
    let mut out = Compound::new();
    out.insert(JavaString::from(m::BLIGHT_FIELD), Tag::String(JavaString::from(text.as_str())));
    Ok(out)
}

fn decode_state(data: &Compound) -> Result<String, AttachmentError> {
    match data.get(&JavaString::from(m::BLIGHT_FIELD)) {
        Some(Tag::String(text)) => Ok(text.to_string_lossy()),
        _ => Err(AttachmentError::Decode),
    }
}

fn encode_log(text: &String) -> Result<Compound, AttachmentError> {
    let mut out = Compound::new();
    out.insert(JavaString::from(m::BLIGHT_LOG_FIELD), Tag::String(JavaString::from(text.as_str())));
    Ok(out)
}

fn decode_log(data: &Compound) -> Result<String, AttachmentError> {
    match data.get(&JavaString::from(m::BLIGHT_LOG_FIELD)) {
        Some(Tag::String(text)) => Ok(text.to_string_lossy()),
        _ => Err(AttachmentError::Decode),
    }
}

/// Заражённость чанка (`biolab:contamination`): `level=2.35;strain=…;station=…`.
pub static STATE: AttachmentType<String> = AttachmentType {
    namespace: m::DATA_NAMESPACE,
    name: m::BLIGHT_DATA,
    default: String::new,
    codec: Some(AttachmentCodec {
        encode: encode_state,
        decode: decode_state,
    }),
    copy_on_death: false,
    sync: false,
};

/// Журнал «что было до порчи» (`biolab:blight_log`): `x y z было стало;…`.
pub static LOG: AttachmentType<String> = AttachmentType {
    namespace: m::DATA_NAMESPACE,
    name: m::BLIGHT_LOG_DATA,
    default: String::new,
    codec: Some(AttachmentCodec {
        encode: encode_log,
        decode: decode_log,
    }),
    copy_on_death: false,
    sync: false,
};

fn server(game: &crate::game::Game<'_>) -> bool {
    game.serving.is_some() || game.options.dedicated.is_some()
}

fn floor(p: [f64; 3]) -> [i32; 3] {
    [p[0].floor() as i32, p[1].floor() as i32, p[2].floor() as i32]
}

/// Данные чанка из сейва, не трогая чанк (см. шапку).
fn read_saved(game: &crate::game::Game<'_>, chunk: [i32; 2]) -> Option<m::ChunkContam> {
    let extras = game.world.chunks.get(&(chunk[0], chunk[1]))?.saved.as_ref()?;
    if !extras.attachments.has(&STATE) && !extras.attachments.has(&LOG) {
        return None;
    }
    let mut map = extras.attachments.clone();
    let state = map.get_existing(&STATE).ok().flatten();
    let log = map.get_existing(&LOG).ok().flatten();
    let mut c = state.map(|t| m::blight_state_from_text(&t)).unwrap_or_default();
    c.log = log.map(|t| m::blight_log_from_text(&t)).unwrap_or_default();
    Some(c)
}

/// Чанк загружен (`native_chunk_load`, сервер).
pub fn chunk_load(game: &mut crate::game::Game<'_>, dimension: &str, chunk: [i32; 2]) {
    if !server(game) {
        return;
    }
    let saved = if dimension == game.dim.id() { read_saved(game, chunk) } else { None };
    let root = game.world_root.clone();
    if let Ok(mut w) = m::contam_world().lock() {
        w.bind(&root);
        w.load((m::dimension_index(dimension), chunk[0], chunk[1]), saved);
    }
}

/// Чанк выгружен (`native_chunk_unload`, сервер): из памяти долой.
pub fn chunk_unload(game: &mut crate::game::Game<'_>, dimension: &str, chunk: [i32; 2]) {
    if !server(game) {
        return;
    }
    if let Ok(mut w) = m::contam_world().lock() {
        w.unload((m::dimension_index(dimension), chunk[0], chunk[1]));
    }
}

/// Записать данные чанка: `Some("")` — снять, `Some(текст)` — записать.
fn write(game: &mut crate::game::Game<'_>, chunk: [i32; 2], state: Option<String>, log: Option<String>) {
    let host: &mut dyn Host = game;
    let holder = Holder::Chunk(chunk);
    for (ty, text) in [(&STATE, state), (&LOG, log)] {
        match text {
            Some(text) if text.is_empty() => {
                let _ = ty.remove(&mut *host, holder);
            }
            Some(text) => {
                let _ = ty.set(&mut *host, holder, text);
            }
            None => {}
        }
    }
}

/// Поставить блок по имени (состояние по умолчанию).
fn place(game: &mut crate::game::Game<'_>, dimension: &str, at: [i32; 3], block: &str) -> bool {
    let Some(state) = game.pack.assets.registry.default_state(block) else {
        return false;
    };
    game.native_set_block(dimension, at, state, horizon_world::level::flags::UPDATE_ALL)
}

/// Вернуть блоки журнала. Больше трёх разом — одной массовой правкой
/// (`native_edit_blocks`, API 126; только там, где ещё стоит наш блок), иначе
/// по одному: массовая правка шлёт игрокам чанк целиком.
fn restore(game: &mut crate::game::Game<'_>, dimension: &str, entries: &[m::BlightEntry]) -> usize {
    if entries.is_empty() {
        return 0;
    }
    if entries.len() > 3 {
        use crate::game::native_world_edit::{NativeBlockEdit, NativeBlockRun};
        let mut edit = NativeBlockEdit::default();
        for e in entries {
            let block = index_of(&mut edit.palette, &e.was);
            let only = index_of(&mut edit.filters, &e.now);
            edit.runs.push(NativeBlockRun {
                x: e.pos[0],
                z: e.pos[2],
                y: e.pos[1],
                height: 1,
                surface: None,
                block,
                only: Some(only),
            });
        }
        if let Ok(changed) = game.native_edit_blocks(dimension, &edit) {
            return changed as usize;
        }
    }
    let mut done = 0;
    for e in entries {
        if game.native_block(e.pos) == e.now && place(game, dimension, e.pos, &e.was) {
            done += 1;
        }
    }
    done
}

fn index_of(list: &mut Vec<String>, name: &str) -> u32 {
    match list.iter().position(|n| n == name) {
        Some(i) => i as u32,
        None => {
            list.push(name.to_string());
            (list.len() - 1) as u32
        }
    }
}

/// Порча одного кандидата случайного тика.
fn blight_candidate(game: &mut crate::game::Game<'_>, dimension: &str, dim: u8, pos: [i32; 3], now: u64) {
    let key = (dim, pos[0] >> 4, pos[2] >> 4);
    let Some((stage, own, vanilla)) = m::contam_world()
        .lock()
        .ok()
        .and_then(|w| w.chunks.get(&key).map(|c| (c.stage(), c.own_blocks(), c.vanilla_blocks())))
    else {
        return;
    };
    if stage < 2 {
        return;
    }
    let block = game.native_block(pos);
    if !m::BLIGHT_GROUND.contains(&block.as_str()) {
        return;
    }
    let above = [pos[0], pos[1] + 1, pos[2]];
    let top = game.native_block(above);
    let own_near = [[1, 0], [-1, 0], [0, 1], [0, -1]]
        .iter()
        .filter(|[dx, dz]| m::BLIGHT_BLOCKS.contains(&game.native_block([pos[0] + dx, pos[1], pos[2] + dz]).as_str()))
        .count() as u8;
    let roll = m::blight_roll(pos, now, 1);
    let mut done = Vec::new();
    for change in m::blight_change(stage, &top, own_near, roll, own, vanilla) {
        match change {
            m::BlightChange::Ground(new) => {
                if place(game, dimension, pos, new) {
                    done.push(m::BlightEntry { pos, was: block.clone(), now: new.to_string() });
                }
            }
            m::BlightChange::Top(new) => {
                if place(game, dimension, above, new) {
                    done.push(m::BlightEntry { pos: above, was: top.clone(), now: new.to_string() });
                }
            }
        }
    }
    if done.is_empty() {
        return;
    }
    if let Ok(mut w) = m::contam_world().lock()
        && let Some(c) = w.chunks.get_mut(&key)
    {
        c.log.extend(done);
        w.dirty_log.insert(key);
    }
}

/// Рост пятен по журналу (ступень 4+) и самоочищение чистого чанка.
fn blight_journal(game: &mut crate::game::Game<'_>, dimension: &str, dim: u8, now: u64) {
    let work: Vec<(m::ChunkKey, Option<(usize, m::BlightChange, m::BlightEntry)>, Vec<m::BlightEntry>)> = {
        let Ok(mut w) = m::contam_world().lock() else {
            return;
        };
        let keys: Vec<m::ChunkKey> = w.chunks.keys().filter(|k| k.0 == dim).copied().collect();
        let mut out = Vec::new();
        for key in keys {
            let Some(c) = w.chunks.get_mut(&key) else {
                continue;
            };
            let roll = m::blight_roll([key.1, 0, key.2], now, 2);
            let grow = m::blight_upgrade(c, roll).map(|(i, change)| (i, change, c.log[i].clone()));
            let heal = if c.stage() == 0 && !c.log.is_empty() && !c.source_alive(now) {
                c.take_restore(m::BLIGHT_HEAL)
            } else {
                Vec::new()
            };
            if !heal.is_empty() {
                w.dirty_log.insert(key);
            }
            if grow.is_some() || !heal.is_empty() {
                out.push((key, grow, heal));
            }
        }
        out
    };
    for (key, grow, heal) in work {
        restore(game, dimension, &heal);
        let Some((i, change, entry)) = grow else {
            continue;
        };
        // Блок мог сломать игрок: тогда запись больше ни к чему.
        let still = game.native_block(entry.pos) == entry.now;
        let added = match change {
            m::BlightChange::Ground(new) if still => place(game, dimension, entry.pos, new).then(|| (i, new.to_string(), None)),
            m::BlightChange::Top(new) if still => {
                let above = [entry.pos[0], entry.pos[1] + 1, entry.pos[2]];
                (game.native_block(above) == "minecraft:air" && place(game, dimension, above, new))
                    .then(|| (i, entry.now.clone(), Some(m::BlightEntry { pos: above, was: "minecraft:air".into(), now: new.to_string() })))
            }
            _ => None,
        };
        let Ok(mut w) = m::contam_world().lock() else {
            return;
        };
        let Some(c) = w.chunks.get_mut(&key) else {
            continue;
        };
        if i >= c.log.len() || c.log[i] != entry {
            continue;
        }
        match added {
            Some((i, now_block, extra)) => {
                c.log[i].now = now_block;
                c.log.extend(extra);
            }
            None if !still => {
                c.log.remove(i);
            }
            None => continue,
        }
        w.dirty_log.insert(key);
    }
}

/// Импульсы станций дезактивации (раз в `STATION_EVERY`).
fn stations(game: &mut crate::game::Game<'_>, dimension: &str, dim: u8, now: u64) {
    let list: Vec<(m::ChunkKey, [i32; 3], u32)> = m::contam_world()
        .lock()
        .map(|w| {
            w.chunks
                .iter()
                .filter(|(k, _)| k.0 == dim)
                .filter_map(|(k, c)| c.station.filter(|(_, charge)| *charge > 0).map(|(at, charge)| (*k, at, charge)))
                .collect()
        })
        .unwrap_or_default();
    for (key, at, charge) in list {
        let standing = game.native_block(at) == m::DECON_STATION;
        let mut restores = Vec::new();
        if let Ok(mut w) = m::contam_world().lock() {
            if let Some(c) = w.chunks.get_mut(&key) {
                c.station = standing.then_some((at, charge - 1));
            }
            w.dirty.insert(key);
            if !standing {
                continue;
            }
            for dx in -m::STATION_RADIUS..=m::STATION_RADIUS {
                for dz in -m::STATION_RADIUS..=m::STATION_RADIUS {
                    let near = (dim, key.1 + dx, key.2 + dz);
                    let Some(c) = w.chunks.get_mut(&near) else {
                        continue;
                    };
                    if c.level <= 0.0 && c.log.is_empty() {
                        continue;
                    }
                    let o = m::clean_chunk(c, m::STATION_LOWER, m::STATION_RESTORE, now);
                    w.dirty.insert(near);
                    if !o.restore.is_empty() {
                        w.dirty_log.insert(near);
                    }
                    restores.extend(o.restore);
                }
            }
        }
        restore(game, dimension, &restores);
        let options = horizon_client::particles::ParticleOptions::simple(horizon_client::particles::ParticleType::Cloud);
        let spot = [f64::from(at[0]) + 0.5, f64::from(at[1]) + 1.2, f64::from(at[2]) + 0.5];
        let _ = game.with_level(|_, level, _| level.native_send_particles(&options, None, spot, [0.3, 0.3, 0.3], 0.02, 12));
    }
}

/// Мобы в заражённых чанках: скот болеет чаще (3+), не плодится (4+),
/// на 5 — получает штамм.
fn mobs(game: &mut crate::game::Game<'_>, dim: u8, now: u64) {
    let Ok(w) = m::contam_world().lock() else {
        return;
    };
    if w.chunks.is_empty() {
        return;
    }
    for mob in game.mobs.iter_mut() {
        if !mob.alive() {
            continue;
        }
        let at = floor([mob.position.x, mob.position.y, mob.position.z]);
        let stage = w.stage((dim, at[0] >> 4, at[2] >> 4));
        if stage < 3 {
            continue;
        }
        // Любовь гаснет раньше, чем пара успеет родить (на это нужно 3 с
        // вместе): хук рождения игра при наших бандлах не зовёт.
        if !m::blight_breeds(stage) && mob.extra.in_love > 0 {
            mob.extra.in_love = 0;
        }
        if !m::catches_infection(&mob.kind.id) {
            continue;
        }
        let sick = mob.extra.effects.iter().any(|(id, ticks, _)| &**id == m::INFECTION_EFFECT && *ticks != 0);
        if !sick && m::blight_roll(at, now, u64::from(mob.id)) < m::blight_sick_chance(stage) {
            mob.add_effect(m::INFECTION_EFFECT, m::INFECTED_MOB_TICKS, 0);
        }
    }
}

/// Шаг мира: раз в секунду на измерение (первый игрок тика).
fn world_step(game: &mut crate::game::Game<'_>, dimension: &str, dim: u8, now: u64) {
    if let Ok(mut w) = m::contam_world().lock() {
        for q in m::take_world_contamination() {
            w.add(&q.dimension, q.pos, q.amount, q.strain, now);
        }
        w.step(dim, now, (m::CONTAM_EVERY as f32) / 20.0);
    }
    for pos in m::take_blight_candidates(dim) {
        blight_candidate(game, dimension, dim, pos, now);
    }
    blight_journal(game, dimension, dim, now);
    if now % m::STATION_EVERY == 0 {
        stations(game, dimension, dim, now);
    }
    mobs(game, dim, now);
    let writes = match m::contam_world().lock() {
        Ok(mut w) => {
            let writes = w.take_writes(dim, now % m::CONTAM_WRITE_EVERY == 0);
            m::blight_mark_any(&w);
            writes
        }
        Err(_) => Vec::new(),
    };
    for (key, state, log) in writes {
        write(game, [key.1, key.2], state, log);
    }
}

/// Штамм культуры на земле (`custom_data`, `STRAIN_KEY`).
fn culture_strain(stack: &ItemStack) -> Option<m::Strain> {
    crate::game::biolab_suit::custom_text(stack, m::STRAIN_KEY).and_then(|t| m::strain_from_text(&t))
}

/// Шаг игрока: команды проверки, колбы на земле, доза, пакет, частицы.
fn player_step(game: &mut crate::game::Game<'_>, dimension: &str, now: u64) {
    let Some(player) = game.native_player() else {
        return;
    };
    let uuid = game.native_acting_id();
    let feet = [player.x, player.y, player.z];
    let pos = floor(feet);
    let key = m::chunk_key(dimension, pos);

    // `/biolab contam [N]` из окна.
    for request in m::take_contam_requests() {
        let line = match m::contam_world().lock() {
            Ok(mut w) => {
                if let Some(level) = request
                    && let Some(c) = w.entry(key)
                {
                    c.level = level;
                    if level <= 0.0 {
                        c.strain = None;
                    }
                    w.dirty.insert(key);
                    m::blight_mark_any(&w);
                }
                let zone = w.zone(key);
                m::contam_report([key.1, key.2], w.chunks.get(&key), zone, now)
            }
            Err(_) => continue,
        };
        crate::game::biolab_say::say(game, &line);
    }

    // Колба с культурой на земле — разбитая колба: живой источник, пока лежит.
    let reach = 16.0;
    let dropped = game.native_item_entities(
        dimension,
        [feet[0] - reach, feet[1] - 8.0, feet[2] - reach],
        [feet[0] + reach, feet[1] + 8.0, feet[2] + reach],
    );
    for item in dropped {
        if &*item.stack.item != m::CULTURE_ID {
            continue;
        }
        let amount = m::CONTAM_FLASK * item.stack.count.min(m::CONTAM_FLASK_MAX) as f32;
        m::queue_world_contamination(dimension, floor(item.position), amount, culture_strain(&item.stack));
    }

    let (chunk, zone, strain) = m::contam_world()
        .lock()
        .map(|w| (w.level(key), w.zone(key), w.chunks.get(&key).and_then(|c| c.strain)))
        .unwrap_or_default();
    let stage = m::contam_stage(chunk);

    // Доза воздухом (спецодежду учтёт тик дозы).
    if stage >= 2 {
        m::queue_dose(uuid, m::DoseChannel::Air, m::contam_dose(stage), strain);
    }
    let _ = game.native_net_send_to_player(uuid, m::NET_BLIGHT, &m::blight_packet(chunk, zone));

    // Частицы: пыль с земли (2+), муть в воде (3+), споры (4+).
    if stage >= 2 {
        use horizon_client::particles::{ParticleOptions, ParticleType};
        let mut puffs = vec![(ParticleOptions::simple(ParticleType::Mycelium), [6.0, 1.0, 6.0], 10)];
        if stage >= 3 {
            puffs.push((ParticleOptions::simple(ParticleType::Underwater), [8.0, 3.0, 8.0], 40));
        }
        if stage >= 4 {
            puffs.push((ParticleOptions::named(crate::game::biolab_particles::SPORE), [6.0, 1.5, 6.0], 4 * i32::from(stage)));
        }
        let at = [feet[0], feet[1] + 0.5, feet[2]];
        let _ = game.with_level(|_, level, _| {
            for (options, spread, count) in &puffs {
                let _ = level.native_send_particles(options, None, at, *spread, 0.01, *count);
            }
        });
    }
}

/// Тик заражения (`native_player_tick`, сервер, раз в секунду).
pub fn tick(game: &mut crate::game::Game<'_>) {
    if !game.native_on(m::ID) || !server(game) {
        return;
    }
    let now = game.ticks;
    if now % m::CONTAM_EVERY != 0 {
        return;
    }
    let dimension = game.dim.id().to_string();
    let dim = m::dimension_index(&dimension);
    let root = game.world_root.clone();
    let first = match m::contam_world().lock() {
        Ok(mut w) => {
            w.bind(&root);
            let first = w.stepped.get(&dim) != Some(&now);
            w.stepped.insert(dim, now);
            first
        }
        Err(_) => return,
    };
    if first {
        world_step(game, &dimension, dim, now);
    }
    player_step(game, &dimension, now);
}

/// Ступень и штамм чанка с точкой `pos`.
pub fn stage_at(dimension: &str, pos: [i32; 3]) -> (u8, Option<m::Strain>) {
    m::contam_world()
        .lock()
        .ok()
        .and_then(|w| w.chunks.get(&m::chunk_key(dimension, pos)).map(|c| (c.stage(), c.strain)))
        .unwrap_or((0, None))
}

/// Урожай ГМ-грядки (`harvest.rs`): на ступени 4+ заражён (M9).
pub fn taint_harvest(game: &crate::game::Game<'_>, pos: [i32; 3], crop: &mut ItemStack) {
    let (stage, strain) = stage_at(game.dim.id(), pos);
    if let Some(dirt) = m::blight_harvest_dirt(stage, strain) {
        use crate::game::biolab_suit as suit;
        let next = m::dirtier(suit::contamination(crop), Some(dirt));
        suit::set_contamination(crop, next);
    }
}

/// Убит моб (`killed.rs`): труп заражённого или больного скота — источник.
pub fn corpse(game: &mut crate::game::Game<'_>, entity: u32, spot: [i32; 3]) {
    if !server(game) {
        return;
    }
    let Some(amount) = game.mobs.iter().find(|mob| mob.id == entity).and_then(|mob| {
        if m::is_infected(&mob.kind.id) {
            Some(m::CONTAM_INFECTED_CORPSE)
        } else if mob.extra.effects.iter().any(|(id, ticks, _)| &**id == m::INFECTION_EFFECT && *ticks != 0) {
            Some(m::CONTAM_SICK_CORPSE)
        } else {
            None
        }
    }) else {
        return;
    };
    let dimension = game.dim.id().to_string();
    m::queue_world_contamination(&dimension, spot, amount, None);
}

/// ПКМ распылителем (`native_use_item`, сервер). Пустой — заправка хлором
/// из инвентаря; иначе — пшик по чанку под ногами.
pub fn spray(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_suit as suit;
    let main = game.selected as i32;
    let slot = if game.native_carried_stack(main).is_some_and(|s| &*s.item == m::SPRAYER) { main } else { -106 };
    let charges = match game.native_carried_stack(slot) {
        Some(stack) if &*stack.item == m::SPRAYER => m::sprayer_charges(suit::custom_text(stack, m::SPRAYER_KEY).as_deref()),
        _ => return,
    };
    if charges == 0 {
        // Хлор — из другой руки или из инвентаря.
        let chlorine = [-106, main].into_iter().chain(0..36).find(|s| {
            *s != slot && game.native_carried_stack(*s).is_some_and(|st| &*st.item == m::DISINFECTANT && st.count > 0)
        });
        let Some(from) = chlorine else {
            crate::game::biolab_say::say(game, "Распылитель пуст: нужен хлор в инвентаре");
            return;
        };
        if let Some(st) = game.native_carried_stack(from) {
            st.count -= 1;
        }
        if let Some(stack) = game.native_carried_stack(slot) {
            suit::set_custom_text(stack, m::SPRAYER_KEY, Some(&m::SPRAYER_CHARGES.to_string()));
        }
        crate::game::biolab_say::say(game, &format!("Распылитель заправлен хлором: {} зарядов", m::SPRAYER_CHARGES));
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };
    let dimension = game.dim.id().to_string();
    let feet = [player.x, player.y, player.z];
    let key = m::chunk_key(&dimension, floor(feet));
    let now = game.ticks;
    let outcome = match m::contam_world().lock() {
        Ok(mut w) => match w.chunks.get_mut(&key) {
            Some(c) => {
                let o = m::clean_chunk(c, m::SPRAYER_LOWER, m::SPRAYER_RESTORE, now);
                w.dirty.insert(key);
                if !o.restore.is_empty() {
                    w.dirty_log.insert(key);
                }
                o
            }
            None => m::CleanOutcome::default(),
        },
        Err(_) => return,
    };
    let used = outcome.before > 0.0 || !outcome.restore.is_empty();
    let left = if used { charges - 1 } else { charges };
    if used && let Some(stack) = game.native_carried_stack(slot) {
        suit::set_custom_text(stack, m::SPRAYER_KEY, Some(&left.to_string()));
    }
    restore(game, &dimension, &outcome.restore);
    // Облачко хлора перед игроком.
    let yaw = f64::from(player.yaw).to_radians();
    let at = [feet[0] - yaw.sin() * 1.5, feet[1] + 1.2, feet[2] + yaw.cos() * 1.5];
    let options = horizon_client::particles::ParticleOptions::simple(horizon_client::particles::ParticleType::Cloud);
    let _ = game.with_level(|_, level, _| level.native_send_particles(&options, None, at, [0.6, 0.4, 0.6], 0.02, 16));
    crate::game::biolab_say::say(game, &m::spray_message(&outcome, left));
}

/// ПКМ по станции дезактивации (`native_right_click_block`, сервер): хлором
/// в руке — заправить, иначе — состояние. Ложь — это не станция.
pub fn station_click(game: &mut crate::game::Game<'_>, pos: [i32; 3]) -> bool {
    if game.native_block(pos) != m::DECON_STATION {
        return false;
    }
    let dimension = game.dim.id().to_string();
    let key = m::chunk_key(&dimension, pos);
    let charge = m::contam_world()
        .lock()
        .ok()
        .and_then(|w| w.chunks.get(&key).and_then(|c| c.station).filter(|(at, _)| *at == pos).map(|(_, n)| n))
        .unwrap_or(0);
    let main = game.selected as i32;
    let mut loaded = 0;
    if let Some(stack) = game.native_carried_stack(main)
        && &*stack.item == m::DISINFECTANT
    {
        loaded = stack.count.min(m::STATION_MAX_CHARGE.saturating_sub(charge));
        stack.count -= loaded;
    }
    let charge = charge + loaded;
    if let Ok(mut w) = m::contam_world().lock()
        && let Some(c) = w.entry(key)
    {
        c.station = Some((pos, charge));
        w.dirty.insert(key);
    }
    crate::game::biolab_say::say(game, &m::station_message(charge, loaded));
    true
}
