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
    let _ = (game, pos, entity);
}

/// Тик блоков сети, которые не машины: накопители (тянут из генераторов),
/// повторитель. Тело — агента «Сеть».
pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let _ = (game, pos, entity);
}
