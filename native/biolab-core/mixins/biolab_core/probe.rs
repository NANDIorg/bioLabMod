// Опознание пробы по частям (GDD 4.23 п. 4). Правила — раздел «M9: машины»
// в `runtime.rs`, под тестами; здесь работа спектрометра с пробой.
//
// Проба — колба с `biolab_sealed`: её состав игрок не видел. Спектр такой
// колбы спектрометр не рисует и имя сразу не даёт. Вместо этого игрок
// заказывает кнопкой дешёвую проверку (`panel_click.rs` → `panel_button.rs`
// кладёт её номер в сейв машины), машина её «делает» за `Probe::ticks`
// тиков и отмечает сделанной в знании мира (`biolab/probes.txt`). Когда
// остаётся один вариант и он подтверждается настоящей формулой, запись в
// журнале закрывается и печатается чертёж — ровно как при обычном анализе.

use crate::game::biolab_core as m;
use horizon_client::chunk::block_entity::ModBlockEntity;
use horizon_world::nbt::Tag;

/// Проба во входе: её состав (игроку не виден) — или None, если во входе
/// обычная колба, пусто или не наша вещь.
fn sample(entity: &ModBlockEntity) -> Option<String> {
    let stack = entity.items.get(m::SLOT_INPUT)?.as_ref()?;
    if &*stack.item != m::FLASK_ID {
        return None;
    }
    let extra = stack.extra.as_ref()?;
    let (_, data) = extra.other.iter().find(|(key, _)| key == "minecraft:custom_data")?;
    if !matches!(data.get(m::SEALED_KEY), Some(Tag::Int(1))) {
        return None;
    }
    match data.get(m::CONTENTS_KEY) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    }
}

/// Проба перестала быть пробой: опознали — печать ставится на саму колбу,
/// чтобы подсказка снова показывала формулу.
fn unseal(entity: &mut ModBlockEntity, contents: &str) {
    let Some(Some(stack)) = entity.items.get_mut(m::SLOT_INPUT) else {
        return;
    };
    let ml = stack
        .extra
        .as_ref()
        .and_then(|extra| extra.other.iter().find(|(key, _)| key == "minecraft:custom_data"))
        .and_then(|(_, data)| match data.get(m::ML_KEY) {
            Some(Tag::Int(v)) => Some(*v),
            _ => None,
        })
        .unwrap_or(0);
    // Печать снимаем значением, а не удалением ключа: `sample` читает её
    // строго как `1`.
    crate::game::biolab_protocol::set_custom(
        stack,
        &[
            (m::CONTENTS_KEY, Tag::String(contents.into())),
            (m::ML_KEY, Tag::Int(ml)),
            (m::SEALED_KEY, Tag::Int(0)),
        ],
        &m::flask_label(contents, ml),
    );
}

/// Заказанная проверка из сейва машины.
fn queued(entity: &ModBlockEntity) -> Option<m::Probe> {
    match entity.nbt.get(&m::PROBE_KEY.into()) {
        Some(Tag::Int(v)) => usize::try_from(*v - 1).ok().and_then(m::Probe::from_id),
        _ => None,
    }
}

fn forget_queue(entity: &mut ModBlockEntity) {
    entity.nbt.insert(m::PROBE_KEY.into(), Tag::Int(0));
}

