// Карантинная камера в игре (GDD 4.18, п. 4). Закрыта ли комната — решила
// проверка каркаса (`biolab_multiblock::tick` зовёт `quarantine_check` раз в
// секунду, сразу — после загрузки); здесь работа фильтра: подробности
// проверки — на экран, клетки комнаты и её доля утечки — в реестр
// `in_quarantine` / `quarantine_leak`, износ патрона. С M9 (GDD 4.23 п. 11)
// камера с пробоинами, дверью без шлюза или без патрона собрана, но
// подтекает (`seal_room`, `seal_factors`, раздел «M9: лаборатория»). И
// утечка тяжёлого штамма из машин вне камеры (`leak`, зовёт тик машины).
// Решения — разделы «M8: карантинная камера» и «M9: лаборатория» в
// `runtime.rs`, под тестами.
//
// Реестр живёт в копии `runtime.rs` игрового крейта (`crate::game::
// biolab_core`): спрашивать его — из хуков игры (тик игрока, тик мира, тик
// машины), не из lib-копии (`crate::biolab_core`), у той свой, пустой.

use crate::game::biolab_core as m;
use horizon_client::chunk::block_entity::ModBlockEntity;
use horizon_client::items::ItemStack;

/// Штамм культуры из `custom_data` стака; без записи — None (дикий, не
/// тяжёлый).
fn strain_of(stack: &ItemStack) -> Option<m::Strain> {
    use horizon_world::nbt::Tag;
    let extra = stack.extra.as_ref()?;
    let (_, data) = extra.other.iter().find(|(key, _)| key == "minecraft:custom_data")?;
    match data.get(m::STRAIN_KEY) {
        Some(Tag::String(s)) => m::strain_from_text(&s.to_string_lossy()),
        _ => None,
    }
}

/// Тик фильтра. `assembly` — закрытая комната (None — не закрыта: камера
/// не держит, патрон не тратится). Числа данных уже нужной длины
/// (`biolab_multiblock::tick`).
pub fn work(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
    assembly: Option<&m::Assembly>,
) -> m::Step {
    if entity.items.len() < m::QUARANTINE_SLOTS {
        entity.items.resize(m::QUARANTINE_SLOTS, None);
    }
    let dimension = game.dim.id().to_string();
    // Проверка была в этом тике — её подробности (точка дыры, размер,
    // клетки) лежат по месту фильтра.
    let checked = m::take_room_check(pos);
    if let Some(result) = &checked {
        m::write_room(&mut entity.data, result);
    }
    let filter = |entity: &ModBlockEntity| {
        entity.items[0].as_ref().and_then(|s| m::filter_left(Some((&*s.item, i64::from(s.damage)))))
    };
    let mut left = filter(&*entity);
    // С M9 (GDD 4.23 п. 11) камера держит не «да/нет», а с долей утечки:
    // собрана — значит в реестре, даже с пробоинами и без патрона; сколько
    // она пропускает — произведение множителей (`seal_factors`). Клетки и
    // герметичность приносит только проверка (раз в секунду), между
    // проверками запись лишь отмечается, герметичность — в числах экрана.
    // Питание (M10, GDD 4.27 пп. 7, 10): собранная камера тянет 15 Вт·с
    // каждый тик. Без заряда фильтр не держит: реестр снимает камеру, как
    // разобранную.
    let powered = assembly.is_some() && crate::game::biolab_power::work(game, pos, entity);
    let registered = if powered {
        match &checked {
            Some(Ok(sealed)) => {
                let factors = m::seal_factors(sealed, left);
                m::write_seal(&mut entity.data, &factors);
                m::hold_room(&dimension, pos, &sealed.room, m::room_leak(&factors));
                true
            }
            _ => m::touch_room(&dimension, pos),
        }
    } else {
        m::forget_room(&dimension, pos);
        false
    };
    let seal = entity.data.get(m::DATA_QR_SEAL).copied().unwrap_or(0);
    let mut held = registered && seal as f32 >= m::SEAL_HOLDS * 100.0;

    // Износ патрона, пока камера собрана и в нём есть что тратить.
    let mut worn = false;
    if registered && left.is_some() && m::filter_wear_due(game.ticks as i64, pos) {
        if let Some(stack) = entity.items[0].as_mut() {
            stack.damage = stack.damage.saturating_add(1);
        }
        left = filter(&*entity);
        if left.is_none() {
            // Износился: камера собрана, но течёт — герметичность пересчитает
            // ближайшая проверка (≤ 1 с).
            entity.items[0] = None;
            held = false;
            worn = true;
        }
    }

    m::write_filter(&mut entity.data, left, held);
    // Лампа фильтра — «камера держит»; шкала и компаратор — остаток патрона.
    entity.data[m::DATA_LIT] = i32::from(held);
    entity.data[m::DATA_LIT_TOTAL] = 1;
    entity.data[m::DATA_TOTAL] = m::FILTER_DURABILITY as i32;
    entity.data[m::DATA_PROGRESS] = left.unwrap_or(0) as i32;
    if worn {
        m::Step::Finish
    } else if held {
        // Для звука — счётчик тиков, а не остаток патрона: фоновый гул
        // звучит, когда «прогресс» кратен `MACHINE_HUM_EVERY`, а остаток
        // стоит на месте по 10 с — гудело бы каждый тик.
        m::Step::Working((game.ticks % 8_000) as i32)
    } else {
        m::Step::Idle
    }
}

