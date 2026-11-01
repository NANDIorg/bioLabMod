// Тик машины: один рецепт за раз, прогресс виден игроку через числа данных
// ванильного меню печки.
fn biolab_core_machine_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut horizon_client::chunk::block_entity::ModBlockEntity,
) {
    use crate::game::biolab_core as m;
    /// Синтезатор: собирает вещество из элементов в слотах. Живёт здесь, а не в
    /// модуле мода: модуль собирается ещё и как обычная библиотека для тестов, где
    /// игровых типов нет.
    fn synthesize(
        entity: &mut horizon_client::chunk::block_entity::ModBlockEntity,
        output_slot: usize,
        journal: &mut Vec<String>,
    ) {
        use crate::game::biolab_core as m;

        use horizon_world::nbt::Tag;
        entity.data.resize(m::DATA_SLOTS, 0);

        // Чертёж в слоте задаёт цель: синтезатор собирает ровно его состав,
        // а сам чертёж не тратится — по нему можно повторять сколько угодно.
        let mut blueprint: Option<String> = None;
        for slot in 0..output_slot {
            let Some(Some(stack)) = entity.items.get(slot) else {
                continue;
            };
            if &*stack.item != m::BLUEPRINT_ID {
                continue;
            }
            if let Some(extra) = stack.extra.as_ref()
                && let Some((_, data)) = extra
                    .other
                    .iter()
                    .find(|(key, _)| key == "minecraft:custom_data")
                && let Some(Tag::String(text)) = data.get(m::CONTENTS_KEY)
            {
                blueprint = Some(text.to_string_lossy());
            }
        }

        // Что лежит во входных слотах.
        let mut formula = m::Formula::default();
        for slot in 0..output_slot {
            let Some(Some(stack)) = entity.items.get(slot) else {
                continue;
            };
            if let Some(index) = m::ELEMENTS.iter().position(|e| e.item == &*stack.item) {
                formula[index] += stack.count;
            }
        }
        // Пока состав меняется, сборка начинается заново: иначе синтезатор
        // хватает первый же устойчивый набор, не дождавшись остальных
        // элементов (на первой проверке он так собрал H2 вместо воды).
        let signature: Vec<i32> = formula.iter().map(|count| *count as i32).collect();
        let changed = match entity.nbt.get(&"biolab_formula".into()) {
            Some(horizon_world::nbt::Tag::IntArray(saved)) => *saved != signature,
            _ => true,
        };
        if changed {
            entity.nbt.insert(
                "biolab_formula".into(),
                horizon_world::nbt::Tag::IntArray(signature),
            );
            entity.data[m::DATA_PROGRESS] = 0;
        }

        // С чертежом машина ждёт ровно его состав, без него — любой устойчивый.
        let matches_blueprint = blueprint
            .as_deref()
            .and_then(m::formula_from_text)
            .is_none_or(|target| target == formula);
        let busy = matches_blueprint
            && m::is_stable(&formula)
            && entity
                .items
                .get(output_slot)
                .is_some_and(|slot| slot.is_none());
        if !busy {
            entity.data[m::DATA_PROGRESS] = 0;
            entity.data[m::DATA_TOTAL] = 0;
            entity.data[m::DATA_LIT] = 0;
            return;
        }

        // Время сборки растёт с числом атомов: сложная молекула собирается дольше.
        let atoms: u32 = formula.iter().sum();
        let total = (40 + 10 * atoms as i32).min(600);
        entity.data[m::DATA_TOTAL] = total;
        entity.data[m::DATA_LIT] = 1;
        entity.data[m::DATA_LIT_TOTAL] = 1;
        let progress = entity.data[m::DATA_PROGRESS] + 1;
        if progress < total {
            entity.data[m::DATA_PROGRESS] = progress;
            return;
        }

        // Готово: забираем все элементы и выдаём колбу с веществом.
        for slot in 0..output_slot {
            if let Some(stack) = entity.items.get_mut(slot) {
                *stack = None;
            }
        }
        let name = m::compound_name_of(&formula);
        let text = m::formula_text(&formula);
        journal.push(text.clone());
        let mut flask = horizon_client::items::ItemStack {
            item: m::FLASK_ID.into(),
            count: 1,
            damage: 0,
            extra: None,
        };
        let data = flask.extra.get_or_insert_with(Default::default);
        data.other.push((
            "minecraft:custom_data".to_string(),
            horizon_world::nbt::Tag::compound([
                (m::CONTENTS_KEY, horizon_world::nbt::Tag::String(text.as_str().into())),
                (m::ML_KEY, horizon_world::nbt::Tag::Int(250)),
            ]),
        ));
        data.custom_name = Some(horizon_world::nbt::Tag::String(
            format!("Колба: {name}, 250 мл").as_str().into(),
        ));
        if let Some(slot) = entity.items.get_mut(output_slot) {
            *slot = Some(flask);
        }
        entity.data[m::DATA_PROGRESS] = 0;
        entity.data[m::DATA_LIT] = 0;
    }

    /// Клонирующий чан: растит мутанта по фрагменту ДНК. Возвращает ген, когда
    /// клон готов; сам спавн делает тело хука — ему виден мир.
    fn grow_clone(
        entity: &mut horizon_client::chunk::block_entity::ModBlockEntity,
    ) -> Option<String> {
        use crate::game::biolab_core as m;
        use horizon_world::nbt::Tag;
        entity.data.resize(m::DATA_SLOTS, 0);

        // Фрагмент ДНК на входе и питательный раствор в слоте топлива.
        let gene = entity
            .items
            .get(m::SLOT_INPUT)
            .and_then(|s| s.as_ref())
            .filter(|stack| &*stack.item == m::FRAGMENT_ID)
            .and_then(|stack| {
                let extra = stack.extra.as_ref()?;
                let (_, data) = extra
                    .other
                    .iter()
                    .find(|(key, _)| key == "minecraft:custom_data")?;
                match data.get(m::GENE_KEY) {
                    Some(Tag::String(s)) => Some(s.to_string_lossy()),
                    _ => None,
                }
            });
        let broth = entity
            .items
            .get(m::SLOT_FUEL)
            .and_then(|s| s.as_ref())
            .map(|stack| (&*stack.item, stack.count));
        if !m::vat_ready(gene.as_deref(), broth) {
            entity.data[m::DATA_PROGRESS] = 0;
            entity.data[m::DATA_TOTAL] = 0;
            entity.data[m::DATA_LIT] = 0;
            return None;
        }

        entity.data[m::DATA_TOTAL] = m::CLONE_TIME;
        entity.data[m::DATA_LIT] = 1;
        entity.data[m::DATA_LIT_TOTAL] = 1;
        let progress = entity.data[m::DATA_PROGRESS] + 1;
        if progress < m::CLONE_TIME {
            entity.data[m::DATA_PROGRESS] = progress;
            return None;
        }

        // Готово: фрагмент и раствор уходят на клона.
        for slot in [m::SLOT_INPUT, m::SLOT_FUEL] {
            if let Some(place) = entity.items.get_mut(slot)
                && let Some(stack) = place.as_mut()
            {
                stack.count = stack.count.saturating_sub(1);
                if stack.count == 0 {
                    *place = None;
                }
            }
        }
        entity.data[m::DATA_PROGRESS] = 0;
        entity.data[m::DATA_LIT] = 0;
        gene
    }

    if !game.native_on(m::ID) {
        return;
    }

    // ГМ-грядка: растёт сама, а на последней стадии подмешивает ген соседа.
    if entity.id == m::GM_PLOT {
        use horizon_world::nbt::Tag;
        let planted = match entity.nbt.get(&m::PLOT_GENOME_KEY.into()) {
            Some(Tag::String(s)) => s.to_string_lossy(),
            _ => return,
        };
        if planted.is_empty() {
            return;
        }
        entity.data.resize(1, 0);
        let layers = game
            .native_block_property(pos, "layers")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);
        if m::plot_ripe(layers) {
            entity.data[0] = 0;
            return;
        }
        entity.data[0] += 1;
        if entity.data[0] < m::PLOT_GROW_EVERY {
            return;
        }
        entity.data[0] = 0;
        let next = m::plot_stage(layers + 1);

        // Скрещивание: сосед, доросший до спелости, отдаёт свой лучший ген.
        let mut genes = m::genome_from_text(&planted);
        if next >= m::PLOT_RIPE {
            for step in [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]] {
                let side = [pos[0] + step[0], pos[1] + step[1], pos[2] + step[2]];
                if game.native_block(side) != m::GM_PLOT {
                    continue;
                }
                let ripe = game
                    .native_block_property(side, "layers")
                    .and_then(|v| v.parse::<i32>().ok())
                    .is_some_and(m::plot_ripe);
                if !ripe {
                    continue;
                }
                let theirs = game
                    .native_with_mod_block_entity(side, |other| {
                        match other.nbt.get(&m::PLOT_GENOME_KEY.into()) {
                            Some(Tag::String(s)) => s.to_string_lossy(),
                            _ => String::new(),
                        }
                    })
                    .unwrap_or_default();
                let theirs = m::genome_from_text(&theirs);
                if theirs.is_empty() {
                    continue;
                }
                genes = m::cross(&genes, &theirs);
            }
        }
        let text = m::genome_to_text(&genes);
        if text != planted {
            entity.nbt.insert(
                m::PLOT_GENOME_KEY.into(),
                Tag::String(text.as_str().into()),
            );
        }
        game.native_set_block_property(pos, "layers", &next.to_string());
        return;
    }

    let Some(machine) = m::machine(&entity.id) else {
        return;
    };
    let output_slot = machine.slots - 1;
    if entity.id == m::SYNTHESIZER {
        let mut journal = Vec::new();
        synthesize(entity, output_slot, &mut journal);
        let world = game.world_root.clone();
        for text in journal {
            m::remember_compound(&world, &text);
        }
        return;
    }
    // Спектрометр: разбирает колбу на состав, закрывает запись в журнале и
    // печатает чертёж. Колбу не тратит — анализ не разрушающий.
    if entity.id == m::SPECTROMETER {
        use horizon_world::nbt::Tag;
        entity.data.resize(m::DATA_SLOTS, 0);
        let contents = entity
            .items
            .get(m::SLOT_INPUT)
            .and_then(|s| s.as_ref())
            .filter(|stack| &*stack.item == m::FLASK_ID)
            .and_then(|stack| {
                let extra = stack.extra.as_ref()?;
                let (_, data) = extra
                    .other
                    .iter()
                    .find(|(key, _)| key == "minecraft:custom_data")?;
                match data.get(m::CONTENTS_KEY) {
                    Some(Tag::String(s)) => Some(s.to_string_lossy()),
                    _ => None,
                }
            });
        let free = entity
            .items
            .get(output_slot)
            .is_some_and(|slot| slot.is_none());
        let Some(contents) = contents.filter(|_| free) else {
            entity.data[m::DATA_PROGRESS] = 0;
            entity.data[m::DATA_TOTAL] = 0;
            entity.data[m::DATA_LIT] = 0;
            return;
        };
        entity.data[m::DATA_TOTAL] = m::ANALYSIS_TIME;
        entity.data[m::DATA_LIT] = 1;
        entity.data[m::DATA_LIT_TOTAL] = 1;
        let progress = entity.data[m::DATA_PROGRESS] + 1;
        if progress < m::ANALYSIS_TIME {
            entity.data[m::DATA_PROGRESS] = progress;
            return;
        }
        entity.data[m::DATA_PROGRESS] = 0;
        entity.data[m::DATA_LIT] = 0;

        let world = game.world_root.clone();
        let name = m::identify_compound(&world, &contents);
        let display = m::compound_display(&world, &contents);
        let mut blueprint = horizon_client::items::ItemStack {
            item: m::BLUEPRINT_ID.into(),
            count: 1,
            damage: 0,
            extra: None,
        };
        let data = blueprint.extra.get_or_insert_with(Default::default);
        data.other.push((
            "minecraft:custom_data".to_string(),
            Tag::compound([(m::CONTENTS_KEY, Tag::String(contents.as_str().into()))]),
        ));
        data.custom_name = Some(Tag::String(
            m::blueprint_label(&contents, display.as_deref()).as_str().into(),
        ));
        if let Some(slot) = entity.items.get_mut(output_slot) {
            *slot = Some(blueprint);
        }
        if let Some(name) = name {
            game.native_chat(&format!("Спектрометр: {contents} — {name}"));
        }
        return;
    }

    if entity.id == m::CLONING_VAT {
        let Some(text) = grow_clone(entity) else {
            return;
        };
        let Some((name, strength)) = m::gene_from_text(&text) else {
            return;
        };
        // Клон выходит над чаном, чтобы не застрять в блоке.
        let spot = [pos[0] as f64 + 0.5, pos[1] as f64 + 1.0, pos[2] as f64 + 0.5];
        let dimension = game.dim.id().to_string();
        let options = crate::game::native_mobs::NativeSpawn::default();
        let Some(id) = game.native_spawn_mob(&dimension, m::CLONE_SPECIES, spot, &options) else {
            return;
        };
        // Метка гена: по ней `biolab-deep` выдаст клону прибавки и внешность,
        // а ванилла запишет ген в сейв вместе с остальными модификаторами.
        if let Some(marker) = m::marker_for(name, strength)
            && let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id)
        {
            use horizon_client::mobs::finalize::{Attr, Op, add_modifier, remove_modifier};
            // Свежему зомби игра успевает выдать случайный геном раньше, чем
            // чан впишет заказанный (проверено в игре: клон вышел с лишним
            // геном и перебором стабильности). Поэтому сначала чистим.
            for old in m::GENE_MARKERS {
                remove_modifier(mob, Attr::FollowRange, old);
            }
            for (attr, name) in [
                (Attr::MaxHealth, m::STAT_MODIFIERS[0]),
                (Attr::MovementSpeed, m::STAT_MODIFIERS[1]),
                (Attr::AttackDamage, m::STAT_MODIFIERS[2]),
                (Attr::Armor, m::STAT_MODIFIERS[3]),
                (Attr::KnockbackResistance, m::STAT_MODIFIERS[4]),
            ] {
                remove_modifier(mob, attr, name);
            }
            mob.health = mob.health.min(mob.attrs.max_health);
            add_modifier(mob, Attr::FollowRange, marker, 0.0, Op::AddValue);
        }
        return;
    }

    entity.data.resize(m::DATA_SLOTS, 0);

    // Реактор греется от блока снизу и работает только в своём диапазоне.
    let mut temperature = match entity.nbt.get(&"biolab_temperature".into()) {
        Some(horizon_world::nbt::Tag::Int(v)) => *v,
        _ => 20,
    };
    if entity.id == m::REACTOR {
        let below = [pos[0], pos[1] - 1, pos[2]];
        let target = m::heat_target(&game.native_block(below));
        temperature = m::next_temperature(temperature, target);
        entity.nbt.insert(
            "biolab_temperature".into(),
            horizon_world::nbt::Tag::Int(temperature),
        );
        entity.data[m::DATA_LIT] = i32::from(target.is_some());
        entity.data[m::DATA_LIT_TOTAL] = 1;
        // Перегрев: содержимое пропадает, реактор остывает рывком.
        if temperature > m::OVERHEAT {
            for slot in entity.items.iter_mut() {
                *slot = None;
            }
            entity.nbt.insert(
                "biolab_temperature".into(),
                horizon_world::nbt::Tag::Int(20),
            );
            entity.data[m::DATA_PROGRESS] = 0;
            return;
        }
    }

    // Что лежит на входе и на выходе.
    let input = entity.items.get(m::SLOT_INPUT).and_then(|s| s.clone());
    let output = entity.items.get(output_slot).and_then(|s| s.clone());

    // Подходящий рецепт: вход совпал и результат влезает в выход.
    let mut found: Option<(i32, horizon_client::items::ItemStack)> = None;
    if let Some(input) = input.as_ref() {
        // Помощники API 36 (`native_recipes_of_type`, `native_recipe_ingredient`,
        // `native_recipe_result`) начинаются с проверки `self.player`, которого на
        // серверной стороне нет, и всегда отдают пусто — машины ими пользоваться не
        // могут. Поэтому читаем таблицу рецептов игры напрямую. Это внутренний API:
        // при обновлении Horizon проверить. Сообщить об этом разработчикам Horizon.
        let recipes: Vec<std::sync::Arc<serde_json::Value>> = game
            .recipes()
            .custom
            .get(&*entity.id)
            .map(|list| list.iter().map(|r| r.json.clone()).collect())
            .unwrap_or_default();
        for json in recipes {
            let Some(ingredient) =
                horizon_client::crafting::Ingredient::parse(&json["ingredient"], game.items())
            else {
                continue;
            };
            if !ingredient.test(input) {
                continue;
            }
            let Some(result) = horizon_client::crafting::recipe_result(&json["result"]) else {
                continue;
            };
            // `items()` — таблица предметов игры, не хук-поинт: при обновлении
            // Horizon проверить. Нужна, чтобы не перелить через стопку.
            let max_stack = game.items().get(&result.item).map_or(64, |i| i.max_stack);
            let fits = m::output_fits(
                output.as_ref().map(|s| (&*s.item, s.count)),
                (&result.item, result.count),
                max_stack,
            );
            if !fits {
                continue;
            }
            // Рецепт реактора может требовать температуру.
            let range = match (json["temperature"][0].as_i64(), json["temperature"][1].as_i64()) {
                (Some(min), Some(max)) => Some((min as i32, max as i32)),
                _ => None,
            };
            if entity.id == m::REACTOR && !m::temperature_fits(temperature, range) {
                continue;
            }
            let time = json["time"].as_i64().unwrap_or(m::DEFAULT_TIME as i64) as i32;
            found = Some((time, result));
            break;
        }
    }

    let progress = entity.data[m::DATA_PROGRESS];
    let time = found.as_ref().map(|(time, _)| *time);
    match m::step(time, progress) {
        m::Step::Idle => {
            entity.data[m::DATA_PROGRESS] = 0;
            entity.data[m::DATA_TOTAL] = 0;
            entity.data[m::DATA_LIT] = 0;
            entity.data[m::DATA_LIT_TOTAL] = 0;
        }
        m::Step::Working(next) => {
            let (total, _) = found.as_ref().expect("Working только с рецептом");
            entity.data[m::DATA_PROGRESS] = next;
            entity.data[m::DATA_TOTAL] = *total;
            // Пламя горит всё время работы: клиент рисует его как у печки.
            entity.data[m::DATA_LIT] = 1;
            entity.data[m::DATA_LIT_TOTAL] = 1;
        }
        m::Step::Finish => {
            let (_, result) = found.expect("Finish только с рецептом");
            // Забираем один предмет со входа.
            if let Some(slot) = entity.items.get_mut(m::SLOT_INPUT)
                && let Some(stack) = slot.as_mut()
            {
                stack.count -= 1;
                if stack.count == 0 {
                    *slot = None;
                }
            }
            // Экстрактор выдаёт не просто предмет, а фрагмент с геном: какой
            // ген достался, решает зерно от места и времени — два образца
            // подряд дают разное.
            let mut result = result;
            if &*result.item == m::FRAGMENT_ID {
                let seed = (pos[0] as i64 * 31 + pos[2] as i64 * 17 + game.ticks as i64) as u64;
                let (name, strength) = m::roll_gene(seed);
                let text = m::gene_text(name, strength);
                let data = result.extra.get_or_insert_with(Default::default);
                data.other.retain(|(key, _)| key != "minecraft:custom_data");
                data.other.push((
                    "minecraft:custom_data".to_string(),
                    horizon_world::nbt::Tag::compound([(
                        m::GENE_KEY,
                        horizon_world::nbt::Tag::String(text.as_str().into()),
                    )]),
                ));
                data.custom_name = Some(horizon_world::nbt::Tag::String(
                    m::fragment_label(&text).as_str().into(),
                ));
                result.count = 1;
            }
            // Инкубатор ведёт штамм: мазок даёт дикую линию, а прогон уже
            // готовой культуры — следующее поколение со сдвигом в одну
            // характеристику. Так профиль выводится селекцией, а не задаётся.
            if &*result.item == m::CULTURE_ID {
                let before = input.as_ref().and_then(|stack| {
                    let extra = stack.extra.as_ref()?;
                    let (_, data) = extra
                        .other
                        .iter()
                        .find(|(key, _)| key == "minecraft:custom_data")?;
                    match data.get(m::STRAIN_KEY) {
                        Some(horizon_world::nbt::Tag::String(s)) => {
                            m::strain_from_text(&s.to_string_lossy())
                        }
                        _ => None,
                    }
                });
                let seed = (pos[0] as i64 * 13 + pos[2] as i64 * 7 + game.ticks as i64) as u64;
                let strain = match before {
                    Some(strain) => m::select(&strain, seed),
                    None => m::WILD,
                };
                let text = m::strain_to_text(&strain);
                let data = result.extra.get_or_insert_with(Default::default);
                data.other.retain(|(key, _)| key != "minecraft:custom_data");
                data.other.push((
                    "minecraft:custom_data".to_string(),
                    horizon_world::nbt::Tag::compound([(
                        m::STRAIN_KEY,
                        horizon_world::nbt::Tag::String(text.as_str().into()),
                    )]),
                ));
                data.custom_name = Some(horizon_world::nbt::Tag::String(
                    format!("Культура {text}").as_str().into(),
                ));
                result.count = 1;
            }
            // Кладём результат в выход.
            if let Some(slot) = entity.items.get_mut(output_slot) {
                match slot.as_mut() {
                    // Два фрагмента с разными генами в одну стопку не кладём.
                    Some(stack)
                        if &*stack.item != m::FRAGMENT_ID && &*stack.item != m::CULTURE_ID =>
                    {
                        stack.count += result.count
                    }
                    Some(_) => {}
                    None => *slot = Some(result),
                }
            }
            entity.data[m::DATA_PROGRESS] = 0;
            entity.data[m::DATA_TOTAL] = 0;
            entity.data[m::DATA_LIT] = 0;
            entity.data[m::DATA_LIT_TOTAL] = 0;
        }
    }
}