/// Тик спектрометра с пробой. None — во входе не проба: тик машины работает
/// как раньше (спектр и обычный анализ).
pub fn work(
    game: &mut crate::game::Game<'_>,
    entity: &mut ModBlockEntity,
    output: usize,
) -> Option<m::Step> {
    let slots = m::machine_data_slots(&entity.id);
    entity.data.resize(slots, 0);
    let Some(contents) = sample(entity) else {
        // Обычная колба: панель опознания выключена, спектр рисуется.
        entity.data[m::DATA_PROBE_DONE] = 0;
        entity.data[m::DATA_PROBE_RESULTS] = 0;
        entity.data[m::DATA_PROBE_RUN] = 0;
        return None;
    };
    let world = game.world_root.clone();
    let Some(results) = m::probe_results_of(&contents) else {
        entity.data[m::DATA_PROBE_DONE] = 0;
        return None;
    };
    // Это вещество в мире уже опознано (его собирали в синтезаторе или
    // опознали другой пробой) — сужать нечего: печать снимается, и колба
    // идёт обычным путём.
    if m::is_identified(&world, &contents) {
        unseal(entity, &contents);
        entity.data[m::DATA_PROBE_DONE] = 0;
        entity.data[m::DATA_PROBE_RUN] = 0;
        return None;
    }
    let done = m::probes_done(&world, &contents);

    // Спектр пробы не показываем: состав — это и есть ответ.
    for i in 0..m::ELEMENTS.len() {
        entity.data[m::DATA_SPECTRUM + i] = 0;
    }
    entity.data[m::DATA_PROBE_DONE] = i32::from(done) + 1;
    entity.data[m::DATA_PROBE_RESULTS] = m::pack_probes(&results);

    // Опознали — обычный анализ дальше не нужен: чертёж печатается сразу.
    let guess = m::probe_guess(done, &results);
    let settled = match &guess {
        m::Guess::One(text) => *text == contents.as_str(),
        m::Guess::Unknown | m::Guess::Ambiguous(_) => true,
        m::Guess::Narrowing(_) => false,
    };
    if settled {
        let free = entity.items.get(output).is_some_and(|slot| slot.is_none());
        if !free {
            entity.data[m::DATA_PROBE_RUN] = 0;
            entity.data[m::DATA_PROGRESS] = 0;
            entity.data[m::DATA_TOTAL] = 0;
            entity.data[m::DATA_LIT] = 0;
            return Some(m::Step::Idle);
        }
        forget_queue(entity);
        entity.data[m::DATA_PROBE_RUN] = 0;
        entity.data[m::DATA_PROGRESS] = 0;
        entity.data[m::DATA_TOTAL] = 0;
        entity.data[m::DATA_LIT] = 0;
        let name = m::identify_compound(&world, &contents);
        let display = m::compound_display(&world, &contents);
        let mut blueprint = horizon_client::items::ItemStack {
            item: m::BLUEPRINT_ID.into(),
            count: 1,
            damage: 0,
            extra: None,
        };
        crate::game::biolab_protocol::set_custom(
            &mut blueprint,
            &[(m::CONTENTS_KEY, Tag::String(contents.as_str().into()))],
            &m::blueprint_label(&contents, display.as_deref()),
        );
        if let Some(slot) = entity.items.get_mut(output) {
            *slot = Some(blueprint);
        }
        unseal(entity, &contents);
        if let Some(name) = name {
            crate::game::biolab_say::say(game, &format!("Спектрометр: {contents} — {name}"));
        }
        return Some(m::Step::Finish);
    }

    // Заказанная проверка: та, которой ещё не делали.
    let Some(probe) = queued(entity).filter(|p| done & p.bit() == 0) else {
        forget_queue(entity);
        entity.data[m::DATA_PROBE_RUN] = 0;
        entity.data[m::DATA_PROGRESS] = 0;
        entity.data[m::DATA_TOTAL] = 0;
        entity.data[m::DATA_LIT] = 0;
        return Some(m::Step::Idle);
    };
    entity.data[m::DATA_PROBE_RUN] = probe.id() as i32 + 1;
    entity.data[m::DATA_TOTAL] = probe.ticks();
    entity.data[m::DATA_LIT] = 1;
    entity.data[m::DATA_LIT_TOTAL] = 1;
    let progress = entity.data[m::DATA_PROGRESS] + 1;
    if progress < probe.ticks() {
        entity.data[m::DATA_PROGRESS] = progress;
        return Some(m::Step::Working(progress));
    }
    // Проверка сделана: её ответ — знание мира.
    let done = m::remember_probe(&world, &contents, probe);
    forget_queue(entity);
    entity.data[m::DATA_PROGRESS] = 0;
    entity.data[m::DATA_LIT] = 0;
    entity.data[m::DATA_PROBE_RUN] = 0;
    entity.data[m::DATA_PROBE_DONE] = i32::from(done) + 1;
    let answer = m::probe_result_text(probe, results[probe.id()]);
    let left = m::probe_candidates(done, &results).len();
    crate::game::biolab_say::say(
        game,
        &format!("{}: {answer}. Вариантов: {left}", probe.title()),
    );
    Some(m::Step::Finish)
}