/// Фильтр сломал игрок (`multiblock_broken.rs`): камера больше не держит.
pub fn broken(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    let dimension = game.dim.id().to_string();
    m::forget_room(&dimension, pos);
}

/// Утечка (GDD 4.18 п. 4, 4.10 «Утечка»): машина работает (лампа горела в
/// прошлом тике) с культурой тяжёлого штамма во входе и стоит не в камере —
/// раз в `LEAK_EVERY` тиков скот рядом заболевает с шансом по заразности, а
/// утечка отмечается: игрок рядом получает дозу по воздуху на своём шаге
/// болезни (`infection.rs`, `leak_near`; M9 — доза, а не сразу штамм), и
/// звенит тревога. Зовёт тик машины только на своём сервере.
pub fn leak(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &ModBlockEntity) {
    let Some(machine) = m::machine(&entity.id) else {
        return;
    };
    // У контроллеров установок свои правила (биореактор — сам).
    if m::multiblock(machine.id).is_some() || !m::leak_due(game.ticks as i64, pos) {
        return;
    }
    let working = entity.data.get(m::DATA_LIT).copied().unwrap_or(0) > 0;
    let cultures: Vec<m::Strain> = m::machine_inputs(machine)
        .into_iter()
        .filter_map(|slot| entity.items.get(slot).and_then(|s| s.as_ref()))
        .filter(|s| &*s.item == m::CULTURE_ID)
        .filter_map(strain_of)
        .collect();
    if cultures.is_empty() {
        return;
    }
    let dimension = game.dim.id().to_string();
    // Негерметичная камера (GDD 4.23 п. 11) пропускает утечку в доле
    // `quarantine_leak` случаев: бросок — `in_quarantine`; герметичная — нет.
    let sealed = m::quarantine_leak(&dimension, pos) <= 0.0 || m::in_quarantine(&dimension, pos);
    let Some(strain) = m::leak_strain(&cultures, working, sealed) else {
        return;
    };
    let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5];
    m::note_leak(&dimension, at, strain);
    // Тревога (M9, GDD 4.23 п. 12): утечку слышно у машины, а не только
    // видно по дозе.
    if let Some(sound) = m::sound(m::Cue::LeakAlarm) {
        let _ = game.with_level(|_, level, _| {
            level.native_play_sound(sound.event, at, sound.source, sound.volume, sound.pitch)
        });
    }
    let ticks = game.ticks;
    let source = (pos[0] as u32).wrapping_mul(31) ^ (pos[2] as u32);
    for mob in game.mobs.iter_mut() {
        if !mob.alive() || !m::catches_infection(&mob.kind.id) {
            continue;
        }
        let spot = [mob.position.x, mob.position.y, mob.position.z];
        if !m::within(at, spot, m::LEAK_RADIUS) || m::in_quarantine_at(&dimension, spot) {
            continue;
        }
        let sick = mob
            .extra
            .effects
            .iter()
            .any(|(id, ticks, _)| &**id == m::INFECTION_EFFECT && *ticks != 0);
        if sick || !m::leak_catches(&strain, m::herd_seed(source, mob.id, ticks)) {
            continue;
        }
        mob.add_effect(m::INFECTION_EFFECT, m::INFECTED_MOB_TICKS, 0);
    }
}
