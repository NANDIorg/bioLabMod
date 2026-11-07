// Тик машины: один рецепт за раз, прогресс виден игроку через числа данных
// ванильного меню печки, а свои числа машины — экрану (GDD 4.15, контракт
// слотов данных).
fn biolab_core_machine_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut horizon_client::chunk::block_entity::ModBlockEntity,
) {
    use crate::game::biolab_core as m;
    /// Строка из `custom_data` предмета: так мод хранит в стаке своё —
    /// вещество колбы, геном урожая, штамм культуры.
    fn custom_text(stack: &horizon_client::items::ItemStack, key: &str) -> Option<String> {
        use horizon_world::nbt::Tag;
        let extra = stack.extra.as_ref()?;
        let (_, data) = extra
            .other
            .iter()
            .find(|(name, _)| name == "minecraft:custom_data")?;
        match data.get(key) {
            Some(Tag::String(s)) => Some(s.to_string_lossy()),
            _ => None,
        }
    }
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
        // Число слотов считаем заранее: `resize` берёт `entity.data`
                    // изменяемо, и читать `entity.id` внутри вызова уже нельзя.
                    let slots = m::machine_data_slots(&entity.id);
                    entity.data.resize(slots, 0);

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

        // Экрану синтезатора — статус сборки и набранный состав (GDD 4.15,
        // контракт слотов данных). Пишем каждый тик: числа меню не сохраняются.
        let target = blueprint.as_deref().and_then(m::formula_from_text);
        entity.data[m::DATA_SYNTH_STATUS] = m::synth_status(&formula, target.as_ref());
        for (i, count) in formula.iter().enumerate() {
            entity.data[m::DATA_SYNTH_ELEMENTS + i] = *count as i32;
        }

        // С чертежом машина ждёт ровно его состав, без него — любой устойчивый.
        let matches_blueprint = target.is_none_or(|target| target == formula);
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
        let total = m::assembly_time(atoms);
        entity.data[m::DATA_TOTAL] = total;
        entity.data[m::DATA_LIT] = 1;
        entity.data[m::DATA_LIT_TOTAL] = 1;
        let progress = entity.data[m::DATA_PROGRESS] + 1;
        if progress < total {
            entity.data[m::DATA_PROGRESS] = progress;
            return;
        }

        // Готово: забираем элементы и выдаём колбу с веществом. Чертёж
        // остаётся в своём слоте — какие слоты чистить, решает
        // `synth_spent_slots` (раньше чистились все входы, и чертёж пропадал).
        let inputs: Vec<Option<String>> = (0..output_slot)
            .map(|slot| {
                entity
                    .items
                    .get(slot)
                    .and_then(|s| s.as_ref())
                    .map(|stack| stack.item.to_string())
            })
            .collect();
        let ids: Vec<Option<&str>> = inputs.iter().map(|id| id.as_deref()).collect();
        for slot in m::synth_spent_slots(&ids) {
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
        // Число слотов считаем заранее: `resize` берёт `entity.data`
                    // изменяемо, и читать `entity.id` внутри вызова уже нельзя.
                    let slots = m::machine_data_slots(&entity.id);
                    entity.data.resize(slots, 0);

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
        // Экрану чана — ген во входе: его клон и получит.
        let (index, strength) = m::gene_data(gene.as_deref());
        entity.data[m::DATA_GENE] = index;
        entity.data[m::DATA_GENE_STRENGTH] = strength;
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

    /// Ванильный звук из таблицы мода всем игрокам рядом с `at`. Звук встаёт
    /// в очередь уровня (`native_play_sound`, API 25), и сервер рассылает его
    /// пакетом — в одиночной игре тоже: окно подключено к своему серверу как
    /// игрок. Без своего сервера (клиент) `native_play_sound` звук не берёт,
    /// поэтому там выходим сразу и не собираем уровень впустую.
    fn play(
        game: &mut crate::game::Game<'_>,
        sound: Option<crate::game::biolab_core::Sound>,
        at: [f64; 3],
    ) {
        let Some(sound) = sound else {
            return;
        };
        if game.serving.is_none() && game.options.dedicated.is_none() {
            return;
        }
        let _ = game.with_level(|_, level, _| {
            level.native_play_sound(sound.event, at, sound.source, sound.volume, sound.pitch)
        });
    }
    /// Звук машины по итогу тика: старт из простоя, редкий фон, готово.
    /// Решает `machine_beat` в модуле мода; здесь только отметка «занята» в
    /// сейве блока — пишем её, лишь когда она меняется.
    fn machine_sound(
        game: &mut crate::game::Game<'_>,
        pos: [i32; 3],
        entity: &mut horizon_client::chunk::block_entity::ModBlockEntity,
        step: crate::game::biolab_core::Step,
    ) {
        use crate::game::biolab_core as m;
        use horizon_world::nbt::Tag;
        let busy = matches!(entity.nbt.get(&m::BUSY_KEY.into()), Some(Tag::Int(1)));
        let (phase, now) = m::machine_beat(busy, step);
        if now != busy {
            entity.nbt.insert(m::BUSY_KEY.into(), Tag::Int(i32::from(now)));
        }
        let Some(phase) = phase else {
            return;
        };
        let at = [
            f64::from(pos[0]) + 0.5,
            f64::from(pos[1]) + 0.5,
            f64::from(pos[2]) + 0.5,
        ];
        play(game, m::machine_sound(&entity.id, phase), at);
    }

    if !game.native_on(m::ID) {
        return;
    }

    // Криобанк (GDD 4.23 п. 9): слоты и холод — образцы внутри не портятся.
    if entity.id == m::CRYOBANK {
        crate::game::biolab_lab::cryobank_tick(game, entity);
        return;
    }

    // Порт установки (GDD 4.18): своей работы нет, только слоты — завести их,
    // чтобы воронка могла положить во входной порт и до сборки установки.
    if m::port(&entity.id).is_some() {
        if entity.items.len() < m::PORT_SLOTS {
            entity.items.resize(m::PORT_SLOTS, None);
        }
        return;
    }

    // Компаратор (GDD 4.15): сигнал машины — её прогресс. Считаем по числам,
    // какими их оставил прошлый тик (тик в тик, компаратору это всё равно), —
    // так одна точка на все ветки ниже с их ранними выходами. Соседей будим
    // только на смене ступени: их 15 на рецепт, а не по разу за тик. Пишет
    // только сервер — у него компаратор и спрашивает.
    if m::is_machine(&entity.id) && (game.serving.is_some() || game.options.dedicated.is_some()) {
        let dimension = game.dim.id();
        if m::remember_signal(dimension, pos, m::comparator_signal(&entity.data)) {
            game.native_update_output_neighbors(dimension, pos);
        }
        // Лампа работы (модель `_on`, GDD 4.12): состояние блока `lit` по
        // числу `DATA_LIT` прошлого тика, той же одной точкой на все ветки.
        // Помощник сам ничего не пишет, если значение уже такое.
        if let Some(lit) = m::machine_lit(&entity.id, &entity.data) {
            game.native_set_block_property(pos, "lit", lit);
        }
        // Утечка тяжёлого штамма вне карантинной камеры (GDD 4.18 п. 4):
        // тоже по числам прошлого тика, одной точкой на все машины.
        crate::game::biolab_quarantine::leak(game, pos, entity);
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
    // Контроллер многоблочной установки (GDD 4.18, M8): каркас сверяет схему,
    // склеивает вид и носит предметы через порты (`biolab_multiblock`), потом
    // работает сама установка — своим модулем. Новая установка — строка здесь.
    if m::multiblock(&entity.id).is_some() {
        let assembly = crate::game::biolab_multiblock::tick(game, pos, entity);
        let step = match entity.id.as_str() {
            m::COLUMN => crate::game::biolab_column::work(game, entity, assembly.as_ref()),
            m::SEQUENCER => crate::game::biolab_sequencer::work(game, pos, entity, assembly.as_ref()),
            m::BIOREACTOR => crate::game::biolab_bioreactor::work(game, pos, entity, assembly.as_ref()),
            m::QUARANTINE => crate::game::biolab_quarantine::work(game, pos, entity, assembly.as_ref()),
            _ => m::Step::Idle,
        };
        machine_sound(game, pos, entity, step);
        return;
    }
    let output_slot = machine.slots - 1;
    if entity.id == m::SYNTHESIZER {
        // Смесь по рецепту (транквилизатор зверолова, GDD 4.20, M5): входы
        // списком, как у стола (`bench_match`), а катализатор (`catalyst`)
        // нужен в сетке, но не тратится — как чертёж. Совпала смесь — она и
        // собирается; иначе синтезатор собирает формулу из элементов.
        let slots = m::machine_data_slots(&entity.id);
        entity.data.resize(slots, 0);
        let recipes: Vec<std::sync::Arc<serde_json::Value>> = game
            .recipes()
            .custom
            .get(&*entity.id)
            .map(|list| list.iter().map(|r| r.json.clone()).collect())
            .unwrap_or_default();
        let filled: Vec<bool> = (0..output_slot)
            .map(|slot| entity.items.get(slot).is_some_and(|s| s.is_some()))
            .collect();
        let mut mix: Option<(i32, horizon_client::items::ItemStack, Vec<usize>, Option<String>)> = None;
        for json in recipes {
            let Some(list) = json["ingredients"].as_array() else {
                continue;
            };
            let ingredients: Option<Vec<horizon_client::crafting::Ingredient>> = list
                .iter()
                .map(|v| horizon_client::crafting::Ingredient::parse(v, game.items()))
                .collect();
            let Some(ingredients) = ingredients else {
                continue;
            };
            let items = &entity.items;
            let Some(plan) = m::bench_match(&filled, ingredients.len(), |entry, slot| {
                items
                    .get(slot)
                    .and_then(|s| s.as_ref())
                    .is_some_and(|stack| ingredients[entry].test(stack))
            }) else {
                continue;
            };
            let Some(result) = horizon_client::crafting::recipe_result(&json["result"]) else {
                continue;
            };
            let time = json["time"].as_i64().unwrap_or(m::DEFAULT_TIME as i64) as i32;
            let catalyst = json["catalyst"].as_str().map(str::to_string);
            mix = Some((time, result, plan, catalyst));
            break;
        }
        if let Some((time, result, plan, catalyst)) = mix {
            let output = entity.items.get(output_slot).and_then(|s| s.clone());
            let max_stack = game.items().get(&result.item).map_or(64, |i| i.max_stack);
            let fits = output.as_ref().is_none_or(|held| held.extra.is_none() && result.extra.is_none())
                && m::output_fits(
                    output.as_ref().map(|s| (&*s.item, s.count)),
                    (&result.item, result.count),
                    max_stack,
                );
            entity.data[m::DATA_SYNTH_STATUS] = m::SYNTH_STABLE;
            let step = if fits { m::step(Some(time), entity.data[m::DATA_PROGRESS]) } else { m::Step::Idle };
            match step {
                m::Step::Idle => {
                    entity.data[m::DATA_PROGRESS] = 0;
                    entity.data[m::DATA_TOTAL] = 0;
                    entity.data[m::DATA_LIT] = 0;
                }
                m::Step::Working(next) => {
                    entity.data[m::DATA_PROGRESS] = next;
                    entity.data[m::DATA_TOTAL] = time;
                    entity.data[m::DATA_LIT] = 1;
                    entity.data[m::DATA_LIT_TOTAL] = 1;
                }
                m::Step::Finish => {
                    for slot in plan {
                        if let Some(place) = entity.items.get_mut(slot)
                            && let Some(stack) = place.as_mut()
                        {
                            // Катализатор остаётся в сетке целым.
                            if catalyst.as_deref() == Some(&*stack.item) {
                                continue;
                            }
                            stack.count = stack.count.saturating_sub(1);
                            if stack.count == 0 {
                                *place = None;
                            }
                        }
                    }
                    if let Some(place) = entity.items.get_mut(output_slot) {
                        match place.as_mut() {
                            Some(held) => held.count += result.count,
                            None => *place = Some(result),
                        }
                    }
                    entity.data[m::DATA_PROGRESS] = 0;
                    entity.data[m::DATA_TOTAL] = 0;
                    entity.data[m::DATA_LIT] = 0;
                }
            }
            machine_sound(game, pos, entity, step);
            return;
        }
        let mut journal = Vec::new();
        synthesize(entity, output_slot, &mut journal);
        // Колба собрана — готово; горит пламя — работа; иначе простой.
        let step = if !journal.is_empty() {
            m::Step::Finish
        } else if entity.data.get(m::DATA_LIT) == Some(&1) {
            m::Step::Working(entity.data[m::DATA_PROGRESS])
        } else {
            m::Step::Idle
        };
        machine_sound(game, pos, entity, step);
        let world = game.world_root.clone();
        for text in journal {
            m::remember_compound(&world, &text);
        }
        return;
    }
    // Лабораторный стол собирает точную вещь, когда во входах ровно набор
    // её рецепта: по предмету на запись, порядок любой, лишнего нет. Какой
    // слот какой записи — решает `bench_match` в модуле мода, здесь только
    // предметы. Экрану — что соберётся (номер в `BENCH_RESULTS`).
    if entity.id == m::LAB_BENCH {
        // Число слотов считаем заранее: `resize` берёт `entity.data`
                    // изменяемо, и читать `entity.id` внутри вызова уже нельзя.
                    let slots = m::machine_data_slots(&entity.id);
                    entity.data.resize(slots, 0);
        let filled: Vec<bool> = (0..output_slot)
            .map(|slot| entity.items.get(slot).is_some_and(|s| s.is_some()))
            .collect();
        let output = entity.items.get(output_slot).and_then(|s| s.clone());
        // Рецепты читаем из таблицы игры, как остальные машины (см. ниже).
        let recipes: Vec<std::sync::Arc<serde_json::Value>> = game
            .recipes()
            .custom
            .get(&*entity.id)
            .map(|list| list.iter().map(|r| r.json.clone()).collect())
            .unwrap_or_default();
        let mut shown = if filled.contains(&true) { m::BENCH_NONE } else { m::BENCH_EMPTY };
        // Совпавший рецепт, если выход его примет: время, результат и из
        // какого слота берёт каждая запись.
        let mut found: Option<(i32, horizon_client::items::ItemStack, Vec<usize>)> = None;
        for json in recipes {
            let Some(list) = json["ingredients"].as_array() else {
                continue;
            };
            // Битая запись — не рецепт: иначе стол собирал бы без неё.
            let ingredients: Option<Vec<horizon_client::crafting::Ingredient>> = list
                .iter()
                .map(|v| horizon_client::crafting::Ingredient::parse(v, game.items()))
                .collect();
            let Some(ingredients) = ingredients else {
                continue;
            };
            let items = &entity.items;
            let plan = m::bench_match(&filled, ingredients.len(), |entry, slot| {
                items
                    .get(slot)
                    .and_then(|s| s.as_ref())
                    .is_some_and(|stack| ingredients[entry].test(stack))
            });
            let Some(plan) = plan else {
                continue;
            };
            let Some(result) = horizon_client::crafting::recipe_result(&json["result"]) else {
                continue;
            };
            // Набор совпал — табло показывает вещь, даже если выход занят:
            // игрок видит, что рецепт верный, и освобождает выход.
            shown = m::bench_result_index(&result.item);
            let max_stack = game.items().get(&result.item).map_or(64, |i| i.max_stack);
            // Стак со своими данными с другим не сливается.
            let plain = output
                .as_ref()
                .is_none_or(|held| held.extra.is_none() && result.extra.is_none());
            let fits = plain
                && m::output_fits(
                    output.as_ref().map(|s| (&*s.item, s.count)),
                    (&result.item, result.count),
                    max_stack,
                );
            if fits {
                let time = json["time"].as_i64().unwrap_or(m::DEFAULT_TIME as i64) as i32;
                found = Some((time, result, plan));
            }
            break;
        }
        // Набор сменился — сборка начинается заново, а не доделывает по
        // чужому прогрессу другую вещь.
        if entity.data[m::DATA_BENCH_RESULT] != shown {
            entity.data[m::DATA_PROGRESS] = 0;
        }
        entity.data[m::DATA_BENCH_RESULT] = shown;
        let progress = entity.data[m::DATA_PROGRESS];
        let time = found.as_ref().map(|(time, _, _)| *time);
        let step = m::step(time, progress);
        match step {
            m::Step::Idle => {
                entity.data[m::DATA_PROGRESS] = 0;
                entity.data[m::DATA_TOTAL] = 0;
                entity.data[m::DATA_LIT] = 0;
                entity.data[m::DATA_LIT_TOTAL] = 0;
            }
            m::Step::Working(next) => {
                entity.data[m::DATA_PROGRESS] = next;
                entity.data[m::DATA_TOTAL] = time.unwrap_or(m::DEFAULT_TIME);
                entity.data[m::DATA_LIT] = 1;
                entity.data[m::DATA_LIT_TOTAL] = 1;
            }
            m::Step::Finish => {
                let Some((_, result, plan)) = found else {
                    return;
                };
                // По предмету из слота каждой записи; остаток стопки — на
                // следующую сборку.
                for slot in plan {
                    if let Some(place) = entity.items.get_mut(slot)
                        && let Some(stack) = place.as_mut()
                    {
                        stack.count = stack.count.saturating_sub(1);
                        if stack.count == 0 {
                            *place = None;
                        }
                    }
                }
                if let Some(place) = entity.items.get_mut(output_slot) {
                    match place.as_mut() {
                        Some(held) => held.count += result.count,
                        None => *place = Some(result),
                    }
                }
                entity.data[m::DATA_PROGRESS] = 0;
                entity.data[m::DATA_TOTAL] = 0;
                entity.data[m::DATA_LIT] = 0;
                entity.data[m::DATA_LIT_TOTAL] = 0;
            }
        }
        machine_sound(game, pos, entity, step);
        return;
    }
    // Спектрометр: разбирает колбу на состав, закрывает запись в журнале и
    // печатает чертёж. Колбу не тратит — анализ не разрушающий.
    if entity.id == m::SPECTROMETER {
        use horizon_world::nbt::Tag;
        // Число слотов считаем заранее: `resize` берёт `entity.data`
                    // изменяемо, и читать `entity.id` внутри вызова уже нельзя.
                    let slots = m::machine_data_slots(&entity.id);
                    entity.data.resize(slots, 0);
        // Проба (GDD 4.23 п. 4): состав не читается, его сужают дешёвые
        // проверки. Обычная колба идёт прежним путём — спектр и анализ.
        if let Some(step) = crate::game::biolab_probe::work(game, entity, output_slot) {
            machine_sound(game, pos, entity, step);
            return;
        }
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
        // Экрану — спектр колбы во входе: число атомов каждого элемента.
        // Нет колбы или в ней не формула — все полосы пустые.
        let spectrum = contents
            .as_deref()
            .and_then(m::formula_from_text)
            .unwrap_or_default();
        for (i, count) in spectrum.iter().enumerate() {
            entity.data[m::DATA_SPECTRUM + i] = *count as i32;
        }
        let free = entity
            .items
            .get(output_slot)
            .is_some_and(|slot| slot.is_none());
        let Some(contents) = contents.filter(|_| free) else {
            entity.data[m::DATA_PROGRESS] = 0;
            entity.data[m::DATA_TOTAL] = 0;
            entity.data[m::DATA_LIT] = 0;
            machine_sound(game, pos, entity, m::Step::Idle);
            return;
        };
        entity.data[m::DATA_TOTAL] = m::ANALYSIS_TIME;
        entity.data[m::DATA_LIT] = 1;
        entity.data[m::DATA_LIT_TOTAL] = 1;
        let progress = entity.data[m::DATA_PROGRESS] + 1;
        if progress < m::ANALYSIS_TIME {
            entity.data[m::DATA_PROGRESS] = progress;
            machine_sound(game, pos, entity, m::Step::Working(progress));
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
        // Звон: анализ закончен, чертёж напечатан.
        machine_sound(game, pos, entity, m::Step::Finish);
        if let Some(name) = name {
            crate::game::biolab_say::say(game, &format!("Спектрометр: {contents} — {name}"));
        }
        return;
    }

    if entity.id == m::CLONING_VAT {
        let Some(text) = grow_clone(entity) else {
            // Работа или простой — по пламени, которое чан выставил себе сам.
            let step = if entity.data.get(m::DATA_LIT) == Some(&1) {
                m::Step::Working(entity.data[m::DATA_PROGRESS])
            } else {
                m::Step::Idle
            };
            machine_sound(game, pos, entity, step);
            return;
        };
        // Клон готов: звук рождения.
        machine_sound(game, pos, entity, m::Step::Finish);
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
        // Геном клона — данные на нём (`biolab:genome`, M11): по ним
        // `biolab-deep` выдаст прибавки, окно нарисует внешность, и с мобом
        // они уйдут в сейв. Свежему зомби игра успевает бросить случайный
        // геном раньше, чем чан впишет заказанный (проверено в игре: клон
        // выходил с лишним геном) — запись данных его просто заменяет, а
        // прибавок бросок не ставит.
        crate::game::biolab_core_data::set_genes(game, id, &[(name, strength)]);
        return;
    }

    // Центрифуга разделяет: рецепт даёт основной выход и, если записан,
    // побочный, а колба с веществом распадается на элементы и возвращается с
    // водой. Выходов четыре, и всё, что машина выдаёт, должно влезть в них
    // целиком — иначе она ждёт, а не теряет добычу.
    if entity.id == m::CENTRIFUGE {
        use horizon_world::nbt::Tag;
        // Число слотов считаем заранее: `resize` берёт `entity.data`
                    // изменяемо, и читать `entity.id` внутри вызова уже нельзя.
                    let slots = m::machine_data_slots(&entity.id);
                    entity.data.resize(slots, 0);
        let outputs = m::machine_outputs(machine);
        let input = entity.items.get(m::SLOT_INPUT).and_then(|s| s.clone());

        // Что выйдет и за сколько тиков.
        let mut job: Option<(i32, Vec<horizon_client::items::ItemStack>)> = None;
        if let Some(input) = input.as_ref() {
            if &*input.item == m::FLASK_ID {
                let parts = custom_text(input, m::CONTENTS_KEY).and_then(|contents| {
                    let items = game.items();
                    m::decompose(&contents, |item| items.get(item).map_or(64, |i| i.max_stack))
                });
                if let Some(parts) = parts {
                    let mut stacks: Vec<horizon_client::items::ItemStack> = parts
                        .stacks
                        .iter()
                        .map(|(item, count)| horizon_client::items::ItemStack {
                            item: (*item).into(),
                            count: *count,
                            damage: 0,
                            extra: None,
                        })
                        .collect();
                    // Колба возвращается с водой — такой же, как новая.
                    let (water, ml) = m::DEFAULT_FILL;
                    let mut flask = horizon_client::items::ItemStack {
                        item: m::FLASK_ID.into(),
                        count: 1,
                        damage: 0,
                        extra: None,
                    };
                    let data = flask.extra.get_or_insert_with(Default::default);
                    data.other.push((
                        "minecraft:custom_data".to_string(),
                        Tag::compound([
                            (m::CONTENTS_KEY, Tag::String(water.into())),
                            (m::ML_KEY, Tag::Int(ml)),
                        ]),
                    ));
                    data.custom_name = Some(Tag::String(m::flask_label(water, ml).as_str().into()));
                    stacks.push(flask);
                    job = Some((m::assembly_time(parts.atoms), stacks));
                }
            } else {
                // Рецепты читаем из таблицы игры, как остальные машины (см. ниже).
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
                    let Some(result) = horizon_client::crafting::recipe_result(&json["result"])
                    else {
                        continue;
                    };
                    let mut stacks = vec![result];
                    // Побочный выход необязателен. Записан, но битый — рецепт
                    // пропускаем, а не выдаём молча без него.
                    if !json["byproduct"].is_null() {
                        let Some(byproduct) =
                            horizon_client::crafting::recipe_result(&json["byproduct"])
                        else {
                            continue;
                        };
                        stacks.push(byproduct);
                    }
                    let time = json["time"].as_i64().unwrap_or(m::DEFAULT_TIME as i64) as i32;
                    job = Some((time, stacks));
                    break;
                }
            }
        }

        // Куда ляжет каждый выход: в такой же стак или в свободный слот.
        let plan = job.as_ref().and_then(|(_, stacks)| {
            let items = game.items();
            let held: Vec<Option<m::Pile>> = outputs
                .iter()
                .map(|slot| {
                    entity
                        .items
                        .get(*slot)
                        .and_then(|s| s.as_ref())
                        .map(|s| m::Pile { item: &*s.item, count: s.count, plain: s.extra.is_none() })
                })
                .collect();
            let piles: Vec<m::Pile> = stacks
                .iter()
                .map(|s| m::Pile { item: &*s.item, count: s.count, plain: s.extra.is_none() })
                .collect();
            m::plan_outputs(&held, &piles, |item| items.get(item).map_or(64, |i| i.max_stack))
        });

        let progress = entity.data[m::DATA_PROGRESS];
        let time = job
            .as_ref()
            .filter(|_| plan.is_some())
            .map(|(time, _)| *time);
        let step = m::step(time, progress);
        match step {
            m::Step::Idle => {
                entity.data[m::DATA_PROGRESS] = 0;
                entity.data[m::DATA_TOTAL] = 0;
                entity.data[m::DATA_LIT] = 0;
                entity.data[m::DATA_LIT_TOTAL] = 0;
            }
            m::Step::Working(next) => {
                entity.data[m::DATA_PROGRESS] = next;
                entity.data[m::DATA_TOTAL] = time.unwrap_or(m::DEFAULT_TIME);
                entity.data[m::DATA_LIT] = 1;
                entity.data[m::DATA_LIT_TOTAL] = 1;
            }
            m::Step::Finish => {
                let (Some((_, stacks)), Some(plan)) = (job, plan) else {
                    return;
                };
                if let Some(slot) = entity.items.get_mut(m::SLOT_INPUT)
                    && let Some(stack) = slot.as_mut()
                {
                    stack.count -= 1;
                    if stack.count == 0 {
                        *slot = None;
                    }
                }
                for (stack, index) in stacks.into_iter().zip(plan) {
                    if let Some(place) = entity.items.get_mut(outputs[index]) {
                        match place.as_mut() {
                            Some(held) => held.count += stack.count,
                            None => *place = Some(stack),
                        }
                    }
                }
                entity.data[m::DATA_PROGRESS] = 0;
                entity.data[m::DATA_TOTAL] = 0;
                entity.data[m::DATA_LIT] = 0;
                entity.data[m::DATA_LIT_TOTAL] = 0;
            }
        }
        machine_sound(game, pos, entity, step);
        return;
    }

    // Число слотов считаем заранее: `resize` берёт `entity.data`
                    // изменяемо, и читать `entity.id` внутри вызова уже нельзя.
                    let slots = m::machine_data_slots(&entity.id);
                    entity.data.resize(slots, 0);

    // Экрану экстрактора — последний выданный ген, инкубатора — штамм
    // культуры во входе. Пишем каждый тик: числа меню не сохраняются.
    if entity.id == m::EXTRACTOR {
        let last = match entity.nbt.get(&m::LAST_GENE_KEY.into()) {
            Some(horizon_world::nbt::Tag::String(s)) => Some(s.to_string_lossy()),
            _ => None,
        };
        let (index, strength) = m::gene_data(last.as_deref());
        entity.data[m::DATA_GENE] = index;
        entity.data[m::DATA_GENE_STRENGTH] = strength;
    }
    if entity.id == m::INCUBATOR {
        let input = entity.items.get(m::SLOT_INPUT).and_then(|s| s.as_ref());
        let strain = input.and_then(|stack| custom_text(stack, m::STRAIN_KEY));
        let values = m::strain_data(input.map(|stack| &*stack.item), strain.as_deref());
        for (i, value) in values.iter().enumerate() {
            entity.data[m::DATA_STRAIN + i] = *value;
        }
    }

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
        // Экрану реактора: градусник и отметка цели нагрева. Диапазон
        // рецепта заполнит поиск рецепта ниже; пока не нашёлся — нули.
        entity.data[m::DATA_TEMPERATURE] = temperature;
        entity.data[m::DATA_HEAT_TARGET] = target.unwrap_or(-1);
        entity.data[m::DATA_RANGE_MIN] = 0;
        entity.data[m::DATA_RANGE_MAX] = 0;
        // Перегрев: содержимое пропадает, реактор остывает рывком.
        if temperature > m::OVERHEAT {
            // Неудача — тоже опыт (GDD 4.23 п. 7): игроки рядом получат очки
            // узлу «Ослабленная культура»; подробность — что сгорело.
            if let Some(burnt) = entity.items.get(m::SLOT_INPUT).and_then(|s| s.as_ref()) {
                let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5];
                m::note_lab_event(game.dim.id(), at, "reactor_overheat", &burnt.item);
            }
            for slot in entity.items.iter_mut() {
                *slot = None;
            }
            entity.nbt.insert(
                "biolab_temperature".into(),
                horizon_world::nbt::Tag::Int(20),
            );
            entity.data[m::DATA_PROGRESS] = 0;
            entity.data[m::DATA_TEMPERATURE] = m::ROOM_TEMPERATURE;
            let at = [
                f64::from(pos[0]) + 0.5,
                f64::from(pos[1]) + 0.5,
                f64::from(pos[2]) + 0.5,
            ];
            play(game, m::sound(m::Cue::Overheat), at);
            machine_sound(game, pos, entity, m::Step::Idle);
            return;
        }
    }

    // Что лежит на входе и на выходе.
    let input = entity.items.get(m::SLOT_INPUT).and_then(|s| s.clone());
    let output = entity.items.get(output_slot).and_then(|s| s.clone());

    // Подходящий рецепт: вход совпал и результат влезает в выход. Третье
    // поле — ген фрагмента, если его выбрал рецепт, а не случай.
    let mut found: Option<(i32, horizon_client::items::ItemStack, Option<(&'static str, i32)>)> =
        None;
    // Диапазон реактора показываем по первому рецепту, чей вход совпал, —
    // даже если температура до него ещё не дошла: ради этого и градусник.
    let mut range_shown = false;
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
            // Рецепт реактора может требовать температуру.
            let range = match (json["temperature"][0].as_i64(), json["temperature"][1].as_i64()) {
                (Some(min), Some(max)) => Some((min as i32, max as i32)),
                _ => None,
            };
            if entity.id == m::REACTOR && !range_shown {
                let (min, max) = range.unwrap_or((0, 0));
                entity.data[m::DATA_RANGE_MIN] = min;
                entity.data[m::DATA_RANGE_MAX] = max;
                range_shown = true;
            }
            let Some(result) = horizon_client::crafting::recipe_result(&json["result"]) else {
                continue;
            };
            // `items()` — таблица предметов игры, не хук-поинт: при обновлении
            // Horizon проверить. Нужна, чтобы не перелить через стопку.
            let max_stack = game.items().get(&result.item).map_or(64, |i| i.max_stack);
            // Фрагмент и культура несут свои данные и в стопку не сливаются:
            // им нужен пустой выход, иначе готовый предмет просто пропал бы.
            let unique = &*result.item == m::FRAGMENT_ID || &*result.item == m::CULTURE_ID;
            let fits = if unique {
                output.is_none()
            } else {
                m::output_fits(
                    output.as_ref().map(|s| (&*s.item, s.count)),
                    (&result.item, result.count),
                    max_stack,
                )
            };
            if !fits {
                continue;
            }
            if entity.id == m::REACTOR && !m::temperature_fits(temperature, range) {
                continue;
            }
            // Ген фрагмента: рецепт может назвать его сам (гены животных),
            // урожай ГМ-грядки отдаёт свой самый сильный, а ткань мутанта —
            // случайный, его решает выдача. Битый ген в рецепте или урожай
            // без генома — не рецепт: машина не тратит вход впустую.
            let mut gene = None;
            if &*result.item == m::FRAGMENT_ID {
                if let Some(name) = json["gene"].as_str() {
                    match m::recipe_gene(name, json["strength"].as_i64()) {
                        Some(chosen) => gene = Some(chosen),
                        None => continue,
                    }
                } else if &*input.item == m::GM_CROP {
                    let genome = custom_text(input, m::PLOT_GENOME_KEY).unwrap_or_default();
                    match m::strongest_gene(&m::genome_from_text(&genome)) {
                        Some(chosen) => gene = Some(chosen),
                        None => continue,
                    }
                }
            }
            let time = json["time"].as_i64().unwrap_or(m::DEFAULT_TIME as i64) as i32;
            found = Some((time, result, gene));
            break;
        }
    }

    let progress = entity.data[m::DATA_PROGRESS];
    // Подбор режима (GDD 4.23 п. 3): у ферментера и экстрактора рецепт идёт
    // по ручкам, и попытка оценивается скрытым окном сырья. Числа панели
    // пишутся каждый тик, даже когда рецепта нет.
    let recipe = input.as_ref().map(|stack| stack.item.to_string());
    let grade = crate::game::biolab_protocol::prepare(game, entity, recipe.as_deref());
    let time = found
        .as_ref()
        .map(|(time, _, _)| match grade {
            Some(grade) => m::protocol_time(*time, grade),
            None => *time,
        });
    let step = m::step(time, progress);
    match step {
        m::Step::Idle => {
            entity.data[m::DATA_PROGRESS] = 0;
            entity.data[m::DATA_TOTAL] = 0;
            entity.data[m::DATA_LIT] = 0;
            entity.data[m::DATA_LIT_TOTAL] = 0;
        }
        m::Step::Working(next) => {
            let total = time.expect("Working только с рецептом");
            entity.data[m::DATA_PROGRESS] = next;
            entity.data[m::DATA_TOTAL] = total;
            // Пламя горит всё время работы: клиент рисует его как у печки.
            entity.data[m::DATA_LIT] = 1;
            entity.data[m::DATA_LIT_TOTAL] = 1;
        }
        m::Step::Finish => {
            let (_, result, gene) = found.expect("Finish только с рецептом");
            // Забираем один предмет со входа.
            if let Some(slot) = entity.items.get_mut(m::SLOT_INPUT)
                && let Some(stack) = slot.as_mut()
            {
                stack.count -= 1;
                if stack.count == 0 {
                    *slot = None;
                }
            }
            // Экстрактор выдаёт не просто предмет, а фрагмент с геном. Ген
            // животного или урожая выбрал рецепт; у ткани мутанта его решает
            // зерно от места и времени — два образца подряд дают разное.
            let mut result = result;
            if &*result.item == m::FRAGMENT_ID {
                let seed = (pos[0] as i64 * 31 + pos[2] as i64 * 17 + game.ticks as i64) as u64;
                let (name, strength) = gene.unwrap_or_else(|| m::roll_gene(seed));
                let text = m::gene_text(name, strength);
                entity.nbt.insert(
                    m::LAST_GENE_KEY.into(),
                    horizon_world::nbt::Tag::String(text.as_str().into()),
                );
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
                // Палочка с мазком с моба (M11-А) даёт его штамм как есть.
                let strain =
                    m::incubated_strain(input.as_ref().map(|stack| &*stack.item), before, seed);
                let text = m::strain_to_text(&strain);
                // Селекция — опыт (GDD 4.23 п. 7): новая линия платит полностью,
                // та же самая — всё меньше. Мазок (первая культура) — не селекция.
                if before.is_some() && input.as_ref().is_some_and(|s| &*s.item == m::CULTURE_ID) {
                    let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5];
                    m::note_lab_event(game.dim.id(), at, "culture_bred", &text);
                }
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
                    m::culture_label(&strain).as_str().into(),
                ));
                result.count = 1;
            }
            // Подбор режима (GDD 4.23 п. 3): в окне — полный выход, рядом —
            // половина, мимо — ничего. Брак выдаёт пробу сам (`finish`).
            result.count = match grade {
                Some(grade) => m::protocol_count(result.count, grade),
                None => result.count,
            };
            // Кладём результат в выход.
            if result.count > 0 && let Some(slot) = entity.items.get_mut(output_slot) {
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
            // Итог попытки: подсказка направления, найденное окно — в сейв,
            // брак — пробой в выход.
            if let (Some(grade), Some(recipe)) = (grade, recipe.as_deref()) {
                crate::game::biolab_protocol::finish(game, pos, entity, recipe, output_slot, grade);
            }
        }
    }
    machine_sound(game, pos, entity, step);
}
