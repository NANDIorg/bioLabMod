// Генный секвенатор в игре (GDD 4.18, п. 3). Собран ли он — решил каркас
// (`biolab_multiblock::tick`); здесь только работа консоли в выбранном
// режиме: сборка вектора из 2–3 фрагментов, антисмысловой вектор из
// фрагмента, чтение генома моба на операционном столе рядом. Режим лежит в
// сейве консоли (`biolab_seq_mode`), его меняют кнопки экрана
// (`sequencer_button.rs`). Решения — подраздел «M8: генный секвенатор» в
// `runtime.rs` (`vector_plan`, `antisense_plan`, `seq_ready`,
// `nearest_patient`, `write_seq`), под тестами.

use crate::game::biolab_core as m;
use horizon_client::chunk::block_entity::ModBlockEntity;
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

fn int(entity: &ModBlockEntity, key: &str) -> Option<i32> {
    match entity.nbt.get(&key.into()) {
        Some(Tag::Int(v)) => Some(*v),
        _ => None,
    }
}

fn text(entity: &ModBlockEntity, key: &str) -> Option<String> {
    match entity.nbt.get(&key.into()) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    }
}

/// Строка гена из фрагмента: None — в слоте не фрагмент, Some("") — фрагмент
/// без гена.
fn fragment_gene(stack: &ItemStack) -> Option<String> {
    if &*stack.item != m::FRAGMENT_ID {
        return None;
    }
    let found = stack.extra.as_ref().and_then(|extra| {
        let (_, data) = extra.other.iter().find(|(key, _)| key == "minecraft:custom_data")?;
        match data.get(m::GENE_KEY) {
            Some(Tag::String(s)) => Some(s.to_string_lossy()),
            _ => None,
        }
    });
    Some(found.unwrap_or_default())
}

/// Сколько лежит в слоте, если там этот предмет.
fn count_of(entity: &ModBlockEntity, slot: usize, item: &str) -> u32 {
    entity.items.get(slot).and_then(|s| s.as_ref()).filter(|s| &*s.item == item).map_or(0, |s| s.count)
}

/// Забрать `n` штук из слота.
fn take(entity: &mut ModBlockEntity, slot: usize, n: u32) {
    if let Some(place) = entity.items.get_mut(slot)
        && let Some(stack) = place.as_mut()
    {
        stack.count = stack.count.saturating_sub(n);
        if stack.count == 0 {
            *place = None;
        }
    }
}

/// Вектор с данными и подписью, как фрагмент у экстрактора (`machine.rs`).
fn vector(item: &str, key: &str, value: &str, label: String) -> ItemStack {
    let mut stack = ItemStack { item: item.into(), count: 1, damage: 0, extra: None };
    let data = stack.extra.get_or_insert_with(Default::default);
    data.other.push(("minecraft:custom_data".to_string(), Tag::compound([(key, Tag::String(value.into()))])));
    data.custom_name = Some(Tag::String(label.as_str().into()));
    stack
}

/// Работа не идёт: прогресс с нуля, лампа гаснет.
fn idle(entity: &mut ModBlockEntity) -> m::Step {
    entity.data[m::DATA_PROGRESS] = 0;
    entity.data[m::DATA_TOTAL] = 0;
    entity.data[m::DATA_LIT] = 0;
    entity.data[m::DATA_LIT_TOTAL] = 0;
    m::Step::Idle
}

/// Шаг работы длиной `time`: прогресс, лампа; Finish — пора выдавать.
fn advance(entity: &mut ModBlockEntity, time: i32) -> m::Step {
    let step = m::step(Some(time), entity.data[m::DATA_PROGRESS]);
    match step {
        m::Step::Idle => return idle(entity),
        m::Step::Working(next) => {
            entity.data[m::DATA_PROGRESS] = next;
            entity.data[m::DATA_TOTAL] = time;
            entity.data[m::DATA_LIT] = 1;
            entity.data[m::DATA_LIT_TOTAL] = 1;
        }
        m::Step::Finish => {
            idle(entity);
        }
    }
    step
}

/// Пациент на операционном столе рядом с консолью: id и геном. Позицию
/// проверяем раньше данных — данные стола читаются только у мобов рядом.
fn patient(game: &mut crate::game::Game<'_>, pos: [i32; 3]) -> Option<(u32, Vec<(&'static str, i32)>)> {
    let near: Vec<(u32, [f64; 3])> = game
        .mobs
        .iter()
        .filter(|mob| mob.alive() && !mob.removed)
        .map(|mob| (mob.id, [mob.position.x, mob.position.y, mob.position.z]))
        .filter(|(id, p)| m::nearest_patient(pos, &[(*id, *p)]).is_some())
        .collect();
    let mut on_table: Vec<(u32, [f64; 3])> = Vec::new();
    for (id, p) in near {
        if crate::game::biolab_core_data::surgery(game, id) & m::TABLE_BIT != 0 {
            on_table.push((id, p));
        }
    }
    let id = m::nearest_patient(pos, &on_table)?;
    Some((id, crate::game::biolab_core_data::genes(game, id)))
}

