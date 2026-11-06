// Многоблочные установки в игре (GDD 4.18, M8): общий каркас, одинаковый для
// всех установок. Решения — раздел «M8» в `runtime.rs` (схема, проверка,
// числа экрана, раскладка по слотам порта), здесь только исполнение на живой
// игре: прочитать блоки, склеить вид, записать сейв, перенести предметы.
//
// Зовут его тик контроллера (`machine.rs`: `tick`, потом работа своей
// установки — `biolab_column::work` у колонны) и ломание контроллера игроком
// (`multiblock_broken.rs`: `broken`). Модуль игры (`crate::game::
// biolab_multiblock`), а не вложенные функции хука: их не позвать из другого
// хука, а установок будет четыре.
//
// Сейв контроллера (`entity.nbt`): `biolab_mb_formed` (1 — собрано),
// `biolab_mb_sections`, `biolab_mb_parts` (склеенные части, `[x, y, z, …]`),
// `biolab_mb_ports` (`[x, y, z, вид, …]`). Числа экрана — `DATA_MB_*`: они не
// сохраняются, после загрузки там 0 (`MB_UNKNOWN`), и проверка идёт сразу.

use crate::game::biolab_core as m;
use horizon_client::chunk::block_entity::ModBlockEntity;
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

fn int(entity: &ModBlockEntity, key: &str) -> i32 {
    match entity.nbt.get(&key.into()) {
        Some(Tag::Int(v)) => *v,
        _ => 0,
    }
}

fn ints(entity: &ModBlockEntity, key: &str) -> Vec<i32> {
    match entity.nbt.get(&key.into()) {
        Some(Tag::IntArray(v)) => v.clone(),
        _ => Vec::new(),
    }
}

/// Склеить (`formed=true`) или расклеить части. Пишем только своим блокам со
/// свойством `formed`: на месте части мог встать чужой блок. Помощник сам
/// ничего не пишет, если значение уже такое.
fn set_formed(game: &mut crate::game::Game<'_>, list: &[[i32; 3]], formed: bool) {
    for &pos in list {
        if m::is_formable(&game.native_block(pos)) {
            game.native_set_block_property(pos, m::FORMED, m::formed_value(formed));
        }
    }
}

/// Итог проверки — в сейв контроллера.
fn save(entity: &mut ModBlockEntity, verdict: &m::Verdict) {
    let empty = m::Assembly::default();
    let (formed, a) = match verdict {
        m::Verdict::Formed(a) => (1, a),
        m::Verdict::Broken(_) => (0, &empty),
    };
    entity.nbt.insert(m::MB_FORMED_KEY.into(), Tag::Int(formed));
    entity.nbt.insert(m::MB_SECTIONS_KEY.into(), Tag::Int(a.sections as i32));
    entity.nbt.insert(m::MB_PARTS_KEY.into(), Tag::IntArray(m::positions_to_ints(&a.parts)));
    entity.nbt.insert(m::MB_PORTS_KEY.into(), Tag::IntArray(m::ports_to_ints(&a.ports)));
}

/// Тик контроллера установки: раз в секунду сверить схему (сразу — после
/// загрузки), склеить или расклеить части, записать итог в сейв и в числа
/// экрана; у собранной раз в `MB_PORT_EVERY` тиков перенести предметы через
/// порты. Вернёт собранную установку или None — тогда работа стоит, а
/// содержимое контроллера остаётся на месте.
pub fn tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
) -> Option<m::Assembly> {
    let mb = m::multiblock(&entity.id)?;
    // Число слотов считаем заранее: `resize` берёт `entity.data` изменяемо.
    let slots = m::machine_data_slots(&entity.id);
    entity.data.resize(slots, 0);
    let ticks = game.ticks as i64;
    if m::check_due(ticks, pos, entity.data[m::DATA_MB_STATE]) {
        let facing = game
            .native_block_property(pos, "facing")
            .and_then(|f| m::Facing::from_name(&f))
            .unwrap_or(m::Facing::North);
        let site = m::Site { origin: pos, facing };
        let verdict = {
            let at = |p: [i32; 3]| game.native_block(p);
            m::check_multiblock(mb, site, &at)
        };
        m::write_verdict(&mut entity.data, &verdict);
        let before = m::positions_from_ints(&ints(entity, m::MB_PARTS_KEY));
        let now: &[[i32; 3]] = match &verdict {
            m::Verdict::Formed(a) => &a.parts,
            m::Verdict::Broken(_) => &[],
        };
        let (glue, unglue) = m::glue_changes(&before, now);
        set_formed(game, &unglue, false);
        set_formed(game, &glue, true);
        save(entity, &verdict);
    }
    if int(entity, m::MB_FORMED_KEY) != 1 {
        return None;
    }
    let assembly = m::Assembly {
        sections: int(entity, m::MB_SECTIONS_KEY).max(0) as u32,
        parts: m::positions_from_ints(&ints(entity, m::MB_PARTS_KEY)),
        ports: m::ports_from_ints(&ints(entity, m::MB_PORTS_KEY)),
    };
    if ticks.rem_euclid(m::MB_PORT_EVERY) == 0 {
        move_ports(game, entity, &assembly);
    }
    Some(assembly)
}

