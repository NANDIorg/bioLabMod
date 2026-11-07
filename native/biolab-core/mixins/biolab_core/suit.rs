// Спецодежда и загрязнение в игре (M9, GDD 4.25, 4.23 п. 13).
//
// Правила и числа — раздел «M9: спецодежда и загрязнение» в `runtime.rs`,
// там же тесты. Здесь — исполнение на живой игре, общее для нескольких хуков:
// тик костюма (`suit_tick.rs`), патрон и хлор (`suit_use.rs`), съеденное
// (`contam_eat.rs`), подсказка (`contam_tooltip.rs`), добыча
// (`contam_drops.rs`), печь (`cook_start.rs`, `cook_done.rs`), рамка маски
// (`suit_mask.rs`), газ снотворного (`sleep.rs`), анализатор (`analyzer.rs`).
// Поэтому это модуль игры (`crate::game::biolab_suit`), как `biolab_sleep`.

use crate::game::biolab_core as m;
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

/// Строка под ключом `key` в `custom_data` стека.
pub fn custom_text(stack: &ItemStack, key: &str) -> Option<String> {
    stack
        .extra
        .as_ref()?
        .other
        .iter()
        .find(|(k, _)| k == "minecraft:custom_data")
        .and_then(|(_, data)| match data.get(key) {
            Some(Tag::String(s)) => Some(s.to_string_lossy()),
            _ => None,
        })
}

/// Записать (или снять, `None`) строку под ключом `key` в `custom_data`,
/// не трогая чужих ключей: у одного стека может быть и штамм культуры, и
/// загрязнение. Пустой `custom_data` убирается целиком.
pub fn set_custom_text(stack: &mut ItemStack, key: &str, value: Option<&str>) {
    let data = stack.extra.get_or_insert_with(Default::default);
    let mut tag = match data.other.iter().position(|(k, _)| k == "minecraft:custom_data") {
        Some(i) => data.other.remove(i).1,
        None => Tag::Compound(Default::default()),
    };
    if !matches!(tag, Tag::Compound(_)) {
        tag = Tag::Compound(Default::default());
    }
    let mut empty = false;
    if let Tag::Compound(map) = &mut tag {
        match value {
            Some(value) => {
                map.insert(key.into(), Tag::String(value.into()));
            }
            None => {
                map.remove(&horizon_world::nbt::JavaString::from(key));
            }
        }
        empty = map.is_empty();
    }
    if !empty {
        data.other.push(("minecraft:custom_data".to_string(), tag));
        data.other.sort_by(|a, b| a.0.cmp(&b.0));
    }
}

/// Загрязнение стека (`CONTAM_KEY`).
pub fn contamination(stack: &ItemStack) -> Option<m::Contamination> {
    custom_text(stack, m::CONTAM_KEY).and_then(|text| m::contamination_from_text(&text))
}

/// Записать загрязнение стека; `None` — стек чистый.
pub fn set_contamination(stack: &mut ItemStack, c: Option<m::Contamination>) {
    if contamination(stack) == c {
        return;
    }
    let text = c.map(|c| m::contamination_text(&c));
    set_custom_text(stack, m::CONTAM_KEY, text.as_deref());
}

/// Что надето, в форме контракта: слоты сверху вниз, `custom_data` в SNBT —
/// как собирает тик дозы (`dose_tick.rs`). Броня игры — `armor[0]` голова …
/// `armor[3]` ступни (`native_carried_slot`: 103 − номер). В LAN — броня
/// действующего игрока.
pub fn worn(game: &crate::game::Game<'_>) -> m::Worn {
    let mut worn = m::Worn::default();
    for (i, stack) in game.armor.iter().enumerate() {
        let Some(stack) = stack.as_ref().filter(|s| s.count > 0) else {
            continue;
        };
        let data = stack
            .extra
            .as_ref()
            .and_then(|extra| extra.other.iter().find(|(key, _)| key == "minecraft:custom_data"))
            .map(|(_, data)| horizon_world::snbt_text::to_snbt(data))
            .unwrap_or_default();
        worn.slots[i] = (stack.item.to_string(), data);
    }
    worn
}

/// Стоит ли рядом работающая горячая машина (`SUIT_HEAT_SOURCES`, отметка
/// `BUSY_KEY`, её ставит тик машины). Читаем блок-сущность напрямую, как
/// фон частиц (`ambient.rs`), чтобы не помечать чанк изменённым.
fn hot_machine_near(game: &crate::game::Game<'_>, at: [i32; 3]) -> bool {
    let r = m::HEAT_RANGE;
    for dx in -r..=r {
        for dy in -2..=2 {
            for dz in -r..=r {
                let pos = [at[0] + dx, at[1] + dy, at[2] + dz];
                if !m::SUIT_HEAT_SOURCES.contains(&game.native_block(pos).as_str()) {
                    continue;
                }
                let busy = matches!(
                    game.world.block_entities.get(&(pos[0], pos[1], pos[2])),
                    Some(horizon_client::chunk::block_entity::BlockEntity::Mod(e))
                        if matches!(e.nbt.get(&m::BUSY_KEY.into()), Some(Tag::Int(1)))
                );
                if busy {
                    return true;
                }
            }
        }
    }
    false
}

