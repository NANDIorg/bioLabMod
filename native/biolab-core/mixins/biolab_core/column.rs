// Ректификационная колонна в игре (GDD 4.18, п. 1). Собрана ли башня и
// сколько в ней тарелок — решил каркас (`biolab_multiblock::tick`); здесь
// только работа кипятильника: рецепт `biolab:column` по сырью во входе,
// фракции по слотам выхода — фракция k (сверху башни вниз) в слот k + 1.
// Решения — раздел «M8» в `runtime.rs` (`column_recipe`, `column_yield`,
// `column_fits`), под тестами.

use crate::game::biolab_core as m;
use horizon_client::chunk::block_entity::ModBlockEntity;
use horizon_client::items::ItemStack;

/// Работа не идёт: прогресс с нуля, лампа гаснет.
fn idle(entity: &mut ModBlockEntity) -> m::Step {
    entity.data[m::DATA_PROGRESS] = 0;
    entity.data[m::DATA_TOTAL] = 0;
    entity.data[m::DATA_LIT] = 0;
    entity.data[m::DATA_LIT_TOTAL] = 0;
    m::Step::Idle
}

/// Тик кипятильника. `assembly` — собранная колонна, None — разобрана:
/// работа стоит, содержимое остаётся. Числа данных уже нужной длины
/// (`biolab_multiblock::tick`).
pub fn work(
    game: &mut crate::game::Game<'_>,
    entity: &mut ModBlockEntity,
    assembly: Option<&m::Assembly>,
) -> m::Step {
    let Some(a) = assembly else {
        return idle(entity);
    };
    if entity.items.len() < m::COLUMN_SLOTS {
        entity.items.resize(m::COLUMN_SLOTS, None);
    }
    let Some(input) = entity.items[0].clone() else {
        return idle(entity);
    };

    // Рецепты читаем из таблицы игры, как остальные машины (`machine.rs`):
    // помощники API 36 на сервере отдают пусто. Разбирает рецепт модуль мода
    // (`column_recipe`, под тестами): входы колонны — голые id, без тегов.
    let recipes: Vec<std::sync::Arc<serde_json::Value>> = game
        .recipes()
        .custom
        .get(m::COLUMN)
        .map(|list| list.iter().map(|r| r.json.clone()).collect())
        .unwrap_or_default();
    let mut job: Option<(i32, Vec<m::Fraction>)> = None;
    for json in recipes {
        let Some(parsed) = m::Json::parse(&json.to_string()) else {
            continue;
        };
        let Some((wanted, fractions, time)) = m::column_recipe(&parsed) else {
            continue;
        };
        // Сырьё со своими данными (колба, культура) колонна не берёт.
        if wanted != &*input.item || input.extra.is_some() {
            continue;
        }
        let n = m::column_yield(fractions.len(), a.sections);
        job = Some((time, fractions.into_iter().take(n).collect()));
        break;
    }
    let Some((time, fractions)) = job else {
        return idle(entity);
    };

    // Каждая фракция — в свой слот, и всё должно влезть, иначе ждём.
    let fits = {
        let items = game.items();
        let held: Vec<Option<m::Pile>> = (1..m::COLUMN_SLOTS)
            .map(|slot| {
                entity.items[slot]
                    .as_ref()
                    .map(|s| m::Pile { item: &*s.item, count: s.count, plain: s.extra.is_none() })
            })
            .collect();
        let piles: Vec<m::Pile> = fractions
            .iter()
            .map(|f| m::Pile { item: &f.item, count: f.count, plain: true })
            .collect();
        m::column_fits(&held, &piles, |item| items.get(item).map_or(64, |i| i.max_stack))
    };
    if !fits {
        return idle(entity);
    }

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
            if let Some(place) = entity.items.get_mut(0)
                && let Some(stack) = place.as_mut()
            {
                stack.count -= 1;
                if stack.count == 0 {
                    *place = None;
                }
            }
            for (k, f) in fractions.into_iter().enumerate() {
                let slot = k + 1;
                match entity.items[slot].as_mut() {
                    Some(held) => held.count += f.count,
                    None => {
                        entity.items[slot] = Some(ItemStack {
                            item: f.item.as_str().into(),
                            count: f.count,
                            damage: 0,
                            extra: None,
                        })
                    }
                }
            }
            entity.data[m::DATA_PROGRESS] = 0;
            entity.data[m::DATA_TOTAL] = 0;
            entity.data[m::DATA_LIT] = 0;
            entity.data[m::DATA_LIT_TOTAL] = 0;
        }
    }
    step
}
