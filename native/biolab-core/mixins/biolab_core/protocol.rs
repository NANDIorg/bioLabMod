// Подбор режима ферментера и экстрактора в игре (GDD 4.23 п. 3). Правила —
// раздел «M9: машины» в `runtime.rs`, под тестами; здесь только работа с
// блок-сущностью и стеками.
//
// Ручки живут в сейве машины (`KNOB_*_KEY`): числа меню не сохраняются, и
// после перезахода режим бы сбросился. Числа экрана машина пишет каждый тик
// (`prepare`), оценку попытки отдаёт тик машины, а `finish` решает, что из
// этого вышло: найденное окно — в сейв, подсказку — на экран и в чат, брак —
// колбой-пробой в выход.

use crate::game::biolab_core as m;
use horizon_client::chunk::block_entity::ModBlockEntity;
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

/// Строка из `custom_data` стека.
pub fn custom_text(stack: &ItemStack, key: &str) -> Option<String> {
    let extra = stack.extra.as_ref()?;
    let (_, data) = extra.other.iter().find(|(name, _)| name == "minecraft:custom_data")?;
    match data.get(key) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    }
}

/// Записать строку в `custom_data` стека, заменив прежнюю запись мода.
pub fn set_custom(stack: &mut ItemStack, pairs: &[(&str, Tag)], name: &str) {
    let data = stack.extra.get_or_insert_with(Default::default);
    data.other.retain(|(key, _)| key != "minecraft:custom_data");
    let entries: Vec<(&str, Tag)> = pairs.iter().map(|(key, value)| (*key, value.clone())).collect();
    data.other.push(("minecraft:custom_data".to_string(), Tag::compound(entries)));
    data.other.sort_by(|a, b| a.0.cmp(&b.0));
    data.custom_name = Some(Tag::String(name.into()));
}

/// Число из сейва машины.
fn saved_int(entity: &ModBlockEntity, key: &str, default: i32) -> i32 {
    match entity.nbt.get(&key.into()) {
        Some(Tag::Int(v)) => *v,
        _ => default,
    }
}

/// Строка из сейва машины.
fn saved_text(entity: &ModBlockEntity, key: &str) -> String {
    match entity.nbt.get(&key.into()) {
        Some(Tag::String(s)) => s.to_string_lossy(),
        _ => String::new(),
    }
}

/// Где стоят ручки машины.
pub fn knobs(entity: &ModBlockEntity) -> (i32, i32) {
    (
        m::knob_clamp(
            m::Knob::Temperature,
            saved_int(entity, m::KNOB_TEMP_KEY, m::knob_default(m::Knob::Temperature)),
        ),
        m::knob_clamp(
            m::Knob::Hold,
            saved_int(entity, m::KNOB_HOLD_KEY, m::knob_default(m::Knob::Hold)),
        ),
    )
}

/// Поставить ручки. Прогресс начатой работы сбрасывается: иначе попытку
/// можно было бы доварить на других числах.
pub fn set_knobs(entity: &mut ModBlockEntity, temperature: i32, hold: i32) {
    let temperature = m::knob_clamp(m::Knob::Temperature, temperature);
    let hold = m::knob_clamp(m::Knob::Hold, hold);
    entity.nbt.insert(m::KNOB_TEMP_KEY.into(), Tag::Int(temperature));
    entity.nbt.insert(m::KNOB_HOLD_KEY.into(), Tag::Int(hold));
    let slots = m::machine_data_slots(&entity.id);
    entity.data.resize(slots, 0);
    entity.data[m::DATA_PROGRESS] = 0;
    entity.data[m::DATA_TOTAL] = 0;
    entity.data[m::DATA_PROTO_TEMP] = temperature;
    entity.data[m::DATA_PROTO_HOLD] = hold;
}

/// Окно сырья: его знает только сервер, по сиду мира.
fn window(game: &crate::game::Game<'_>, machine: &str, recipe: &str) -> m::Window {
    m::protocol_window(game.level.seed, machine, recipe)
}

/// Числа панели — экрану, оценка попытки — тику машины. `input` — id сырья
/// во входе. None — у машины ручек нет: тик работает как раньше.
pub fn prepare(
    game: &crate::game::Game<'_>,
    entity: &mut ModBlockEntity,
    input: Option<&str>,
) -> Option<m::Grade> {
    if !m::has_protocol(&entity.id) {
        return None;
    }
    let slots = m::machine_data_slots(&entity.id);
    entity.data.resize(slots, 0);
    let (temperature, hold) = knobs(entity);
    entity.data[m::DATA_PROTO_TEMP] = temperature;
    entity.data[m::DATA_PROTO_HOLD] = hold;
    entity.data[m::DATA_PROTO_AIM] = saved_int(entity, m::AIM_KEY, 0);
    let Some(recipe) = input else {
        entity.data[m::DATA_PROTO_STATE] = m::PROTO_NO_RECIPE;
        return None;
    };
    let found = m::found_get(&saved_text(entity, m::FOUND_KEY), recipe).is_some();
    entity.data[m::DATA_PROTO_STATE] =
        if found { m::PROTO_FOUND } else { m::PROTO_SEARCHING };
    let machine = entity.id.to_string();
    Some(m::protocol_grade(temperature, hold, &window(game, &machine, recipe)))
}