/// Заражённое место у клетки `pos`: заразная лужа в ней (`Fluid::infects`).
fn infectious_puddle(game: &crate::game::Game<'_>, pos: [i32; 3]) -> bool {
    m::fluid_of_block(&game.native_block(pos)).is_some_and(|f| f.infects > 0)
}

/// Остатки термостойкого штамма — на выход печей этого уровня
/// (`CookBook`: хуки печи только читают, пишем здесь). Выход с остатком не
/// сливается с чистым: печь ждёт, пока его заберут, — так и задумано.
fn apply_cooked(game: &mut crate::game::Game<'_>, dimension: &str) {
    let ready = match m::cook_book().lock() {
        Ok(mut book) => book.take_ready(dimension),
        Err(_) => return,
    };
    for (pos, rest) in ready {
        if let Some(horizon_client::chunk::block_entity::BlockEntity::Furnace(f)) =
            game.world.block_entities.get_mut(&(pos[0], pos[1], pos[2]))
            && let Some(out) = f.items[2].as_mut()
        {
            let next = m::dirtier(contamination(out), Some(rest));
            set_contamination(out, next);
        }
    }
}

/// Секунда костюма на сервере: цвет деталей, расход фильтра, грязь от лужи и
/// утечки на надетом и на предметах вокруг, контакт с грязным в руках,
/// перегрев, остаток в печах. Зовёт `suit_tick.rs` с тика игрока.
pub fn tick(game: &mut crate::game::Game<'_>) {
    if !game.native_on(m::ID) || game.ticks % 20 != 0 {
        return;
    }
    // Мир настоящий только у своего сервера или выделенного.
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };
    let uuid = game.native_acting_id();
    let dimension = player.dimension.to_string();
    let feet = [player.x, player.y, player.z];
    let at = [feet[0].floor() as i32, feet[1].floor() as i32, feet[2].floor() as i32];
    let mut news: Vec<String> = Vec::new();

    // Детали: цвет слоя на теле (`dyed_color` стека) и фильтр респиратора.
    for slot in 100..=103 {
        let Some(stack) = game.native_carried_stack(slot) else {
            continue;
        };
        let Some(piece) = m::suit_piece(&stack.item) else {
            continue;
        };
        let filter = (piece.id == m::RESPIRATOR_ID)
            .then(|| m::respirator_filter(custom_text(stack, m::SUIT_KEY).as_deref()));
        let data = stack.extra.get_or_insert_with(Default::default);
        if data.dyed_color != Some(piece.colour) {
            data.dyed_color = Some(piece.colour);
        }
        if let Some(left) = filter {
            let (next, said) = m::filter_tick(left);
            if next != left {
                set_custom_text(stack, m::SUIT_KEY, Some(&m::suit_text(next)));
            }
            news.extend(said.map(str::to_string));
        }
    }

    // Грязь на надетом: заразная лужа под ногами, утечка рядом. В держащей
    // камере утечки нет (`leak_near`), лужа пачкает и там.
    let puddle = infectious_puddle(game, at);
    let leak = m::leak_near(&dimension, feet);
    if puddle || leak.is_some() {
        for slot in 100..=103 {
            let dirt = m::suit_dirt(103 - slot as usize, puddle, leak.is_some());
            if dirt == 0 {
                continue;
            }
            if let Some(stack) = game.native_carried_stack(slot) {
                let next = m::contaminate(contamination(stack), dirt, leak);
                set_contamination(stack, next);
            }
        }
    }

    // Предметы на земле вокруг: лежит в заразной луже или у утечки — грязнее.
    let reach = 8.0;
    let dropped = game.native_item_entities(
        &dimension,
        [feet[0] - reach, feet[1] - 4.0, feet[2] - reach],
        [feet[0] + reach, feet[1] + 4.0, feet[2] + reach],
    );
    for item in dropped {
        let pos = [
            item.position[0].floor() as i32,
            item.position[1].floor() as i32,
            item.position[2].floor() as i32,
        ];
        let place = m::place_contamination(infectious_puddle(game, pos), m::leak_near(&dimension, item.position));
        let Some((rate, strain)) = place else {
            continue;
        };
        let before = contamination(&item.stack);
        let next = m::contaminate(before, rate, strain);
        if next == before {
            continue;
        }
        let mut stack = item.stack.clone();
        set_contamination(&mut stack, next);
        game.native_set_item_entity(&dimension, item.id, stack);
    }

    // Грязное в руках — доза контактом (спецодежду учтёт тик дозы).
    for slot in [game.selected as i32, -106] {
        if let Some(c) = game.native_carried_stack(slot).and_then(|stack| contamination(stack)) {
            m::queue_dose(uuid, m::DoseChannel::Contact, m::handle_dose(&c), c.strain);
        }
    }

    // Перегрев в закрытом костюме у жара.
    let sealed = m::sealed(&worn(game));
    let hot = sealed && (m::HOT_DIMENSIONS.contains(&dimension.as_str()) || hot_machine_near(game, at));
    let (heat, said) = m::heat_tick(uuid, sealed, hot);
    if m::overheated(heat) {
        game.add_player_effect(horizon_client::effects::EffectInstance::new("slowness", 40, 0));
        game.add_player_effect(horizon_client::effects::EffectInstance::new("weakness", 40, 0));
    }
    news.extend(said.map(str::to_string));

    // Остаток термостойкого штамма — на выход печей.
    apply_cooked(game, &dimension);

    for line in news {
        crate::game::biolab_say::say(game, &line);
    }
}