/// Контроллер сломал игрок (`multiblock_broken.rs`): расклеить части, которые
/// он склеил, — иначе они так и стояли бы «собранными». Блок-сущность в этот
/// момент ещё в мире и не тикает, её сейв читается.
pub fn broken(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    let parts = game
        .native_with_mod_block_entity(pos, |e| {
            if m::multiblock(&e.id).is_none() {
                return Vec::new();
            }
            m::positions_from_ints(&ints(e, m::MB_PARTS_KEY))
        })
        .unwrap_or_default();
    set_formed(game, &parts, false);
}

/// Стопка предмета. `items()` — таблица предметов игры, не хук-поинт: при
/// обновлении Horizon проверить.
fn max_of(game: &crate::game::Game<'_>, stack: &ItemStack) -> u32 {
    game.items().get(&stack.item).map_or(64, |i| i.max_stack)
}

/// Порты (GDD 4.18): из входных портов — во входы контроллера то, что вход
/// берёт (`slot_accepts`); из выходов контроллера — в выходные порты. Как
/// воронка, но целыми стаками. Порты не тикают, поэтому их блок-сущности в
/// мире и доступны `native_with_mod_block_entity` (API 18). Стопку предмета
/// знает только таблица предметов игры, поэтому сперва читаем порт, считаем
/// раскладку, а уже потом пишем.
fn move_ports(game: &mut crate::game::Game<'_>, entity: &mut ModBlockEntity, a: &m::Assembly) {
    let Some(machine) = m::machine(&entity.id) else {
        return;
    };
    if entity.items.len() < machine.slots {
        entity.items.resize(machine.slots, None);
    }

    // Входные порты → входы контроллера (у биореактора туда же — топливо).
    let inputs = m::machine_inputs(machine);
    for port in a.inputs() {
        let Some(held) = game.native_with_mod_block_entity(port, |p| p.items.clone()) else {
            continue;
        };
        for (from, stack) in held.iter().enumerate() {
            let Some(stack) = stack else {
                continue;
            };
            let max = max_of(game, stack);
            let mut left = stack.count;
            let mut moves: Vec<(usize, u32)> = Vec::new();
            for &slot in &inputs {
                if left == 0 {
                    break;
                }
                if !m::slot_accepts(machine, slot, &stack.item) {
                    continue;
                }
                let have = entity.items[slot]
                    .as_ref()
                    .map(|h| (h.same_item_same_components(stack), h.count));
                let put = m::stack_room(have, max).min(left);
                if put > 0 {
                    moves.push((slot, put));
                    left -= put;
                }
            }
            let taken = stack.count - left;
            if taken == 0 {
                continue;
            }
            let took = game
                .native_with_mod_block_entity(port, |p| {
                    let Some(place) = p.items.get_mut(from) else {
                        return false;
                    };
                    let Some(s) = place.as_mut() else {
                        return false;
                    };
                    if !s.same_item_same_components(stack) || s.count < taken {
                        return false;
                    }
                    s.count -= taken;
                    if s.count == 0 {
                        *place = None;
                    }
                    true
                })
                .unwrap_or(false);
            if !took {
                continue;
            }
            for (slot, put) in moves {
                match entity.items[slot].as_mut() {
                    Some(h) => h.count += put,
                    None => entity.items[slot] = Some(ItemStack { count: put, ..stack.clone() }),
                }
            }
        }
    }

    // Выходы контроллера → выходные порты.
    for slot in m::machine_outputs(machine) {
        for port in a.outputs() {
            let Some(stack) = entity.items.get(slot).and_then(|s| s.clone()) else {
                break;
            };
            let max = max_of(game, &stack);
            let Some(held) = game.native_with_mod_block_entity(port, |p| {
                (0..m::PORT_SLOTS)
                    .map(|i| {
                        p.items
                            .get(i)
                            .and_then(|s| s.as_ref())
                            .map(|s| (s.same_item_same_components(&stack), s.count))
                    })
                    .collect::<Vec<_>>()
            }) else {
                continue;
            };
            let plan = m::port_plan(&held, stack.count, max);
            let moved: u32 = plan.iter().map(|(_, n)| n).sum();
            if moved == 0 {
                continue;
            }
            game.native_with_mod_block_entity(port, |p| {
                if p.items.len() < m::PORT_SLOTS {
                    p.items.resize(m::PORT_SLOTS, None);
                }
                for &(i, n) in &plan {
                    match p.items[i].as_mut() {
                        Some(h) => h.count += n,
                        None => p.items[i] = Some(ItemStack { count: n, ..stack.clone() }),
                    }
                }
            });
            if let Some(place) = entity.items.get_mut(slot)
                && let Some(s) = place.as_mut()
            {
                s.count -= moved.min(s.count);
                if s.count == 0 {
                    *place = None;
                }
            }
        }
    }
}