/// Колба-проба: брак, состав которой ещё предстоит опознать (GDD 4.23 п. 4).
pub fn spoil_flask(seed: u64) -> ItemStack {
    let contents = m::spoil_compound(seed);
    let (_, ml) = m::DEFAULT_FILL;
    let mut flask = ItemStack { item: m::FLASK_ID.into(), count: 1, damage: 0, extra: None };
    set_custom(
        &mut flask,
        &[
            (m::CONTENTS_KEY, Tag::String(contents.into())),
            (m::ML_KEY, Tag::Int(ml)),
            (m::SEALED_KEY, Tag::Int(1)),
        ],
        &m::sample_label(ml),
    );
    flask
}

/// Попытка закончилась: подсказка игроку, найденное окно — в сейв, брак —
/// пробой в выход. `output` — слот выхода машины.
pub fn finish(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
    recipe: &str,
    output: usize,
    grade: m::Grade,
) {
    let machine = entity.id.to_string();
    let window = window(game, &machine, recipe);
    let (temperature, hold) = knobs(entity);
    // Подсказку экран показывает до следующей попытки: числа меню не
    // сохраняются, поэтому она лежит в сейве машины.
    let code = m::aim_code(
        m::knob_aim(temperature, window.temperature),
        m::knob_aim(hold, window.hold),
    );
    entity.nbt.insert(m::AIM_KEY.into(), Tag::Int(code));
    let slots = m::machine_data_slots(&entity.id);
    entity.data.resize(slots, 0);
    entity.data[m::DATA_PROTO_AIM] = code;
    if grade == m::Grade::Exact {
        let saved = saved_text(entity, m::FOUND_KEY);
        let next = m::found_put(&saved, recipe, window.temperature, window.hold);
        entity.nbt.insert(m::FOUND_KEY.into(), Tag::String(next.as_str().into()));
        entity.data[m::DATA_PROTO_STATE] = m::PROTO_FOUND;
    }
    if grade == m::Grade::Spoiled {
        // Брак — не пустая потеря: в выход падает проба. Занят выход —
        // брак просто пропал, как пропало бы сырьё.
        let seed = (pos[0] as i64 * 17 + pos[2] as i64 * 7 + game.ticks as i64).unsigned_abs();
        if let Some(slot) = entity.items.get_mut(output)
            && slot.is_none()
        {
            *slot = Some(spoil_flask(seed));
        }
    }
    crate::game::biolab_say::say(game, &m::protocol_report(temperature, hold, &window));
}

/// Карта протокола по машине: пустая — записать найденное окно, записанная —
/// поставить ручки. true — клик наш, меню открывать не надо.
pub fn use_card(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    stack: &ItemStack,
) -> bool {
    if &*stack.item != m::PROTOCOL_CARD {
        return false;
    }
    let block = game.native_block(pos);
    if !m::has_protocol(&block) {
        return false;
    }
    let card = custom_text(stack, m::CARD_KEY).and_then(|text| m::card_from_text(&text));
    // Что лежит во входе машины и какое окно она уже знает.
    let seen = game.native_with_mod_block_entity(pos, |entity| {
        let input = entity
            .items
            .get(m::SLOT_INPUT)
            .and_then(|s| s.as_ref())
            .map(|s| s.item.to_string());
        (input, saved_text(entity, m::FOUND_KEY))
    });
    let Some((input, found)) = seen else {
        return false;
    };
    let line = m::card_message(card.as_ref(), &block, input.as_deref());
    match card {
        // Записанная карта ставит ручки — своему сырью и своей машине.
        Some(card) if card.machine == block && input.as_deref() == Some(card.recipe.as_str()) => {
            let _ = game.native_with_mod_block_entity(pos, |entity| {
                set_knobs(entity, card.temperature, card.hold);
            });
        }
        // Пустая карта пишет найденное окно того сырья, что стоит во входе.
        None => {
            let Some(recipe) = input.as_deref() else {
                crate::game::biolab_say::say(game, "Положите во вход сырьё: карта пишет режим под него");
                return true;
            };
            let Some((temperature, hold)) = m::found_get(&found, recipe) else {
                crate::game::biolab_say::say(game, "Режим этого сырья ещё не найден");
                return true;
            };
            let card = m::Card {
                machine: block.clone(),
                recipe: recipe.to_string(),
                temperature,
                hold,
            };
            let written = write_card(game, &card);
            let line = if written {
                m::card_label(&card)
            } else {
                "Карту записать не вышло".to_string()
            };
            crate::game::biolab_say::say(game, &line);
            return true;
        }
        _ => {}
    }
    crate::game::biolab_say::say(game, &line);
    true
}

/// Записать карту в руке игрока. Стопка у карты одна (GDD 4.23 п. 3), так
/// что меняем сам стек в слоте — как мазок на последней палочке
/// (`infected_swab.rs`).
fn write_card(game: &mut crate::game::Game<'_>, card: &m::Card) -> bool {
    let label = m::card_label(card);
    let text = m::card_to_text(card);
    // Рука, которой кликнули: основная, если карта в ней, иначе левая.
    let main = game.selected as i32;
    let hand = if game.native_carried_stack(main).is_some_and(|held| &*held.item == m::PROTOCOL_CARD)
    {
        main
    } else {
        -106
    };
    match game.native_carried_stack(hand) {
        Some(held) if &*held.item == m::PROTOCOL_CARD => {
            set_custom(held, &[(m::CARD_KEY, Tag::String(text.as_str().into()))], &label);
            true
        }
        _ => false,
    }
}
