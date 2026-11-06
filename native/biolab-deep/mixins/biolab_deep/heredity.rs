// Гены по наследству (GDD 4.26, В; M11-18): то, что видит только игра, —
// мобы уровня, данные на них и память игры о снятых мобах. Правила — чистые
// функции `runtime.rs` (раздел «M11-Б/В: урон и наследование»).
//
// Хуки: `breed.rs` и `baby_spawn.rs` запоминают рождение, `joined.rs`
// отдаёт детёнышу геном по наследству вместо природного броска,
// `converted.rs` переносит геном и срезы на превратившегося моба.
//
// Модуль живёт в игре: `crate::game::biolab_heredity`. Внутренние поля
// `Game` (`mobs`, `ticks`, `dim`, `persist.uuids`, `entity_attachments`) —
// не хук-поинты: при обновлении Horizon проверить.

use crate::game::biolab_deep as d;
use crate::game::biolab_deep_data as data;
use horizon_client::mod_api::{Holder, Host};

/// Родители `parent` и `partner` вот-вот родят: запомнить их геномы, точку
/// и тик. Детёныш появится в точке `parent` (`spawn_mob` в `breed`).
pub fn expect_birth(game: &mut crate::game::Game<'_>, parent: u32, partner: u32) {
    let Some((at, egg)) = game.mobs.iter().find(|m| m.id == parent).map(|m| {
        (
            [m.position.x, m.position.y, m.position.z],
            d::hatches_from_egg(m.kind.id.as_str()),
        )
    }) else {
        return;
    };
    if egg {
        return;
    }
    let genomes = (data::genome(game, parent), data::genome(game, partner));
    let birth = d::Birth {
        dimension: game.dim.id().to_owned(),
        at,
        tick: game.ticks,
        parents: (parent, partner),
        genomes,
    };
    if let Ok(mut births) = d::births().lock() {
        births.expect(birth);
    }
}

/// Новый моб `id` — детёныш ожидаемого рождения? Тогда ответ — его геном по
/// наследству строкой данных (пустая — гены не передались, и бросать ему
/// всё равно не надо). `None` — не детёныш мутантов.
pub fn inherited(game: &mut crate::game::Game<'_>, id: u32) -> Option<String> {
    let at = game
        .mobs
        .iter()
        .find(|m| m.id == id)
        .map(|m| [m.position.x, m.position.y, m.position.z])?;
    let birth = d::births().lock().ok()?.claim(game.dim.id(), at, game.ticks)?;
    Some(birth.heir(id))
}

/// Моб `old` превратился в `new`: геном, отметку броска и срезы — новому.
pub fn carry_over(game: &mut crate::game::Game<'_>, old: u32, new: u32) {
    let (genome, checked, surgery) = left_behind(game, old);
    if let Some(text) = d::converted_genome(genome.as_deref(), checked) {
        data::set_genome(game, new, &text);
        data::set_checked(game, new);
    }
    let bits = d::converted_surgery(surgery);
    if bits != 0 {
        data::set_surgery(game, new, bits);
    }
}

/// Моб `old` вот-вот станет видом `outcome` (`native_living_conversion_pre`):
/// запомнить, что достанется новому. Нужно молнии — там пин не зовёт `post`.
pub fn expect_conversion(game: &mut crate::game::Game<'_>, old: u32, outcome: &str) {
    let Some(at) = game
        .mobs
        .iter()
        .find(|m| m.id == old)
        .map(|m| [m.position.x, m.position.y, m.position.z])
    else {
        return;
    };
    let (genome, checked, surgery) = left_behind(game, old);
    let conversion = d::Conversion {
        dimension: game.dim.id().to_owned(),
        at,
        tick: game.ticks,
        outcome: outcome.to_owned(),
        genome: d::converted_genome(genome.as_deref(), checked),
        surgery: d::converted_surgery(surgery),
    };
    if let Ok(mut list) = d::conversions().lock() {
        list.expect(conversion);
    }
}

/// Новый моб `id` вида `kind` — результат ожидаемого превращения? Тогда
/// записать ему геном и срезы старого и ответить `true`.
pub fn converted_here(game: &mut crate::game::Game<'_>, id: u32, kind: &str) -> bool {
    let Some(at) = game
        .mobs
        .iter()
        .find(|m| m.id == id)
        .map(|m| [m.position.x, m.position.y, m.position.z])
    else {
        return false;
    };
    let Some(conversion) = d::conversions()
        .lock()
        .ok()
        .and_then(|mut list| list.claim(game.dim.id(), kind, at, game.ticks))
    else {
        return false;
    };
    data::set_checked(game, id);
    if let Some(text) = conversion.genome {
        data::set_genome(game, id, &text);
    }
    if conversion.surgery != 0 {
        data::set_surgery(game, id, conversion.surgery);
    }
    true
}

/// Данные моба, которого уже сняли с уровня: геном, был ли бросок, биты
/// операционной. `Host` их не отдаёт — носителя нет (`attachment_entity_exists`
/// пропускает снятых, `game/mod_api_host.rs` пина), — но сами данные лежат в
/// памяти игры по uuid моба до `prune_entity_attachments` в следующем тике.
/// Если моб всё же ещё в мире — обычным путём.
fn left_behind(game: &mut crate::game::Game<'_>, id: u32) -> (Option<String>, bool, u8) {
    {
        let host: &mut dyn Host = &mut *game;
        let holder = Holder::Entity(id);
        if let Ok(genome) = data::GENOME.get_existing(&mut *host, holder) {
            let checked = data::CHECKED.has(&mut *host, holder).unwrap_or(false);
            let surgery = data::SURGERY.get_existing(&mut *host, holder).ok().flatten().unwrap_or(0);
            return (genome, checked, surgery);
        }
    }
    let Some(uuid) = game.persist.uuids.get(&id).copied() else {
        return (None, false, 0);
    };
    let Some(store) = game.entity_attachments.get_mut(&uuid) else {
        return (None, false, 0);
    };
    let genome = store.get_existing(&data::GENOME).ok().flatten();
    let checked = store.has(&data::CHECKED);
    let surgery = store.get_existing(&data::SURGERY).ok().flatten().unwrap_or(0);
    (genome, checked, surgery)
}
