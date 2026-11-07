// Биореактор в игре (GDD 4.18, п. 2). Собран ли куб и где порты — решил
// каркас (`biolab_multiblock::tick`); здесь работа контроллера: температура
// среды от источника под днищем, партия до восьми предметов по рецептам
// ферментера и инкубатора, порция раствора на партию, перегрев, скисание вне
// карантина и общий вид передней стенки (`formed` контроллера). Решения —
// раздел «M8: биореактор» в `runtime.rs` (`bioreactor_take`,
// `bioreactor_time`, `medium_target`, `bioreactor_sours`), под тестами.

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

/// Строка из `custom_data` стака: штамм культуры, мазок с моба.
fn custom_text(stack: &ItemStack, key: &str) -> Option<String> {
    let extra = stack.extra.as_ref()?;
    let (_, data) = extra.other.iter().find(|(name, _)| name == "minecraft:custom_data")?;
    match data.get(key) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    }
}

/// Звук у контроллера — как `play` в `machine.rs` (тот вложен в хук, его
/// отсюда не позвать): только на стороне, которая ведёт мир.
fn play(game: &mut crate::game::Game<'_>, sound: Option<m::Sound>, pos: [i32; 3]) {
    let Some(sound) = sound else {
        return;
    };
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5];
    let _ = game.with_level(|_, level, _| {
        level.native_play_sound(sound.event, at, sound.source, sound.volume, sound.pitch)
    });
}

/// Работа не идёт: прогресс с нуля, лампа гаснет.
fn idle(entity: &mut ModBlockEntity) -> m::Step {
    entity.data[m::DATA_PROGRESS] = 0;
    entity.data[m::DATA_TOTAL] = 0;
    entity.data[m::DATA_LIT] = 0;
    entity.data[m::DATA_LIT_TOTAL] = 0;
    m::Step::Idle
}

/// Общий вид передней стенки: собран — `formed=true` (модель рисует раму на
/// всю грань 3×3), разобран — обычный люк. Пишем, только когда меняется.
fn show_formed(game: &mut crate::game::Game<'_>, pos: [i32; 3], formed: bool) {
    let want = m::formed_value(formed);
    if game.native_block_property(pos, m::FORMED).as_deref() != Some(want) {
        game.native_set_block_property(pos, m::FORMED, want);
    }
}

/// Рецепт ферментера или инкубатора, разобранный для партии.
struct Recipe {
    ingredient: horizon_client::crafting::Ingredient,
    result: ItemStack,
    time: i32,
}

/// Рецепты, которые варит биореактор, — из таблицы игры, как читают их сами
/// ферментер и инкубатор (`machine.rs`; помощники API 36 на сервере пусты).
/// Порядок — `BIOREACTOR_RECIPES`: на вход подходит первый совпавший.
fn recipes(game: &crate::game::Game<'_>) -> Vec<Recipe> {
    let mut out = Vec::new();
    for kind in m::BIOREACTOR_RECIPES {
        let list: Vec<std::sync::Arc<serde_json::Value>> = game
            .recipes()
            .custom
            .get(kind)
            .map(|list| list.iter().map(|r| r.json.clone()).collect())
            .unwrap_or_default();
        for json in list {
            let Some(ingredient) = horizon_client::crafting::Ingredient::parse(&json["ingredient"], game.items()) else {
                continue;
            };
            let Some(result) = horizon_client::crafting::recipe_result(&json["result"]) else {
                continue;
            };
            let time = json["time"].as_i64().unwrap_or(m::DEFAULT_TIME as i64) as i32;
            out.push(Recipe { ingredient, result, time });
        }
    }
    out
}

/// Своими данными не складываются: фрагмент и культура.
fn unique(item: &str) -> bool {
    item == m::FRAGMENT_ID || item == m::CULTURE_ID
}

/// Куда в выходы лягут стаки (номера слотов выхода по порядку) или None —
/// не влезает. Стопку знает только таблица предметов игры.
fn plan(game: &crate::game::Game<'_>, entity: &ModBlockEntity, out: &[(&str, u32, bool)]) -> Option<Vec<usize>> {
    let items = game.items();
    let held: Vec<Option<m::Pile>> = (m::BIOREACTOR_FIRST_OUTPUT..m::BIOREACTOR_SLOTS)
        .map(|slot| {
            entity.items[slot]
                .as_ref()
                .map(|s| m::Pile { item: &*s.item, count: s.count, plain: s.extra.is_none() })
        })
        .collect();
    let piles: Vec<m::Pile> = out.iter().map(|&(item, count, plain)| m::Pile { item, count, plain }).collect();
    m::plan_outputs(&held, &piles, |item| items.get(item).map_or(64, |i| i.max_stack))
}

