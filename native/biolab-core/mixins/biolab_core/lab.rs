// Лаборатория M9 в игре (GDD 4.23 п. 9): порча образцов вне холода,
// криобанк и термобокс. Правила — раздел «M9: лаборатория» в `runtime.rs`
// (`spoil_steps`, `spoil_sample`, `cold_push`, `freshness_line`), под
// тестами; здесь только чтение и запись `custom_data` стака.
//
// Срок порчи — игровое время (`world.game_time`, идёт в сейве мира) в
// `custom_data` образца (`SPOIL_KEY`, Long). Инвентарь игрока раз в
// `SPOIL_EVERY` тиков ставит срок новому образцу, портит просроченный и,
// если в инвентаре термобокс, сдвигает срок на три четверти прошедшего.
// Криобанк сдвигает срок своих образцов на всё прошедшее время — внутри они
// не портятся, даже пока чанк выгружен (сдвиг — от его последнего тика).
// Сундук и земля срок не трогают: образец там портится по часам и
// испортится, когда снова попадёт в инвентарь или в криобанк.

use crate::game::biolab_core as m;
use horizon_client::chunk::block_entity::ModBlockEntity;
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

/// Последний учёт времени криобанком, в его сейве.
const CRYO_AT_KEY: &str = "biolab_cryo_at";

fn custom(stack: &ItemStack) -> Option<&Tag> {
    let extra = stack.extra.as_ref()?;
    extra.other.iter().find(|(key, _)| key == "minecraft:custom_data").map(|(_, tag)| tag)
}

fn custom_text(stack: &ItemStack, key: &str) -> Option<String> {
    match custom(stack)?.get(key) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    }
}

/// Срок порчи образца; None — ещё не ставили.
pub fn deadline(stack: &ItemStack) -> Option<i64> {
    match custom(stack)?.get(m::SPOIL_KEY) {
        Some(Tag::Long(v)) => Some(*v),
        Some(Tag::Int(v)) => Some(i64::from(*v)),
        _ => None,
    }
}

/// Записать ключ в `custom_data`, не трогая остальные (штамм, загрязнение
/// и что там ещё положили другие части мода).
fn put(stack: &mut ItemStack, key: &str, value: Tag) {
    let extra = stack.extra.get_or_insert_with(Default::default);
    let index = match extra.other.iter().position(|(k, _)| k == "minecraft:custom_data") {
        Some(i) => i,
        None => {
            extra.other.push(("minecraft:custom_data".to_string(), Tag::Compound(Default::default())));
            extra.other.sort_by(|a, b| a.0.cmp(&b.0));
            extra.other.iter().position(|(k, _)| k == "minecraft:custom_data").unwrap_or(0)
        }
    };
    if !matches!(extra.other[index].1, Tag::Compound(_)) {
        extra.other[index].1 = Tag::Compound(Default::default());
    }
    if let Tag::Compound(map) = &mut extra.other[index].1 {
        map.insert(key.into(), value);
    }
}

/// Подпись образца по его данным (как у инвентаря); нет — имя из языка.
fn relabel(stack: &mut ItemStack) {
    let item = stack.item.to_string();
    let view: &ItemStack = stack;
    let label = m::item_label(&item, |key| custom_text(view, key));
    if let Some(extra) = stack.extra.as_mut() {
        extra.custom_name = label.map(|l| Tag::String(l.as_str().into()));
    }
}

/// Учесть время образцу: поставить срок, испортить просроченный, сдвинуть
/// срок холодом на `push` тиков. Вернёт строку, если образец испортился.
pub fn age(stack: &mut ItemStack, now: i64, push: i64) -> Option<String> {
    let kind = m::sample_kind(&stack.item)?;
    let Some(due) = deadline(stack) else {
        put(stack, m::SPOIL_KEY, Tag::Long(now + m::SAMPLE_SHELF));
        return None;
    };
    let steps = m::spoil_steps(due, now);
    if steps == 0 {
        if push > 0 {
            put(stack, m::SPOIL_KEY, Tag::Long(due + push));
        }
        return None;
    }
    let data = m::sample_key(kind).and_then(|key| custom_text(stack, key));
    match m::spoil_sample(kind, data.as_deref(), steps) {
        m::Spoiled::Item(id) => {
            let count = stack.count;
            *stack = ItemStack { item: id.into(), count, damage: 0, extra: None };
            Some("Образец сгнил: ткань вне холода не хранится".to_string())
        }
        m::Spoiled::Data(text) => {
            if let Some(key) = m::sample_key(kind) {
                put(stack, key, Tag::String(text.as_str().into()));
            }
            put(stack, m::SPOIL_KEY, Tag::Long(m::next_deadline(due, steps) + push));
            relabel(stack);
            let item = stack.item.to_string();
            let view: &ItemStack = stack;
            let name = m::item_label(&item, |key| custom_text(view, key)).unwrap_or(item);
            Some(format!("Образец портится вне холода: {name}"))
        }
    }
}

/// Сервер ли это: данные стака меняет только он.
fn serving(game: &crate::game::Game<'_>) -> bool {
    game.serving.is_some() || game.options.dedicated.is_some()
}

/// Есть ли у игрока термобокс (инвентарь и вторая рука).
fn carries_thermobox(game: &mut crate::game::Game<'_>) -> bool {
    (0..36).chain([-106]).any(|slot| game.native_carried_stack(slot).is_some_and(|s| &*s.item == m::THERMOBOX))
}

/// Тик стака в инвентаре (`inventory.rs`): образцы раз в `SPOIL_EVERY`.
pub fn inventory_tick(game: &mut crate::game::Game<'_>, slot: i32) {
    let now = game.world.game_time;
    if now % m::SPOIL_EVERY != 0 || !serving(game) {
        return;
    }
    if !game.native_carried_stack(slot).is_some_and(|s| m::is_sample(&s.item)) {
        return;
    }
    let cold = if carries_thermobox(game) { m::Cold::Thermobox } else { m::Cold::Warm };
    let push = m::cold_push(cold, m::SPOIL_EVERY);
    let Some(stack) = game.native_carried_stack(slot) else {
        return;
    };
    if let Some(line) = age(stack, now, push) {
        crate::game::biolab_say::say(game, &line);
    }
}

/// Тик криобанка (`machine.rs`): слоты и холод.
pub fn cryobank_tick(game: &mut crate::game::Game<'_>, entity: &mut ModBlockEntity) {
    if entity.items.len() < m::CRYOBANK_SLOTS {
        entity.items.resize(m::CRYOBANK_SLOTS, None);
    }
    if !serving(game) {
        return;
    }
    let now = game.world.game_time;
    let last = match entity.nbt.get(&CRYO_AT_KEY.into()) {
        Some(Tag::Long(v)) => *v,
        _ => now,
    };
    let elapsed = now - last;
    if elapsed > 0 && elapsed < m::SPOIL_EVERY {
        return;
    }
    // Часы назад (`/time` их не трогает, но мир могли подменить) — без сдвига.
    let push = m::cold_push(m::Cold::Cryobank, elapsed.max(0));
    for stack in entity.items.iter_mut().flatten() {
        let _ = age(stack, now, push);
    }
    entity.nbt.insert(CRYO_AT_KEY.into(), Tag::Long(now));
}

/// Строки подсказки образца: свежесть и как её сберечь.
pub fn freshness_lines(game: &crate::game::Game<'_>, stack: &ItemStack) -> Vec<String> {
    if !m::is_sample(&stack.item) {
        return Vec::new();
    }
    vec![m::freshness_line(deadline(stack), game.world.game_time), m::FRESHNESS_HINT.to_string()]
}