/// Тик консоли. `assembly` — собранный секвенатор, None — разобран: работа
/// стоит, содержимое остаётся. Числа данных уже нужной длины
/// (`biolab_multiblock::tick` по `machine_data_slots`).
pub fn work(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
    assembly: Option<&m::Assembly>,
) -> m::Step {
    if entity.items.len() < m::SEQUENCER_SLOTS {
        entity.items.resize(m::SEQUENCER_SLOTS, None);
    }
    let mode = m::SeqMode::from_id(int(entity, m::SEQ_MODE_KEY).unwrap_or(0)).unwrap_or(m::SeqMode::Vector);
    // Пары, которые лаборатория уже узнала (GDD 4.23 п. 5): монитор называет
    // только их, остальные остаются скрытыми.
    let known = m::known_pairs(&game.world_root.clone());
    if assembly.is_none() {
        m::write_seq(&mut entity.data, mode, m::SeqStatus::Idle, &[], known);
        return idle(entity);
    }

    let phosphorus = count_of(entity, m::SEQ_SLOT_PHOSPHORUS, m::PHOSPHORUS_ID);
    let broth = count_of(entity, m::SEQ_SLOT_BROTH, m::BROTH_ID);
    let output_free = entity.items[m::SEQ_SLOT_OUTPUT].is_none();

    if mode == m::SeqMode::Read {
        let Some((id, genes)) = patient(game, pos) else {
            entity.nbt.remove(&m::SEQ_PATIENT_KEY.into());
            entity.nbt.remove(&m::SEQ_READ_KEY.into());
            m::write_seq(&mut entity.data, mode, m::SeqStatus::NoPatient, &[], known);
            return idle(entity);
        };
        let now = m::genome_data(&genes);
        let saved = int(entity, m::SEQ_PATIENT_KEY).zip(text(entity, m::SEQ_READ_KEY));
        let saved = saved.as_ref().map(|(i, t)| (*i as u32, t.as_str()));
        if m::read_still_valid((id, &now), saved) {
            m::write_seq(&mut entity.data, mode, m::SeqStatus::Read, &genes, known);
            return idle(entity);
        }
        let ready = m::seq_ready(m::SeqStatus::Working, m::seq_supplies(mode, 0), phosphorus, broth, true);
        if ready != m::SeqStatus::Working {
            m::write_seq(&mut entity.data, mode, ready, &[], known);
            return idle(entity);
        }
        m::write_seq(&mut entity.data, mode, m::SeqStatus::Reading, &[], known);
        let step = advance(entity, mode.time());
        if step == m::Step::Finish {
            take(entity, m::SEQ_SLOT_PHOSPHORUS, 1);
            entity.nbt.insert(m::SEQ_PATIENT_KEY.into(), Tag::Int(id as i32));
            entity.nbt.insert(m::SEQ_READ_KEY.into(), Tag::String(now.as_str().into()));
            m::write_seq(&mut entity.data, mode, m::SeqStatus::Read, &genes, known);
        }
        return step;
    }

    // Сборка: фрагменты из трёх входов.
    let texts: Vec<Option<String>> = (0..m::SEQ_FRAGMENT_SLOTS)
        .map(|slot| entity.items[slot].as_ref().and_then(fragment_gene))
        .collect();
    let fragments: Vec<Option<&str>> = texts.iter().map(|t| t.as_deref()).collect();
    let plan = match mode {
        m::SeqMode::Vector => m::vector_plan(&fragments),
        _ => m::antisense_plan(&fragments),
    };
    let need = m::seq_supplies(mode, plan.genes.len());
    let ready = m::seq_ready(plan.status, need, phosphorus, broth, output_free);
    m::write_seq(&mut entity.data, mode, ready, &plan.genes, known);
    // Несовместимая пара (GDD 4.23 п. 5): в пробирке образец гибнет, зато
    // лаборатория узнаёт пару — записываем её один раз и говорим рядом.
    if ready == m::SeqStatus::Lethal
        && let Some(pair) = m::lethal_in(&plan.genes)
    {
        let world = game.world_root.clone();
        if m::learn_pair(&world, pair) {
            let at = [pos[0] as f64 + 0.5, pos[1] as f64 + 1.0, pos[2] as f64 + 0.5];
            let dim = game.dim.id().to_string();
            m::note_gene_news(&dim, at, &format!("Образец погиб в пробирке: {}", m::pair_title(pair)));
            m::note_gene_news(&dim, at, &m::pair_learned_message(pair));
        }
    }
    if ready != m::SeqStatus::Working {
        return idle(entity);
    }
    let step = advance(entity, mode.time());
    if step != m::Step::Finish {
        return step;
    }
    take(entity, m::SEQ_SLOT_PHOSPHORUS, need.0);
    take(entity, m::SEQ_SLOT_BROTH, need.1);
    let made = match mode {
        m::SeqMode::Vector => {
            // Уходят все фрагменты: каждый стал геном вектора.
            for slot in 0..m::SEQ_FRAGMENT_SLOTS {
                if texts[slot].is_some() {
                    take(entity, slot, 1);
                }
            }
            let value = m::vector_text(&plan.genes);
            vector(m::GENE_VECTOR, m::VECTOR_KEY, &value, m::vector_label(&value))
        }
        _ => {
            // Уходит первый фрагмент — его ген и стал антисмыслом.
            if let Some(slot) = texts.iter().position(Option::is_some) {
                take(entity, slot, 1);
            }
            let name = plan.genes.first().map_or("", |(n, _)| *n);
            vector(m::ANTISENSE_VECTOR, m::ANTISENSE_KEY, name, m::antisense_label(name))
        }
    };
    entity.items[m::SEQ_SLOT_OUTPUT] = Some(made);
    step
}