/// Списать партию: сырьё по плану и порцию раствора.
fn spend(entity: &mut ModBlockEntity, take: &[(usize, u32)]) {
    for &(slot, n) in take.iter().chain(std::iter::once(&(m::BIOREACTOR_BROTH, 1))) {
        if let Some(place) = entity.items.get_mut(slot)
            && let Some(stack) = place.as_mut()
        {
            stack.count = stack.count.saturating_sub(n);
            if stack.count == 0 {
                *place = None;
            }
        }
    }
}

fn remember(entity: &mut ModBlockEntity, last: i32) {
    entity.nbt.insert(m::BIO_LAST_KEY.into(), Tag::Int(last));
    entity.data[m::DATA_BIO_LAST] = last;
}

/// Тик контроллера. `assembly` — собранный куб, None — разобран: работа
/// стоит, содержимое остаётся, среда остывает.
pub fn work(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
    assembly: Option<&m::Assembly>,
) -> m::Step {
    if entity.items.len() < m::BIOREACTOR_SLOTS {
        entity.items.resize(m::BIOREACTOR_SLOTS, None);
    }
    if entity.data.len() < m::DATA_BIO_END {
        entity.data.resize(m::DATA_BIO_END, 0);
    }
    let ticks = game.ticks as i64;

    // Среда: источник под серединой днища греет её до `medium_target`,
    // шаг раз в `BIOREACTOR_HEAT_EVERY` тиков (модель реактора).
    let target = if assembly.is_some() {
        let facing = game
            .native_block_property(pos, "facing")
            .and_then(|f| m::Facing::from_name(&f))
            .unwrap_or(m::Facing::North);
        m::medium_target(&game.native_block(m::place(pos, facing, m::BIOREACTOR_HEAT_AT)))
    } else {
        None
    };
    let mut temperature = int(entity, m::BIO_TEMPERATURE_KEY).unwrap_or(m::ROOM_TEMPERATURE);
    if m::medium_due(ticks) {
        temperature = m::next_temperature(temperature, target);
        entity.nbt.insert(m::BIO_TEMPERATURE_KEY.into(), Tag::Int(temperature));
    }
    // Карантин (контракт волны 2): экрану — раз в секунду, партии — в миг,
    // когда она готова. Проверка камеры может быть не дешёвой.
    let dimension = game.dim.id().to_string();
    if ticks.rem_euclid(m::MB_CHECK_EVERY) == 0 {
        entity.data[m::DATA_BIO_QUARANTINE] = i32::from(m::in_quarantine(&dimension, pos));
    }
    entity.data[m::DATA_BIO_TEMPERATURE] = temperature;
    entity.data[m::DATA_BIO_TARGET] = target.unwrap_or(-1);
    entity.data[m::DATA_BIO_LAST] = int(entity, m::BIO_LAST_KEY).unwrap_or(m::BATCH_NONE);
    entity.data[m::DATA_BIO_BATCH] = 0;
    show_formed(game, pos, assembly.is_some());
    if assembly.is_none() {
        return idle(entity);
    }

    // Партия: у каждого входа — первый рецепт, чей вход совпал; берём по
    // порядку слотов до восьми предметов.
    let recipes = recipes(game);
    let matched: Vec<Option<usize>> = (0..m::BIOREACTOR_INPUTS)
        .map(|slot| {
            let stack = entity.items[slot].as_ref()?;
            recipes.iter().position(|r| r.ingredient.test(stack))
        })
        .collect();
    let counts: Vec<Option<u32>> = (0..m::BIOREACTOR_INPUTS)
        .map(|slot| matched[slot].and(entity.items[slot].as_ref().map(|s| s.count)))
        .collect();
    let take = m::bioreactor_take(&counts);
    let batch: u32 = take.iter().map(|(_, n)| n).sum();
    entity.data[m::DATA_BIO_BATCH] = batch as i32;
    let broth = entity.items[m::BIOREACTOR_BROTH]
        .as_ref()
        .is_some_and(|s| &*s.item == m::BROTH_ID && s.count > 0);
    if take.is_empty() || !broth {
        return idle(entity);
    }
    // По предмету: (слот входа, рецепт).
    let units: Vec<(usize, usize)> = take
        .iter()
        .flat_map(|&(slot, n)| std::iter::repeat_n((slot, matched[slot].unwrap_or(0)), n as usize))
        .collect();
    let times: Vec<i32> = units.iter().map(|&(_, r)| recipes[r].time).collect();
    let time = m::bioreactor_time(&times);

    // Всё готовое должно влезть в выходы — и слизь тоже, если партия скиснет.
    let fits = {
        let out: Vec<(&str, u32, bool)> = units
            .iter()
            .map(|&(_, r)| {
                let result = &recipes[r].result;
                (&*result.item, result.count, !unique(&result.item) && result.extra.is_none())
            })
            .collect();
        plan(game, entity, &out).is_some() && plan(game, entity, &[(m::SOUR_ITEM, batch, true)]).is_some()
    };
    if !fits {
        return idle(entity);
    }

    let progress = entity.data[m::DATA_PROGRESS];
    match m::medium(temperature) {
        // Холодно: партия ждёт, начатое не пропадает.
        m::Medium::Cold => {
            entity.data[m::DATA_TOTAL] = if progress > 0 { time } else { 0 };
            entity.data[m::DATA_LIT] = 0;
            entity.data[m::DATA_LIT_TOTAL] = 0;
            return m::Step::Idle;
        }
        // Перегрев: начатая партия испорчена — сырьё и раствор пропали.
        m::Medium::Hot => {
            if progress > 0 {
                spend(entity, &take);
                remember(entity, m::BATCH_SPOILED);
                // Неудача — тоже опыт (GDD 4.23 п. 7).
                let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5];
                m::note_lab_event(&dimension, at, "bioreactor_overheat", "");
                play(game, m::sound(m::Cue::Overheat), pos);
            }
            return idle(entity);
        }
        m::Medium::Fit => {}
    }

    let step = m::step(Some(time), progress);
    match step {
        m::Step::Idle => return idle(entity),
        m::Step::Working(next) => {
            entity.data[m::DATA_PROGRESS] = next;
            entity.data[m::DATA_TOTAL] = time;
            entity.data[m::DATA_LIT] = 1;
            entity.data[m::DATA_LIT_TOTAL] = 1;
        }
        m::Step::Finish => {
            let seed = (pos[0] as i64 * 13 + pos[2] as i64 * 7 + pos[1] as i64 * 3 + ticks) as u64;
            let quarantine = m::in_quarantine(&dimension, pos);
            entity.data[m::DATA_BIO_QUARANTINE] = i32::from(quarantine);
            let sour = m::bioreactor_sours(seed, quarantine);
            let mut made: Vec<ItemStack> = Vec::new();
            if sour {
                made.push(ItemStack { item: m::SOUR_ITEM.into(), count: batch, damage: 0, extra: None });
            } else {
                for (k, &(slot, r)) in units.iter().enumerate() {
                    let mut result = recipes[r].result.clone();
                    // Культура ведёт штамм, как в инкубаторе: мазок с моба —
                    // его штамм, своя культура — следующее поколение.
                    if &*result.item == m::CULTURE_ID {
                        let input = entity.items[slot].as_ref();
                        let before = input
                            .and_then(|s| custom_text(s, m::STRAIN_KEY))
                            .and_then(|s| m::strain_from_text(&s));
                        let strain = m::incubated_strain(input.map(|s| &*s.item), before, seed.wrapping_add(k as u64));
                        let text = m::strain_to_text(&strain);
                        let data = result.extra.get_or_insert_with(Default::default);
                        data.other.retain(|(key, _)| key != "minecraft:custom_data");
                        data.other.push((
                            "minecraft:custom_data".to_string(),
                            Tag::compound([(m::STRAIN_KEY, Tag::String(text.as_str().into()))]),
                        ));
                        data.custom_name = Some(Tag::String(m::culture_label(&strain).as_str().into()));
                        result.count = 1;
                    }
                    made.push(result);
                }
            }
            let places = {
                let out: Vec<(&str, u32, bool)> = made
                    .iter()
                    .map(|s| (&*s.item, s.count, !unique(&s.item) && s.extra.is_none()))
                    .collect();
                plan(game, entity, &out)
            };
            let Some(places) = places else {
                return idle(entity);
            };
            spend(entity, &take);
            for (stack, index) in made.into_iter().zip(places) {
                let slot = m::BIOREACTOR_FIRST_OUTPUT + index;
                match entity.items[slot].as_mut() {
                    Some(held) => held.count += stack.count,
                    None => entity.items[slot] = Some(stack),
                }
            }
            remember(entity, if sour { m::BATCH_SOUR } else { m::BATCH_DONE });
            if sour {
                // Скисшая партия учит карантину (GDD 4.23 п. 7).
                let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5];
                m::note_lab_event(&dimension, at, "batch_sour", "");
                play(game, Some(m::BIOREACTOR_SOUR_SOUND), pos);
            }
            entity.data[m::DATA_PROGRESS] = 0;
            entity.data[m::DATA_TOTAL] = 0;
            entity.data[m::DATA_LIT] = 0;
            entity.data[m::DATA_LIT_TOTAL] = 0;
        }
    }
    step
}
