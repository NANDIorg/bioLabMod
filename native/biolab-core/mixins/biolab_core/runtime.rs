// BioLab, нативная часть на стабильных хук-поинтах: строка в Options > Mods и
// команда /biolab. Этот файл — одновременно Cargo-библиотека мода (его тесты) и
// модуль, который бандл добавляет в игру как `crate::game::biolab_core`.
//
// Дальше здесь появятся: машины (block entity + экраны), химия, болезни,
// фармакология и генетика растений — всё, что обходится хук-поинтами.
// Поведение эффектов, мутанты и их внешний вид живут в отдельном бандле
// biolab-deep, который пиннится на коммит игры.

/// Идентификатор мода (папка настроек), имя, версия.
pub const ID: &str = "biolab_core";
pub const NAME: &str = "BioLab";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str =
    "Био-химическая лаборатория: химия, микробиология, фармакология и генетика. /biolab — состояние мода.";

/// Подсистемы и их готовность. Команда /biolab печатает этот список, чтобы в
/// игре было видно, что уже включено, а что ещё не написано.
pub const SUBSYSTEMS: [(&str, bool); 6] = [
    ("Машины", true),
    ("Химия", true),
    ("Микробиология", true),
    ("Фармакология", true),
    ("Генетика", true),
    ("Мутации (biolab-deep)", true),
];

/// Строка подсистемы для чата: галочка у готовых, прочерк у остальных.
pub fn subsystem_line(name: &str, ready: bool) -> String {
    format!("{} {name}", if ready { "✓" } else { "—" })
}

/// Ответ команды /biolab: заголовок и состояние подсистем.
pub fn status_lines() -> Vec<String> {
    let mut lines = vec![format!("{NAME} {VERSION}")];
    lines.extend(
        SUBSYSTEMS
            .iter()
            .map(|(name, ready)| subsystem_line(name, *ready)),
    );
    lines
}

/// Имя команды без слеша и аргументов: `biolab status` → `biolab`.
pub fn command_name(command: &str) -> &str {
    command.split(' ').next().unwrap_or("")
}

/// Колба: предмет контентного мода, который несёт вещество в `custom_data`.
pub const FLASK_ID: &str = "biolab:flask";
/// Ключи внутри `minecraft:custom_data` колбы.
pub const CONTENTS_KEY: &str = "biolab_contents";
pub const ML_KEY: &str = "biolab_ml";
/// Чем заполнена новая колба, пока нет машин: прототип проверяет запись и
/// сохранение данных в предмете.
/// Чем наполняется пустая колба. Формулой, а не словом: так спектрометр,
/// журнал и чертежи видят в ней то же вещество, что и синтезатор.
pub const DEFAULT_FILL: (&str, i32) = ("H2O", 250);

/// Название вещества для подписи на предмете.
pub fn compound_name(id: &str) -> &str {
    match id {
        "water" => "вода",
        "ethanol" => "этанол",
        "sodium_chloride" => "раствор соли",
        "hydrochloric_acid" => "соляная кислота",
        _ => id,
    }
}

/// Подпись колбы: «Колба: вода, 250 мл».
pub fn flask_label(contents: &str, ml: i32) -> String {
    format!("Колба: {}, {ml} мл", compound_name(contents))
}

/// Состояние игрока в мире: болезни, кровь, прогресс лаборатории. Пока это
/// прототип механизма — счётчик тиков и уровень заражения; поля добавятся
/// вместе с подсистемами.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerState {
    /// Сколько тиков игрок провёл в этом мире с модом.
    pub ticks: u64,
    /// Уровень заражения 0–100.
    pub infection: u32,
    /// Кровь: по каждому веществу депо, концентрация и привыкание.
    pub drugs: [DrugState; DRUG_COUNT],
    /// Сколько тиков ещё держится иммунитет от вакцины.
    pub immunity: u32,
    /// Сколько тиков ещё держится ломка после стимулятора.
    pub withdrawal: u32,
    /// Гены игрока: сила 0–3 по порядку таблицы `GENES`.
    pub genes: [u8; GENES.len()],
    /// Накопленная мутационная нагрузка от мутагена.
    pub mutagenesis: u32,
    /// Штамм, которым игрок болен: четыре характеристики, нули — здоров.
    pub strain: [u8; 4],
    /// Выдана ли книга-руководство: второй раз она не приходит.
    pub manual: bool,
    /// Изученные узлы карты исследований: бит на узел `RESEARCH`.
    pub research: u64,
}

/// Как часто писать состояние на диск, в тиках (раз в 5 секунд).
pub const SAVE_EVERY: u64 = 100;

/// Файл состояния игрока внутри папки мира. Живёт вместе с миром: копия,
/// бэкап и удаление мира забирают его с собой.
pub fn state_path(world: &std::path::Path, uuid: u128) -> std::path::PathBuf {
    world
        .join("biolab")
        .join("players")
        .join(format!("{uuid:032x}.txt"))
}

/// Состояние строками `ключ=значение`: без зависимостей, читается глазами.
pub fn state_to_text(s: &PlayerState) -> String {
    let mut text = format!(
        "ticks={}\ninfection={}\nimmunity={}\nwithdrawal={}\nmutagenesis={}\nmanual={}\n",
        s.ticks, s.infection, s.immunity, s.withdrawal, s.mutagenesis, u8::from(s.manual)
    );
    if s.research != 0 {
        text.push_str(&format!("research={}\n", research_to_text(s.research)));
    }
    // Вещества пишем по имени, а не по номеру: новая версия мода может
    // переставить таблицу, и старый файл всё равно прочитается.
    if s.strain[0] > 0 {
        text.push_str(&format!("strain={}\n", strain_to_text(&Strain::of(s.strain.map(i32::from)))));
    }
    for (kind, strength) in GENES.iter().zip(&s.genes) {
        if *strength > 0 {
            text.push_str(&format!("gene.{}={}\n", kind.name, strength));
        }
    }
    for (drug, state) in DRUGS.iter().zip(&s.drugs) {
        if state.depot == 0 && state.blood == 0 && state.tolerance == 0 {
            continue;
        }
        text.push_str(&format!(
            "drug.{}={},{},{},{}\n",
            drug.id,
            state.depot,
            state.blood,
            state.tolerance,
            u8::from(state.was_active)
        ));
    }
    text
}

/// Одно вещество из строки файла: `депо,кровь,привыкание,был_в_окне`.
fn drug_state_from_text(value: &str) -> Option<DrugState> {
    let mut parts = value.split(',');
    let mut next = || parts.next()?.trim().parse::<u32>().ok();
    Some(DrugState {
        depot: next()?,
        blood: next()?,
        tolerance: next()?,
        was_active: next().unwrap_or(0) != 0,
    })
}

/// Состояние из файла. Незнакомые и битые строки пропускаем: файл от новой
/// версии мода не должен ломать старую.
pub fn state_from_text(text: &str) -> PlayerState {
    let mut s = PlayerState::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "ticks" => s.ticks = value.trim().parse().unwrap_or(s.ticks),
            "infection" => s.infection = value.trim().parse().unwrap_or(s.infection),
            "immunity" => s.immunity = value.trim().parse().unwrap_or(s.immunity),
            "withdrawal" => s.withdrawal = value.trim().parse().unwrap_or(s.withdrawal),
            "mutagenesis" => s.mutagenesis = value.trim().parse().unwrap_or(s.mutagenesis),
            "manual" => s.manual = value.trim() == "1",
            "research" => s.research = research_from_text(value),
            "strain" => {
                if let Some(strain) = strain_from_text(value.trim()) {
                    s.strain = strain.values().map(|v| v as u8);
                }
            }
            key => {
                // `drug.<имя>=...`: незнакомое вещество пропускаем молча.
                if let Some(id) = key.strip_prefix("gene.") {
                    // `gene.<имя>=<сила>`: незнакомый ген пропускаем молча.
                    if let Some(index) = GENES.iter().position(|g| g.name == id)
                        && let Ok(strength) = value.trim().parse::<u8>()
                        && (1..=3).contains(&strength)
                    {
                        s.genes[index] = strength;
                    }
                    continue;
                }
                if let Some(id) = key.strip_prefix("drug.")
                    && let Some(index) = drug_index(id)
                    && let Some(state) = drug_state_from_text(value)
                {
                    s.drugs[index] = state;
                }
            }
        }
    }
    s
}

type StateKey = (std::path::PathBuf, u128);

fn states() -> &'static std::sync::Mutex<std::collections::HashMap<StateKey, PlayerState>> {
    static STATES: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<StateKey, PlayerState>>,
    > = std::sync::OnceLock::new();
    STATES.get_or_init(Default::default)
}

/// Поставить уровень заражения и сразу сохранить: смена стадии — редкое
/// событие, ждать очередного автосохранения незачем.
pub fn set_infection(world: &std::path::Path, uuid: u128, infection: u32) {
    let Ok(mut table) = states().lock() else {
        return;
    };
    let state = table.entry((world.to_path_buf(), uuid)).or_default();
    // Заражение без явного штамма (команда отладки, случайный источник) —
    // это дикая линия: иначе болезнь некому «ужесточать» при срыве курса.
    if state.infection == 0 && infection > 0 && state.strain[0] == 0 {
        state.strain = WILD.values().map(|v| v as u8);
    }
    if infection == 0 {
        state.strain = [0; 4];
    }
    state.infection = infection.min(100);
    let path = state_path(world, uuid);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, state_to_text(state));
}

/// Тик игрока: при первом тике в мире состояние читается с диска, дальше
/// живёт в памяти и раз в `SAVE_EVERY` тиков пишется обратно.
pub fn player_tick(world: &std::path::Path, uuid: u128) -> Option<PlayerState> {
    let mut table = states().lock().ok()?;
    let state = table
        .entry((world.to_path_buf(), uuid))
        .or_insert_with(|| {
            std::fs::read_to_string(state_path(world, uuid))
                .map(|t| state_from_text(&t))
                .unwrap_or_default()
        });
    state.ticks += 1;
    if state.ticks % SAVE_EVERY == 0 {
        let path = state_path(world, uuid);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, state_to_text(state));
    }
    Some(*state)
}

// ---- Машины ----
//
// Каркас один на все машины мода: блок с внутренним состоянием, слоты,
// свой тип рецептов и тик обработки. Перегонный куб — первая такая машина.
// Меню берём ванильное, печкино: три слота (вход, топливо, выход) и четыре
// числа данных, из которых клиент сам рисует пламя и стрелку прогресса.

/// Машины мода: блок, тип рецептов и id блок-сущности совпадают по имени.
/// `slots` и `menu` — сколько у машины слотов и какое ванильное меню ей
/// открывать. Машины на рецептах берут вход из слота 0 и кладут в последний;
/// синтезатор читает все слоты, кроме последнего.
pub struct Machine {
    pub id: &'static str,
    pub slots: usize,
    pub menu: &'static str,
}

pub const MACHINES: [Machine; 11] = [
    Machine { id: "biolab:still", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:incubator", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:fermenter", slots: 3, menu: "minecraft:furnace" },
    // Центрифуга разделяет: вход и четыре выхода, меню воронки.
    Machine { id: "biolab:centrifuge", slots: 5, menu: "minecraft:hopper" },
    Machine { id: "biolab:synthesizer", slots: 9, menu: "minecraft:generic_3x3" },
    Machine { id: "biolab:reactor", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:press", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:extractor", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:cloning_vat", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:spectrometer", slots: 3, menu: "minecraft:furnace" },
    // Лабораторный стол — верстак точных вещей: сетка 3×3, как у
    // синтезатора, 0–7 входы, 8 — выход.
    Machine { id: "biolab:lab_bench", slots: 9, menu: "minecraft:generic_3x3" },
];

/// Машина с таким id, если она наша.
pub fn machine(id: &str) -> Option<&'static Machine> {
    MACHINES.iter().find(|m| m.id == id)
}

/// Наша ли это машина.
pub fn is_machine(id: &str) -> bool {
    machine(id).is_some()
}

/// Синтезатор собирает вещество из элементов, а не варит рецепт.
pub const SYNTHESIZER: &str = "biolab:synthesizer";

/// Слоты в меню печки.
pub const SLOT_INPUT: usize = 0;
pub const SLOT_FUEL: usize = 1;
pub const SLOT_OUTPUT: usize = 2;

/// Числа данных меню печки: пламя (0 из 1) и стрелка (2 из 3).
pub const DATA_LIT: usize = 0;
pub const DATA_LIT_TOTAL: usize = 1;
pub const DATA_PROGRESS: usize = 2;
pub const DATA_TOTAL: usize = 3;
pub const DATA_SLOTS: usize = 4;

// Свои числа машин для их экранов (GDD 4.16, контракт слотов данных). Идут
// после четырёх общих; сколько всего у машины — `machine_data_slots`.
/// Реактор: температура, цель нагрева (−1 — нет источника), диапазон рецепта.
pub const DATA_TEMPERATURE: usize = 4;
pub const DATA_HEAT_TARGET: usize = 5;
pub const DATA_RANGE_MIN: usize = 6;
pub const DATA_RANGE_MAX: usize = 7;
/// Синтезатор: статус сборки и число атомов каждого элемента по порядку `ELEMENTS`.
pub const DATA_SYNTH_STATUS: usize = 4;
pub const DATA_SYNTH_ELEMENTS: usize = 5;
/// Статусы синтезатора.
pub const SYNTH_EMPTY: i32 = 0;
pub const SYNTH_UNSTABLE: i32 = 1;
pub const SYNTH_STABLE: i32 = 2;
pub const SYNTH_NOT_BLUEPRINT: i32 = 3;
/// Спектрометр: состав колбы во входе по порядку `ELEMENTS`.
pub const DATA_SPECTRUM: usize = 4;
/// Клонирующий чан и экстрактор: номер гена в `GENES` (−1 — нет) и сила.
pub const DATA_GENE: usize = 4;
pub const DATA_GENE_STRENGTH: usize = 5;
/// Инкубатор: штамм культуры во входе, четыре характеристики.
pub const DATA_STRAIN: usize = 4;
/// Лабораторный стол: что соберётся из выложенного — номер результата в
/// `BENCH_RESULTS` или `BENCH_EMPTY`, `BENCH_NONE`, `BENCH_UNLISTED`.
pub const DATA_BENCH_RESULT: usize = 4;

/// Сколько слотов данных у машины: четыре общих и её собственные.
pub fn machine_data_slots(id: &str) -> usize {
    match id {
        REACTOR => DATA_RANGE_MAX + 1,
        SYNTHESIZER => DATA_SYNTH_ELEMENTS + ELEMENTS.len(),
        SPECTROMETER => DATA_SPECTRUM + ELEMENTS.len(),
        CLONING_VAT | "biolab:extractor" => DATA_GENE_STRENGTH + 1,
        "biolab:incubator" => DATA_STRAIN + 4,
        LAB_BENCH => DATA_BENCH_RESULT + 1,
        _ => DATA_SLOTS,
    }
}

/// Сколько тиков машина работает, если рецепт не сказал иного.
pub const DEFAULT_TIME: i32 = 100;

/// Что делать машине в этом тике.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Работы нет: сбросить прогресс.
    Idle,
    /// Идёт обработка: прогресс стал таким.
    Working(i32),
    /// Рецепт закончен: забрать вход, выдать выход.
    Finish,
}

/// Шаг машины. `recipe_time` — время текущего рецепта, если вход подходит и
/// выход влезает; `progress` — сколько уже натикало.
pub fn step(recipe_time: Option<i32>, progress: i32) -> Step {
    let Some(time) = recipe_time.filter(|t| *t > 0) else {
        return Step::Idle;
    };
    if progress + 1 >= time {
        Step::Finish
    } else {
        Step::Working(progress + 1)
    }
}

/// Влезет ли результат в слот выхода: пусто, или тот же предмет с запасом.
pub fn output_fits(slot: Option<(&str, u32)>, result: (&str, u32), max_stack: u32) -> bool {
    match slot {
        None => true,
        Some((item, count)) => item == result.0 && count + result.1 <= max_stack,
    }
}

// ---- Роли машин (GDD 4.15) ----
//
// У каждой машины своя ось: перегонный куб разбирает неживое на элементы,
// ферментер работает с живым, центрифуга разделяет, экстрактор достаёт гены.
// Здесь только решения — какие слоты у машины, как разложить колбу, куда
// положить выходы, какой ген выдать и какие числа показать экрану. Предметы
// двигает тик машины в `machine.rs`.

pub const CENTRIFUGE: &str = "biolab:centrifuge";
pub const INCUBATOR: &str = "biolab:incubator";
/// Последний ген, выданный экстрактором, — в сейве машины: экран его
/// показывает, а числа меню не сохраняются и после перезахода были бы нулём,
/// то есть гигантизмом.
pub const LAST_GENE_KEY: &str = "biolab_last_gene";
/// Сколько видов элементов центрифуга достаёт из колбы: из четырёх выходов
/// один всегда нужен под саму колбу, которая возвращается с водой.
pub const DECOMPOSE_KINDS: usize = 3;

/// Слоты, куда воронки кладут вход. Слот топлива меню печки нужен только
/// клонирующему чану (питательный раствор): у остальных машин он пустует, и
/// воронка, кладущая туда вход, просто забивала бы его.
pub fn machine_inputs(machine: &Machine) -> Vec<usize> {
    match machine.menu {
        "minecraft:furnace" if machine.id == CLONING_VAT => vec![SLOT_INPUT, SLOT_FUEL],
        "minecraft:furnace" | "minecraft:hopper" => vec![SLOT_INPUT],
        _ => (0..machine.slots - 1).collect(),
    }
}

/// Слоты, откуда воронки забирают выход. У центрифуги на меню воронки вход
/// один, а все остальные слоты — выходы.
pub fn machine_outputs(machine: &Machine) -> Vec<usize> {
    match machine.menu {
        "minecraft:hopper" => (1..machine.slots).collect(),
        _ => vec![machine.slots - 1],
    }
}

/// Сколько тиков собирается или разлагается вещество: чем больше атомов,
/// тем дольше. Потолок — чтобы огромная колба не крутилась вечно.
pub fn assembly_time(atoms: u32) -> i32 {
    40u32.saturating_add(atoms.saturating_mul(10)).min(600) as i32
}

/// Колба, разложенная на элементы: стаки по выходам и сколько всего атомов
/// (от них зависит время).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decomposed {
    pub stacks: Vec<(&'static str, u32)>,
    pub atoms: u32,
}

/// Разложение колбы в центрифуге: вещество — обратно в элементы, по атому
/// на предмет. Стак больше стопки делится. Не разлагается:
/// - вода — колба возвращается из центрифуги с водой, и иначе одна колба
///   давала бы водород без конца;
/// - вещество из больше чем `DECOMPOSE_KINDS` элементов — не хватит выходов;
/// - старая колба со словом вместо формулы.
pub fn decompose(contents: &str, max_stack: impl Fn(&str) -> u32) -> Option<Decomposed> {
    let formula = formula_from_text(contents)?;
    if formula_text(&formula) == DEFAULT_FILL.0 {
        return None;
    }
    if formula.iter().filter(|count| **count > 0).count() > DECOMPOSE_KINDS {
        return None;
    }
    let mut stacks = Vec::new();
    for (count, element) in formula.iter().zip(ELEMENTS.iter()) {
        let max = max_stack(element.item).max(1);
        let mut left = *count;
        while left > 0 {
            let take = left.min(max);
            stacks.push((element.item, take));
            left -= take;
        }
    }
    Some(Decomposed { stacks, atoms: formula.iter().sum() })
}

/// Стак для раскладки по выходам. `plain` — нет своих данных: такой стак
/// можно слить с таким же. Колба с веществом, фрагмент, культура — не плоские.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pile<'a> {
    pub item: &'a str,
    pub count: u32,
    pub plain: bool,
}

/// Куда лечь выходам машины с несколькими выходами: в такой же стак, если
/// туда влезает целиком, иначе в первый свободный слот. Вернёт номер слота
/// (в `slots`) для каждого выхода по порядку или None, если хоть один не
/// влезает, — тогда машина ждёт, а не теряет добычу.
pub fn plan_outputs<'a>(
    slots: &[Option<Pile<'a>>],
    piles: &[Pile<'a>],
    max_stack: impl Fn(&str) -> u32,
) -> Option<Vec<usize>> {
    let mut held = slots.to_vec();
    let mut plan = Vec::with_capacity(piles.len());
    for pile in piles {
        let max = max_stack(pile.item);
        let same = held.iter().position(|slot| {
            slot.is_some_and(|s| {
                pile.plain && s.plain && s.item == pile.item && s.count + pile.count <= max
            })
        });
        let index = same.or_else(|| held.iter().position(Option::is_none))?;
        held[index] = Some(match held[index] {
            Some(s) => Pile { count: s.count + pile.count, ..s },
            None => *pile,
        });
        plan.push(index);
    }
    Some(plan)
}

/// Ген, который называет рецепт экстрактора (`"gene"`, `"strength"`, сила по
/// умолчанию 1): гены животных достаются не случайно, кроличья лапка всегда
/// даёт пружинные ноги. Незнакомый ген или сила вне 1–3 — рецепт битый.
pub fn recipe_gene(name: &str, strength: Option<i64>) -> Option<(&'static str, i32)> {
    let kind = gene(name)?;
    let strength = strength.unwrap_or(1);
    (1..=3).contains(&strength).then_some((kind.name, strength as i32))
}

/// Самый сильный ген урожая ГМ-грядки — его экстрактор и достаёт. При
/// равной силе берём более дорогой (он ценнее), при полном равенстве — тот,
/// что раньше в геноме.
pub fn strongest_gene(genes: &[(&'static str, i32)]) -> Option<(&'static str, i32)> {
    let rank = |(name, strength): (&str, i32)| (strength, gene(name).map_or(0, |k| k.cost));
    genes.iter().copied().fold(None, |best, next| match best {
        Some(b) if rank(b) >= rank(next) => Some(b),
        _ => Some(next),
    })
}

/// Числа гена для экрана чана и экстрактора: номер в `GENES` (−1 — гена нет)
/// и сила. Ген — строка из предмета (`chitin:2`).
pub fn gene_data(text: Option<&str>) -> (i32, i32) {
    let Some((name, strength)) = text.and_then(gene_from_text) else {
        return (-1, 0);
    };
    let index = GENES.iter().position(|g| g.name == name).map_or(-1, |i| i as i32);
    (index, strength)
}

/// Штамм культуры во входе инкубатора для экрана. Нули — во входе не
/// культура; культура без записи штамма — дикая: инкубатор и ведёт её линию
/// от дикой.
pub fn strain_data(item: Option<&str>, strain: Option<&str>) -> [i32; 4] {
    if item != Some(CULTURE_ID) {
        return [0; 4];
    }
    strain.and_then(strain_from_text).unwrap_or(WILD).values()
}

/// Статус синтезатора для экрана. Несовпадение с чертежом важнее
/// устойчивости: с чертежом машина ждёт ровно его состав.
pub fn synth_status(formula: &Formula, blueprint: Option<&Formula>) -> i32 {
    if formula.iter().all(|count| *count == 0) {
        SYNTH_EMPTY
    } else if blueprint.is_some_and(|target| target != formula) {
        SYNTH_NOT_BLUEPRINT
    } else if is_stable(formula) {
        SYNTH_STABLE
    } else {
        SYNTH_UNSTABLE
    }
}

// ---- Лабораторный стол ----
//
// Верстак лаборатории. Точные вещи — шприцы, дротики, аэрозоли, мази,
// приборы, мутаген — обычный верстак больше не собирает: их рецепты своего
// типа `biolab:lab_bench`. Рецепт — список `ingredients`, как у бесформенного
// рецепта верстака, плюс `result {id, count}` и `time` в тиках. Стол собирает,
// когда во входах ровно набор рецепта, и ждёт, если выход занят.

pub const LAB_BENCH: &str = "biolab:lab_bench";

/// Входы стола пусты.
pub const BENCH_EMPTY: i32 = -2;
/// Выложенное не совпало ни с одним рецептом.
pub const BENCH_NONE: i32 = -1;
/// Рецепт совпал, но его результата нет в `BENCH_RESULTS` (рецепт из
/// чужого датапака): стол соберёт, а экран не знает имени.
pub const BENCH_UNLISTED: i32 = -3;

/// Результаты рецептов стола, id и имя. Клиенту результат можно передать
/// только числом слота данных, поэтому номер — место в этой таблице, а имя
/// отсюда же рисует табло. Тест сверяет таблицу с рецептами стола в
/// `content/` и с именами в языковом файле.
pub const BENCH_RESULTS: [(&str, &str); 19] = [
    ("biolab:syringe", "Шприц"),
    ("biolab:syringe_penicillin", "Пенициллин, инъекция"),
    ("biolab:syringe_sedative", "Седативное, инъекция"),
    ("biolab:syringe_stimulant", "Стимулятор, инъекция"),
    ("biolab:syringe_venom", "Яд, инъекция"),
    ("biolab:syringe_vaccine", "Вакцина, инъекция"),
    ("biolab:syringe_mutagen", "Мутаген, инъекция"),
    ("biolab:dart_sedative", "Дротик с седативным"),
    ("biolab:dart_venom", "Дротик с ядом"),
    ("biolab:spray_analgesic", "Обезболивающее, аэрозоль"),
    ("biolab:spray_stimulant", "Стимулятор, аэрозоль"),
    ("biolab:ointment_analgesic", "Обезболивающая мазь"),
    ("biolab:analyzer", "Анализатор крови"),
    ("biolab:microscope", "Микроскоп"),
    ("biolab:scalpel", "Зазубренный скальпель"),
    ("biolab:swab", "Ватная палочка"),
    ("biolab:bandage", "Бинт"),
    ("biolab:mutagen", "Мутаген"),
    ("biolab:nutrient_broth", "Питательный раствор"),
];

/// Номер результата для слота данных стола.
pub fn bench_result_index(item: &str) -> i32 {
    BENCH_RESULTS.iter().position(|r| r.0 == item).map_or(BENCH_UNLISTED, |i| i as i32)
}

/// Сопоставление входов стола с рецептом. `filled` — занят ли каждый вход,
/// `fits(запись, слот)` — подходит ли предмет слота под запись рецепта.
/// Каждая запись берёт один предмет из своего слота, слоты у записей разные,
/// и занятых слотов ровно столько, сколько записей: лишнего во входах нет.
/// Порядок неважен. Стопка в слоте не делится между записями — как на
/// верстаке, зато её хватает на несколько сборок подряд. Вернёт для каждой
/// записи номер её слота или None, если набор не совпал.
///
/// Записи могут пересекаться (тег или список предметов), поэтому жадно
/// не выйдет: ищем паросочетание дополняющими путями. Входов восемь —
/// перебор мгновенный.
pub fn bench_match(filled: &[bool], ingredients: usize, fits: impl Fn(usize, usize) -> bool) -> Option<Vec<usize>> {
    fn claim(
        entry: usize,
        filled: &[bool],
        fits: &dyn Fn(usize, usize) -> bool,
        owner: &mut [Option<usize>],
        seen: &mut [bool],
    ) -> bool {
        for slot in 0..filled.len() {
            if !filled[slot] || seen[slot] || !fits(entry, slot) {
                continue;
            }
            seen[slot] = true;
            let free = match owner[slot] {
                None => true,
                Some(other) => claim(other, filled, fits, owner, seen),
            };
            if free {
                owner[slot] = Some(entry);
                return true;
            }
        }
        false
    }

    let used = filled.iter().filter(|f| **f).count();
    if ingredients == 0 || used != ingredients {
        return None;
    }
    let mut owner: Vec<Option<usize>> = vec![None; filled.len()];
    for entry in 0..ingredients {
        let mut seen = vec![false; filled.len()];
        if !claim(entry, filled, &fits, &mut owner, &mut seen) {
            return None;
        }
    }
    let mut plan = vec![0; ingredients];
    for (slot, entry) in owner.iter().enumerate() {
        if let Some(entry) = entry {
            plan[*entry] = slot;
        }
    }
    Some(plan)
}

#[cfg(test)]
mod bench_tests {
    use super::*;

    /// Сопоставление по именам: запись — список подходящих предметов.
    fn matched(slots: &[Option<&str>], recipe: &[&[&str]]) -> Option<Vec<usize>> {
        let filled: Vec<bool> = slots.iter().map(Option::is_some).collect();
        bench_match(&filled, recipe.len(), |entry, slot| {
            slots[slot].is_some_and(|item| recipe[entry].contains(&item))
        })
    }

    #[test]
    fn the_bench_takes_exactly_the_recipe_set_in_any_order() {
        let syringe: &[&[&str]] = &[&["nugget"], &["pane"], &["nugget"]];
        let slots = [Some("pane"), None, Some("nugget"), None, None, None, None, Some("nugget")];
        let plan = matched(&slots, syringe).expect("набор совпал");
        let mut taken = plan.clone();
        taken.sort();
        assert_eq!(taken, vec![0, 2, 7], "каждая запись — свой слот");
        assert_eq!(slots[plan[1]], Some("pane"));
        // Не хватает самородка, лишняя палка, пусто.
        assert_eq!(matched(&[Some("pane"), Some("nugget")], syringe), None);
        assert_eq!(matched(&[Some("pane"), Some("nugget"), Some("nugget"), Some("stick")], syringe), None);
        assert_eq!(matched(&[None; 8], syringe), None);
        // Чужой предмет вместо самородка.
        assert_eq!(matched(&[Some("pane"), Some("nugget"), Some("stick")], syringe), None);
        // Рецепт без записей ничего не собирает.
        assert_eq!(matched(&[None; 8], &[]), None);
    }

    #[test]
    fn overlapping_entries_still_find_their_slots() {
        // Первая запись берёт и то и другое, вторая — только «a»: жадный
        // выбор отдал бы «a» первой и провалил вторую.
        let recipe: &[&[&str]] = &[&["a", "b"], &["a"]];
        let plan = matched(&[Some("a"), Some("b")], recipe).expect("паросочетание есть");
        assert_eq!(plan, vec![1, 0]);
        assert_eq!(matched(&[Some("b"), Some("b")], recipe), None);
    }

    #[test]
    fn bench_results_are_numbered_by_the_table() {
        assert_eq!(bench_result_index("biolab:syringe"), 0);
        assert_eq!(bench_result_index("biolab:nutrient_broth"), BENCH_RESULTS.len() as i32 - 1);
        assert_eq!(bench_result_index("minecraft:stick"), BENCH_UNLISTED);
        assert_eq!(machine_data_slots(LAB_BENCH), DATA_BENCH_RESULT + 1);
        let bench = machine(LAB_BENCH).expect("стол — машина");
        assert_eq!(machine_inputs(bench), (0..8).collect::<Vec<_>>());
        assert_eq!(machine_outputs(bench), vec![8]);
    }

    /// Рецепты стола в `content/`: результат и записи.
    fn bench_recipes() -> Vec<(String, String, usize)> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/recipe");
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let json = Json::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
            if json.get("type").and_then(Json::str) != Some(LAB_BENCH) {
                continue;
            }
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let result = json.get("result").and_then(|r| r.get("id")).and_then(Json::str).expect(&name);
            let entries = json.get("ingredients").and_then(Json::arr).map_or(0, <[Json]>::len);
            out.push((name, result.to_string(), entries));
        }
        out
    }

    #[test]
    fn the_result_table_matches_the_bench_recipes() {
        let recipes = bench_recipes();
        assert!(!recipes.is_empty());
        let mut from_content: Vec<&str> = recipes.iter().map(|(_, result, _)| result.as_str()).collect();
        from_content.sort();
        from_content.dedup();
        let mut table: Vec<&str> = BENCH_RESULTS.iter().map(|r| r.0).collect();
        table.sort();
        let before = table.len();
        table.dedup();
        assert_eq!(table.len(), before, "результат в таблице дважды");
        assert_eq!(table, from_content, "таблица стола разошлась с рецептами");
        for (name, _, entries) in &recipes {
            assert!((1..=8).contains(entries), "{name}: записей {entries}, а входов у стола восемь");
        }
        // Имя на табло — то же, что у предмета в игре.
        let lang = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/assets/biolab/lang/ru_ru.json");
        let lang = Json::parse(&std::fs::read_to_string(lang).unwrap()).unwrap();
        for (id, name) in &BENCH_RESULTS {
            let key = format!("item.biolab.{}", id.trim_start_matches("biolab:"));
            assert_eq!(lang.get(&key).and_then(Json::str), Some(*name), "{id}");
        }
    }

    #[test]
    fn precise_things_left_the_crafting_table() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/recipe");
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let json = Json::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let kind = json.get("type").and_then(Json::str).unwrap_or("");
            if !kind.starts_with("minecraft:crafting") {
                continue;
            }
            let result = json.get("result").and_then(|r| r.get("id")).and_then(Json::str).unwrap_or("");
            assert_eq!(bench_result_index(result), BENCH_UNLISTED, "{}: {result} собирается на верстаке", path.display());
        }
    }
}

// ---- Заражение ----
//
// Игровая абстракция: у игрока есть число 0–100, оно растёт само и падает от
// препарата. По порогам число превращается в стадию, а стадия — в ванильные
// эффекты-симптомы. Всё хранится в том же файле состояния, что и остальное.

/// Цвета панели анализатора.
pub const PANEL_BACKGROUND: u32 = 0x7800_0000;
pub const PANEL_TITLE: u32 = 0x7C_FF4A;
pub const PANEL_INK: u32 = 0xFF_FFFF;

/// Канал, по которому сервер сообщает клиенту уровень заражения.
/// Файл состояния для этого не годится: у клиентской стороны свой `Game`, и
/// корня мира в нём нет — это выяснилось на первой же проверке HUD.
pub const NET_INFECTION: &str = "biolab_core:infection";
/// Сколько веществ уже в журнале: приходит вместе с уровнем заражения.
fn known_journal() -> &'static std::sync::atomic::AtomicU64 {
    static VALUE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    &VALUE
}

pub fn remember_journal(count: u32) {
    known_journal().store(u64::from(count), std::sync::atomic::Ordering::Relaxed);
}

pub fn shown_journal() -> u32 {
    known_journal().load(std::sync::atomic::Ordering::Relaxed) as u32
}

/// Последний уровень, присланный сервером: его и показывает анализатор.
fn known_infection() -> &'static std::sync::atomic::AtomicU64 {
    static VALUE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
    &VALUE
}

/// Клиент получил уровень от сервера.
pub fn remember_infection(level: u32) {
    known_infection().store(u64::from(level.min(100)), std::sync::atomic::Ordering::Relaxed);
}

/// Что показывать анализатору; None — сервер ещё ничего не прислал.
pub fn shown_infection() -> Option<u32> {
    match known_infection().load(std::sync::atomic::Ordering::Relaxed) {
        u64::MAX => None,
        level => Some(level as u32),
    }
}

/// Анализатор крови: предмет, который показывает состояние в углу экрана.
pub const ANALYZER_ID: &str = "biolab:analyzer";

/// Заражённая еда и насколько она поднимает уровень.
pub const TAINTED_FOOD: [(&str, u32); 2] =
    [("minecraft:rotten_flesh", 15), ("minecraft:spider_eye", 10)];

/// На сколько поднимает уровень близкий контакт с заражённым мобом.
pub const CONTACT_STEP: u32 = 3;
/// Насколько близко нужно оказаться, в блоках.
pub const CONTACT_RANGE: f64 = 2.0;

/// Поднимает ли эта еда заражение.
pub fn tainted_food(item: &str) -> Option<u32> {
    TAINTED_FOOD
        .iter()
        .find(|(id, _)| *id == item)
        .map(|(_, amount)| *amount)
}

/// Новый уровень после контакта с источником.
pub fn after_exposure(infection: u32, amount: u32) -> u32 {
    (infection + amount).min(100)
}

/// Название стадии для игрока.
pub fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Clear => "здоров",
        Stage::Incubating => "инкубация",
        Stage::Symptoms => "симптомы",
        Stage::Crisis => "кризис",
    }
}

/// Строки анализатора крови.
pub fn analyzer_lines(infection: u32, journal: u32, drugs: &[DrugState; DRUG_COUNT]) -> Vec<String> {
    analyzer_lines_with(infection, journal, drugs, &WILD)
}

/// То же, но со штаммом: стадия и строка штамма считаются по нему.
pub fn analyzer_lines_with(
    infection: u32,
    journal: u32,
    drugs: &[DrugState; DRUG_COUNT],
    strain: &Strain,
) -> Vec<String> {
    let mut lines = vec![
        "Анализ крови".to_string(),
        format!("Заражение: {infection}%"),
        format!("Стадия: {}", stage_name(stage_with(infection, strain))),
        format!("Журнал: {journal} веществ"),
    ];
    if infection > 0 {
        let v = strain.values();
        lines.push(format!(
            "Штамм: з{} и{} т{} с{}",
            v[0], v[1], v[2], v[3]
        ));
    }
    lines.extend(blood_lines(drugs));
    lines
}

/// Кровь, присланная сервером: её показывает анализатор.
fn known_blood() -> &'static std::sync::Mutex<[DrugState; DRUG_COUNT]> {
    static VALUE: std::sync::OnceLock<std::sync::Mutex<[DrugState; DRUG_COUNT]>> =
        std::sync::OnceLock::new();
    VALUE.get_or_init(|| std::sync::Mutex::new([DrugState::default(); DRUG_COUNT]))
}

/// Клиент получил кровь от сервера.
pub fn remember_blood(drugs: [DrugState; DRUG_COUNT]) {
    if let Ok(mut value) = known_blood().lock() {
        *value = drugs;
    }
}

/// Что показывать анализатору про кровь.
pub fn shown_blood() -> [DrugState; DRUG_COUNT] {
    known_blood()
        .lock()
        .map(|value| *value)
        .unwrap_or([DrugState::default(); DRUG_COUNT])
}

/// Свой эффект-метка: показывает игроку, что он болен.
pub const INFECTION_EFFECT: &str = "biolab:infection";

/// Сообщение после приёма препарата.
pub fn remedy_message(before: u32, after: u32) -> String {
    if after == 0 {
        format!("Заражение снято (было {before}%)")
    } else {
        format!("Заражение: {before}% → {after}%")
    }
}

/// Предмет, который лечит.
pub const REMEDY_ID: &str = "biolab:remedy";
/// Насколько препарат сбивает заражение.
pub const REMEDY_RELIEF: u32 = 40;
/// Сколько заражение растёт за шаг и как часто делается шаг.
pub const INFECTION_STEP: u32 = 1;
pub const INFECTION_EVERY: u64 = 60;

/// Стадия болезни по уровню заражения.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Здоров.
    Clear,
    /// Инкубация: симптомов ещё нет.
    Incubating,
    /// Симптомы: слабость и голод.
    Symptoms,
    /// Кризис: к ним добавляется тошнота.
    Crisis,
}

/// Стадия по уровню заражения.
pub fn stage(infection: u32) -> Stage {
    match infection {
        0 => Stage::Clear,
        1..=29 => Stage::Incubating,
        30..=69 => Stage::Symptoms,
        _ => Stage::Crisis,
    }
}

/// Какие ванильные эффекты держать на игроке в этой стадии.
pub fn symptoms(stage: Stage) -> &'static [&'static str] {
    match stage {
        Stage::Clear | Stage::Incubating => &[],
        Stage::Symptoms => &["weakness", "hunger"],
        Stage::Crisis => &["weakness", "hunger", "nausea"],
    }
}

/// Новый уровень заражения после приёма препарата.
pub fn after_remedy(infection: u32) -> u32 {
    infection.saturating_sub(REMEDY_RELIEF)
}

/// Новый уровень заражения на очередном шаге болезни.
pub fn after_step(infection: u32) -> u32 {
    if infection == 0 {
        return 0;
    }
    (infection + INFECTION_STEP).min(100)
}

// ---- Элементы и вещества ----
//
// Игровая абстракция: у каждого элемента есть число связей, и из набора
// элементов складывается формула. Собирается она или рассыпается, решает
// простое правило связей, а свойства получаются из состава. Никакой настоящей
// химии здесь нет — это игровые числа, как уровни зачарований.

/// Элемент: предмет мода, символ, число связей и «вес» для расчёта свойств.
pub struct Element {
    pub item: &'static str,
    pub symbol: &'static str,
    pub bonds: u32,
    pub mass: u32,
}

pub const ELEMENTS: [Element; 18] = [
    Element { item: "biolab:element_hydrogen", symbol: "H", bonds: 1, mass: 1 },
    Element { item: "biolab:element_carbon", symbol: "C", bonds: 4, mass: 12 },
    Element { item: "biolab:element_nitrogen", symbol: "N", bonds: 3, mass: 14 },
    Element { item: "biolab:element_oxygen", symbol: "O", bonds: 2, mass: 16 },
    Element { item: "biolab:element_sodium", symbol: "Na", bonds: 1, mass: 23 },
    Element { item: "biolab:element_sulfur", symbol: "S", bonds: 2, mass: 32 },
    Element { item: "biolab:element_calcium", symbol: "Ca", bonds: 2, mass: 40 },
    Element { item: "biolab:element_iron", symbol: "Fe", bonds: 3, mass: 56 },
    Element { item: "biolab:element_copper", symbol: "Cu", bonds: 2, mass: 64 },
    Element { item: "biolab:element_iodine", symbol: "I", bonds: 1, mass: 127 },
    Element { item: "biolab:element_phosphorus", symbol: "P", bonds: 3, mass: 31 },
    Element { item: "biolab:element_chlorine", symbol: "Cl", bonds: 1, mass: 35 },
    Element { item: "biolab:element_potassium", symbol: "K", bonds: 1, mass: 39 },
    Element { item: "biolab:element_magnesium", symbol: "Mg", bonds: 2, mass: 24 },
    Element { item: "biolab:element_silicon", symbol: "Si", bonds: 4, mass: 28 },
    Element { item: "biolab:element_gold", symbol: "Au", bonds: 1, mass: 197 },
    // Два фантазийных: в журнале помечены как не существующие в природе.
    Element { item: "biolab:element_redstonium", symbol: "Rs", bonds: 3, mass: 70 },
    Element { item: "biolab:element_glaurium", symbol: "Gl", bonds: 2, mass: 90 },
];

/// Элемент по предмету.
pub fn element(item: &str) -> Option<&'static Element> {
    ELEMENTS.iter().find(|e| e.item == item)
}

/// Набор элементов: сколько каких атомов положили, в порядке `ELEMENTS`.
pub type Formula = [u32; ELEMENTS.len()];

/// Держится ли формула вместе. Правило игровое: сумма связей чётная, и ни
/// одному атому не нужно больше связей, чем дают все остальные вместе.
pub fn is_stable(formula: &Formula) -> bool {
    let atoms: u32 = formula.iter().sum();
    if atoms < 2 {
        return false;
    }
    let total: u32 = formula
        .iter()
        .zip(ELEMENTS.iter())
        .map(|(count, e)| count * e.bonds)
        .sum();
    if total % 2 != 0 {
        return false;
    }
    let largest = formula
        .iter()
        .zip(ELEMENTS.iter())
        .filter(|(count, _)| **count > 0)
        .map(|(_, e)| e.bonds)
        .max()
        .unwrap_or(0);
    largest <= total - largest
}

/// Запись формулы: сначала углерод и водород, потом остальные по порядку
/// таблицы — так же, как химики пишут органику.
pub fn formula_text(formula: &Formula) -> String {
    let mut order: Vec<usize> = (0..ELEMENTS.len()).collect();
    // Сначала углерод и водород, остальные по алфавиту.
    order.sort_by_key(|i| {
        let rank = match ELEMENTS[*i].symbol {
            "C" => 0,
            "H" => 1,
            _ => 2,
        };
        (rank, ELEMENTS[*i].symbol)
    });
    let mut out = String::new();
    for i in order {
        match formula[i] {
            0 => {}
            1 => out.push_str(ELEMENTS[i].symbol),
            n => out.push_str(&format!("{}{n}", ELEMENTS[i].symbol)),
        }
    }
    out
}

/// Масса молекулы: по ней определяется агрегатное состояние.
pub fn formula_mass(formula: &Formula) -> u32 {
    formula
        .iter()
        .zip(ELEMENTS.iter())
        .map(|(count, e)| count * e.mass)
        .sum()
}

/// Агрегатное состояние.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Gas,
    Liquid,
    Solid,
}

pub fn state_of(formula: &Formula) -> State {
    match formula_mass(formula) {
        0..=30 => State::Gas,
        31..=150 => State::Liquid,
        _ => State::Solid,
    }
}

/// Сколько в наборе атомов этого символа.
pub fn count_of(formula: &Formula, symbol: &str) -> u32 {
    ELEMENTS
        .iter()
        .position(|e| e.symbol == symbol)
        .map_or(0, |i| formula[i])
}

/// Биоактивность 0–3: годится ли вещество в основу препарата. Правило
/// игровое: нужна углеродная основа, азот и кислород, и чем сложнее
/// молекула, тем выше.
pub fn bioactivity(formula: &Formula) -> u32 {
    let (c, n, o) = (
        count_of(formula, "C"),
        count_of(formula, "N"),
        count_of(formula, "O"),
    );
    if c < 3 || n == 0 || o == 0 {
        return 0;
    }
    match c + n + o {
        0..=5 => 1,
        6..=11 => 2,
        _ => 3,
    }
}

/// Питательность: только углерод, водород и кислород, водорода вдвое больше
/// кислорода — игровой признак сахаров.
pub fn is_nutritious(formula: &Formula) -> bool {
    let (c, h, o) = (
        count_of(formula, "C"),
        count_of(formula, "H"),
        count_of(formula, "O"),
    );
    let others: u32 = formula.iter().sum::<u32>() - c - h - o;
    others == 0 && c > 0 && o > 0 && h == o * 2
}

/// Цвет вещества в колбе по заметному элементу.
pub fn colour_of(formula: &Formula) -> u32 {
    for (symbol, colour) in [
        ("Gl", 0xF2E27A),
        ("Au", 0xE8C24A),
        ("Rs", 0xC8402E),
        ("Cu", 0x4FB8C8),
        ("Fe", 0xB06A3A),
        ("I", 0x7A4FB8),
        ("S", 0xD8C84A),
        ("P", 0xE8A050),
        ("Cl", 0xC8E86A),
        ("Si", 0x8A8A96),
        ("Na", 0xE0E0E0),
        ("K", 0xD8D0E8),
        ("Mg", 0xDCE8E0),
        ("Ca", 0xF0EBDC),
        ("C", 0x404040),
    ] {
        if count_of(formula, symbol) > 0 {
            return colour;
        }
    }
    0x9AC8E0
}

/// Известные вещества. Ключ — формула в записи движка (углерод и водород
/// впереди, остальные по алфавиту), поэтому «поваренная соль» хранится как
/// `ClNa`: игроку видно название, а не формулу.
pub const KNOWN: [(&str, &str); 34] = [
    ("H2O", "вода"),
    ("CO2", "углекислый газ"),
    ("CH4", "болотный газ"),
    ("ClNa", "поваренная соль"),
    ("INa", "йодид натрия"),
    ("O2Si", "кварц"),
    ("CCaO3", "мел"),
    ("C6H12O6", "глюкоза"),
    ("C8H10N4O2", "бодрящий алкалоид"),
    ("Gl2Rs2", "светящийся сплав"),
    ("H2O2", "перекись"),
    ("H3N", "аммиак"),
    ("HCl", "соляная кислота"),
    ("H2S", "сероводород"),
    ("H2O4S", "серная кислота"),
    ("CaO", "негашёная известь"),
    ("H2CaO2", "гашёная известь"),
    ("KNO3", "селитра"),
    ("Fe2O3", "ржавчина"),
    ("CuO", "окалина меди"),
    ("CuO4S", "медный купорос"),
    ("C2H6O", "этанол"),
    ("C3H8O3", "глицерин"),
    ("CHCl3", "хлороформ"),
    ("CH2O", "формальдегид"),
    ("C2H4O2", "уксусная кислота"),
    ("MgO", "жжёная магнезия"),
    ("ClK", "калийная соль"),
    ("HNaO", "щёлочь"),
    ("O10P4", "фосфорный ангидрид"),
    ("O2S", "сернистый газ"),
    ("AuCl3", "хлорид золота"),
    ("C2H2", "ацетилен"),
    ("Cl2Mg", "горькая соль"),
];

/// Название вещества: каноническое, если формула известна, иначе сама формула.
pub fn compound_name_of(formula: &Formula) -> String {
    let text = formula_text(formula);
    KNOWN
        .iter()
        .find(|(known, _)| *known == text)
        .map_or(text, |(_, name)| (*name).to_string())
}

/// Формула из записи (`C6H12O6`). Нужна, чтобы прочитать обратно то, что
/// лежит в колбе: в предмете хранится текст, а не массив.
pub fn formula_from_text(text: &str) -> Option<Formula> {
    let mut formula = Formula::default();
    let mut rest = text;
    while !rest.is_empty() {
        let element = ELEMENTS
            .iter()
            .filter(|e| rest.starts_with(e.symbol))
            // `Ca` длиннее `C`: берём самый длинный подходящий символ.
            .max_by_key(|e| e.symbol.len())?;
        let after = &rest[element.symbol.len()..];
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        let count: u32 = if digits.is_empty() {
            1
        } else {
            digits.parse().ok()?
        };
        let index = ELEMENTS.iter().position(|e| e.symbol == element.symbol)?;
        formula[index] += count;
        rest = &after[digits.len()..];
    }
    (formula.iter().sum::<u32>() > 0).then_some(formula)
}

/// Строки подсказки на колбе: что внутри и что с этим делать.
pub fn flask_tooltip(contents: &str, ml: i32) -> Vec<String> {
    let Some(formula) = formula_from_text(contents) else {
        // Старая колба с именем вещества вместо формулы.
        return vec![format!("{ml} мл")];
    };
    let mut lines = vec![format!("{}, {ml} мл", formula_text(&formula))];
    lines.push(
        match state_of(&formula) {
            State::Gas => "Газ",
            State::Liquid => "Жидкость",
            State::Solid => "Твёрдое",
        }
        .to_string(),
    );
    match bioactivity(&formula) {
        0 => {}
        level => lines.push(format!("Биоактивность: {level} из 3")),
    }
    if is_nutritious(&formula) {
        lines.push("Питательное".to_string());
    }
    if KNOWN.iter().all(|(known, _)| *known != contents) {
        lines.push("Не опознано".to_string());
    }
    lines
}

/// Журнал открытий: какие вещества игрок уже собирал в этом мире.
/// Строка записи — `формула<таб>имя<таб>опознано`. Старые записи из одной
/// формулы читаются как неопознанные и безымянные.
pub fn journal_path(world: &std::path::Path) -> std::path::PathBuf {
    world.join("biolab").join("journal.txt")
}

/// Запись журнала.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discovery {
    pub formula: String,
    /// Имя от спектрометра или от игрока; пусто — безымянное.
    pub name: String,
    /// Опознано ли вещество: до спектрометра свойства не подтверждены.
    pub identified: bool,
}

/// Разобрать строку журнала.
pub fn discovery_from_line(line: &str) -> Option<Discovery> {
    let mut parts = line.split('\t');
    let formula = parts.next()?.trim().to_string();
    if formula.is_empty() {
        return None;
    }
    Some(Discovery {
        formula,
        name: parts.next().unwrap_or("").trim().to_string(),
        identified: parts.next().is_some_and(|v| v.trim() == "1"),
    })
}

/// Записать строку журнала.
pub fn discovery_to_line(entry: &Discovery) -> String {
    format!("{}\t{}\t{}", entry.formula, entry.name, u8::from(entry.identified))
}

/// Все открытия этого мира.
pub fn discoveries(world: &std::path::Path) -> Vec<Discovery> {
    std::fs::read_to_string(journal_path(world))
        .unwrap_or_default()
        .lines()
        .filter_map(discovery_from_line)
        .collect()
}

/// Переписать журнал целиком.
fn write_discoveries(world: &std::path::Path, entries: &[Discovery]) {
    let path = journal_path(world);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let text: String = entries.iter().map(|e| discovery_to_line(e) + "\n").collect();
    let _ = std::fs::write(&path, text);
}

/// Записать вещество в журнал; true, если оно там впервые. Новое вещество
/// попадает туда неопознанным: свойства ещё надо подтвердить спектрометром.
pub fn remember_compound(world: &std::path::Path, contents: &str) -> bool {
    let mut entries = discoveries(world);
    if entries.iter().any(|e| e.formula == contents) {
        return false;
    }
    entries.push(Discovery {
        formula: contents.to_string(),
        name: String::new(),
        identified: false,
    });
    write_discoveries(world, &entries);
    true
}

/// Опознать вещество: запись закрывается, имя берётся из таблицы известных
/// или выдаётся по номеру открытия. None — уже было опознано.
pub fn identify_compound(world: &std::path::Path, contents: &str) -> Option<String> {
    let mut entries = discoveries(world);
    let index = entries.iter().position(|e| e.formula == contents);
    let number = index.map_or(entries.len() + 1, |i| i + 1);
    let name = KNOWN
        .iter()
        .find(|(text, _)| *text == contents)
        .map(|(_, name)| (*name).to_string())
        .unwrap_or_else(|| format!("соединение №{number}"));
    match index {
        Some(i) => {
            if entries[i].identified {
                return None;
            }
            entries[i].name = name.clone();
            entries[i].identified = true;
        }
        None => entries.push(Discovery {
            formula: contents.to_string(),
            name: name.clone(),
            identified: true,
        }),
    }
    write_discoveries(world, &entries);
    Some(name)
}

/// Дать опознанному веществу своё имя; false — такого в журнале нет или оно
/// ещё не опознано.
pub fn rename_compound(world: &std::path::Path, contents: &str, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let mut entries = discoveries(world);
    let Some(entry) = entries.iter_mut().find(|e| e.formula == contents && e.identified) else {
        return false;
    };
    entry.name = name.to_string();
    write_discoveries(world, &entries);
    true
}

/// Как называть вещество игроку: своё имя из журнала, если оно опознано.
pub fn compound_display(world: &std::path::Path, contents: &str) -> Option<String> {
    discoveries(world)
        .into_iter()
        .find(|e| e.formula == contents && e.identified && !e.name.is_empty())
        .map(|e| e.name)
}

/// Опознано ли вещество в этом мире.
pub fn is_identified(world: &std::path::Path, contents: &str) -> bool {
    discoveries(world)
        .iter()
        .any(|e| e.formula == contents && e.identified)
}

/// Сколько веществ в журнале.
pub fn journal_count(world: &std::path::Path) -> u32 {
    discoveries(world).len() as u32
}

/// Строки журнала для чата: что собрано, как названо и что ещё не опознано.
pub fn journal_lines(world: &std::path::Path) -> Vec<String> {
    let entries = discoveries(world);
    let mut lines = vec!["Лабораторный журнал".to_string()];
    if entries.is_empty() {
        lines.push("Пока пусто: соберите вещество в синтезаторе.".to_string());
        return lines;
    }
    for (i, entry) in entries.iter().enumerate() {
        let number = i + 1;
        lines.push(match (entry.identified, entry.name.is_empty()) {
            (true, false) => format!("{number}. {} — {}", entry.formula, entry.name),
            (true, true) => format!("{number}. {}", entry.formula),
            (false, _) => format!("{number}. {} — не опознано", entry.formula),
        });
    }
    lines
}

// ---- Книга-руководство ----
//
// Мод большой, и разбираться в нём по вики — не тот опыт, который хочется
// дать. Поэтому при первом входе в мир игрок получает «Журнал лаборанта» —
// карту исследований (см. «Карта исследований» ниже).

/// Книга мода: свой предмет. Открывает карту исследований — свой экран
/// в `biolab-deep` на месте ванильной книги.
pub const MANUAL_ID: &str = "biolab:lab_manual";
/// Сколько страниц бывает у одной записи.
pub const MANUAL_MAX_PAGES: usize = 100;

/// Отметить, что книга выдана; false — она уже была.
pub fn take_manual(world: &std::path::Path, uuid: u128) -> bool {
    let Ok(mut table) = states().lock() else {
        return false;
    };
    let state = table
        .entry((world.to_path_buf(), uuid))
        .or_insert_with(|| {
            std::fs::read_to_string(state_path(world, uuid))
                .map(|t| state_from_text(&t))
                .unwrap_or_default()
        });
    if state.manual {
        return false;
    }
    state.manual = true;
    let saved = *state;
    drop(table);
    let path = state_path(world, uuid);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, state_to_text(&saved));
    true
}

// ---- Разметка записей журнала ----
//
// Разметка страницы — те же текстовые данные мода, но со строками-командами:
//   `# Заголовок`      — заголовок страницы
//   `@item <id>`       — ряд иконок предметов (через пробел)
//   `@craft <файл>`    — рецепт из `data/biolab/recipe/<файл>.json`
//   прочие строки      — обычный текст

/// Что рисуется на строке страницы.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// Заголовок страницы.
    Title(String),
    /// Обычная строка.
    Text(String),
    /// Ряд иконок: предметы по их id.
    Items(Vec<String>),
    /// Рецепт: имя файла в `data/biolab/recipe` без `.json`.
    Craft(String),
    /// Пустая строка-отбивка.
    Gap,
}

/// Разобрать страницу в строки для отрисовки.
pub fn page_lines(page: &str) -> Vec<Line> {
    page.lines()
        .map(|line| {
            let line = line.trim_end();
            if let Some(title) = line.strip_prefix("# ") {
                Line::Title(title.trim().to_string())
            } else if let Some(items) = line.strip_prefix("@item ") {
                Line::Items(items.split_whitespace().map(|i| i.to_string()).collect())
            } else if let Some(item) = line.strip_prefix("@craft ") {
                Line::Craft(item.trim().to_string())
            } else if line.is_empty() {
                Line::Gap
            } else {
                Line::Text(line.to_string())
            }
        })
        .collect()
}

/// Заголовок страницы для оглавления: первый `#` или первая строка.
pub fn page_title(page: &str) -> String {
    page_lines(page)
        .into_iter()
        .find_map(|line| match line {
            Line::Title(title) => Some(title),
            Line::Text(text) => Some(text),
            _ => None,
        })
        .unwrap_or_else(|| "Страница".to_string())
}


/// Спектрометр: опознаёт вещество в колбе и печатает чертёж.
pub const SPECTROMETER: &str = "biolab:spectrometer";
/// Сколько тиков идёт анализ.
pub const ANALYSIS_TIME: i32 = 160;
/// Чертёж: предмет с формулой, по которому синтезатор собирает вещество.
pub const BLUEPRINT_ID: &str = "biolab:blueprint";

/// Подпись чертежа.
pub fn blueprint_label(contents: &str, name: Option<&str>) -> String {
    match name {
        Some(name) => format!("Чертёж: {name} ({contents})"),
        None => format!("Чертёж: {contents}"),
    }
}

/// Реактор: температура внутри и пределы работы.
pub const REACTOR: &str = "biolab:reactor";
/// Сколько градусов добавляет нагрев снизу за тик и сколько уходит само.
pub const HEAT_PER_TICK: i32 = 2;
pub const COOL_PER_TICK: i32 = 1;
/// Выше этого реактор перегревается и сбрасывает содержимое.
pub const OVERHEAT: i32 = 900;
/// Комнатная температура, ниже которой реактор не остывает.
pub const ROOM_TEMPERATURE: i32 = 20;

/// Чем греют реактор снизу и до скольки градусов. Источник не греет выше
/// своего потолка, поэтому выбор источника — это выбор режима: на магме
/// реактор работает спокойно, на лаве заходит за предел и перегревается.
pub const HEAT_SOURCES: [(&str, i32); 5] = [
    ("minecraft:campfire", 300),
    ("minecraft:soul_campfire", 350),
    ("minecraft:magma_block", 500),
    ("minecraft:fire", 700),
    ("minecraft:lava", 950),
];

/// До скольки греет этот блок; None — не греет.
pub fn heat_target(block: &str) -> Option<i32> {
    HEAT_SOURCES
        .iter()
        .find(|(id, _)| *id == block)
        .map(|(_, target)| *target)
}

/// Новая температура: ползёт к потолку источника, без источника — к
/// комнатной.
pub fn next_temperature(current: i32, target: Option<i32>) -> i32 {
    match target {
        Some(target) if current < target => (current + HEAT_PER_TICK).min(target),
        Some(target) if current > target => (current - COOL_PER_TICK).max(target),
        Some(_) => current,
        None => (current - COOL_PER_TICK).max(ROOM_TEMPERATURE),
    }
}

/// Подходит ли температура рецепту: `[min, max]` из его JSON.
pub fn temperature_fits(temperature: i32, range: Option<(i32, i32)>) -> bool {
    match range {
        None => true,
        Some((min, max)) => temperature >= min && temperature <= max,
    }
}

// ---- Псевдожидкости ----
//
// Настоящих жидкостей платформа не даёт (вода и лава зашиты в закрытый
// список), поэтому лужа — это блок со слоями, как снег. Она не течёт: стоит
// там, где разлита, действует на стоящего в ней и подсыхает с каждым
// срабатыванием. Разливается ведром или аварией реактора.

/// Лужа: блок, ведро, которым её ставят, и что она делает со стоящим в ней.
pub struct Fluid {
    pub block: &'static str,
    pub bucket: &'static str,
    /// Ванильный или наш эффект, который получает стоящий в луже.
    pub effect: &'static str,
    /// Насколько лужа поднимает заражение за срабатывание (0 — не заражает).
    pub infects: u32,
}

pub const FLUIDS: [Fluid; 4] = [
    Fluid {
        block: "biolab:acid_puddle",
        bucket: "biolab:acid_puddle_bucket",
        effect: "biolab:acid_burn",
        infects: 0,
    },
    Fluid {
        block: "biolab:sludge",
        bucket: "biolab:sludge_bucket",
        effect: "poison",
        infects: 0,
    },
    Fluid {
        block: "biolab:slime",
        bucket: "biolab:slime_bucket",
        effect: "slowness",
        infects: 4,
    },
    // Разлитый мутаген: сам по себе не травит, но переписывает геном.
    Fluid {
        block: "biolab:mutagen_puddle",
        bucket: "biolab:mutagen_bucket",
        effect: MUTAGENESIS_EFFECT,
        infects: 0,
    },
];

/// Лужа по имени блока.
pub fn fluid_of_block(block: &str) -> Option<&'static Fluid> {
    FLUIDS.iter().find(|f| f.block == block)
}

/// Лужа, которую разливает это ведро.
pub fn fluid_of_bucket(item: &str) -> Option<&'static Fluid> {
    FLUIDS.iter().find(|f| f.bucket == item)
}

/// Как часто лужа действует на стоящего в ней.
pub const FLUID_EVERY: u64 = 20;
/// Сколько слоёв у свежеразлитой лужи.
pub const FLUID_LAYERS: i32 = 5;

/// Сколько слоёв останется после срабатывания; 0 — лужа высохла.
pub fn next_layers(layers: i32) -> i32 {
    (layers - 1).max(0)
}

// ---- Фармакология ----
//
// Препарат не «срабатывает», а попадает в кровь и там живёт своей жизнью:
// всасывается, выводится по периоду полувыведения и действует только пока
// концентрация в терапевтическом окне. Ниже окна — пусто, выше — токсикоз.
// Частый приём поднимает привыкание, и окно уезжает вверх: прежняя доза
// перестаёт помогать, а двойная уже травит. Все числа игровые.

/// Своё название для эффекта: каждому — поведение (см. `biolab-deep`).
pub const TOXICOSIS_EFFECT: &str = "biolab:toxicosis";
pub const WITHDRAWAL_EFFECT: &str = "biolab:withdrawal";
pub const IMMUNITY_EFFECT: &str = "biolab:immunity";
pub const SEDATION_EFFECT: &str = "biolab:sedation";
pub const BLEEDING_EFFECT: &str = "biolab:bleeding";
pub const MUTAGENESIS_EFFECT: &str = "biolab:mutagenesis";

/// Действующее вещество: как ведёт себя в крови и что делает.
pub struct Drug {
    /// Короткое имя для файла состояния.
    pub id: &'static str,
    /// Название для анализатора.
    pub name: &'static str,
    /// Период полувыведения в шагах кинетики (`PHARMA_EVERY` тиков).
    pub half_life: u32,
    /// Терапевтическое окно концентрации: ниже — не действует, выше — травит.
    pub window: (u32, u32),
    /// Эффекты, пока концентрация в окне.
    pub effects: &'static [&'static str],
    /// Насколько сбивает заражение за шаг в окне (0 — не лечит).
    pub cures: u32,
    /// Держит ли иммунитет к болезни, пока в окне.
    pub immunity: bool,
    /// Чистит ли кровь от остальных веществ (антидот).
    pub clears: bool,
    /// Остаётся ли после него ломка.
    pub withdrawal: bool,
}

/// Сколько веществ знает мод. Порядок важен: по нему идут `PlayerState::drugs`
/// и байты сетевого пакета.
pub const DRUG_COUNT: usize = 7;

pub const DRUGS: [Drug; DRUG_COUNT] = [
    Drug {
        id: "penicillin",
        name: "Пенициллин",
        half_life: 90,
        window: (20, 90),
        effects: &[],
        cures: 4,
        immunity: false,
        clears: false,
        withdrawal: false,
    },
    Drug {
        id: "analgesic",
        name: "Обезболивающее",
        half_life: 40,
        window: (15, 70),
        effects: &["resistance"],
        cures: 0,
        immunity: false,
        clears: false,
        withdrawal: false,
    },
    Drug {
        id: "stimulant",
        name: "Стимулятор",
        half_life: 30,
        window: (10, 60),
        effects: &["speed", "haste"],
        cures: 0,
        immunity: false,
        clears: false,
        withdrawal: true,
    },
    Drug {
        id: "antidote",
        name: "Антидот",
        half_life: 20,
        window: (10, 120),
        effects: &[],
        cures: 0,
        immunity: false,
        clears: true,
        withdrawal: false,
    },
    Drug {
        id: "vaccine",
        name: "Вакцина",
        half_life: 60,
        window: (20, 100),
        effects: &[],
        cures: 0,
        immunity: true,
        clears: false,
        withdrawal: false,
    },
    Drug {
        id: "sedative",
        name: "Седативное",
        half_life: 45,
        window: (10, 50),
        effects: &[SEDATION_EFFECT, "slowness"],
        cures: 0,
        immunity: false,
        clears: false,
        withdrawal: false,
    },
    Drug {
        id: "venom",
        name: "Яд",
        half_life: 50,
        window: (5, 40),
        effects: &[TOXICOSIS_EFFECT],
        cures: 0,
        immunity: false,
        clears: false,
        withdrawal: false,
    },
];

/// Номер вещества по имени.
pub fn drug_index(id: &str) -> Option<usize> {
    DRUGS.iter().position(|d| d.id == id)
}

/// Форма препарата: от неё зависит, сколько дозы дойдёт до крови и как быстро.
pub struct Form {
    pub id: &'static str,
    pub name: &'static str,
    /// Какая доля дозы доходит до крови, в процентах.
    pub bioavailability: u32,
    /// Инъекция и аэрозоль идут прямо в кровь, таблетка и мазь — через депо.
    pub direct: bool,
}

pub const FORMS: [Form; 4] = [
    Form { id: "pill", name: "таблетка", bioavailability: 60, direct: false },
    Form { id: "injection", name: "инъекция", bioavailability: 100, direct: true },
    Form { id: "ointment", name: "мазь", bioavailability: 40, direct: false },
    Form { id: "spray", name: "аэрозоль", bioavailability: 75, direct: true },
];

pub const FORM_PILL: usize = 0;
pub const FORM_INJECTION: usize = 1;
pub const FORM_OINTMENT: usize = 2;
pub const FORM_SPRAY: usize = 3;

/// Готовый препарат: предмет, вещество в нём, форма и доза.
pub struct Medicine {
    pub item: &'static str,
    pub drug: usize,
    pub form: usize,
    pub dose: u32,
}

pub const MEDICINES: [Medicine; 12] = [
    Medicine { item: "biolab:pill_penicillin", drug: 0, form: FORM_PILL, dose: 100 },
    Medicine { item: "biolab:syringe_penicillin", drug: 0, form: FORM_INJECTION, dose: 80 },
    Medicine { item: "biolab:pill_analgesic", drug: 1, form: FORM_PILL, dose: 90 },
    Medicine { item: "biolab:ointment_analgesic", drug: 1, form: FORM_OINTMENT, dose: 120 },
    Medicine { item: "biolab:pill_stimulant", drug: 2, form: FORM_PILL, dose: 90 },
    Medicine { item: "biolab:syringe_stimulant", drug: 2, form: FORM_INJECTION, dose: 55 },
    Medicine { item: "biolab:pill_antidote", drug: 3, form: FORM_PILL, dose: 120 },
    Medicine { item: "biolab:syringe_vaccine", drug: 4, form: FORM_INJECTION, dose: 60 },
    Medicine { item: "biolab:syringe_sedative", drug: 5, form: FORM_INJECTION, dose: 40 },
    Medicine { item: "biolab:syringe_venom", drug: 6, form: FORM_INJECTION, dose: 35 },
    // Аэрозоль: действует быстрее таблетки, но дозы в баллончике меньше.
    Medicine { item: "biolab:spray_analgesic", drug: 1, form: FORM_SPRAY, dose: 60 },
    Medicine { item: "biolab:spray_stimulant", drug: 2, form: FORM_SPRAY, dose: 45 },
];

/// Дротики: те же вещества, но вводятся издали, по мобу под прицелом.
pub const DARTS: [(&str, &str); 2] = [
    ("biolab:dart_sedative", "biolab:syringe_sedative"),
    ("biolab:dart_venom", "biolab:syringe_venom"),
];

/// Дальность, на которой дротик достаёт цель.
pub const DART_RANGE: f64 = 24.0;
/// Насколько узкий у дротика прицел: косинус угла между взглядом и целью.
pub const DART_AIM: f64 = 0.96;

/// Чем снаряжён дротик: шприц с тем же веществом.
pub fn dart_syringe(item: &str) -> Option<&'static str> {
    DARTS
        .iter()
        .find(|(dart, _)| *dart == item)
        .map(|(_, syringe)| *syringe)
}

/// Пустой шприц: остаётся в руке после инъекции.
pub const SYRINGE_EMPTY: &str = "biolab:syringe";

/// Препарат по предмету.
pub fn medicine(item: &str) -> Option<&'static Medicine> {
    MEDICINES.iter().find(|m| m.item == item)
}

/// Кровь по одному веществу.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrugState {
    /// Непоглощённое: таблетка и мазь отдают дозу не сразу.
    pub depot: u32,
    /// Концентрация в крови.
    pub blood: u32,
    /// Привыкание 0–`TOLERANCE_MAX`: поднимает всё окно.
    pub tolerance: u32,
    /// Было ли вещество в окне на прошлом шаге: по спаду начинается ломка.
    pub was_active: bool,
}

/// Как часто идёт шаг кинетики, в тиках (раз в секунду).
pub const PHARMA_EVERY: u64 = 20;
/// Какая доля депо всасывается за шаг, в процентах.
pub const ABSORB_PERCENT: u32 = 25;
/// Сколько привыкания добавляет один приём и где его потолок.
pub const TOLERANCE_PER_DOSE: u32 = 6;
pub const TOLERANCE_MAX: u32 = 60;
/// Раз в сколько шагов привыкание спадает на единицу.
pub const TOLERANCE_EVERY: u64 = 30;
/// Какую долю чужих веществ убирает антидот за шаг, в процентах.
pub const ANTIDOTE_CLEAR: u32 = 30;
/// Сколько тиков держится иммунитет после вакцины (20 минут).
pub const IMMUNITY_TICKS: u32 = 24_000;
/// Сколько тиков держится ломка (15 секунд).
pub const WITHDRAWAL_TICKS: u32 = 300;
/// Насколько долго висят эффекты, которые подновляет шаг кинетики.
pub const PHARMA_EFFECT_TICKS: i32 = 60;

/// Где концентрация относительно окна.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// Вещества в крови нет.
    Absent,
    /// Есть, но до окна не дотягивает.
    Subtherapeutic,
    /// В окне: препарат работает.
    Therapeutic,
    /// Выше окна: побочки и урон.
    Overdose,
}

/// Окно с поправкой на привыкание: обе границы уезжают вверх.
pub fn window_with_tolerance(window: (u32, u32), tolerance: u32) -> (u32, u32) {
    let t = tolerance.min(TOLERANCE_MAX);
    let (min, max) = window;
    (min + min * t / 100, max + max * t / 100)
}

/// Где находится вещество по своему окну.
pub fn level_of(drug: &Drug, state: &DrugState) -> Level {
    let (min, max) = window_with_tolerance(drug.window, state.tolerance);
    match state.blood {
        0 => Level::Absent,
        blood if blood < min => Level::Subtherapeutic,
        blood if blood <= max => Level::Therapeutic,
        _ => Level::Overdose,
    }
}

/// Название уровня для анализатора.
pub fn level_text(level: Level) -> &'static str {
    match level {
        Level::Absent => "нет",
        Level::Subtherapeutic => "мало",
        Level::Therapeutic => "в окне",
        Level::Overdose => "перебор",
    }
}

/// Сколько уйдёт из депо в кровь за шаг.
pub fn absorb(depot: u32) -> u32 {
    if depot == 0 {
        return 0;
    }
    (depot * ABSORB_PERCENT / 100).clamp(1, depot)
}

/// Концентрация после одного шага выведения. За `half_life` шагов её остаётся
/// половина: доля ухода за шаг — это ln2 / период, то есть 0,693.
pub fn decay(blood: u32, half_life: u32) -> u32 {
    if blood == 0 {
        return 0;
    }
    let drop = (blood * 693).div_ceil(1000 * half_life.max(1)).max(1);
    blood.saturating_sub(drop)
}

/// Насколько сильный токсикоз даёт перебор: ноль, пока концентрация в окне.
pub fn overdose_amplifier(drug: &Drug, state: &DrugState) -> i32 {
    let (_, max) = window_with_tolerance(drug.window, state.tolerance);
    if max == 0 || state.blood <= max {
        return 0;
    }
    (((state.blood - max) * 3 / max) as i32).clamp(0, 3)
}

/// Приём препарата: доза уходит в кровь или в депо, привыкание растёт.
pub fn take_dose(state: &mut PlayerState, med: &Medicine) {
    let form = &FORMS[med.form];
    let amount = med.dose * form.bioavailability / 100;
    let drug = &mut state.drugs[med.drug];
    if form.direct {
        drug.blood += amount;
    } else {
        drug.depot += amount;
    }
    drug.tolerance = (drug.tolerance + TOLERANCE_PER_DOSE).min(TOLERANCE_MAX);
}

/// Что фармакология просит сделать на этом шаге.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pharma {
    /// Эффекты, которые нужно подновить на игроке: имя и уровень.
    pub effects: Vec<(&'static str, i32)>,
    /// Насколько сбить заражение.
    pub cured: u32,
    /// Держится ли иммунитет к болезни.
    pub immune: bool,
    /// Курс прерван, и штамм только что стал устойчивее.
    pub hardened: bool,
}

/// Шаг кинетики: всасывание, выведение, привыкание, иммунитет и ломка.
/// Меняет состояние и возвращает, что повесить на игрока.
pub fn pharma_step(state: &mut PlayerState) -> Pharma {
    let mut out = Pharma::default();
    // Антидот работает до остальных: он снижает их концентрацию на этом же
    // шаге, иначе игрок успел бы получить передозировку, уже приняв противоядие.
    let clearing = DRUGS
        .iter()
        .zip(&state.drugs)
        .any(|(drug, s)| drug.clears && level_of(drug, s) == Level::Therapeutic);

    for (i, drug) in DRUGS.iter().enumerate() {
        let mut s = state.drugs[i];
        let moved = absorb(s.depot);
        s.depot -= moved;
        s.blood += moved;
        s.blood = decay(s.blood, drug.half_life);
        if clearing && !drug.clears {
            s.blood -= s.blood * ANTIDOTE_CLEAR / 100;
            s.depot -= s.depot * ANTIDOTE_CLEAR / 100;
        }
        if state.ticks % (PHARMA_EVERY * TOLERANCE_EVERY) == 0 {
            s.tolerance = s.tolerance.saturating_sub(1);
        }

        let level = level_of(drug, &s);
        if level == Level::Therapeutic || level == Level::Overdose {
            for name in drug.effects {
                out.effects.push((name, 0));
            }
        }
        match level {
            Level::Therapeutic => {
                out.cured += drug.cures;
                if drug.immunity {
                    out.immune = true;
                }
            }
            Level::Overdose => out
                .effects
                .push((TOXICOSIS_EFFECT, overdose_amplifier(drug, &s))),
            _ => {}
        }

        let active = level == Level::Therapeutic;
        let fell_off = s.was_active && !active;
        s.was_active = active;
        state.drugs[i] = s;
        if fell_off && drug.withdrawal {
            state.withdrawal = WITHDRAWAL_TICKS;
        }
    }

    // Вакцина держит иммунитет, пока в окне; дальше он тает сам.
    if out.immune {
        state.immunity = IMMUNITY_TICKS;
    } else {
        state.immunity = state.immunity.saturating_sub(PHARMA_EVERY as u32);
    }
    if state.immunity > 0 {
        out.immune = true;
        out.effects.push((IMMUNITY_EFFECT, 0));
    }
    if state.withdrawal > 0 {
        state.withdrawal = state.withdrawal.saturating_sub(PHARMA_EVERY as u32);
        out.effects.push((WITHDRAWAL_EFFECT, 0));
        out.effects.push(("weakness", 0));
        out.effects.push(("slowness", 0));
    }
    out
}

/// Иммунитет не даёт заражению расти.
pub fn after_exposure_with(infection: u32, amount: u32, immune: bool) -> u32 {
    if immune {
        infection
    } else {
        after_exposure(infection, amount)
    }
}

/// Что шприц делает мобу: эффекты, длительность и уровень. Седативное вешает
/// ещё и ванильное замедление — своя седация только отбивает агрессию.
pub fn mob_injection(item: &str) -> Option<(&'static [&'static str], i32, i32)> {
    match item {
        "biolab:syringe_venom" => Some((&[TOXICOSIS_EFFECT], 400, 1)),
        "biolab:syringe_sedative" => Some((&[SEDATION_EFFECT, "slowness"], 600, 0)),
        "biolab:syringe_penicillin" => Some((&["regeneration"], 200, 0)),
        "biolab:syringe_mutagen" => Some((&[MUTAGENESIS_EFFECT], 400, 0)),
        _ => None,
    }
}

/// Состояние для анализатора: заражение, журнал и кровь по всем веществам.
/// Байты считает сервер, рисует клиент — в одиночной игре это один процесс,
/// но канал тот же, что был бы по сети.
pub fn pharma_packet(state: &PlayerState, journal: u32) -> Vec<u8> {
    let mut data = vec![state.infection.min(100) as u8, journal.min(255) as u8];
    for drug in &state.drugs {
        data.push(drug.blood.min(255) as u8);
    }
    for drug in &state.drugs {
        data.push(drug.tolerance.min(255) as u8);
    }
    // Хвост: штамм болезни, чтобы анализатор показал её профиль.
    data.extend_from_slice(&state.strain);
    data
}

/// Штамм из пакета; его может не быть — тогда линия дикая.
pub fn strain_from_packet(data: &[u8]) -> Strain {
    let tail = 2 + DRUG_COUNT * 2;
    let mut values = [0; 4];
    for (i, value) in values.iter_mut().enumerate() {
        *value = i32::from(data.get(tail + i).copied().unwrap_or(0));
    }
    if values[0] == 0 {
        return WILD;
    }
    Strain::of(values)
}

/// Штамм, присланный сервером.
fn known_strain() -> &'static std::sync::Mutex<Strain> {
    static VALUE: std::sync::OnceLock<std::sync::Mutex<Strain>> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| std::sync::Mutex::new(WILD))
}

pub fn remember_strain(strain: Strain) {
    if let Ok(mut value) = known_strain().lock() {
        *value = strain;
    }
}

pub fn shown_strain() -> Strain {
    known_strain().lock().map(|v| *v).unwrap_or(WILD)
}

/// Кровь из пакета. Короткий пакет — от старой версии: что не пришло, то нули.
pub fn pharma_from_packet(data: &[u8]) -> [DrugState; DRUG_COUNT] {
    let mut drugs = [DrugState::default(); DRUG_COUNT];
    for (i, drug) in drugs.iter_mut().enumerate() {
        drug.blood = data.get(2 + i).copied().unwrap_or(0) as u32;
        drug.tolerance = data.get(2 + DRUG_COUNT + i).copied().unwrap_or(0) as u32;
    }
    drugs
}

/// Строки анализатора про кровь: только то, что в ней есть.
pub fn blood_lines(drugs: &[DrugState; DRUG_COUNT]) -> Vec<String> {
    let mut lines = Vec::new();
    for (drug, state) in DRUGS.iter().zip(drugs) {
        if state.blood == 0 {
            continue;
        }
        let mut line = format!(
            "{}: {} ({})",
            drug.name,
            state.blood,
            level_text(level_of(drug, state))
        );
        if state.tolerance > 0 {
            line.push_str(&format!(", привык. {}%", state.tolerance));
        }
        lines.push(line);
    }
    lines
}

/// Принять препарат: доза уходит в кровь или в депо и состояние сразу
/// пишется на диск — приём редкое событие, ждать автосохранения незачем.
pub fn take_medicine(world: &std::path::Path, uuid: u128, med: &Medicine) -> Option<PlayerState> {
    let mut table = states().lock().ok()?;
    let state = table
        .entry((world.to_path_buf(), uuid))
        .or_insert_with(|| {
            std::fs::read_to_string(state_path(world, uuid))
                .map(|t| state_from_text(&t))
                .unwrap_or_default()
        });
    take_dose(state, med);
    let saved = *state;
    let path = state_path(world, uuid);
    drop(table);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, state_to_text(&saved));
    Some(saved)
}

/// Шаг кинетики на состоянии игрока в этом мире: что повесить и что стало с
/// заражением. На диск не пишем — это делает автосохранение в `player_tick`.
/// Здесь же ловится сорванный курс: если лекарство вышло из крови, а болезнь
/// осталась, штамм становится устойчивее.
pub fn pharma_tick(world: &std::path::Path, uuid: u128) -> Option<(Pharma, PlayerState)> {
    let mut table = states().lock().ok()?;
    let state = table.get_mut(&(world.to_path_buf(), uuid))?;
    let before = level_of(&DRUGS[COURSE_DRUG], &state.drugs[COURSE_DRUG]);
    let pharma = pharma_step(state);
    if pharma.cured > 0 {
        state.infection = state.infection.saturating_sub(pharma.cured);
    }
    let after = level_of(&DRUGS[COURSE_DRUG], &state.drugs[COURSE_DRUG]);
    let mut hardened = false;
    if course_broken(state, before, after)
        && let Some(strain) = player_strain(state)
    {
        let harder = harden(&strain);
        if harder != strain {
            set_player_strain(state, &harder);
            hardened = true;
        }
    }
    let mut pharma = pharma;
    pharma.hardened = hardened;
    Some((pharma, *state))
}

/// Сообщение после приёма: что принято и где теперь концентрация.
pub fn dose_message(med: &Medicine, state: &PlayerState) -> String {
    let drug = &DRUGS[med.drug];
    let form = &FORMS[med.form];
    let s = &state.drugs[med.drug];
    let level = level_of(drug, s);
    format!(
        "{} ({}): в крови {}, {}",
        drug.name,
        form.name,
        s.blood,
        level_text(level)
    )
}

/// Что остаётся в руке после приёма: от шприца — пустой шприц.
pub fn after_use(item: &str) -> Option<&'static str> {
    item.starts_with("biolab:syringe_").then_some(SYRINGE_EMPTY)
}

/// Сообщение после инъекции мобу.
pub fn injection_message(item: &str, mob: &str) -> String {
    let what = MEDICINES
        .iter()
        .find(|m| m.item == item)
        .map(|m| DRUGS[m.drug].name)
        .unwrap_or(if item == "biolab:syringe_mutagen" {
            "Мутаген"
        } else {
            "Препарат"
        });
    let target = mob.strip_prefix("minecraft:").unwrap_or(mob);
    format!("{what} введён: {target}")
}

/// Инъекция мобу и приём внутрь ловят один и тот же клик: хук сущности
/// забирает его первым, но хук предмета всё равно срабатывает — проверено в
/// игре, шприц яда доставался и корове, и самому игроку. Поэтому помним
/// момент инъекции и пропускаем приём, если он пришёл сразу следом.
fn last_injection() -> &'static std::sync::atomic::AtomicU64 {
    static AT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
    &AT
}

/// Запомнить, что шприц только что ушёл мобу.
pub fn remember_injection(ticks: u64) {
    last_injection().store(ticks, std::sync::atomic::Ordering::Relaxed);
}

/// Был ли только что укол мобу: тот же тик или соседний.
pub fn just_injected(ticks: u64) -> bool {
    match last_injection().load(std::sync::atomic::Ordering::Relaxed) {
        u64::MAX => false,
        at => ticks.saturating_sub(at) <= 1,
    }
}

// ---- Генетика ----
//
// Ген — признак с силой 1–3. У мобов он меняет характеристики, вид и
// поведение (это делает `biolab-deep`), у игрока — даёт постоянный эффект и
// свою побочку. Сам ген живёт в предмете-фрагменте, в мобе — меткой
// (модификатором атрибута с именем гена), а у игрока — в состоянии мира.
//
// Имена генов, их цена и формат метки — общий контракт с `biolab-deep`:
// менять их можно только в обоих бандлах сразу, иначе мутанты из старых
// миров перестанут узнаваться.

/// Ген глазами игрока: цена стабильности и что он даёт.
pub struct GeneKind {
    pub name: &'static str,
    pub title: &'static str,
    /// Цена за единицу силы, в очках стабильности.
    pub cost: i32,
    /// Что ген делает у мутанта.
    pub about: &'static str,
    /// Эффект, который ген даёт человеку. None — в человеке не приживается.
    pub player: Option<&'static str>,
    /// Чем человек за него платит.
    pub side: Option<&'static str>,
}

pub const GENES: [GeneKind; 32] = [
    GeneKind { name: "gigantism", title: "Гигантизм", cost: 2, about: "крупнее, живучее и сильнее", player: Some("health_boost"), side: Some("slowness") },
    GeneKind { name: "dwarfism", title: "Карликовость", cost: 1, about: "мельче и шустрее, но слабее", player: Some("speed"), side: Some("weakness") },
    GeneKind { name: "hypermetabolism", title: "Гиперметаболизм", cost: 2, about: "быстрее и злее, но хрупче", player: Some("speed"), side: Some("hunger") },
    GeneKind { name: "chitin", title: "Хитиновый панцирь", cost: 2, about: "броня и устойчивость к отбрасыванию", player: Some("resistance"), side: Some("slowness") },
    GeneKind { name: "dense_bone", title: "Плотная кость", cost: 2, about: "тяжёлый: с места не сдвинуть", player: Some("resistance"), side: Some("mining_fatigue") },
    GeneKind { name: "brute_force", title: "Грубая сила", cost: 3, about: "бьёт заметно больнее", player: Some("strength"), side: Some("slowness") },
    GeneKind { name: "sinew", title: "Жилистость", cost: 2, about: "выносливее без потери скорости", player: Some("health_boost"), side: None },
    GeneKind { name: "spring_legs", title: "Пружинные ноги", cost: 1, about: "прыгает выше", player: Some("jump_boost"), side: None },
    GeneKind { name: "thin_skin", title: "Тонкая кожа", cost: 1, about: "хрупкий, зато вёрткий", player: Some("speed"), side: Some("weakness") },
    GeneKind { name: "bioluminescence", title: "Биолюминесценция", cost: 1, about: "светится ядовито-зелёным", player: Some("glowing"), side: None },
    GeneKind { name: "albinism", title: "Альбинизм", cost: 1, about: "белый как мел", player: Some("night_vision"), side: None },
    GeneKind { name: "melanism", title: "Меланизм", cost: 1, about: "угольно-чёрный, в темноте не видно", player: Some("invisibility"), side: Some("blindness") },
    GeneKind { name: "iridescence", title: "Радужная чешуя", cost: 2, about: "отливает синим, сбивает прицел", player: Some("speed"), side: None },
    GeneKind { name: "rust_hide", title: "Ржавая шкура", cost: 1, about: "цвета окалины, чуть крепче", player: Some("resistance"), side: None },
    GeneKind { name: "pallor", title: "Бледность", cost: 1, about: "обескровленный вид, слаб днём", player: None, side: None },
    GeneKind { name: "violet_blood", title: "Фиолетовая кровь", cost: 2, about: "необычная биохимия", player: Some("regeneration"), side: Some("nausea") },
    GeneKind { name: "regeneration", title: "Регенерация", cost: 3, about: "затягивает раны на ходу", player: Some("regeneration"), side: Some("hunger") },
    GeneKind { name: "thermal", title: "Термостойкость", cost: 2, about: "не горит", player: Some("fire_resistance"), side: Some("mining_fatigue") },
    GeneKind { name: "cryophilia", title: "Холодолюбивость", cost: 2, about: "вокруг него стынет воздух", player: Some("resistance"), side: Some("slowness") },
    GeneKind { name: "photophobia", title: "Светобоязнь", cost: 1, about: "на свету мечется и бежит в тень", player: None, side: None },
    GeneKind { name: "aquatic", title: "Жаберные складки", cost: 2, about: "дышит под водой", player: Some("water_breathing"), side: None },
    GeneKind { name: "toxin_tolerance", title: "Стойкость к ядам", cost: 2, about: "яды на него не действуют", player: Some("resistance"), side: None },
    GeneKind { name: "acid_glands", title: "Кислотные железы", cost: 3, about: "кислотная аура вокруг", player: None, side: None },
    GeneKind { name: "volatile", title: "Нестабильный геном", cost: 3, about: "после смерти остаётся лужа кислоты", player: None, side: None },
    GeneKind { name: "necrosis", title: "Некроз", cost: 2, about: "после смерти остаётся заразная слизь", player: None, side: None },
    GeneKind { name: "toxic_blood", title: "Ядовитая кровь", cost: 2, about: "убийца получает яд", player: None, side: None },
    GeneKind { name: "thorns", title: "Костяные шипы", cost: 3, about: "ранит того, кто его бьёт", player: None, side: None },
    GeneKind { name: "healing_aura", title: "Лечащая аура", cost: 3, about: "лечит мутантов вокруг", player: None, side: None },
    GeneKind { name: "predator", title: "Хищник", cost: 2, about: "охотится на мирных животных", player: None, side: None },
    GeneKind { name: "pack_instinct", title: "Стайность", cost: 2, about: "сильнее, когда рядом свои", player: None, side: None },
    GeneKind { name: "loyalty", title: "Лояльность", cost: 2, about: "не нападает на людей", player: None, side: None },
    GeneKind { name: "rending_claws", title: "Рвущие когти", cost: 3, about: "раны от него кровоточат", player: None, side: None },
];

/// Сколько генов уживается в одном организме и какую сумму цен он выдержит.
pub const MAX_GENES: usize = 3;
pub const STABILITY: i32 = 6;

/// Метки генов на мобе: модификатор дальности преследования с нулевой
/// прибавкой, чьё имя и есть ген. Тот же список, что в `biolab-deep`.
pub const GENE_MARKERS: [&str; 96] = [
    "biolab:gene/gigantism/1",
    "biolab:gene/gigantism/2",
    "biolab:gene/gigantism/3",
    "biolab:gene/dwarfism/1",
    "biolab:gene/dwarfism/2",
    "biolab:gene/dwarfism/3",
    "biolab:gene/hypermetabolism/1",
    "biolab:gene/hypermetabolism/2",
    "biolab:gene/hypermetabolism/3",
    "biolab:gene/chitin/1",
    "biolab:gene/chitin/2",
    "biolab:gene/chitin/3",
    "biolab:gene/dense_bone/1",
    "biolab:gene/dense_bone/2",
    "biolab:gene/dense_bone/3",
    "biolab:gene/brute_force/1",
    "biolab:gene/brute_force/2",
    "biolab:gene/brute_force/3",
    "biolab:gene/sinew/1",
    "biolab:gene/sinew/2",
    "biolab:gene/sinew/3",
    "biolab:gene/spring_legs/1",
    "biolab:gene/spring_legs/2",
    "biolab:gene/spring_legs/3",
    "biolab:gene/thin_skin/1",
    "biolab:gene/thin_skin/2",
    "biolab:gene/thin_skin/3",
    "biolab:gene/bioluminescence/1",
    "biolab:gene/bioluminescence/2",
    "biolab:gene/bioluminescence/3",
    "biolab:gene/albinism/1",
    "biolab:gene/albinism/2",
    "biolab:gene/albinism/3",
    "biolab:gene/melanism/1",
    "biolab:gene/melanism/2",
    "biolab:gene/melanism/3",
    "biolab:gene/iridescence/1",
    "biolab:gene/iridescence/2",
    "biolab:gene/iridescence/3",
    "biolab:gene/rust_hide/1",
    "biolab:gene/rust_hide/2",
    "biolab:gene/rust_hide/3",
    "biolab:gene/pallor/1",
    "biolab:gene/pallor/2",
    "biolab:gene/pallor/3",
    "biolab:gene/violet_blood/1",
    "biolab:gene/violet_blood/2",
    "biolab:gene/violet_blood/3",
    "biolab:gene/regeneration/1",
    "biolab:gene/regeneration/2",
    "biolab:gene/regeneration/3",
    "biolab:gene/thermal/1",
    "biolab:gene/thermal/2",
    "biolab:gene/thermal/3",
    "biolab:gene/cryophilia/1",
    "biolab:gene/cryophilia/2",
    "biolab:gene/cryophilia/3",
    "biolab:gene/photophobia/1",
    "biolab:gene/photophobia/2",
    "biolab:gene/photophobia/3",
    "biolab:gene/aquatic/1",
    "biolab:gene/aquatic/2",
    "biolab:gene/aquatic/3",
    "biolab:gene/toxin_tolerance/1",
    "biolab:gene/toxin_tolerance/2",
    "biolab:gene/toxin_tolerance/3",
    "biolab:gene/acid_glands/1",
    "biolab:gene/acid_glands/2",
    "biolab:gene/acid_glands/3",
    "biolab:gene/volatile/1",
    "biolab:gene/volatile/2",
    "biolab:gene/volatile/3",
    "biolab:gene/necrosis/1",
    "biolab:gene/necrosis/2",
    "biolab:gene/necrosis/3",
    "biolab:gene/toxic_blood/1",
    "biolab:gene/toxic_blood/2",
    "biolab:gene/toxic_blood/3",
    "biolab:gene/thorns/1",
    "biolab:gene/thorns/2",
    "biolab:gene/thorns/3",
    "biolab:gene/healing_aura/1",
    "biolab:gene/healing_aura/2",
    "biolab:gene/healing_aura/3",
    "biolab:gene/predator/1",
    "biolab:gene/predator/2",
    "biolab:gene/predator/3",
    "biolab:gene/pack_instinct/1",
    "biolab:gene/pack_instinct/2",
    "biolab:gene/pack_instinct/3",
    "biolab:gene/loyalty/1",
    "biolab:gene/loyalty/2",
    "biolab:gene/loyalty/3",
    "biolab:gene/rending_claws/1",
    "biolab:gene/rending_claws/2",
    "biolab:gene/rending_claws/3",
];

/// Ген по имени.
pub fn gene(name: &str) -> Option<&'static GeneKind> {
    GENES.iter().find(|g| g.name == name)
}

/// Метка для гена такой силы.
pub fn marker_for(name: &str, strength: i32) -> Option<&'static str> {
    let index = GENES.iter().position(|g| g.name == name)?;
    (1..=3)
        .contains(&strength)
        .then(|| GENE_MARKERS.get(index * 3 + (strength - 1) as usize).copied())
        .flatten()
}

/// Ген по метке: `biolab:gene/chitin/2`.
pub fn gene_of_marker(marker: &str) -> Option<(&'static str, i32)> {
    let rest = marker.strip_prefix("biolab:gene/")?;
    let (name, strength) = rest.rsplit_once('/')?;
    let kind = gene(name)?;
    let strength = strength.parse::<i32>().ok()?;
    (1..=3).contains(&strength).then_some((kind.name, strength))
}

/// Предметы генетики.
pub const TISSUE_ID: &str = "biolab:mutant_tissue";
pub const FRAGMENT_ID: &str = "biolab:dna_fragment";
pub const MICROSCOPE_ID: &str = "biolab:microscope";
pub const BROTH_ID: &str = "biolab:nutrient_broth";
/// Ключ, под которым ген лежит в `custom_data` фрагмента.
pub const GENE_KEY: &str = "biolab_gene";

/// Ген строкой для предмета: `chitin:2`.
pub fn gene_text(name: &str, strength: i32) -> String {
    format!("{name}:{strength}")
}

/// Ген из строки предмета.
pub fn gene_from_text(text: &str) -> Option<(&'static str, i32)> {
    let (name, strength) = text.split_once(':')?;
    let kind = gene(name)?;
    let strength = strength.parse::<i32>().ok()?;
    (1..=3).contains(&strength).then_some((kind.name, strength))
}

/// Подпись фрагмента в инвентаре.
pub fn fragment_label(text: &str) -> String {
    match gene_from_text(text) {
        Some((name, strength)) => {
            let kind = gene(name).expect("ген только что нашёлся");
            format!("Фрагмент ДНК: {} {}", kind.title, "+".repeat(strength as usize))
        }
        None => "Фрагмент ДНК: пустой".to_string(),
    }
}

/// Подсказка на фрагменте: что это за ген и чего он стоит.
pub fn fragment_tooltip(text: &str) -> Vec<String> {
    let Some((name, strength)) = gene_from_text(text) else {
        return vec!["Ген не прочитан".to_string()];
    };
    let kind = gene(name).expect("ген только что нашёлся");
    let mut lines = vec![
        format!("{}, сила {strength}", kind.title),
        format!("У мутанта: {}", kind.about),
        format!("Стабильность: {}", kind.cost * strength),
    ];
    lines.push(match (kind.player, kind.side) {
        (None, _) => "В человеке не приживается".to_string(),
        (Some(_), Some(side)) => format!("Человеку: с побочкой ({side})"),
        (Some(_), None) => "Человеку: без побочек".to_string(),
    });
    lines
}

/// Какой ген достанется из образца. Детерминированно от зерна, чтобы прогон
/// повторялся: зерно даёт машина (позиция и тик).
pub fn roll_gene(seed: u64) -> (&'static str, i32) {
    let kind = &GENES[(seed % GENES.len() as u64) as usize];
    // Дорогой ген не бывает сильным: иначе фрагмент некуда применить.
    let strength = (1 + ((seed / 7) % 3) as i32).min(STABILITY / kind.cost);
    (kind.name, strength.max(1))
}

/// Строки микроскопа: геном моба по его меткам.
pub fn genome_lines(genes: &[(&'static str, i32)]) -> Vec<String> {
    if genes.is_empty() {
        return vec!["Обычный организм, мутаций нет".to_string()];
    }
    let mut lines = vec!["Геном:".to_string()];
    let mut total = 0;
    for (name, strength) in genes {
        let Some(kind) = gene(name) else { continue };
        total += kind.cost * strength;
        lines.push(format!("  {} {} — {}", kind.title, "+".repeat(*strength as usize), kind.about));
    }
    lines.push(format!("Стабильность: {total} из {STABILITY}"));
    lines
}

/// Чем закончилась попытка вживить ген.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Splice {
    /// Ген принят.
    Done,
    /// Ген в человеке не приживается (у мобов работает).
    NotForHumans,
    /// Перебор по стабильности: организм такого не выдержит.
    Unstable,
    /// Мест под гены больше нет.
    NoRoom,
    /// Этот ген уже есть, и не слабее.
    Already,
}

/// Гены игрока: имя и сила, по порядку таблицы.
pub fn player_genes(state: &PlayerState) -> Vec<(&'static str, i32)> {
    GENES
        .iter()
        .enumerate()
        .filter_map(|(i, kind)| match state.genes[i] {
            0 => None,
            strength => Some((kind.name, i32::from(strength))),
        })
        .collect()
}

/// Во что обходится геном игрока.
pub fn player_stability(state: &PlayerState) -> i32 {
    player_genes(state)
        .iter()
        .filter_map(|(name, strength)| Some(gene(name)?.cost * strength))
        .sum()
}

/// Генотерапия: вживить себе ген.
pub fn splice_player(state: &mut PlayerState, name: &str, strength: i32) -> Splice {
    let Some(index) = GENES.iter().position(|g| g.name == name) else {
        return Splice::NotForHumans;
    };
    let kind = &GENES[index];
    if kind.player.is_none() {
        return Splice::NotForHumans;
    }
    let have = i32::from(state.genes[index]);
    if have >= strength {
        return Splice::Already;
    }
    if have == 0 && player_genes(state).len() >= MAX_GENES {
        return Splice::NoRoom;
    }
    let mut next = *state;
    next.genes[index] = strength.clamp(1, 3) as u8;
    if player_stability(&next) > STABILITY {
        return Splice::Unstable;
    }
    *state = next;
    Splice::Done
}

/// Сообщение о генотерапии.
pub fn splice_message(result: Splice, name: &str, strength: i32) -> String {
    let title = gene(name).map_or("Ген", |k| k.title);
    match result {
        Splice::Done => format!("{title} вживлён, сила {strength}"),
        Splice::NotForHumans => format!("{title} в человеке не приживается"),
        Splice::Unstable => "Организм не выдержит: перебор по стабильности".to_string(),
        Splice::NoRoom => format!("Больше {MAX_GENES} генов человек не несёт"),
        Splice::Already => format!("{title} уже вживлён, и не слабее"),
    }
}

/// Эффекты генов игрока: что держать на нём и чем он за это платит.
pub fn gene_effects(state: &PlayerState) -> Vec<(&'static str, i32)> {
    let mut out = Vec::new();
    for (name, strength) in player_genes(state) {
        let Some(kind) = gene(name) else { continue };
        if let Some(effect) = kind.player {
            out.push((effect, strength - 1));
        }
        // Побочка слабее самого гена: платить, но не разориться.
        if let Some(side) = kind.side {
            out.push((side, 0));
        }
    }
    out
}

/// Клонирующий чан: фрагмент ДНК плюс питательный раствор — и в мир выходит
/// мутант с заданным геном.
pub const CLONING_VAT: &str = "biolab:cloning_vat";
/// Сколько тиков растёт клон.
pub const CLONE_TIME: i32 = 400;
/// Кого выращивает чан. Своих видов платформа не даёт, поэтому зомби.
pub const CLONE_SPECIES: &str = "minecraft:zombie";
/// Сколько раствора уходит на клона.
pub const CLONE_BROTH: u32 = 1;

/// Можно ли запускать чан: нужен фрагмент с геном и раствор.
pub fn vat_ready(input: Option<&str>, fuel: Option<(&str, u32)>) -> bool {
    input.and_then(gene_from_text).is_some()
        && fuel.is_some_and(|(item, count)| item == BROTH_ID && count >= CLONE_BROTH)
}

/// Экстрактор ДНК: из мутантной ткани выходит фрагмент.
pub const EXTRACTOR: &str = "biolab:extractor";

/// Примет ли моб ещё один ген: те же правила, что у человека, только гены,
/// не приживающиеся в человеке, мобу как раз и предназначены.
pub fn splice_mob(have: &[(&'static str, i32)], name: &str, strength: i32) -> Splice {
    let Some(kind) = gene(name) else {
        return Splice::NotForHumans;
    };
    let current = have
        .iter()
        .find(|(id, _)| *id == kind.name)
        .map(|(_, s)| *s)
        .unwrap_or(0);
    if current >= strength {
        return Splice::Already;
    }
    if current == 0 && have.len() >= MAX_GENES {
        return Splice::NoRoom;
    }
    let total: i32 = have
        .iter()
        .filter(|(id, _)| *id != kind.name)
        .filter_map(|(id, s)| Some(gene(id)?.cost * s))
        .sum();
    if total + kind.cost * strength > STABILITY {
        return Splice::Unstable;
    }
    Splice::Done
}

/// Генотерапия игрока в этом мире: меняет состояние и сразу пишет его на диск.
pub fn splice_world_player(
    world: &std::path::Path,
    uuid: u128,
    name: &str,
    strength: i32,
) -> Option<Splice> {
    let mut table = states().lock().ok()?;
    let state = table
        .entry((world.to_path_buf(), uuid))
        .or_insert_with(|| {
            std::fs::read_to_string(state_path(world, uuid))
                .map(|t| state_from_text(&t))
                .unwrap_or_default()
        });
    let result = splice_player(state, name, strength);
    let saved = *state;
    drop(table);
    if result == Splice::Done {
        let path = state_path(world, uuid);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, state_to_text(&saved));
    }
    Some(result)
}

/// Сколько ткани падает с мутанта: по куску за ген.
pub fn tissue_drop(genes: usize) -> u32 {
    genes.min(3) as u32
}

/// Радиус кислотной ауры по метке гена: железы жгут всех вокруг.
/// Тот же расчёт, что в `biolab-deep` (`mutants::acid_aura`).
pub fn aura_radius(marker: &str) -> Option<f64> {
    match gene_of_marker(marker) {
        Some(("acid_glands", strength)) => Some(2.0 + f64::from(strength)),
        _ => None,
    }
}

/// Что сказать игроку, когда с мутанта достали ткань.
pub fn tissue_message(count: u32, genes: &[(&'static str, i32)]) -> String {
    let names: Vec<&str> = genes
        .iter()
        .filter_map(|(name, _)| Some(gene(name)?.title))
        .collect();
    if count == 0 {
        return "Мутантная ткань пропала: рюкзак полон".to_string();
    }
    format!("Мутантная ткань ×{count} ({})", names.join(", "))
}

/// Модификаторы, которыми геном меняет атрибуты мутанта. Ставит их
/// `biolab-deep`, а здесь список нужен, чтобы снять их с клона: свежему мобу
/// игра успевает выдать случайный геном раньше, чем чан впишет заказанный.
pub const STAT_MODIFIERS: [&str; 5] = [
    "biolab:genome_health",
    "biolab:genome_speed",
    "biolab:genome_damage",
    "biolab:genome_armor",
    "biolab:genome_knockback",
];

// ---- Мутаген на игроке ----
//
// Человеку мутаген достаётся не уколом, а лужей или аварией: эффект копит
// нагрузку, и на пороге в геном вписывается случайный побочный ген — не тот,
// что выбрал бы сам игрок.

/// Сколько тиков под мутагеном нужно, чтобы случилась мутация.
pub const MUTAGEN_LOAD: u32 = 160;
/// Лужа мутагена.
pub const MUTAGEN_PUDDLE: &str = "biolab:mutagen_puddle";

/// Накопить игроку нагрузку; Some(ген) — организм сорвался и вписал его.
/// Ген берётся только из тех, что в человеке приживаются.
pub fn mutagen_tick(state: &mut PlayerState, amount: u32, seed: u64) -> Option<&'static str> {
    state.mutagenesis += amount;
    if state.mutagenesis < MUTAGEN_LOAD {
        return None;
    }
    state.mutagenesis = 0;
    let count = GENES.len() as u64;
    for step in 0..count {
        let index = ((seed + step) % count) as usize;
        let kind = &GENES[index];
        if kind.player.is_none() {
            continue;
        }
        let strength = 1 + ((seed / 7) % 2) as i32;
        if splice_player(state, kind.name, strength) == Splice::Done {
            return Some(kind.name);
        }
    }
    None
}

/// Сообщение о случайной мутации игрока.
pub fn mutagen_message(name: &str) -> String {
    let title = gene(name).map_or("Ген", |k| k.title);
    format!("Мутаген сработал: в геном вписался {title}")
}

/// Мутаген на игроке в этом мире: накопить нагрузку и, если организм
/// сорвался, вписать ген. Состояние сразу пишется на диск.
pub fn mutagen_world_tick(
    world: &std::path::Path,
    uuid: u128,
    amount: u32,
    seed: u64,
) -> Option<&'static str> {
    let mut table = states().lock().ok()?;
    let state = table.get_mut(&(world.to_path_buf(), uuid))?;
    let gene = mutagen_tick(state, amount, seed);
    let saved = *state;
    drop(table);
    if gene.is_some() {
        let path = state_path(world, uuid);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, state_to_text(&saved));
    }
    gene
}

// ---- Растения: ГМ-грядка ----
//
// Грядка — блок со слоями, как сугроб: слой показывает стадию роста. Геном
// растения живёт в блок-сущности строкой (`gigantism:2,chitin:1`), как у
// мутанта. Сажают в грядку фрагмент ДНК или собранный урожай, а при
// созревании соседняя зрелая грядка подмешивает свой ген — это и есть
// скрещивание.

pub const GM_PLOT: &str = "biolab:gm_plot";
/// Предмет урожая: еда, которая действует по геному растения.
pub const GM_CROP: &str = "biolab:gm_crop";
/// Ключ генома в блок-сущности грядки и в урожае.
pub const PLOT_GENOME_KEY: &str = "biolab_genome";
/// Стадий роста: 1 — только посажено, 4 — спелое.
pub const PLOT_RIPE: i32 = 4;
/// Сколько тиков занимает одна стадия.
pub const PLOT_GROW_EVERY: i32 = 200;
/// Сколько урожая даёт спелая грядка.
pub const PLOT_YIELD: u32 = 2;
/// Как долго действует ген урожая, в тиках за единицу силы.
pub const CROP_EFFECT_TICKS: i32 = 200;

/// Геном из строки: пары `ген:сила` через запятую, незнакомое пропускаем.
pub fn genome_from_text(text: &str) -> Vec<(&'static str, i32)> {
    text.split(',')
        .filter(|part| !part.is_empty())
        .filter_map(gene_from_text)
        .collect()
}

/// Геном строкой.
pub fn genome_to_text(genes: &[(&'static str, i32)]) -> String {
    genes
        .iter()
        .map(|(name, strength)| gene_text(name, *strength))
        .collect::<Vec<_>>()
        .join(",")
}

/// Стадия роста по числу слоёв блока.
pub fn plot_stage(layers: i32) -> i32 {
    layers.clamp(1, PLOT_RIPE)
}

/// Спелая ли грядка.
pub fn plot_ripe(layers: i32) -> bool {
    plot_stage(layers) >= PLOT_RIPE
}

/// Скрещивание: к своему геному добавляется ген соседа, который ещё влезает
/// по стабильности. Берём самый сильный подходящий — так за несколько
/// поколений линия усиливается, а не топчется на месте.
pub fn cross(mine: &[(&'static str, i32)], theirs: &[(&'static str, i32)]) -> Vec<(&'static str, i32)> {
    let mut result = mine.to_vec();
    let mut best: Option<(&'static str, i32)> = None;
    for (name, strength) in theirs {
        if splice_mob(&result, name, *strength) != Splice::Done {
            continue;
        }
        let Some(kind) = gene(name) else { continue };
        let weight = kind.cost * strength;
        if best.is_none_or(|(bn, bs)| {
            gene(bn).map_or(0, |k| k.cost * bs) < weight
        }) {
            best = Some((kind.name, *strength));
        }
    }
    if let Some((name, strength)) = best {
        match result.iter_mut().find(|(id, _)| *id == name) {
            Some(slot) => slot.1 = slot.1.max(strength),
            None => result.push((name, strength)),
        }
    }
    result
}

/// Что урожай делает со съевшим: по эффекту на каждый ген, которому есть
/// что дать человеку. Побочки у еды нет — она слабее генотерапии.
pub fn crop_effects(genes: &[(&'static str, i32)]) -> Vec<(&'static str, i32, i32)> {
    genes
        .iter()
        .filter_map(|(name, strength)| {
            let kind = gene(name)?;
            Some((kind.player?, CROP_EFFECT_TICKS * strength, strength - 1))
        })
        .collect()
}

/// Подпись урожая и грядки.
pub fn crop_label(genes: &[(&'static str, i32)]) -> String {
    if genes.is_empty() {
        return "Урожай: без генома".to_string();
    }
    let names: Vec<&str> = genes
        .iter()
        .filter_map(|(name, _)| Some(gene(name)?.title))
        .collect();
    format!("Урожай: {}", names.join(", "))
}

/// Строки подсказки на урожае.
pub fn crop_tooltip(text: &str) -> Vec<String> {
    let genes = genome_from_text(text);
    if genes.is_empty() {
        return vec!["Без генома: обычная еда".to_string()];
    }
    let mut lines = Vec::new();
    for (name, strength) in &genes {
        let Some(kind) = gene(name) else { continue };
        lines.push(match kind.player {
            Some(effect) => format!("{} {}: {effect}", kind.title, "+".repeat(*strength as usize)),
            None => format!("{} {}: в еде не действует", kind.title, "+".repeat(*strength as usize)),
        });
    }
    lines
}

// ---- Штаммы ----
//
// Болезнь — игровая абстракция: четыре числа 1–5 и ничего больше. Заразность
// решает, как быстро растёт уровень, инкубация — когда начнутся симптомы,
// тяжесть — сколько их, стойкость — сколько препарата нужно на курс.
// Штамм живёт в чашке с культурой (NBT) и в состоянии заражённого игрока.
// Вывести нужный профиль можно только селекцией, поколение за поколением:
// каждый прогон через инкубатор сдвигает одну характеристику на единицу.

/// Штамм: четыре характеристики 1–5.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Strain {
    pub infectivity: i32,
    pub incubation: i32,
    pub severity: i32,
    pub resistance: i32,
}

/// Ключ штамма в `custom_data` чашки.
pub const STRAIN_KEY: &str = "biolab_strain";

/// Дикий штамм: с него начинается любая линия.
pub const WILD: Strain = Strain {
    infectivity: 2,
    incubation: 3,
    severity: 2,
    resistance: 1,
};

impl Strain {
    /// Числа штамма по порядку: заразность, инкубация, тяжесть, стойкость.
    pub fn values(&self) -> [i32; 4] {
        [
            self.infectivity,
            self.incubation,
            self.severity,
            self.resistance,
        ]
    }

    /// Собрать штамм из четырёх чисел, обрезав их до 1–5.
    pub fn of(values: [i32; 4]) -> Strain {
        Strain {
            infectivity: values[0].clamp(1, 5),
            incubation: values[1].clamp(1, 5),
            severity: values[2].clamp(1, 5),
            resistance: values[3].clamp(1, 5),
        }
    }
}

/// Названия характеристик для микроскопа.
pub const STRAIN_NAMES: [&str; 4] = ["Заразность", "Инкубация", "Тяжесть", "Стойкость"];

/// Штамм строкой: `3,2,4,1`.
pub fn strain_to_text(strain: &Strain) -> String {
    strain
        .values()
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// Штамм из строки; всё, что не похоже на четыре числа, — не штамм.
pub fn strain_from_text(text: &str) -> Option<Strain> {
    let mut values = [0; 4];
    let mut parts = text.split(',');
    for value in values.iter_mut() {
        *value = parts.next()?.trim().parse::<i32>().ok()?;
        if !(1..=5).contains(value) {
            return None;
        }
    }
    parts.next().is_none().then(|| Strain::of(values))
}

/// Строки микроскопа и подсказки.
pub fn strain_lines(strain: &Strain) -> Vec<String> {
    let mut lines = vec!["Штамм".to_string()];
    for (name, value) in STRAIN_NAMES.iter().zip(strain.values()) {
        lines.push(format!("  {name}: {value} из 5"));
    }
    lines
}

/// Селекция: прогон через инкубатор сдвигает одну характеристику на единицу.
/// Куда именно — решает зерно, так что нужный профиль выводится за много
/// поколений, с тупиковыми линиями.
pub fn select(strain: &Strain, seed: u64) -> Strain {
    let mut values = strain.values();
    let index = (seed % 4) as usize;
    let up = (seed / 4) % 2 == 0;
    values[index] += if up { 1 } else { -1 };
    Strain::of(values)
}

/// Насколько заражение растёт за шаг болезни.
pub fn strain_step(strain: &Strain) -> u32 {
    (1 + strain.infectivity / 2) as u32
}

/// С какого уровня начинаются симптомы: длинная инкубация отодвигает их.
pub fn strain_onset(strain: &Strain) -> u32 {
    (10 + 5 * strain.incubation) as u32
}

/// Сколько заражения снимает одна доза препарата: стойкий штамм отдаёт меньше.
pub fn strain_relief(strain: &Strain) -> u32 {
    REMEDY_RELIEF.saturating_sub(6 * strain.resistance as u32).max(5)
}

/// Стадия болезни с учётом штамма.
pub fn stage_with(infection: u32, strain: &Strain) -> Stage {
    let onset = strain_onset(strain);
    match infection {
        0 => Stage::Clear,
        level if level < onset => Stage::Incubating,
        level if level < onset + 35 => Stage::Symptoms,
        _ => Stage::Crisis,
    }
}

/// Симптомы с учётом тяжести: чем тяжелее штамм, тем длиннее список.
pub fn symptoms_with(stage: Stage, strain: &Strain) -> Vec<&'static str> {
    let heavy = ["weakness", "hunger", "nausea", "mining_fatigue", "slowness"];
    match stage {
        Stage::Clear | Stage::Incubating => Vec::new(),
        Stage::Symptoms => heavy[..(strain.severity.clamp(1, 3) as usize)].to_vec(),
        Stage::Crisis => heavy[..((strain.severity + 2).clamp(2, 5) as usize)].to_vec(),
    }
}

/// Штамм игрока; пустая запись значит, что он здоров.
pub fn player_strain(state: &PlayerState) -> Option<Strain> {
    let values = state.strain.map(i32::from);
    (values[0] > 0).then(|| Strain::of(values))
}

/// Записать игроку штамм, которым он заразился.
pub fn set_player_strain(state: &mut PlayerState, strain: &Strain) {
    state.strain = strain.values().map(|v| v.clamp(1, 5) as u8);
}

/// Чашка с культурой: в ней живёт штамм.
pub const CULTURE_ID: &str = "biolab:culture";
/// Насколько поднимает заражение чашка, вылитая на себя.
pub const CULTURE_DOSE: u32 = 25;

/// Заражение игрока конкретным штаммом: уровень поднимается, а штамм
/// записывается, если игрок ещё здоров. Штамм меняется только после
/// выздоровления — две болезни разом мод не ведёт.
pub fn infect_world_player(
    world: &std::path::Path,
    uuid: u128,
    strain: &Strain,
    amount: u32,
) -> Option<u32> {
    let mut table = states().lock().ok()?;
    let state = table
        .entry((world.to_path_buf(), uuid))
        .or_insert_with(|| {
            std::fs::read_to_string(state_path(world, uuid))
                .map(|t| state_from_text(&t))
                .unwrap_or_default()
        });
    if state.immunity > 0 {
        return Some(state.infection);
    }
    if state.infection == 0 {
        set_player_strain(state, strain);
    }
    state.infection = after_exposure(state.infection, amount);
    let saved = *state;
    drop(table);
    let path = state_path(world, uuid);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, state_to_text(&saved));
    Some(saved.infection)
}

/// Штамм, которым болен игрок, или дикий, если записи нет.
pub fn strain_or_wild(state: &PlayerState) -> Strain {
    player_strain(state).unwrap_or(WILD)
}

/// Строки анализатора про болезнь: стадия считается по штамму.
pub fn infection_lines(infection: u32, strain: &Strain) -> Vec<String> {
    vec![
        format!("Заражение: {infection}%"),
        format!("Стадия: {}", stage_name(stage_with(infection, strain))),
    ]
}

/// Сколько урона возвращают костяные шипы за удар.
pub fn thorns_damage(strength: i32) -> f32 {
    1.0 + 0.5 * strength.clamp(0, 3) as f32
}

/// День ли сейчас: светобоязливые мутанты в это время не охотятся.
/// Сутки в игре — 24000 тиков, день начинается на нуле и кончается на 12000.
pub fn daylight(day_time: i64) -> bool {
    (0..12000).contains(&day_time.rem_euclid(24000))
}

/// Что ген делает, кроме чисел. Та же таблица, что в `biolab-deep`: обе
/// собираются из одного описания, а тест ниже следит, чтобы в core не
/// осталось гена без строки поведения.
pub const GENE_BEHAVIOUR: [&[&str]; 32] = [
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &["regen"],
    &["fireproof"],
    &["chill_aura"],
    &["photophobia"],
    &[],
    &["toxin_proof"],
    &["acid_aura"],
    &["volatile"],
    &["necrosis"],
    &["toxic_blood"],
    &["thorns"],
    &["heal_aura"],
    &["predator"],
    &["pack"],
    &["loyal"],
    &["rend"],
];

/// Сила поведения в наборе генов; 0 — такого гена нет.
pub fn behaviour_strength(genes: &[(&'static str, i32)], flag: &str) -> i32 {
    genes
        .iter()
        .filter(|(name, _)| {
            GENES
                .iter()
                .position(|k| k.name == *name)
                .and_then(|i| GENE_BEHAVIOUR.get(i))
                .is_some_and(|flags| flags.contains(&flag))
        })
        .map(|(_, strength)| *strength)
        .max()
        .unwrap_or(0)
}

/// Радиус ауры по её силе.
pub fn aura_of(strength: i32) -> f64 {
    match strength {
        0 => 0.0,
        s => 2.0 + f64::from(s),
    }
}

/// Как часто пересчитываются ауры и цели мутантов, в тиках.
pub const BEHAVIOUR_EVERY: u64 = 10;
/// Насколько далеко хищник ищет добычу.
pub const HUNT_RANGE: f64 = 16.0;
/// Сколько своих рядом нужно стае, чтобы дать прибавку.
pub const PACK_NEIGHBOURS: usize = 2;
/// Радиус, в котором стая считает своих.
pub const PACK_RANGE: f64 = 8.0;

/// Кто в каком вольере: вольер запоминает подопытного и возвращает его,
/// если тот отошёл. Гасить скорость бесполезно — ванильный тик моба
/// перезаписывает её своей, и корова спокойно уходила из клетки.
fn penned() -> &'static std::sync::Mutex<std::collections::HashMap<[i32; 3], u32>> {
    static TABLE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<[i32; 3], u32>>,
    > = std::sync::OnceLock::new();
    TABLE.get_or_init(Default::default)
}

/// Записать подопытного за вольером.
pub fn hold_subject(pen: [i32; 3], mob: u32) {
    if let Ok(mut table) = penned().lock() {
        table.insert(pen, mob);
    }
}

/// Кто числится за этим вольером.
pub fn subject_of(pen: [i32; 3]) -> Option<u32> {
    penned().lock().ok()?.get(&pen).copied()
}

/// Выпустить подопытного (вольер сломан или моб погиб).
pub fn release_subject(pen: [i32; 3]) {
    if let Ok(mut table) = penned().lock() {
        table.remove(&pen);
    }
}

/// Все занятые вольеры: нужны, чтобы вернуть сбежавшего, даже когда рядом с
/// вольером уже никого нет.
pub fn all_subjects() -> Vec<([i32; 3], u32)> {
    penned()
        .lock()
        .map(|table| table.iter().map(|(pen, mob)| (*pen, *mob)).collect())
        .unwrap_or_default()
}

/// Куда вольер возвращает подопытного.
pub fn pen_centre(pen: [i32; 3]) -> [f64; 3] {
    [
        f64::from(pen[0]) + 0.5,
        f64::from(pen[1]) + 1.0,
        f64::from(pen[2]) + 0.5,
    ]
}

// ---- Лаборатория: вольер и пульт ----
//
// Вольер держит подопытного на месте: моб в его круге теряет цель и не
// разбегается, пока игрок ставит опыты. Пульт рядом собирает отчёт: кто в
// вольерах, чем болен, что у него в геноме и что ему вводили.

pub const PEN: &str = "biolab:pen";
pub const CONSOLE: &str = "biolab:console";
/// Радиус, в котором вольер удерживает подопытного.
pub const PEN_RANGE: f64 = 2.5;
/// Как далеко пульт видит вольеры и подопытных.
pub const CONSOLE_RANGE: f64 = 12.0;
/// Сколько последних записей показывает пульт.
pub const TRIAL_LINES: usize = 5;

/// Журнал испытаний в папке мира.
pub fn trials_path(world: &std::path::Path) -> std::path::PathBuf {
    world.join("biolab").join("trials.txt")
}

/// Записать опыт: что и кому ввели.
pub fn record_trial(world: &std::path::Path, line: &str) {
    let path = trials_path(world);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    text.push_str(line);
    text.push('\n');
    // Журнал не растёт бесконечно: держим последнюю сотню записей.
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    let keep = lines[lines.len().saturating_sub(100)..].join("\n");
    let _ = std::fs::write(&path, format!("{keep}\n"));
}

/// Последние записи журнала, новые сверху.
pub fn trial_lines(world: &std::path::Path, count: usize) -> Vec<String> {
    let text = std::fs::read_to_string(trials_path(world)).unwrap_or_default();
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    lines
        .iter()
        .rev()
        .take(count)
        .map(|l| (*l).to_string())
        .collect()
}

/// Строка журнала об инъекции подопытному.
pub fn trial_record(item: &str, mob: &str) -> String {
    let what = MEDICINES
        .iter()
        .find(|m| m.item == item)
        .map(|m| DRUGS[m.drug].name)
        .unwrap_or(match item {
            "biolab:syringe_mutagen" => "Мутаген",
            FRAGMENT_ID => "Фрагмент ДНК",
            CULTURE_ID => "Культура",
            _ => "Препарат",
        });
    let target = mob.strip_prefix("minecraft:").unwrap_or(mob);
    format!("{target} ← {what}")
}

/// Отчёт пульта: подопытные и журнал опытов.
pub fn console_lines(
    subjects: &[(String, f32, f32, Vec<(&'static str, i32)>, bool)],
    trials: &[String],
) -> Vec<String> {
    let mut lines = vec!["Пульт наблюдения".to_string()];
    if subjects.is_empty() {
        lines.push("Вольеры пусты".to_string());
    }
    for (kind, health, max_health, genes, sick) in subjects {
        let name = kind.strip_prefix("minecraft:").unwrap_or(kind);
        lines.push(format!(
            "{name}: {:.0}/{:.0} HP{}",
            health,
            max_health,
            if *sick { ", болен" } else { "" }
        ));
        if genes.is_empty() {
            lines.push("  геном: чистый".to_string());
        } else {
            for (gene_name, strength) in genes {
                let Some(kind) = gene(gene_name) else { continue };
                lines.push(format!("  {} {}", kind.title, "+".repeat(*strength as usize)));
            }
        }
    }
    if !trials.is_empty() {
        lines.push("Последние опыты:".to_string());
        for line in trials {
            lines.push(format!("  {line}"));
        }
    }
    lines
}

/// Полное выздоровление: болезнь, штамм, кровь и ломка обнуляются. Нужно для
/// проверок (`/biolab cure`) и как итог успешного курса.
pub fn cure_world_player(world: &std::path::Path, uuid: u128) {
    let Ok(mut table) = states().lock() else {
        return;
    };
    let state = table
        .entry((world.to_path_buf(), uuid))
        .or_insert_with(|| {
            std::fs::read_to_string(state_path(world, uuid))
                .map(|t| state_from_text(&t))
                .unwrap_or_default()
        });
    state.infection = 0;
    state.strain = [0; 4];
    state.withdrawal = 0;
    state.drugs = [DrugState::default(); DRUG_COUNT];
    let saved = *state;
    drop(table);
    let path = state_path(world, uuid);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, state_to_text(&saved));
}

/// Прерванный курс: болезнь осталась, а препарат вышел из крови — штамм
/// привыкает. Так лечиться надо до конца, а не «пока не полегчает».
pub const COURSE_DRUG: usize = 0;

/// Нужно ли ужесточить штамм: лечение шло, но не довели.
pub fn course_broken(state: &PlayerState, before: Level, after: Level) -> bool {
    state.infection > 0
        && before == Level::Therapeutic
        && after != Level::Therapeutic
        && player_strain(state).is_some()
}

/// Штамм после сорванного курса: стойкость растёт на единицу.
pub fn harden(strain: &Strain) -> Strain {
    Strain::of([
        strain.infectivity,
        strain.incubation,
        strain.severity,
        strain.resistance + 1,
    ])
}

// ---- Кровотечение ----
//
// Эффект `biolab:bleeding` бьёт понемногу, но долго, и сам не проходит
// быстро: его снимают бинтом. Ставят его зазубренным скальпелем (мобу) и
// когти мутанта-хищника (игроку).

pub const BANDAGE_ID: &str = "biolab:bandage";
pub const SCALPEL_ID: &str = "biolab:scalpel";
pub const BLEEDING: &str = "biolab:bleeding";
/// Сколько тиков кровоточит рана от скальпеля или когтей.
pub const BLEEDING_TICKS: i32 = 200;

/// Что бинт делает: снимает кровотечение и чуть лечит.
pub const BANDAGE_HEAL: f32 = 2.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_starts_with_name_and_version() {
        let lines = status_lines();
        assert_eq!(lines[0], "BioLab 0.1.0");
        assert_eq!(lines.len(), 1 + SUBSYSTEMS.len());
    }

    #[test]
    fn ready_subsystems_are_marked() {
        assert_eq!(subsystem_line("Химия", true), "✓ Химия");
        assert_eq!(subsystem_line("Химия", false), "— Химия");
    }

    #[test]
    fn flask_label_names_the_contents() {
        assert_eq!(flask_label("water", 250), "Колба: вода, 250 мл");
        assert_eq!(flask_label("unknown_x", 10), "Колба: unknown_x, 10 мл");
    }

    #[test]
    fn player_state_round_trips() {
        let s = PlayerState { ticks: 12345, infection: 7, ..Default::default() };
        assert_eq!(state_from_text(&state_to_text(&s)), s);
    }

    #[test]
    fn broken_state_lines_are_skipped() {
        let s = state_from_text("ticks=50\nmystery=1\ninfection=oops\ngarbage\n");
        assert_eq!(s, PlayerState { ticks: 50, infection: 0, ..Default::default() });
    }

    #[test]
    fn state_lives_inside_the_world_folder() {
        let p = state_path(std::path::Path::new("/saves/w"), 0xAB);
        assert!(p.starts_with("/saves/w/biolab/players"));
        assert!(p.ends_with("000000000000000000000000000000ab.txt"));
    }

    #[test]
    fn state_survives_a_restart_on_disk() {
        let world = std::env::temp_dir().join(format!("biolab-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&world);
        for _ in 0..SAVE_EVERY {
            player_tick(&world, 1);
        }
        // Новый процесс видит только диск: читаем файл напрямую.
        let saved = state_from_text(&std::fs::read_to_string(state_path(&world, 1)).unwrap());
        assert_eq!(saved.ticks, SAVE_EVERY);
        let _ = std::fs::remove_dir_all(&world);
    }

    #[test]
    fn every_element_has_its_own_item_and_symbol() {
        for (i, e) in ELEMENTS.iter().enumerate() {
            assert!(e.item.starts_with("biolab:element_"), "{}", e.item);
            assert!(e.bonds >= 1 && e.mass >= 1, "{}", e.symbol);
            for other in &ELEMENTS[i + 1..] {
                assert_ne!(e.item, other.item);
                assert_ne!(e.symbol, other.symbol);
            }
        }
    }

    #[test]
    fn known_compounds_are_stable_and_named() {
        for (text, name) in KNOWN {
            let mut f = Formula::default();
            // Собираем формулу обратно по символам, чтобы проверить запись.
            let mut rest = text;
            while !rest.is_empty() {
                let (symbol, after) = ELEMENTS
                    .iter()
                    .filter(|e| rest.starts_with(e.symbol))
                    .max_by_key(|e| e.symbol.len())
                    .map(|e| (e.symbol, &rest[e.symbol.len()..]))
                    .unwrap_or_else(|| panic!("в {text} непонятный символ: {rest}"));
                let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
                let index = ELEMENTS.iter().position(|e| e.symbol == symbol).unwrap();
                f[index] += digits.parse().unwrap_or(1);
                rest = &after[digits.len()..];
            }
            assert!(is_stable(&f), "{name} ({text}) должно держаться вместе");
            assert_eq!(formula_text(&f), text, "запись {name} разошлась");
            assert_eq!(compound_name_of(&f), name);
        }
    }

    #[test]
    fn fantasy_elements_are_marked_in_the_table() {
        assert!(ELEMENTS.iter().any(|e| e.symbol == "Rs"));
        assert!(ELEMENTS.iter().any(|e| e.symbol == "Gl"));
    }

    #[test]
    fn formula_survives_text_round_trip() {
        let mut f = Formula::default();
        f[1] = 6;
        f[0] = 12;
        f[3] = 6;
        let text = formula_text(&f);
        assert_eq!(formula_from_text(&text), Some(f));
    }

    #[test]
    fn two_letter_symbols_are_read_whole() {
        let f = formula_from_text("CCaO3").expect("мел читается");
        assert_eq!(count_of(&f, "C"), 1);
        assert_eq!(count_of(&f, "Ca"), 1);
        assert_eq!(count_of(&f, "O"), 3);
    }

    #[test]
    fn nonsense_text_is_not_a_formula() {
        assert_eq!(formula_from_text("Zz"), None);
        assert_eq!(formula_from_text(""), None);
        assert_eq!(formula_from_text("вода"), None);
    }

    #[test]
    fn tooltip_describes_the_contents() {
        let lines = flask_tooltip("H2O", 250);
        assert_eq!(lines[0], "H2O, 250 мл");
        assert!(lines.iter().any(|l| l == "Газ"));
        assert!(!lines.iter().any(|l| l == "Не опознано"), "вода известна");
        let unknown = flask_tooltip("CFe2", 250);
        assert!(unknown.iter().any(|l| l == "Не опознано"));
    }

    #[test]
    fn heat_sources_have_their_own_ceilings() {
        assert_eq!(heat_target("minecraft:magma_block"), Some(500));
        assert!(heat_target("minecraft:lava").unwrap() > OVERHEAT, "лава опасна");
        assert_eq!(heat_target("minecraft:stone"), None);
    }

    #[test]
    fn temperature_creeps_towards_the_source() {
        let magma = heat_target("minecraft:magma_block");
        assert_eq!(next_temperature(20, magma), 20 + HEAT_PER_TICK);
        assert_eq!(next_temperature(500, magma), 500, "на потолке стоит");
        assert_eq!(next_temperature(600, magma), 600 - COOL_PER_TICK, "сверху опускается");
    }

    #[test]
    fn without_a_source_it_cools_to_room() {
        assert_eq!(next_temperature(ROOM_TEMPERATURE, None), ROOM_TEMPERATURE);
        assert_eq!(next_temperature(100, None), 100 - COOL_PER_TICK);
    }

    #[test]
    fn only_the_hottest_source_overheats() {
        let mut magma = ROOM_TEMPERATURE;
        for _ in 0..1000 {
            magma = next_temperature(magma, heat_target("minecraft:magma_block"));
        }
        assert!(magma <= OVERHEAT, "на магме реактор безопасен");
        let mut lava = ROOM_TEMPERATURE;
        for _ in 0..1000 {
            lava = next_temperature(lava, heat_target("minecraft:lava"));
        }
        assert!(lava > OVERHEAT, "на лаве перегревается");
    }

    #[test]
    fn recipes_can_demand_a_temperature() {
        assert!(temperature_fits(300, None), "рецепт без требований");
        assert!(temperature_fits(300, Some((200, 400))));
        assert!(!temperature_fits(100, Some((200, 400))));
        assert!(!temperature_fits(500, Some((200, 400))));
    }

    #[test]
    fn buckets_and_puddles_match() {
        for fluid in &FLUIDS {
            assert_eq!(fluid_of_block(fluid.block).map(|f| f.bucket), Some(fluid.bucket));
            assert_eq!(fluid_of_bucket(fluid.bucket).map(|f| f.block), Some(fluid.block));
        }
        assert!(fluid_of_block("minecraft:water").is_none());
        assert!(fluid_of_bucket("minecraft:bucket").is_none());
    }

    #[test]
    fn puddles_dry_out() {
        assert_eq!(next_layers(FLUID_LAYERS), FLUID_LAYERS - 1);
        assert_eq!(next_layers(1), 0, "последний слой высыхает");
        assert_eq!(next_layers(0), 0);
    }

    #[test]
    fn only_slime_infects() {
        let slime = fluid_of_block("biolab:slime").unwrap();
        assert!(slime.infects > 0);
        assert_eq!(fluid_of_block("biolab:acid_puddle").unwrap().infects, 0);
    }

    #[test]
    fn medicines_point_at_real_drugs_and_forms() {
        for (i, med) in MEDICINES.iter().enumerate() {
            assert!(med.drug < DRUG_COUNT, "{}", med.item);
            assert!(med.form < FORMS.len(), "{}", med.item);
            assert!(med.dose > 0, "{}", med.item);
            assert!(med.item.starts_with("biolab:"), "{}", med.item);
            for other in &MEDICINES[i + 1..] {
                assert_ne!(med.item, other.item);
            }
        }
        assert_eq!(medicine("biolab:pill_penicillin").map(|m| m.drug), Some(0));
        assert!(medicine("minecraft:apple").is_none());
        for (i, drug) in DRUGS.iter().enumerate() {
            assert_eq!(drug_index(drug.id), Some(i));
        }
        assert_eq!(drug_index("adrenaline"), None);
    }

    #[test]
    fn every_drug_can_be_taken_somehow() {
        for (i, drug) in DRUGS.iter().enumerate() {
            assert!(
                MEDICINES.iter().any(|m| m.drug == i),
                "{} не выпускается ни в одной форме",
                drug.id
            );
            let (min, max) = drug.window;
            assert!(min > 0 && min < max, "окно {} бессмысленно", drug.id);
            assert!(drug.half_life > 0, "{}", drug.id);
        }
    }

    #[test]
    fn injection_goes_straight_to_blood_and_a_pill_waits_in_the_depot() {
        let mut state = PlayerState::default();
        take_dose(&mut state, medicine("biolab:syringe_penicillin").unwrap());
        assert_eq!(state.drugs[0].blood, 80, "инъекция — вся доза сразу");
        assert_eq!(state.drugs[0].depot, 0);

        let mut state = PlayerState::default();
        take_dose(&mut state, medicine("biolab:pill_penicillin").unwrap());
        assert_eq!(state.drugs[0].blood, 0, "таблетка не действует мгновенно");
        assert_eq!(state.drugs[0].depot, 60, "60% дозы дойдёт до крови");
        assert_eq!(state.drugs[0].tolerance, TOLERANCE_PER_DOSE);
    }

    #[test]
    fn the_depot_empties_into_the_blood() {
        let mut state = PlayerState::default();
        take_dose(&mut state, medicine("biolab:pill_penicillin").unwrap());
        let mut peak = 0;
        for _ in 0..30 {
            state.ticks += PHARMA_EVERY;
            pharma_step(&mut state);
            peak = peak.max(state.drugs[0].blood);
        }
        assert!(peak > 0, "таблетка так и не всосалась");
        let (min, max) = window_with_tolerance(DRUGS[0].window, TOLERANCE_PER_DOSE);
        assert!(peak >= min && peak <= max, "пик {peak} вне окна {min}..{max}");
        assert_eq!(state.drugs[0].depot, 0, "депо должно опустеть");
    }

    #[test]
    fn half_life_halves_the_blood() {
        for drug in &DRUGS {
            let start = 200;
            let mut blood = start;
            for _ in 0..drug.half_life {
                blood = decay(blood, drug.half_life);
            }
            assert!(
                blood >= start * 35 / 100 && blood <= start * 55 / 100,
                "{}: за период осталось {blood} из {start}",
                drug.id
            );
        }
        assert_eq!(decay(0, 10), 0);
        assert_eq!(decay(1, 10_000), 0, "остаток всегда уходит");
    }

    #[test]
    fn tolerance_moves_the_window_up() {
        let drug = &DRUGS[2];
        let (min, max) = drug.window;
        assert_eq!(window_with_tolerance((min, max), 0), (min, max));
        let (min_t, max_t) = window_with_tolerance((min, max), TOLERANCE_MAX);
        assert!(min_t > min && max_t > max);
        // Прежняя доза в прежнем окне работала, а с привыканием — уже мало.
        let dose = DrugState { blood: min + 1, ..Default::default() };
        assert_eq!(level_of(drug, &dose), Level::Therapeutic);
        let used = DrugState { blood: min + 1, tolerance: TOLERANCE_MAX, ..Default::default() };
        assert_eq!(level_of(drug, &used), Level::Subtherapeutic);
    }

    #[test]
    fn overdose_brings_toxicosis() {
        let mut state = PlayerState::default();
        // Три инъекции подряд: окно кончается там, где начинается отравление.
        for _ in 0..3 {
            take_dose(&mut state, medicine("biolab:syringe_penicillin").unwrap());
        }
        state.ticks += PHARMA_EVERY;
        let out = pharma_step(&mut state);
        assert_eq!(level_of(&DRUGS[0], &state.drugs[0]), Level::Overdose);
        let amp = out
            .effects
            .iter()
            .find(|(name, _)| *name == TOXICOSIS_EFFECT)
            .map(|(_, amp)| *amp);
        assert!(amp.is_some(), "перебор без токсикоза: {:?}", out.effects);
        assert_eq!(out.cured, 0, "в переборе пенициллин не лечит");
    }

    #[test]
    fn penicillin_in_the_window_cures() {
        let mut state = PlayerState::default();
        take_dose(&mut state, medicine("biolab:syringe_penicillin").unwrap());
        state.ticks += PHARMA_EVERY;
        let out = pharma_step(&mut state);
        assert_eq!(level_of(&DRUGS[0], &state.drugs[0]), Level::Therapeutic);
        assert_eq!(out.cured, DRUGS[0].cures);
        assert!(!out.immune);
    }

    #[test]
    fn a_vaccine_holds_immunity_long_after_it_leaves_the_blood() {
        let mut state = PlayerState::default();
        take_dose(&mut state, medicine("biolab:syringe_vaccine").unwrap());
        state.ticks += PHARMA_EVERY;
        let out = pharma_step(&mut state);
        assert!(out.immune, "вакцина в окне не дала иммунитета");
        assert_eq!(state.immunity, IMMUNITY_TICKS);
        // Кровь очистилась, а иммунитет ещё держится.
        state.drugs[4] = DrugState::default();
        state.ticks += PHARMA_EVERY;
        let out = pharma_step(&mut state);
        assert!(out.immune);
        assert!(out.effects.contains(&(IMMUNITY_EFFECT, 0)));
        assert_eq!(after_exposure_with(10, 20, true), 10, "иммунный не заражается");
        assert_eq!(after_exposure_with(10, 20, false), 30);
    }

    #[test]
    fn the_antidote_washes_the_rest_out() {
        let mut state = PlayerState::default();
        take_dose(&mut state, medicine("biolab:syringe_stimulant").unwrap());
        let before = state.drugs[2].blood;
        take_dose(&mut state, medicine("biolab:pill_antidote").unwrap());
        // Таблетке угля нужен шаг-другой, чтобы попасть в кровь.
        for _ in 0..3 {
            state.ticks += PHARMA_EVERY;
            pharma_step(&mut state);
        }
        assert_eq!(level_of(&DRUGS[3], &state.drugs[3]), Level::Therapeutic);
        let plain = {
            let mut other = PlayerState::default();
            take_dose(&mut other, medicine("biolab:syringe_stimulant").unwrap());
            for _ in 0..3 {
                other.ticks += PHARMA_EVERY;
                pharma_step(&mut other);
            }
            other.drugs[2].blood
        };
        assert!(
            state.drugs[2].blood < plain,
            "с антидотом стимулятора осталось {} против {plain} без него (вход {before})",
            state.drugs[2].blood
        );
    }

    #[test]
    fn a_stimulant_leaves_withdrawal_behind() {
        let mut state = PlayerState::default();
        take_dose(&mut state, medicine("biolab:syringe_stimulant").unwrap());
        let mut saw_speed = false;
        let mut saw_withdrawal = false;
        for _ in 0..200 {
            state.ticks += PHARMA_EVERY;
            let out = pharma_step(&mut state);
            saw_speed |= out.effects.contains(&("speed", 0));
            saw_withdrawal |= out.effects.contains(&(WITHDRAWAL_EFFECT, 0));
        }
        assert!(saw_speed, "стимулятор не разогнал");
        assert!(saw_withdrawal, "после спада не наступила ломка");
        assert!(out_of_withdrawal(&state), "ломка должна была пройти");
    }

    fn out_of_withdrawal(state: &PlayerState) -> bool {
        state.withdrawal == 0
    }

    #[test]
    fn only_syringes_work_on_mobs() {
        let (effects, ticks, _) = mob_injection("biolab:syringe_venom").unwrap();
        assert_eq!(effects, &[TOXICOSIS_EFFECT]);
        assert!(ticks > 0);
        let (effects, _, _) = mob_injection("biolab:syringe_sedative").unwrap();
        assert!(effects.contains(&SEDATION_EFFECT) && effects.contains(&"slowness"));
        assert!(mob_injection("biolab:pill_stimulant").is_none());
        assert!(mob_injection("minecraft:stick").is_none());
    }

    #[test]
    fn the_analyzer_packet_survives_the_wire() {
        let mut state = PlayerState::default();
        state.infection = 42;
        take_dose(&mut state, medicine("biolab:syringe_venom").unwrap());
        state.drugs[6].tolerance = 12;
        let data = pharma_packet(&state, 7);
        assert_eq!(data[0], 42);
        assert_eq!(data[1], 7);
        let back = pharma_from_packet(&data);
        assert_eq!(back[6].blood, state.drugs[6].blood);
        assert_eq!(back[6].tolerance, 12);
        // Старый двухбайтовый пакет не должен ломать клиент.
        assert_eq!(pharma_from_packet(&[1, 2]), [DrugState::default(); DRUG_COUNT]);
        let lines = blood_lines(&back);
        assert!(lines.iter().any(|l| l.starts_with("Яд: ")), "{lines:?}");
        assert!(lines.iter().any(|l| l.contains("привык.")), "{lines:?}");
        assert!(blood_lines(&[DrugState::default(); DRUG_COUNT]).is_empty());
    }

    #[test]
    fn blood_survives_a_save_and_load() {
        let mut state = PlayerState { ticks: 500, infection: 31, ..Default::default() };
        take_dose(&mut state, medicine("biolab:pill_stimulant").unwrap());
        take_dose(&mut state, medicine("biolab:syringe_vaccine").unwrap());
        state.immunity = 1234;
        state.withdrawal = 56;
        state.drugs[2].was_active = true;
        let text = state_to_text(&state);
        assert_eq!(state_from_text(&text), state, "{text}");
        // Файл от новой версии мода со незнакомым веществом читается молча.
        let text = format!("{text}drug.adrenaline=1,2,3,1\n");
        assert_eq!(state_from_text(&text), state);
    }

    #[test]
    fn a_used_syringe_stays_in_the_hand() {
        assert_eq!(after_use("biolab:syringe_venom"), Some(SYRINGE_EMPTY));
        assert_eq!(after_use("biolab:pill_antidote"), None);
        assert_eq!(after_use(SYRINGE_EMPTY), None);
    }

    #[test]
    fn the_press_is_a_machine_like_the_rest() {
        let press = machine("biolab:press").expect("пресс в таблице машин");
        assert_eq!(press.slots, 3);
        assert!(is_machine("biolab:press"));
    }

    #[test]
    fn a_dose_is_reported_to_the_player() {
        let mut state = PlayerState::default();
        let med = medicine("biolab:syringe_stimulant").unwrap();
        take_dose(&mut state, med);
        let text = dose_message(med, &state);
        assert!(text.starts_with("Стимулятор (инъекция)"), "{text}");
        assert!(text.contains("в окне"), "{text}");
    }

    #[test]
    fn an_injection_names_the_drug_and_the_target() {
        let text = injection_message("biolab:syringe_venom", "minecraft:cow");
        assert_eq!(text, "Яд введён: cow");
        assert!(injection_message("biolab:syringe", "pig").starts_with("Препарат"));
    }

    #[test]
    fn an_injection_shields_the_next_click() {
        remember_injection(100);
        assert!(just_injected(100), "тот же тик");
        assert!(just_injected(101), "соседний тик");
        assert!(!just_injected(105), "а через пять тиков это уже новый клик");
    }

    #[test]
    fn genes_are_unique_priced_and_described() {
        for (i, kind) in GENES.iter().enumerate() {
            assert!(kind.cost > 0, "{} бесплатен", kind.name);
            assert!(!kind.title.is_empty() && !kind.about.is_empty(), "{}", kind.name);
            assert!(kind.cost <= STABILITY, "{} не влезает даже один", kind.name);
            for other in &GENES[i + 1..] {
                assert_ne!(kind.name, other.name);
                assert_ne!(kind.title, other.title);
            }
            // Ген, который не приживается у человека, не обещает и побочки.
            if kind.player.is_none() {
                assert!(kind.side.is_none(), "{}", kind.name);
            }
        }
        assert!(gene("telepathy").is_none());
    }

    #[test]
    fn markers_match_the_genes() {
        for kind in &GENES {
            for strength in 1..=3 {
                let marker = marker_for(kind.name, strength).expect(kind.name);
                assert_eq!(gene_of_marker(marker), Some((kind.name, strength)));
            }
            assert_eq!(marker_for(kind.name, 0), None);
            assert_eq!(marker_for(kind.name, 4), None);
        }
        assert_eq!(marker_for("telepathy", 1), None);
        assert_eq!(gene_of_marker("biolab:genome_health"), None);
        assert_eq!(gene_of_marker("biolab:gene/ghost/1"), None);
    }

    #[test]
    fn a_fragment_says_what_it_carries() {
        let text = gene_text("chitin", 2);
        assert_eq!(gene_from_text(&text), Some(("chitin", 2)));
        assert_eq!(gene_from_text("chitin:9"), None);
        assert_eq!(gene_from_text("ghost:1"), None);
        assert_eq!(gene_from_text(""), None);

        let lines = fragment_tooltip(&text);
        assert!(lines[0].starts_with("Хитиновый панцирь"), "{lines:?}");
        assert!(lines.iter().any(|l| l.contains("Стабильность: 4")), "{lines:?}");
        assert!(fragment_label(&text).contains("++"));
        assert_eq!(fragment_tooltip("мусор"), ["Ген не прочитан"]);

        // Ген, который человеку не вживить, честно об этом пишет.
        let lines = fragment_tooltip(&gene_text("volatile", 1));
        assert!(lines.iter().any(|l| l.contains("не приживается")), "{lines:?}");
    }

    #[test]
    fn extracted_genes_are_usable() {
        for seed in 0..500 {
            let (name, strength) = roll_gene(seed);
            let kind = gene(name).expect(name);
            assert!((1..=3).contains(&strength));
            assert!(kind.cost * strength <= STABILITY, "{name} силы {strength} некуда применить");
        }
        assert_eq!(roll_gene(42), roll_gene(42), "зерно задаёт ген");
    }

    #[test]
    fn the_microscope_reads_a_genome() {
        assert_eq!(genome_lines(&[]), ["Обычный организм, мутаций нет"]);
        let lines = genome_lines(&[("gigantism", 2), ("bioluminescence", 1)]);
        assert!(lines.iter().any(|l| l.contains("Гигантизм")), "{lines:?}");
        assert!(lines.last().unwrap().contains("5 из 6"), "{lines:?}");
    }

    #[test]
    fn gene_therapy_keeps_the_player_in_one_piece() {
        let mut state = PlayerState::default();
        assert_eq!(splice_player(&mut state, "chitin", 2), Splice::Done);
        assert_eq!(player_genes(&state), [("chitin", 2)]);
        assert_eq!(player_stability(&state), 4);
        assert_eq!(splice_player(&mut state, "chitin", 1), Splice::Already);
        assert_eq!(splice_player(&mut state, "regeneration", 1), Splice::Unstable);
        assert_eq!(splice_player(&mut state, "volatile", 1), Splice::NotForHumans);
        assert_eq!(splice_player(&mut state, "bioluminescence", 1), Splice::Done);

        // Третий ген ещё влезает по месту, но уже не по стабильности.
        let mut roomy = PlayerState::default();
        for name in ["bioluminescence", "hypermetabolism", "gigantism"] {
            assert_eq!(splice_player(&mut roomy, name, 1), Splice::Done, "{name}");
        }
        assert_eq!(splice_player(&mut roomy, "thermal", 1), Splice::NoRoom);
    }

    #[test]
    fn genes_pay_with_side_effects() {
        let mut state = PlayerState::default();
        splice_player(&mut state, "chitin", 2);
        let effects = gene_effects(&state);
        assert!(effects.contains(&("resistance", 1)), "{effects:?}");
        assert!(effects.contains(&("slowness", 0)), "побочка обязательна: {effects:?}");
        assert!(gene_effects(&PlayerState::default()).is_empty());
    }

    #[test]
    fn the_vat_needs_a_gene_and_broth() {
        assert!(vat_ready(Some("chitin:1"), Some((BROTH_ID, 1))));
        assert!(!vat_ready(Some("chitin:1"), Some(("minecraft:coal", 8))), "уголь не раствор");
        assert!(!vat_ready(Some("мусор"), Some((BROTH_ID, 1))), "ген не прочитан");
        assert!(!vat_ready(None, Some((BROTH_ID, 1))));
        assert!(!vat_ready(Some("chitin:1"), None));
    }

    #[test]
    fn player_genes_survive_a_save() {
        let mut state = PlayerState { ticks: 7, infection: 3, ..Default::default() };
        splice_player(&mut state, "thermal", 1);
        splice_player(&mut state, "bioluminescence", 2);
        let text = state_to_text(&state);
        assert_eq!(state_from_text(&text), state, "{text}");
        // Ген из новой версии мода старую не ломает.
        let text = format!("{text}gene.telepathy=2\n");
        assert_eq!(state_from_text(&text), state);
    }

    #[test]
    fn a_mob_takes_genes_by_the_same_rules() {
        assert_eq!(splice_mob(&[], "volatile", 1), Splice::Done, "мобу можно всё");
        assert_eq!(splice_mob(&[("chitin", 2)], "chitin", 1), Splice::Already);
        assert_eq!(splice_mob(&[("chitin", 1)], "chitin", 2), Splice::Done, "усиление");
        assert_eq!(splice_mob(&[("regeneration", 2)], "chitin", 1), Splice::Unstable);
        let full = [("bioluminescence", 1), ("hypermetabolism", 1), ("gigantism", 1)];
        assert_eq!(splice_mob(&full, "thermal", 1), Splice::NoRoom);
        assert_eq!(splice_mob(&[], "telepathy", 1), Splice::NotForHumans);
    }

    #[test]
    fn mutants_drop_tissue_per_gene() {
        assert_eq!(tissue_drop(0), 0);
        assert_eq!(tissue_drop(2), 2);
        assert_eq!(tissue_drop(9), 3, "больше трёх кусков не выпадает");
    }

    #[test]
    fn only_acid_glands_burn_the_air() {
        assert_eq!(aura_radius("biolab:gene/acid_glands/1"), Some(3.0));
        assert!(aura_radius("biolab:gene/acid_glands/3") > aura_radius("biolab:gene/acid_glands/1"));
        assert_eq!(aura_radius("biolab:gene/chitin/2"), None);
        assert_eq!(aura_radius("biolab:genome_armor"), None);
    }

    #[test]
    fn tissue_message_names_the_genes() {
        let text = tissue_message(2, &[("chitin", 1), ("volatile", 2)]);
        assert!(text.contains("×2"), "{text}");
        assert!(text.contains("Хитиновый панцирь"), "{text}");
        assert!(tissue_message(0, &[("chitin", 1)]).contains("рюкзак полон"));
    }

    #[test]
    fn mutagen_rewrites_the_player_eventually() {
        let mut state = PlayerState::default();
        assert_eq!(mutagen_tick(&mut state, MUTAGEN_LOAD - 1, 0), None, "рано");
        let name = mutagen_tick(&mut state, 1, 0).expect("на пороге ген вписался");
        assert_eq!(state.mutagenesis, 0);
        let kind = gene(name).expect(name);
        assert!(kind.player.is_some(), "мутаген вписал ген не для человека");
        assert!(mutagen_message(name).contains(kind.title));

        // Когда места не осталось, мутировать некуда и срыв проходит впустую.
        let mut full = PlayerState::default();
        splice_player(&mut full, "regeneration", 2);
        assert_eq!(mutagen_tick(&mut full, MUTAGEN_LOAD, 0), None);
    }

    #[test]
    fn a_plot_ripens_by_layers() {
        assert_eq!(plot_stage(0), 1, "ниже первой стадии не бывает");
        assert_eq!(plot_stage(9), PLOT_RIPE, "выше спелости тоже");
        assert!(!plot_ripe(1) && !plot_ripe(3));
        assert!(plot_ripe(PLOT_RIPE));
    }

    #[test]
    fn genome_text_round_trips() {
        let genes = vec![("chitin", 1), ("thermal", 2)];
        let text = genome_to_text(&genes);
        assert_eq!(text, "chitin:1,thermal:2");
        assert_eq!(genome_from_text(&text), genes);
        assert!(genome_from_text("").is_empty());
        assert_eq!(genome_from_text("ghost:1,chitin:2"), [("chitin", 2)], "чужое пропускаем");
    }

    #[test]
    fn crossing_takes_the_best_gene_that_fits() {
        // Пустая линия берёт лучший ген соседа.
        let child = cross(&[], &[("bioluminescence", 1), ("gigantism", 2)]);
        assert_eq!(child, [("gigantism", 2)], "взялся самый весомый");

        // Если у соседа ничего не влезает, геном не меняется.
        let mine = vec![("regeneration", 2)];
        assert_eq!(cross(&mine, &[("gigantism", 2)]), mine);

        // Свой ген только усиливается.
        let mine = vec![("chitin", 1)];
        assert_eq!(cross(&mine, &[("chitin", 2)]), [("chitin", 2)]);

        // Скрещивание не ломает предел стабильности.
        let child = cross(&[("bioluminescence", 1)], &[("chitin", 2), ("thermal", 2)]);
        let total: i32 = child
            .iter()
            .filter_map(|(name, s)| Some(gene(name)?.cost * s))
            .sum();
        assert!(total <= STABILITY, "{child:?} стоит {total}");
    }

    #[test]
    fn a_crop_feeds_what_its_genes_give() {
        let effects = crop_effects(&[("thermal", 2), ("volatile", 1)]);
        assert_eq!(effects.len(), 1, "нестабильный геном в еде не действует");
        let (effect, ticks, amplifier) = effects[0];
        assert_eq!(effect, "fire_resistance");
        assert_eq!(ticks, CROP_EFFECT_TICKS * 2);
        assert_eq!(amplifier, 1);
        assert!(crop_effects(&[]).is_empty());

        assert!(crop_label(&[("thermal", 1)]).contains("Термостойкость"));
        assert_eq!(crop_label(&[]), "Урожай: без генома");
        let lines = crop_tooltip("volatile:1");
        assert!(lines[0].contains("не действует"), "{lines:?}");
        assert_eq!(crop_tooltip(""), ["Без генома: обычная еда"]);
    }

    #[test]
    fn a_strain_is_four_numbers() {
        let text = strain_to_text(&WILD);
        assert_eq!(text, "2,3,2,1");
        assert_eq!(strain_from_text(&text), Some(WILD));
        assert_eq!(strain_from_text("2,3,2"), None, "мало чисел");
        assert_eq!(strain_from_text("2,3,2,1,4"), None, "много чисел");
        assert_eq!(strain_from_text("2,3,2,9"), None, "вне диапазона");
        assert_eq!(strain_from_text("мусор"), None);
        let lines = strain_lines(&WILD);
        assert!(lines.iter().any(|l| l.contains("Заразность: 2")), "{lines:?}");
    }

    #[test]
    fn selection_moves_one_number_at_a_time() {
        for seed in 0..40 {
            let next = select(&WILD, seed);
            let changed: Vec<usize> = (0..4)
                .filter(|i| next.values()[*i] != WILD.values()[*i])
                .collect();
            assert!(changed.len() <= 1, "сдвинулось больше одного: {next:?}");
            for value in next.values() {
                assert!((1..=5).contains(&value), "{next:?}");
            }
        }
        // За много поколений линия доходит до предела.
        let mut strain = WILD;
        for seed in 0..200 {
            strain = select(&strain, seed * 7 + 2);
        }
        for value in strain.values() {
            assert!((1..=5).contains(&value));
        }
    }

    #[test]
    fn the_strain_shapes_the_disease() {
        let mild = Strain::of([1, 5, 1, 1]);
        let nasty = Strain::of([5, 1, 5, 5]);
        assert!(strain_step(&nasty) > strain_step(&mild), "заразность ускоряет");
        assert!(strain_onset(&mild) > strain_onset(&nasty), "инкубация тянет время");
        assert!(strain_relief(&nasty) < strain_relief(&mild), "стойкий лечится хуже");
        assert!(strain_relief(&nasty) >= 5, "хоть сколько-то снимает всегда");

        // Инкубация: при одном уровне мягкий штамм ещё молчит, злой уже бьёт.
        assert_eq!(stage_with(20, &mild), Stage::Incubating);
        assert_eq!(stage_with(20, &nasty), Stage::Symptoms);
        assert_eq!(stage_with(0, &nasty), Stage::Clear);
        assert!(symptoms_with(Stage::Symptoms, &nasty).len() > symptoms_with(Stage::Symptoms, &mild).len());
        assert!(symptoms_with(Stage::Incubating, &nasty).is_empty());
    }

    #[test]
    fn a_player_carries_the_strain_that_infected_them() {
        let mut state = PlayerState::default();
        assert_eq!(player_strain(&state), None, "здоров");
        let nasty = Strain::of([4, 2, 3, 2]);
        set_player_strain(&mut state, &nasty);
        assert_eq!(player_strain(&state), Some(nasty));
        let text = state_to_text(&state);
        assert_eq!(state_from_text(&text), state, "{text}");
    }

    #[test]
    fn the_strain_sticks_until_recovery() {
        let dir = std::env::temp_dir().join(format!("biolab-strain-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let uuid = 0x5241_4e53_u128;
        let nasty = Strain::of([5, 1, 4, 3]);
        assert_eq!(infect_world_player(&dir, uuid, &nasty, 20), Some(20));
        // Второй источник с другим штаммом поднимает уровень, но не подменяет
        // болезнь: лечиться игрок будет от первой.
        let other = Strain::of([1, 5, 1, 1]);
        assert_eq!(infect_world_player(&dir, uuid, &other, 10), Some(30));
        let state = state_from_text(&std::fs::read_to_string(state_path(&dir, uuid)).unwrap());
        assert_eq!(player_strain(&state), Some(nasty));
        assert_eq!(strain_or_wild(&PlayerState::default()), WILD);
        let lines = infection_lines(30, &nasty);
        assert!(lines[1].contains("симптомы"), "{lines:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_strain_rides_along_in_the_packet() {
        let mut state = PlayerState { infection: 44, ..Default::default() };
        let nasty = Strain::of([5, 1, 4, 2]);
        set_player_strain(&mut state, &nasty);
        let data = pharma_packet(&state, 1);
        assert_eq!(strain_from_packet(&data), nasty);
        // Пакет от старой версии мода без хвоста — дикая линия.
        assert_eq!(strain_from_packet(&[1, 2]), WILD);
        remember_strain(nasty);
        assert_eq!(shown_strain(), nasty);
        let lines = analyzer_lines_with(44, 0, &[DrugState::default(); DRUG_COUNT], &nasty);
        assert!(lines.iter().any(|l| l.starts_with("Штамм: ")), "{lines:?}");
    }

    #[test]
    fn every_gene_has_a_behaviour_row() {
        assert_eq!(GENE_BEHAVIOUR.len(), GENES.len());
        for (kind, flags) in GENES.iter().zip(GENE_BEHAVIOUR) {
            for flag in flags {
                assert!(!flag.is_empty(), "{}", kind.name);
            }
        }
        // Гены с поведением узнаются по набору.
        let genes = [("acid_glands", 2), ("loyalty", 1)];
        assert_eq!(behaviour_strength(&genes, "acid_aura"), 2);
        assert_eq!(behaviour_strength(&genes, "loyal"), 1);
        assert_eq!(behaviour_strength(&genes, "predator"), 0);
        assert_eq!(behaviour_strength(&[], "regen"), 0);
        assert_eq!(aura_of(0), 0.0);
        assert!(aura_of(3) > aura_of(1));
    }

    #[test]
    fn the_pool_is_wide_enough_to_build_with() {
        assert!(GENES.len() >= 30, "генов всего {}", GENES.len());
        assert_eq!(GENE_MARKERS.len(), GENES.len() * 3);
        // У каждого поведения есть хотя бы один носитель.
        for flag in [
            "regen", "fireproof", "acid_aura", "volatile", "necrosis", "toxic_blood",
            "thorns", "heal_aura", "chill_aura", "photophobia", "predator", "pack",
            "loyal", "toxin_proof", "rend",
        ] {
            assert!(
                GENE_BEHAVIOUR.iter().any(|flags| flags.contains(&flag)),
                "{flag} без гена"
            );
        }
    }

    #[test]
    fn daylight_is_the_first_half_of_the_day() {
        assert!(daylight(0) && daylight(6000) && daylight(11999));
        assert!(!daylight(13000) && !daylight(18000) && !daylight(23999));
        assert!(daylight(24000), "следующие сутки считаются так же");
        assert!(!daylight(-6000), "и предыдущие тоже");
    }

    #[test]
    fn thorns_hurt_by_strength() {
        assert!(thorns_damage(1) > 0.0);
        assert!(thorns_damage(3) > thorns_damage(1));
        assert_eq!(thorns_damage(9), thorns_damage(3), "сильнее трёх не бывает");
    }

    #[test]
    fn the_console_reports_subjects_and_trials() {
        let empty: Vec<(String, f32, f32, Vec<(&'static str, i32)>, bool)> = Vec::new();
        let lines = console_lines(&empty, &[]);
        assert_eq!(lines, ["Пульт наблюдения", "Вольеры пусты"]);

        let subjects = vec![
            ("minecraft:cow".to_string(), 7.0, 10.0, vec![("chitin", 2)], true),
            ("minecraft:pig".to_string(), 10.0, 10.0, Vec::new(), false),
        ];
        let lines = console_lines(&subjects, &["cow ← Пенициллин".to_string()]);
        assert!(lines.iter().any(|l| l.starts_with("cow: 7/10 HP, болен")), "{lines:?}");
        assert!(lines.iter().any(|l| l.contains("Хитиновый панцирь ++")), "{lines:?}");
        assert!(lines.iter().any(|l| l.contains("геном: чистый")), "{lines:?}");
        assert!(lines.iter().any(|l| l.contains("cow ← Пенициллин")), "{lines:?}");
    }

    #[test]
    fn the_trial_log_keeps_the_latest() {
        let dir = std::env::temp_dir().join(format!("biolab-trials-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for i in 0..120 {
            record_trial(&dir, &format!("опыт {i}"));
        }
        let last = trial_lines(&dir, TRIAL_LINES);
        assert_eq!(last.len(), TRIAL_LINES);
        assert_eq!(last[0], "опыт 119", "новые записи сверху");
        let all = trial_lines(&dir, 1000);
        assert_eq!(all.len(), 100, "журнал не растёт бесконечно");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_trial_record_names_what_was_given() {
        assert_eq!(trial_record("biolab:syringe_penicillin", "minecraft:cow"), "cow ← Пенициллин");
        assert!(trial_record("biolab:syringe_mutagen", "pig").contains("Мутаген"));
        assert!(trial_record(FRAGMENT_ID, "pig").contains("Фрагмент"));
        assert!(trial_record("minecraft:stick", "pig").contains("Препарат"));
    }

    #[test]
    fn a_pen_remembers_its_subject() {
        let pen = [7, -60, 7];
        assert_eq!(subject_of(pen), None);
        hold_subject(pen, 42);
        assert_eq!(subject_of(pen), Some(42));
        release_subject(pen);
        assert_eq!(subject_of(pen), None);
        let centre = pen_centre(pen);
        assert_eq!(centre, [7.5, -59.0, 7.5]);
    }

    #[test]
    fn sprays_and_darts_round_out_the_shelf() {
        let spray = medicine("biolab:spray_analgesic").expect("аэрозоль есть");
        assert_eq!(spray.form, FORM_SPRAY);
        assert!(FORMS[spray.form].direct, "аэрозоль идёт сразу в кровь");
        // Каждая форма хоть кем-то выпускается.
        for (i, form) in FORMS.iter().enumerate() {
            assert!(MEDICINES.iter().any(|m| m.form == i), "{} не выпускают", form.id);
        }
        // Дротик — это тот же шприц, только издали.
        for (dart, syringe) in DARTS {
            assert_eq!(dart_syringe(dart), Some(syringe));
            assert!(mob_injection(syringe).is_some(), "{syringe} ничего не делает мобу");
        }
        assert_eq!(dart_syringe("biolab:syringe_venom"), None);
    }

    #[test]
    fn a_broken_course_hardens_the_strain() {
        let mut state = PlayerState { infection: 40, ..Default::default() };
        set_player_strain(&mut state, &WILD);
        // Лечение шло и оборвалось — штамм привыкает.
        assert!(course_broken(&state, Level::Therapeutic, Level::Subtherapeutic));
        // Долечили до нуля — претензий нет.
        let cured = PlayerState { infection: 0, ..state };
        assert!(!course_broken(&cured, Level::Therapeutic, Level::Absent));
        // Лечения и не было.
        assert!(!course_broken(&state, Level::Absent, Level::Absent));

        let harder = harden(&WILD);
        assert_eq!(harder.resistance, WILD.resistance + 1);
        assert_eq!(harder.infectivity, WILD.infectivity, "остальное не трогаем");
        assert_eq!(harden(&Strain::of([1, 1, 1, 5])).resistance, 5, "выше пяти некуда");
        assert!(strain_relief(&harder) < strain_relief(&WILD), "лечить стало труднее");
    }

    #[test]
    fn the_journal_keeps_names_and_what_is_still_unknown() {
        let dir = std::env::temp_dir().join(format!("biolab-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        assert!(remember_compound(&dir, "H2O"), "первая запись");
        assert!(!remember_compound(&dir, "H2O"), "повтор не пишется");
        assert!(remember_compound(&dir, "CFe2"));
        assert_eq!(journal_count(&dir), 2);
        assert!(!is_identified(&dir, "H2O"), "собрал — ещё не значит опознал");
        assert_eq!(compound_display(&dir, "H2O"), None);
        let lines = journal_lines(&dir);
        assert!(lines.iter().any(|l| l.contains("H2O — не опознано")), "{lines:?}");

        // Спектрометр закрывает запись: известному веществу — его имя.
        assert_eq!(identify_compound(&dir, "H2O").as_deref(), Some("вода"));
        assert!(is_identified(&dir, "H2O"));
        assert_eq!(compound_display(&dir, "H2O").as_deref(), Some("вода"));
        assert_eq!(identify_compound(&dir, "H2O"), None, "второй раз нечего опознавать");
        // Незнакомому — номер открытия.
        assert_eq!(identify_compound(&dir, "CFe2").as_deref(), Some("соединение №2"));

        // Своё имя закрепляется за веществом.
        assert!(rename_compound(&dir, "CFe2", "карбид Нанди"));
        assert_eq!(compound_display(&dir, "CFe2").as_deref(), Some("карбид Нанди"));
        assert!(!rename_compound(&dir, "CFe2", "   "), "пустое имя не принимается");
        assert!(!rename_compound(&dir, "H2S", "что-то"), "такого открытия нет");

        // Журнал переживает перечитывание.
        let entries = discoveries(&dir);
        assert_eq!(entries.len(), 2);
        assert_eq!(discovery_from_line(&discovery_to_line(&entries[0])), Some(entries[0].clone()));
        // Старая запись из одной формулы читается как неопознанная.
        assert_eq!(
            discovery_from_line("C2H6O"),
            Some(Discovery { formula: "C2H6O".into(), name: String::new(), identified: false })
        );
        assert_eq!(discovery_from_line(""), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_blueprint_says_what_it_builds() {
        assert_eq!(blueprint_label("H2O", Some("вода")), "Чертёж: вода (H2O)");
        assert_eq!(blueprint_label("CFe2", None), "Чертёж: CFe2");
    }

    #[test]
    fn the_canon_is_wide_enough() {
        assert!(KNOWN.len() >= 30, "канонических веществ всего {}", KNOWN.len());
    }


    #[test]
    fn the_manual_comes_once() {
        let dir = std::env::temp_dir().join(format!("biolab-manual-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let uuid = 0x4d41_4e55_u128;
        assert!(take_manual(&dir, uuid), "первый вход в мир");
        assert!(!take_manual(&dir, uuid), "второй раз книга не приходит");
        let state = state_from_text(&std::fs::read_to_string(state_path(&dir, uuid)).unwrap());
        assert!(state.manual, "отметка пережила запись на диск");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_page_breaks_into_drawable_lines() {
        let page = "# Машины\n\nВсе работают как печка.\n@item biolab:still biolab:reactor\n@craft microscope";
        let lines = page_lines(page);
        assert_eq!(lines[0], Line::Title("Машины".to_string()));
        assert_eq!(lines[1], Line::Gap);
        assert_eq!(lines[2], Line::Text("Все работают как печка.".to_string()));
        assert_eq!(
            lines[3],
            Line::Items(vec!["biolab:still".to_string(), "biolab:reactor".to_string()])
        );
        assert_eq!(lines[4], Line::Craft("microscope".to_string()));
        assert_eq!(page_title(page), "Машины");
        assert_eq!(page_title("Просто строка\nвторая"), "Просто строка");
        assert_eq!(page_title(""), "Страница");
    }


    #[test]
    fn single_atom_is_not_a_compound() {
        let mut f = Formula::default();
        f[0] = 1;
        assert!(!is_stable(&f));
    }

    #[test]
    fn water_holds_together() {
        let mut f = Formula::default();
        f[0] = 2; // H
        f[3] = 1; // O
        assert!(is_stable(&f));
        assert_eq!(formula_text(&f), "H2O");
        assert_eq!(compound_name_of(&f), "вода");
        assert_eq!(state_of(&f), State::Gas);
    }

    #[test]
    fn odd_bond_count_falls_apart() {
        let mut f = Formula::default();
        f[0] = 1; // H
        f[3] = 1; // O
        assert!(!is_stable(&f), "одна связь остаётся свободной");
    }

    #[test]
    fn carbon_needs_partners() {
        let mut f = Formula::default();
        f[1] = 1; // C, четыре связи
        f[0] = 2; // H, две связи
        assert!(!is_stable(&f), "углероду не хватает партнёров");
        f[0] = 4;
        assert!(is_stable(&f));
        assert_eq!(formula_text(&f), "CH4");
    }

    #[test]
    fn organic_formulas_start_with_carbon() {
        let mut f = Formula::default();
        f[1] = 6; // C
        f[0] = 12; // H
        f[3] = 6; // O
        assert_eq!(formula_text(&f), "C6H12O6");
        assert_eq!(compound_name_of(&f), "глюкоза");
        assert!(is_nutritious(&f));
        assert_eq!(state_of(&f), State::Solid);
    }

    #[test]
    fn bioactivity_needs_carbon_nitrogen_and_oxygen() {
        let mut f = Formula::default();
        f[1] = 6;
        f[0] = 12;
        f[3] = 6;
        assert_eq!(bioactivity(&f), 0, "сахар лекарством не станет");
        f[2] = 4; // N
        assert!(bioactivity(&f) > 0);
    }

    #[test]
    fn unknown_formula_keeps_its_own_name() {
        // Углерод с железом в таблице известных не значится.
        let mut f = Formula::default();
        f[1] = 1; // C
        f[7] = 2; // Fe
        assert_eq!(compound_name_of(&f), "CFe2");
        // А вот окалина меди теперь каноническая и своё имя имеет.
        let mut known = Formula::default();
        known[8] = 1; // Cu
        known[3] = 1; // O
        assert_eq!(compound_name_of(&known), "окалина меди");
        assert_eq!(colour_of(&known), 0x4FB8C8, "медь красит в голубой");
    }

    #[test]
    fn machine_table_knows_its_blocks() {
        assert!(is_machine("biolab:still"));
        assert!(is_machine("biolab:incubator"));
        assert!(!is_machine("biolab:flask"));
        assert!(!is_machine("minecraft:furnace"));
    }

    #[test]
    fn machine_waits_without_a_recipe() {
        assert_eq!(step(None, 0), Step::Idle);
        assert_eq!(step(Some(0), 5), Step::Idle, "нулевое время — не рецепт");
    }

    #[test]
    fn machine_counts_up_then_finishes() {
        assert_eq!(step(Some(3), 0), Step::Working(1));
        assert_eq!(step(Some(3), 1), Step::Working(2));
        assert_eq!(step(Some(3), 2), Step::Finish);
    }

    #[test]
    fn one_tick_recipe_finishes_at_once() {
        assert_eq!(step(Some(1), 0), Step::Finish);
    }

    #[test]
    fn output_slot_must_have_room() {
        assert!(output_fits(None, ("biolab:agar", 1), 16));
        assert!(output_fits(Some(("biolab:agar", 15)), ("biolab:agar", 1), 16));
        assert!(!output_fits(Some(("biolab:agar", 16)), ("biolab:agar", 1), 16));
        assert!(!output_fits(Some(("minecraft:stick", 1)), ("biolab:agar", 1), 16));
    }

    #[test]
    fn remedy_message_says_what_happened() {
        assert_eq!(remedy_message(50, 10), "Заражение: 50% → 10%");
        assert_eq!(remedy_message(30, 0), "Заражение снято (было 30%)");
    }

    #[test]
    fn tainted_food_is_recognised() {
        assert_eq!(tainted_food("minecraft:rotten_flesh"), Some(15));
        assert_eq!(tainted_food("minecraft:bread"), None);
    }

    #[test]
    fn exposure_adds_up_to_a_hundred() {
        assert_eq!(after_exposure(0, 15), 15);
        assert_eq!(after_exposure(95, 15), 100, "выше ста не растёт");
    }

    #[test]
    fn analyzer_shows_level_and_stage() {
        let empty = [DrugState::default(); DRUG_COUNT];
        let lines = analyzer_lines(0, 3, &empty);
        assert_eq!(lines[1], "Заражение: 0%");
        assert_eq!(lines[2], "Стадия: здоров");
        assert_eq!(analyzer_lines(80, 0, &empty)[2], "Стадия: кризис");
        assert_eq!(lines[3], "Журнал: 3 веществ");
        assert_eq!(lines.len(), 4, "пустая кровь не занимает строк");
        // Вещество в крови добавляет свою строку.
        let mut drugs = empty;
        drugs[2].blood = 30;
        let lines = analyzer_lines(0, 0, &drugs);
        assert!(lines.last().unwrap().starts_with("Стимулятор: 30"), "{lines:?}");
    }

    #[test]
    fn stages_follow_the_level() {
        assert_eq!(stage(0), Stage::Clear);
        assert_eq!(stage(1), Stage::Incubating);
        assert_eq!(stage(30), Stage::Symptoms);
        assert_eq!(stage(70), Stage::Crisis);
        assert_eq!(stage(100), Stage::Crisis);
    }

    #[test]
    fn healthy_player_has_no_symptoms() {
        assert!(symptoms(stage(0)).is_empty());
        assert!(symptoms(stage(10)).is_empty(), "инкубация ещё без симптомов");
        assert!(!symptoms(stage(50)).is_empty());
    }

    #[test]
    fn remedy_lowers_the_level_and_can_cure() {
        assert_eq!(after_remedy(100), 60);
        assert_eq!(after_remedy(40), 0);
        assert_eq!(after_remedy(10), 0, "один препарат добивает лёгкую стадию");
        assert_eq!(stage(after_remedy(30)), Stage::Clear);
    }

    #[test]
    fn infection_grows_only_once_started() {
        assert_eq!(after_step(0), 0, "здоровый не заболевает сам собой");
        assert_eq!(after_step(1), 1 + INFECTION_STEP);
        assert_eq!(after_step(100), 100, "выше ста не растёт");
    }

    #[test]
    fn command_name_drops_arguments() {
        assert_eq!(command_name("biolab status"), "biolab");
        assert_eq!(command_name("biolab"), "biolab");
        assert_eq!(command_name(""), "");
    }

    // ---- Роли машин ----

    #[test]
    fn hoppers_feed_the_fuel_slot_only_to_the_vat() {
        for machine in &MACHINES {
            let inputs = machine_inputs(machine);
            let outputs = machine_outputs(machine);
            assert!(!inputs.is_empty() && !outputs.is_empty(), "{}", machine.id);
            assert!(inputs.iter().chain(&outputs).all(|s| *s < machine.slots), "{}", machine.id);
            assert!(inputs.iter().all(|s| !outputs.contains(s)), "{}", machine.id);
            if machine.menu == "minecraft:furnace" {
                assert_eq!(inputs.contains(&SLOT_FUEL), machine.id == CLONING_VAT, "{}", machine.id);
                assert_eq!(outputs, vec![SLOT_OUTPUT], "{}", machine.id);
            }
        }
        let centrifuge = machine(CENTRIFUGE).unwrap();
        assert_eq!(machine_inputs(centrifuge), vec![0]);
        assert_eq!(machine_outputs(centrifuge), vec![1, 2, 3, 4]);
        let synth = machine(SYNTHESIZER).unwrap();
        assert_eq!(machine_inputs(synth), (0..8).collect::<Vec<_>>());
        assert_eq!(machine_outputs(synth), vec![8]);
    }

    #[test]
    fn a_flask_falls_apart_into_its_elements() {
        let salt = decompose("ClNa", |_| 64).unwrap();
        assert_eq!(salt.stacks, vec![("biolab:element_sodium", 1), ("biolab:element_chlorine", 1)]);
        assert_eq!(salt.atoms, 2);
        let acid = decompose("H2O4S", |_| 64).unwrap();
        assert_eq!(
            acid.stacks,
            vec![("biolab:element_hydrogen", 2), ("biolab:element_oxygen", 4), ("biolab:element_sulfur", 1)]
        );
        assert_eq!(assembly_time(acid.atoms), 40 + 10 * 7);
        // Стак больше стопки делится на два.
        let big = decompose("C70H2", |_| 64).unwrap();
        assert_eq!(
            big.stacks,
            vec![("biolab:element_hydrogen", 2), ("biolab:element_carbon", 64), ("biolab:element_carbon", 6)]
        );
        assert_eq!(assembly_time(big.atoms), 600, "потолок времени");
    }

    #[test]
    fn water_and_crowded_flasks_stay_whole() {
        assert_eq!(decompose(DEFAULT_FILL.0, |_| 64), None, "вода дала бы водород без конца");
        assert_eq!(decompose("OH2", |_| 64), None, "вода в любой записи");
        assert_eq!(decompose("C8H10N4O2", |_| 64), None, "четыре элемента: выходов не хватит");
        assert!(decompose("C6H12O6", |_| 64).is_some(), "три — в самый раз");
        assert_eq!(decompose("water", |_| 64), None, "старая колба со словом");
    }

    #[test]
    fn outputs_join_their_stack_or_take_a_free_slot() {
        let p = |item, count, plain| Pile { item, count, plain };
        let slots = [Some(p("minecraft:flint", 63, true)), None, Some(p("minecraft:sand", 1, true)), None];
        // Кремень в полный стак не влезает — в свободный; песок — к песку.
        let plan = plan_outputs(&slots, &[p("minecraft:flint", 2, true), p("minecraft:sand", 1, true)], |_| 64);
        assert_eq!(plan, Some(vec![1, 2]));
        // Два выхода подряд в один свободный слот: второй ложится к первому.
        let plan = plan_outputs(&[None, Some(p("x", 1, false))], &[p("a", 1, true), p("a", 2, true)], |_| 64);
        assert_eq!(plan, Some(vec![0, 0]));
        // Колба с данными не сливается даже с такой же колбой.
        let flask = p(FLASK_ID, 1, false);
        assert_eq!(plan_outputs(&[Some(flask), None], &[flask], |_| 16), Some(vec![1]));
        assert_eq!(plan_outputs(&[Some(flask)], &[flask], |_| 16), None);
        // Не влезает хоть что-то — машина ждёт.
        let full = [Some(p("y", 1, true)), Some(p("z", 1, true)), None, None];
        let piles = [p("a", 1, true), p("b", 1, true), p("c", 1, true)];
        assert_eq!(plan_outputs(&full, &piles, |_| 64), None);
    }

    #[test]
    fn a_full_flask_needs_all_four_outputs() {
        // Глюкоза: три элемента и колба с водой — ровно четыре выхода.
        let sugar = decompose("C6H12O6", |_| 64).unwrap();
        let mut piles: Vec<Pile> = sugar.stacks.iter().map(|(item, count)| Pile { item, count: *count, plain: true }).collect();
        piles.push(Pile { item: FLASK_ID, count: 1, plain: false });
        assert_eq!(plan_outputs(&[None; 4], &piles, |_| 64), Some(vec![0, 1, 2, 3]));
        let busy = [None, None, None, Some(Pile { item: "minecraft:flint", count: 1, plain: true })];
        assert_eq!(plan_outputs(&busy, &piles, |_| 64), None);
    }

    #[test]
    fn extractor_recipes_name_real_genes() {
        assert_eq!(recipe_gene("spring_legs", None), Some(("spring_legs", 1)));
        assert_eq!(recipe_gene("thermal", Some(2)), Some(("thermal", 2)));
        assert_eq!(recipe_gene("thermal", Some(4)), None);
        assert_eq!(recipe_gene("wings", Some(1)), None);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/recipe");
        let mut animals = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let j = Json::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
            if j.get("type").and_then(Json::str) != Some(EXTRACTOR) {
                continue;
            }
            assert_eq!(j.get("result").and_then(|r| r.get("id")).and_then(Json::str), Some(FRAGMENT_ID), "{}", path.display());
            if let Some(name) = j.get("gene") {
                let strength = j.get("strength").and_then(Json::num).map(|n| n as i64);
                assert!(recipe_gene(name.str().unwrap_or(""), strength).is_some(), "{}", path.display());
                animals += 1;
            }
        }
        assert_eq!(animals, 6, "шесть животных из GDD 4.15");
    }

    #[test]
    fn machines_keep_to_their_roles() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/recipe");
        let mut by_result = std::collections::HashMap::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
            if let Some(RecipeView::Machine { input, machine, result, byproduct, .. }) = recipe_view(&text) {
                // Побочный выход бывает только у центрифуги.
                assert!(byproduct.is_none() || machine == CENTRIFUGE, "{result}");
                by_result.insert((input, result), machine);
            }
        }
        let made_by = |input: &str, result: &str| by_result.get(&(input.to_string(), result.to_string())).map(String::as_str);
        for (input, extract) in [
            ("minecraft:birch_leaves", "biolab:extract_salicylate"),
            ("minecraft:poppy", "biolab:extract_sedative"),
            ("minecraft:cocoa_beans", "biolab:extract_caffeine"),
            ("minecraft:spider_eye", "minecraft:fermented_spider_eye"),
            ("minecraft:wheat", BROTH_ID),
        ] {
            assert_eq!(made_by(input, extract), Some("biolab:fermenter"), "{extract}: живое — в ферментер");
        }
        for (input, result) in [
            ("minecraft:magma_cream", "minecraft:slime_ball"),
            ("minecraft:gravel", "minecraft:flint"),
            ("minecraft:glistering_melon_slice", "minecraft:melon_slice"),
        ] {
            assert_eq!(made_by(input, result), Some(CENTRIFUGE), "{result}");
        }
    }

    #[test]
    fn a_crop_gives_up_its_strongest_gene() {
        assert_eq!(strongest_gene(&[]), None);
        assert_eq!(strongest_gene(&[("dwarfism", 1), ("chitin", 2), ("albinism", 3)]), Some(("albinism", 3)));
        // Равная сила: дороже — ценнее.
        assert_eq!(strongest_gene(&[("dwarfism", 2), ("chitin", 2)]), Some(("chitin", 2)));
        // Полное равенство: первый по геному.
        assert_eq!(strongest_gene(&[("chitin", 1), ("thermal", 1)]), Some(("chitin", 1)));
        assert_eq!(strongest_gene(&genome_from_text("gigantism:1,thermal:2")), Some(("thermal", 2)));
    }

    #[test]
    fn screens_get_genes_strains_and_synth_status_as_numbers() {
        assert_eq!(gene_data(None), (-1, 0));
        assert_eq!(gene_data(Some("chitin:2")), (3, 2));
        assert_eq!(gene_data(Some("мусор")), (-1, 0));
        assert_eq!(strain_data(None, None), [0; 4]);
        assert_eq!(strain_data(Some("biolab:swab"), Some("5,5,5,5")), [0; 4]);
        assert_eq!(strain_data(Some(CULTURE_ID), None), WILD.values());
        assert_eq!(strain_data(Some(CULTURE_ID), Some("3,2,4,1")), [3, 2, 4, 1]);

        let water = formula_from_text("H2O").unwrap();
        let hydrogen = formula_from_text("H").unwrap();
        assert_eq!(synth_status(&Formula::default(), None), SYNTH_EMPTY);
        assert_eq!(synth_status(&water, None), SYNTH_STABLE);
        assert_eq!(synth_status(&hydrogen, None), SYNTH_UNSTABLE);
        assert_eq!(synth_status(&water, Some(&water)), SYNTH_STABLE);
        assert_eq!(synth_status(&water, Some(&formula_from_text("H2O2").unwrap())), SYNTH_NOT_BLUEPRINT);
        // Числа влезают в отведённые машине слоты данных.
        assert_eq!(machine_data_slots(SYNTHESIZER), DATA_SYNTH_ELEMENTS + water.len());
        assert_eq!(machine_data_slots(EXTRACTOR), DATA_GENE_STRENGTH + 1);
        assert_eq!(machine_data_slots(CENTRIFUGE), DATA_SLOTS);
    }
}

// ---- Карта исследований ----
//
// «Журнал лаборанта» — карта, как в Таумономиконе: пять наук, в каждой узлы,
// связанные линиями. Узел загорается сам, когда игрок сделал что-то новое:
// получил предмет, опознал вещество, заразился, принял лекарство. Здесь
// только данные и чистая логика: таблица узлов, условия, раскладка экрана,
// попадание мышью и разбор страниц. Рисует и ловит мышь экран в `biolab-deep`,
// который подменяет собой ванильную книгу.

/// Канал, по которому сервер раз в секунду сообщает клиенту изученные узлы.
pub const NET_RESEARCH: &str = "biolab_core:research";
/// Тексты записей: по блоку на узел, правятся без пересборки.
pub const RESEARCH_PATH: &str = "data/biolab/research/ru.txt";

/// Наука — вкладка карты.
pub struct Science {
    pub title: &'static str,
    pub icon: &'static str,
}

pub const SCIENCES: [Science; 5] = [
    Science { title: "Основы", icon: "biolab:lab_manual" },
    Science { title: "Химия", icon: "biolab:element_carbon" },
    Science { title: "Микробиология", icon: "biolab:culture" },
    Science { title: "Фармакология", icon: "biolab:pill_penicillin" },
    Science { title: "Генетика", icon: "biolab:dna_fragment" },
];

/// Что зажигает узел.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unlock {
    /// Изучен с самого начала.
    Start,
    /// В инвентаре оказался любой из этих предметов.
    Item(&'static [&'static str]),
    /// В журнале хотя бы столько веществ.
    Discovered(u32),
    /// Опознано хотя бы столько веществ.
    Identified(u32),
    /// Игрок заражён.
    Infected,
    /// Штамм игрока стал стойче дикого: курс брошен на полпути.
    Hardened,
    /// Держится иммунитет от вакцины.
    Immune,
    /// В крови есть лекарство.
    Medicine,
    /// Передозировка или ломка.
    Overdose,
    /// У игрока есть хотя бы один ген.
    Gene,
}

/// Узел карты.
pub struct Research {
    pub id: &'static str,
    /// Номер науки в `SCIENCES`.
    pub science: usize,
    /// Клетка на карте: столбец 0–5, строка 0–3.
    pub at: (u8, u8),
    pub icon: &'static str,
    pub title: &'static str,
    /// Узлы той же науки, после которых этот становится доступен.
    pub after: &'static [&'static str],
    pub unlock: Unlock,
    /// Подсказка, как узел открыть.
    pub hint: &'static str,
}

const ELEMENT_ITEMS: &[&str] = &[
    "biolab:element_hydrogen", "biolab:element_carbon", "biolab:element_nitrogen",
    "biolab:element_oxygen", "biolab:element_sodium", "biolab:element_magnesium",
    "biolab:element_silicon", "biolab:element_phosphorus", "biolab:element_sulfur",
    "biolab:element_chlorine", "biolab:element_potassium", "biolab:element_calcium",
    "biolab:element_iron", "biolab:element_copper", "biolab:element_iodine",
    "biolab:element_gold", "biolab:element_glaurium", "biolab:element_redstonium",
];

const fn node(
    id: &'static str,
    science: usize,
    at: (u8, u8),
    icon: &'static str,
    title: &'static str,
    after: &'static [&'static str],
    unlock: Unlock,
    hint: &'static str,
) -> Research {
    Research { id, science, at, icon, title, after, unlock, hint }
}

pub const RESEARCH: [Research; 47] = [
    // Основы
    node("manual", 0, (0, 1), "biolab:lab_manual", "Журнал лаборанта", &[], Unlock::Start, "Есть с самого начала."),
    node("bench", 0, (1, 1), "biolab:lab_bench", "Лабораторный стол", &["manual"], Unlock::Item(&["biolab:lab_bench"]), "Соберите лабораторный стол."),
    node("flask", 0, (2, 0), "biolab:flask", "Колба", &["bench"], Unlock::Item(&["biolab:flask"]), "Сделайте колбу."),
    node("analyzer", 0, (3, 0), "biolab:analyzer", "Анализатор крови", &["flask"], Unlock::Item(&["biolab:analyzer"]), "Соберите анализатор крови."),
    node("swab", 0, (2, 2), "biolab:swab", "Ватная палочка", &["bench"], Unlock::Item(&["biolab:swab"]), "Сделайте ватную палочку."),
    node("agar", 0, (3, 2), "biolab:agar", "Агар", &["swab"], Unlock::Item(&["biolab:agar"]), "Перегоните сушёную ламинарию в агар."),
    node("bandage", 0, (4, 2), "biolab:bandage", "Бинт", &["agar"], Unlock::Item(&["biolab:bandage"]), "Сделайте бинт из шерсти и агара."),
    // Химия
    node("still", 1, (0, 1), "biolab:still", "Перегонный куб", &[], Unlock::Item(&["biolab:still"]), "Соберите перегонный куб."),
    node("elements", 1, (1, 1), "biolab:element_carbon", "Элементы", &["still"], Unlock::Item(ELEMENT_ITEMS), "Добудьте любой элемент в перегонном кубе."),
    node("synth", 1, (2, 1), "biolab:synthesizer", "Синтезатор", &["elements"], Unlock::Item(&["biolab:synthesizer"]), "Соберите синтезатор."),
    node("bonds", 1, (2, 0), "biolab:element_hydrogen", "Правило связей", &["synth"], Unlock::Discovered(1), "Соберите в синтезаторе любое вещество."),
    node("spectro", 1, (3, 1), "biolab:spectrometer", "Спектрометр", &["synth"], Unlock::Item(&["biolab:spectrometer"]), "Соберите спектрометр."),
    node("identify", 1, (4, 0), "biolab:flask", "Опознание", &["spectro"], Unlock::Identified(1), "Опознайте вещество спектрометром."),
    node("blueprint", 1, (4, 1), "biolab:blueprint", "Чертежи", &["spectro"], Unlock::Item(&["biolab:blueprint"]), "Получите чертёж из спектрометра."),
    node("reactor", 1, (3, 2), "biolab:reactor", "Реактор", &["synth"], Unlock::Item(&["biolab:reactor"]), "Соберите реактор."),
    node("collection", 1, (5, 0), "biolab:element_gold", "Десять веществ", &["identify"], Unlock::Identified(10), "Опознайте десять веществ."),
    node("separation", 1, (2, 2), "biolab:centrifuge", "Разделение", &["synth"], Unlock::Item(&["biolab:centrifuge"]), "Соберите центрифугу."),
    // Микробиология
    node("incubator", 2, (0, 1), "biolab:incubator", "Инкубатор", &[], Unlock::Item(&["biolab:incubator"]), "Соберите инкубатор."),
    node("culture", 2, (1, 1), "biolab:culture", "Культура", &["incubator"], Unlock::Item(&["biolab:culture"]), "Вырастите культуру из мазка."),
    node("strain", 2, (2, 0), "biolab:microscope", "Штамм", &["culture"], Unlock::Item(&["biolab:microscope"]), "Соберите микроскоп."),
    node("weakened", 2, (2, 1), "biolab:culture_weakened", "Ослабленная культура", &["culture"], Unlock::Item(&["biolab:culture_weakened"]), "Нагрейте культуру в реакторе."),
    node("vaccine", 2, (3, 1), "biolab:syringe_vaccine", "Вакцина", &["weakened"], Unlock::Item(&["biolab:syringe_vaccine"]), "Наберите вакцину в шприц."),
    node("immunity", 2, (4, 1), "minecraft:golden_apple", "Иммунитет", &["vaccine"], Unlock::Immune, "Привейтесь вакциной."),
    node("infection", 2, (0, 3), "minecraft:rotten_flesh", "Заражение", &[], Unlock::Infected, "Подхватите заразу: гнилая плоть, зомби рядом, лужа."),
    node("remedy", 2, (1, 3), "biolab:remedy", "Лекарство", &["infection"], Unlock::Item(&["biolab:remedy"]), "Сделайте лекарство из культуры."),
    node("hardened", 2, (2, 3), "minecraft:fermented_spider_eye", "Стойкий штамм", &["remedy"], Unlock::Hardened, "Бросьте курс лечения на полпути."),
    // Фармакология
    node("mold", 3, (0, 1), "biolab:mold", "Плесень", &[], Unlock::Item(&["biolab:mold"]), "Положите хлеб в инкубатор."),
    node("fermenter", 3, (1, 1), "biolab:fermenter", "Ферментер", &["mold"], Unlock::Item(&["biolab:fermenter"]), "Соберите ферментер."),
    node("extracts", 3, (2, 1), "biolab:extract_penicillin", "Экстракты", &["fermenter"], Unlock::Item(&["biolab:extract_penicillin", "biolab:extract_salicylate", "biolab:extract_caffeine", "biolab:extract_sedative", "biolab:extract_venom", "biolab:extract_raw"]), "Получите любой экстракт."),
    node("pills", 3, (3, 0), "biolab:press", "Таблетки", &["extracts"], Unlock::Item(&["biolab:pill_penicillin", "biolab:pill_analgesic", "biolab:pill_stimulant", "biolab:pill_antidote"]), "Спрессуйте таблетку."),
    node("injections", 3, (3, 1), "biolab:syringe", "Инъекции", &["extracts"], Unlock::Item(&["biolab:syringe_penicillin", "biolab:syringe_sedative", "biolab:syringe_stimulant", "biolab:syringe_venom"]), "Наберите лекарство в шприц."),
    node("topical", 3, (3, 2), "biolab:ointment_analgesic", "Мази и аэрозоли", &["extracts"], Unlock::Item(&["biolab:ointment_analgesic", "biolab:spray_analgesic", "biolab:spray_stimulant"]), "Сделайте мазь или аэрозоль."),
    node("antidote", 3, (2, 3), "biolab:pill_antidote", "Антидот", &["extracts"], Unlock::Item(&["biolab:pill_antidote"]), "Спрессуйте антидот из угля."),
    node("window", 3, (4, 0), "biolab:pill_analgesic", "Терапевтическое окно", &["pills"], Unlock::Medicine, "Примите любое лекарство."),
    node("overdose", 3, (5, 0), "biolab:pill_stimulant", "Передозировка", &["window"], Unlock::Overdose, "Перестарайтесь с дозой или бросьте стимулятор."),
    node("darts", 3, (4, 1), "biolab:dart_sedative", "Дротики", &["injections"], Unlock::Item(&["biolab:dart_sedative", "biolab:dart_venom"]), "Сделайте дротик."),
    // Генетика
    node("scalpel", 4, (0, 1), "biolab:scalpel", "Скальпель", &[], Unlock::Item(&["biolab:scalpel"]), "Сделайте скальпель."),
    node("tissue", 4, (1, 1), "biolab:mutant_tissue", "Ткань мутанта", &["scalpel"], Unlock::Item(&["biolab:mutant_tissue"]), "Добудьте мутанта скальпелем."),
    node("extractor", 4, (2, 1), "biolab:extractor", "Экстрактор ДНК", &["tissue"], Unlock::Item(&["biolab:extractor"]), "Соберите экстрактор ДНК."),
    node("fragment", 4, (3, 1), "biolab:dna_fragment", "Фрагмент ДНК", &["extractor"], Unlock::Item(&["biolab:dna_fragment"]), "Достаньте ген из ткани."),
    node("mutagen", 4, (2, 3), "biolab:mutagen", "Мутаген", &["extractor"], Unlock::Item(&["biolab:mutagen", "biolab:mutagen_bucket", "biolab:syringe_mutagen"]), "Сделайте мутаген."),
    node("splicing", 4, (4, 0), "biolab:microscope", "Сплайсинг", &["fragment"], Unlock::Item(&["biolab:microscope"]), "Соберите микроскоп."),
    node("vat", 4, (5, 0), "biolab:cloning_vat", "Клонирующий чан", &["splicing"], Unlock::Item(&["biolab:cloning_vat"]), "Соберите клонирующий чан."),
    node("pen", 4, (5, 1), "biolab:pen", "Вольер и пульт", &["vat"], Unlock::Item(&["biolab:pen", "biolab:console"]), "Сделайте вольер или пульт."),
    node("therapy", 4, (4, 2), "biolab:syringe", "Генотерапия", &["fragment"], Unlock::Gene, "Примените фрагмент ДНК на себе."),
    node("crops", 4, (5, 2), "biolab:gm_plot", "ГМ-грядка", &["therapy"], Unlock::Item(&["biolab:gm_plot", "biolab:gm_crop"]), "Сделайте ГМ-грядку."),
    node("animal_genes", 4, (3, 2), "minecraft:rabbit_foot", "Гены животных", &["extractor"], Unlock::Item(&["minecraft:rabbit_foot", "minecraft:glow_ink_sac", "minecraft:turtle_scute", "minecraft:magma_cream"]), "Добудьте лапку кролика, щиток черепахи или магму."),
];

/// Номер узла по id.
pub fn research_index(id: &str) -> Option<usize> {
    RESEARCH.iter().position(|r| r.id == id)
}

/// Бит узла в маске изученного.
pub fn research_bit(index: usize) -> u64 {
    1u64 << index
}

/// Что игрок сейчас держит и что записано в журнале: всё, что нужно
/// условиям, кроме самого состояния игрока.
pub struct ResearchFacts<'a> {
    pub items: &'a [String],
    pub discovered: u32,
    pub identified: u32,
}

/// Выполнено ли условие узла.
pub fn research_met(r: &Research, s: &PlayerState, f: &ResearchFacts) -> bool {
    match r.unlock {
        Unlock::Start => true,
        Unlock::Item(ids) => f.items.iter().any(|item| ids.contains(&item.as_str())),
        Unlock::Discovered(n) => f.discovered >= n,
        Unlock::Identified(n) => f.identified >= n,
        Unlock::Infected => s.infection > 0,
        Unlock::Hardened => s.strain[0] > 0 && i32::from(s.strain[3]) > WILD.resistance,
        Unlock::Immune => s.immunity > 0,
        Unlock::Medicine => s.drugs.iter().any(|d| d.blood > 0),
        Unlock::Overdose => {
            s.withdrawal > 0
                || DRUGS.iter().zip(&s.drugs).any(|(drug, state)| {
                    state.blood > window_with_tolerance(drug.window, state.tolerance).1
                })
        }
        Unlock::Gene => s.genes.iter().any(|g| *g > 0),
    }
}

/// Маска узлов, чьи условия выполнены прямо сейчас.
pub fn research_now(s: &PlayerState, f: &ResearchFacts) -> u64 {
    RESEARCH
        .iter()
        .enumerate()
        .filter(|(_, r)| research_met(r, s, f))
        .fold(0, |mask, (i, _)| mask | research_bit(i))
}

/// Изученное: записать новые узлы в состояние игрока и сохранить. Вернёт
/// всю маску и номера только что открытых узлов.
pub fn research_tick(
    world: &std::path::Path,
    uuid: u128,
    facts: &ResearchFacts,
) -> (u64, Vec<usize>) {
    let Ok(mut table) = states().lock() else {
        return (0, Vec::new());
    };
    let state = table.entry((world.to_path_buf(), uuid)).or_default();
    let fresh = research_now(state, facts) & !state.research;
    if fresh != 0 {
        state.research |= fresh;
        let path = state_path(world, uuid);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, state_to_text(state));
    }
    let opened = (0..RESEARCH.len())
        .filter(|i| fresh & research_bit(*i) != 0)
        .collect();
    (state.research, opened)
}

/// Сообщение в чат об открытом узле.
pub fn research_message(index: usize) -> String {
    format!("Журнал лаборанта: новая запись «{}»", RESEARCH[index].title)
}

/// Маска изученного в файле: id через запятую, чтобы перестановка таблицы
/// не путала сохранения.
pub fn research_to_text(mask: u64) -> String {
    RESEARCH
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & research_bit(*i) != 0)
        .map(|(_, r)| r.id)
        .collect::<Vec<_>>()
        .join(",")
}

/// Маска из файла: незнакомые id пропускаем.
pub fn research_from_text(text: &str) -> u64 {
    text.split(',')
        .filter_map(|id| research_index(id.trim()))
        .fold(0, |mask, i| mask | research_bit(i))
}

/// Пакет для клиента: маска, восемь байт.
pub fn research_packet(mask: u64) -> Vec<u8> {
    mask.to_le_bytes().to_vec()
}

pub fn research_from_packet(data: &[u8]) -> Option<u64> {
    Some(u64::from_le_bytes(data.get(..8)?.try_into().ok()?))
}

fn known_research() -> &'static std::sync::atomic::AtomicU64 {
    static KNOWN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    &KNOWN
}

/// Клиент запомнил, что изучено (пришло с сервера).
pub fn remember_research(mask: u64) {
    known_research().store(mask, std::sync::atomic::Ordering::Relaxed);
}

/// Что изучено, как его знает клиент. Первый узел изучен всегда.
pub fn shown_research() -> u64 {
    known_research().load(std::sync::atomic::Ordering::Relaxed) | research_bit(0)
}

/// Как узел выглядит на карте.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeState {
    /// Изучен: золотая рамка.
    Done,
    /// Все предшественники изучены: можно открыть, подсказка видна.
    Open,
    /// Часть предшественников изучена: виден знак вопроса.
    Locked,
    /// Ни один предшественник и ни один потомок не изучен: узла не видно.
    Hidden,
}

pub fn node_state(mask: u64, index: usize) -> NodeState {
    if mask & research_bit(index) != 0 {
        return NodeState::Done;
    }
    let parents: Vec<usize> = RESEARCH[index]
        .after
        .iter()
        .filter_map(|id| research_index(id))
        .collect();
    let done = parents.iter().filter(|p| mask & research_bit(**p) != 0).count();
    // Игрок мог забежать вперёд: изученный потомок показывает пропущенный
    // узел вопросом, чтобы у него была связь с остальной картой.
    let child_done = RESEARCH.iter().enumerate().any(|(i, r)| {
        mask & research_bit(i) != 0 && r.after.contains(&RESEARCH[index].id)
    });
    if done == parents.len() {
        NodeState::Open
    } else if done > 0 || child_done {
        NodeState::Locked
    } else {
        NodeState::Hidden
    }
}

/// Сколько изучено: подпись в углу карты.
pub fn research_progress(mask: u64) -> String {
    let done = (0..RESEARCH.len()).filter(|i| mask & research_bit(*i) != 0).count();
    format!("Изучено {done} из {}", RESEARCH.len())
}

/// Подсказка у курсора над узлом.
pub fn research_tooltip(mask: u64, index: usize) -> Vec<String> {
    let r = &RESEARCH[index];
    match node_state(mask, index) {
        NodeState::Done => vec![r.title.to_string(), "Изучено — щёлкните, чтобы читать".to_string()],
        NodeState::Open => vec![r.title.to_string(), r.hint.to_string()],
        NodeState::Locked | NodeState::Hidden => {
            let missing: Vec<&str> = r
                .after
                .iter()
                .filter_map(|id| research_index(id))
                .filter(|p| mask & research_bit(*p) == 0)
                .map(|p| RESEARCH[p].title)
                .collect();
            vec!["???".to_string(), format!("Сначала: {}", missing.join(", "))]
        }
    }
}

/// Страницы записи узла из файла записей: блок от `= id` до следующего `= `,
/// страницы внутри разделены `---`. Нет блока — страница из подсказки.
pub fn research_pages(text: &str, index: usize) -> Vec<String> {
    let r = &RESEARCH[index];
    let mut block: Option<String> = None;
    let mut inside = false;
    for line in text.lines() {
        if let Some(id) = line.strip_prefix("= ") {
            if inside {
                break;
            }
            inside = id.trim() == r.id;
            if inside {
                block = Some(String::new());
            }
            continue;
        }
        if inside && let Some(b) = block.as_mut() {
            b.push_str(line);
            b.push('\n');
        }
    }
    let pages: Vec<String> = block
        .unwrap_or_default()
        .split("\n---")
        .map(|p| p.trim_matches('\n').to_string())
        .filter(|p| !p.trim().is_empty())
        .take(MANUAL_MAX_PAGES)
        .collect();
    if pages.is_empty() {
        vec![format!("# {}\n{}", r.title, r.hint)]
    } else {
        pages
    }
}

// Раскладка экрана в пикселях интерфейса. Рамка по центру, вкладки слева от
// неё, внутри — поле узлов 6×4 клетки с полосой заголовка и подписью снизу.

pub const RESEARCH_COLS: f32 = 6.0;
pub const RESEARCH_ROWS: f32 = 4.0;
pub const NODE_SIZE: f32 = 24.0;
pub const TAB_SIZE: f32 = 22.0;
/// Высота полосы заголовка и подписи внизу поля.
pub const RESEARCH_HEADER: f32 = 16.0;
pub const RESEARCH_FOOTER: f32 = 13.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResearchLayout {
    /// Рамка-обложка.
    pub frame: [f32; 4],
    /// Тёмное поле внутри рамки.
    pub field: [f32; 4],
    /// Сетка узлов: поле без заголовка и подписи.
    pub grid: [f32; 4],
    /// Страница записи поверх поля.
    pub page: [f32; 4],
}

pub fn research_layout(width: f32, height: f32) -> ResearchLayout {
    let w = (width - 64.0).clamp(220.0, 400.0);
    let h = (height - 20.0).clamp(160.0, 240.0);
    // Вкладки торчат слева: сдвигаем рамку вправо на их ширину пополам.
    let x = ((width - w) / 2.0 + TAB_SIZE / 2.0).floor();
    let y = ((height - h) / 2.0).floor();
    let field = [x + 6.0, y + 6.0, w - 12.0, h - 12.0];
    let grid = [
        field[0],
        field[1] + RESEARCH_HEADER,
        field[2],
        field[3] - RESEARCH_HEADER - RESEARCH_FOOTER,
    ];
    let page = [field[0] + 6.0, field[1] + 4.0, field[2] - 12.0, field[3] - 8.0];
    ResearchLayout { frame: [x, y, w, h], field, grid, page }
}

pub fn tab_rect(l: &ResearchLayout, science: usize) -> [f32; 4] {
    [
        l.frame[0] - TAB_SIZE,
        l.frame[1] + 10.0 + science as f32 * (TAB_SIZE + 4.0),
        TAB_SIZE,
        TAB_SIZE,
    ]
}

pub fn node_center(l: &ResearchLayout, r: &Research) -> [f32; 2] {
    let [x, y, w, h] = l.grid;
    let (cw, ch) = (w / RESEARCH_COLS, h / RESEARCH_ROWS);
    [
        (x + cw * (f32::from(r.at.0) + 0.5)).floor(),
        (y + ch * (f32::from(r.at.1) + 0.5)).floor(),
    ]
}

pub fn node_rect(l: &ResearchLayout, r: &Research) -> [f32; 4] {
    let [cx, cy] = node_center(l, r);
    [cx - NODE_SIZE / 2.0, cy - NODE_SIZE / 2.0, NODE_SIZE, NODE_SIZE]
}

/// Кнопки внизу страницы: назад, вперёд и «к карте».
pub fn page_buttons(l: &ResearchLayout) -> [[f32; 4]; 3] {
    let [x, y, w, h] = l.page;
    let row = y + h - 14.0;
    [
        [x + 6.0, row, 18.0, 12.0],
        [x + w - 24.0, row, 18.0, 12.0],
        [x + w / 2.0 - 28.0, row, 56.0, 12.0],
    ]
}

pub fn inside(rect: [f32; 4], at: [f32; 2]) -> bool {
    at[0] >= rect[0] && at[1] >= rect[1] && at[0] < rect[0] + rect[2] && at[1] < rect[1] + rect[3]
}

/// Что сейчас открыто на экране журнала.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResearchView {
    pub open: bool,
    pub science: usize,
    /// Открытая запись: узел и страница.
    pub page: Option<(usize, usize)>,
}

fn research_view_cell() -> &'static std::sync::Mutex<ResearchView> {
    static VIEW: std::sync::OnceLock<std::sync::Mutex<ResearchView>> = std::sync::OnceLock::new();
    VIEW.get_or_init(Default::default)
}

pub fn research_view() -> ResearchView {
    research_view_cell().lock().map(|v| *v).unwrap_or_default()
}

pub fn set_research_view(view: ResearchView) {
    if let Ok(mut v) = research_view_cell().lock() {
        *v = view;
    }
}

/// Открыть журнал на карте той науки, что была открыта в прошлый раз.
pub fn open_research() {
    let view = research_view();
    set_research_view(ResearchView { open: true, page: None, ..view });
}

pub fn close_research() {
    let view = research_view();
    set_research_view(ResearchView { open: false, page: None, ..view });
}

/// Куда попал щелчок.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchHit {
    Tab(usize),
    Node(usize),
    PagePrevious,
    PageNext,
    PageBack,
    Nothing,
}

pub fn research_hit(l: &ResearchLayout, view: &ResearchView, mask: u64, at: [f32; 2]) -> ResearchHit {
    for science in 0..SCIENCES.len() {
        if inside(tab_rect(l, science), at) {
            return ResearchHit::Tab(science);
        }
    }
    if view.page.is_some() {
        let [previous, next, back] = page_buttons(l);
        return if inside(previous, at) {
            ResearchHit::PagePrevious
        } else if inside(next, at) {
            ResearchHit::PageNext
        } else if inside(back, at) || !inside(l.page, at) {
            ResearchHit::PageBack
        } else {
            ResearchHit::Nothing
        };
    }
    RESEARCH
        .iter()
        .enumerate()
        .find(|(i, r)| {
            r.science == view.science
                && node_state(mask, *i) != NodeState::Hidden
                && inside(node_rect(l, r), at)
        })
        .map_or(ResearchHit::Nothing, |(i, _)| ResearchHit::Node(i))
}

/// Щелчок меняет вид: вкладка, запись, страница. `pages` — сколько страниц у
/// открытой записи.
pub fn research_click(view: ResearchView, hit: ResearchHit, mask: u64, pages: usize) -> ResearchView {
    match hit {
        ResearchHit::Tab(science) => ResearchView { science, page: None, ..view },
        ResearchHit::Node(i) => match node_state(mask, i) {
            NodeState::Done | NodeState::Open => ResearchView { page: Some((i, 0)), ..view },
            NodeState::Locked | NodeState::Hidden => view,
        },
        ResearchHit::PagePrevious => ResearchView {
            page: view.page.map(|(i, p)| (i, p.saturating_sub(1))),
            ..view
        },
        ResearchHit::PageNext => ResearchView {
            page: view.page.map(|(i, p)| (i, (p + 1).min(pages.max(1) - 1))),
            ..view
        },
        ResearchHit::PageBack => ResearchView { page: None, ..view },
        ResearchHit::Nothing => view,
    }
}

/// Клавиша на экране журнала (`escape`, `left`, `right`, `1`…). Вернёт новый
/// вид и надо ли закрыть экран.
pub fn research_key(view: ResearchView, key: &str, pages: usize) -> (ResearchView, bool) {
    match key {
        "escape" | "backspace" | "e" => match view.page {
            Some(_) => (ResearchView { page: None, ..view }, false),
            None => (ResearchView { open: false, ..view }, true),
        },
        "left" | "a" => (research_click(view, ResearchHit::PagePrevious, 0, pages), false),
        "right" | "d" => (research_click(view, ResearchHit::PageNext, 0, pages), false),
        digit => match digit.parse::<usize>() {
            Ok(n) if (1..=SCIENCES.len()).contains(&n) => {
                (ResearchView { science: n - 1, page: None, ..view }, false)
            }
            _ => (view, false),
        },
    }
}

// Цвета экрана: ARGB для заливок, RGB для текста.
pub const RESEARCH_SHADE: u32 = 0xD0_08050C;
pub const RESEARCH_GOLD: u32 = 0xFF_C9A45A;
pub const RESEARCH_LEATHER: u32 = 0xFF_3C2A1C;
pub const RESEARCH_FIELD: u32 = 0xFF_150F1B;
pub const RESEARCH_TAB: u32 = 0xFF_2A2033;
pub const RESEARCH_TAB_ON: u32 = 0xFF_4A3858;
pub const RESEARCH_TAB_EDGE: u32 = 0xFF_6D5634;
pub const RESEARCH_DONE_EDGE: u32 = 0xFF_E8C56A;
pub const RESEARCH_DONE_FILL: u32 = 0xFF_3B2E22;
pub const RESEARCH_OPEN_EDGE: u32 = 0xFF_8F74B8;
pub const RESEARCH_OPEN_GLOW: u32 = 0xFF_C2A6EE;
pub const RESEARCH_OPEN_FILL: u32 = 0xFF_241B2C;
pub const RESEARCH_LOCK_EDGE: u32 = 0xFF_463A50;
pub const RESEARCH_LOCK_FILL: u32 = 0xFF_1B1520;
pub const RESEARCH_LINE_DIM: u32 = 0xFF_3A2F44;
pub const RESEARCH_TIP_FILL: u32 = 0xF0_100010;
pub const RESEARCH_TIP_EDGE: u32 = 0xFF_5A2D9A;
pub const RESEARCH_TEXT_GOLD: u32 = 0xE8C56A;
pub const RESEARCH_TEXT_DIM: u32 = 0x9A8CAA;
pub const RESEARCH_TEXT_WHITE: u32 = 0xFFFFFF;
pub const RESEARCH_TEXT_LOCK: u32 = 0x6C5C78;

// Рецепты для страниц: `@craft <файл>` рисует рецепт из
// `data/biolab/recipe/<файл>.json`. JSON разбираем сами: зависимостей у
// логики мода нет, а формат рецепта маленький.

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn parse(text: &str) -> Option<Json> {
        let mut p = JsonParser { s: text.as_bytes(), i: 0 };
        let value = p.value()?;
        p.ws();
        (p.i == p.s.len()).then_some(value)
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    s: &'a [u8],
    i: usize,
}

impl JsonParser<'_> {
    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Option<Json> {
        if self.s.get(self.i..self.i + word.len())? == word.as_bytes() {
            self.i += word.len();
            Some(value)
        } else {
            None
        }
    }

    fn value(&mut self) -> Option<Json> {
        self.ws();
        match *self.s.get(self.i)? {
            b'{' => {
                self.i += 1;
                let mut fields = Vec::new();
                if self.eat(b'}') {
                    return Some(Json::Obj(fields));
                }
                loop {
                    self.ws();
                    let key = self.string()?;
                    if !self.eat(b':') {
                        return None;
                    }
                    fields.push((key, self.value()?));
                    if self.eat(b',') {
                        continue;
                    }
                    return self.eat(b'}').then_some(Json::Obj(fields));
                }
            }
            b'[' => {
                self.i += 1;
                let mut items = Vec::new();
                if self.eat(b']') {
                    return Some(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    if self.eat(b',') {
                        continue;
                    }
                    return self.eat(b']').then_some(Json::Arr(items));
                }
            }
            b'"' => self.string().map(Json::Str),
            b't' => self.literal("true", Json::Bool(true)),
            b'f' => self.literal("false", Json::Bool(false)),
            b'n' => self.literal("null", Json::Null),
            _ => {
                let start = self.i;
                while self
                    .s
                    .get(self.i)
                    .is_some_and(|c| c.is_ascii_digit() || b"+-.eE".contains(c))
                {
                    self.i += 1;
                }
                std::str::from_utf8(&self.s[start..self.i]).ok()?.parse().ok().map(Json::Num)
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        if self.s.get(self.i) != Some(&b'"') {
            return None;
        }
        self.i += 1;
        let mut out = Vec::new();
        loop {
            let c = *self.s.get(self.i)?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = *self.s.get(self.i)?;
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'u' => {
                            let hex = std::str::from_utf8(self.s.get(self.i..self.i + 4)?).ok()?;
                            self.i += 4;
                            let ch = char::from_u32(u32::from_str_radix(hex, 16).ok()?).unwrap_or('?');
                            let mut buf = [0; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                c => out.push(c),
            }
        }
        String::from_utf8(out).ok()
    }
}

/// Рецепт, как его рисует страница.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipeView {
    /// Верстак: сетка 3×3 и результат.
    Grid {
        cells: [Option<String>; 9],
        result: String,
        count: u32,
        shapeless: bool,
    },
    /// Лабораторный стол: ингредиенты сеткой, как без формы на верстаке,
    /// результат и время. Страница подписывает, что это стол, а не верстак.
    Bench {
        cells: [Option<String>; 9],
        result: String,
        count: u32,
        seconds: u32,
    },
    /// Машина мода: вход, машина, время, результат и побочный выход
    /// центрифуги, если он есть.
    Machine {
        input: String,
        machine: String,
        result: String,
        count: u32,
        seconds: u32,
        byproduct: Option<(String, u32)>,
    },
}

/// Выход рецепта: `{"id", "count"}` или просто id. Так записаны и основной
/// результат, и побочный выход центрифуги.
fn stack_of(j: &Json) -> Option<(String, u32)> {
    match j {
        Json::Str(s) => Some((s.clone(), 1)),
        _ => Some((
            j.get("id")?.str()?.to_string(),
            j.get("count").and_then(Json::num).map_or(1, |n| n.max(1.0) as u32),
        )),
    }
}

/// Предмет ингредиента: строка, `{"item": …}` или первый из списка. Теги
/// (`#…`) не рисуем — у них нет одной иконки.
fn ingredient_id(j: &Json) -> Option<String> {
    match j {
        Json::Str(s) if !s.starts_with('#') => Some(s.clone()),
        Json::Obj(_) => j.get("item").and_then(Json::str).map(str::to_string),
        Json::Arr(a) => a.first().and_then(ingredient_id),
        _ => None,
    }
}

pub fn recipe_path(name: &str) -> String {
    format!("data/biolab/recipe/{name}.json")
}

pub fn recipe_view(text: &str) -> Option<RecipeView> {
    let j = Json::parse(text)?;
    let kind = j.get("type")?.str()?;
    let (result, count) = stack_of(j.get("result")?)?;
    match kind {
        "minecraft:crafting_shaped" => {
            let pattern = j.get("pattern")?.arr()?;
            let key = j.get("key")?;
            let mut cells: [Option<String>; 9] = Default::default();
            for (row, line) in pattern.iter().take(3).enumerate() {
                for (col, ch) in line.str()?.chars().take(3).enumerate() {
                    if ch != ' ' {
                        cells[row * 3 + col] = key.get(&ch.to_string()).and_then(ingredient_id);
                    }
                }
            }
            Some(RecipeView::Grid { cells, result, count, shapeless: false })
        }
        "minecraft:crafting_shapeless" => {
            let mut cells: [Option<String>; 9] = Default::default();
            for (cell, ingredient) in cells.iter_mut().zip(j.get("ingredients")?.arr()?) {
                *cell = ingredient_id(ingredient);
            }
            Some(RecipeView::Grid { cells, result, count, shapeless: true })
        }
        // Стол — тоже `biolab:`, но рецепт у него списком, как у верстака,
        // а не одним входом машины. Входов восемь: девятая клетка пустует.
        LAB_BENCH => {
            let ingredients = j.get("ingredients")?.arr()?;
            if ingredients.is_empty() || ingredients.len() > 8 {
                return None;
            }
            let mut cells: [Option<String>; 9] = Default::default();
            for (cell, ingredient) in cells.iter_mut().zip(ingredients) {
                *cell = Some(ingredient_id(ingredient)?);
            }
            let ticks = j.get("time").and_then(Json::num).unwrap_or(DEFAULT_TIME as f64).max(1.0) as u32;
            Some(RecipeView::Bench { cells, result, count, seconds: ticks.div_ceil(20) })
        }
        machine if machine.starts_with("biolab:") => {
            let ticks = j.get("time").and_then(Json::num).unwrap_or(200.0).max(1.0) as u32;
            // Побочный выход записан — значит обязан разобраться: битый
            // рецепт лучше поймать тестом, чем молча нарисовать без него.
            let byproduct = match j.get("byproduct") {
                Some(b) => Some(stack_of(b)?),
                None => None,
            };
            Some(RecipeView::Machine {
                input: ingredient_id(j.get("ingredient")?)?,
                machine: machine.to_string(),
                result,
                count,
                seconds: ticks.div_ceil(20),
                byproduct,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod research_tests {
    use super::*;

    #[test]
    fn the_map_is_consistent() {
        assert!(RESEARCH.len() <= 64, "маска изученного — 64 бита");
        let mut cells = std::collections::HashSet::new();
        for (i, r) in RESEARCH.iter().enumerate() {
            assert_eq!(research_index(r.id), Some(i), "{} дважды", r.id);
            assert!(r.science < SCIENCES.len(), "{}", r.id);
            assert!(f32::from(r.at.0) < RESEARCH_COLS && f32::from(r.at.1) < RESEARCH_ROWS, "{}", r.id);
            assert!(cells.insert((r.science, r.at)), "{}: клетка занята", r.id);
            for parent in r.after {
                let p = research_index(parent).unwrap_or_else(|| panic!("{}: нет узла {parent}", r.id));
                assert_eq!(RESEARCH[p].science, r.science, "{}: связь через науку", r.id);
                assert!(p < i, "{}: предшественник ниже по таблице", r.id);
            }
        }
        // У каждой науки есть корень, видимый с самого начала.
        for science in 0..SCIENCES.len() {
            assert!(
                RESEARCH
                    .iter()
                    .enumerate()
                    .any(|(i, r)| r.science == science && node_state(research_bit(0), i) != NodeState::Hidden),
                "{}",
                SCIENCES[science].title
            );
        }
    }

    #[test]
    fn every_item_of_the_map_exists_in_content() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        for r in &RESEARCH {
            let mut items: Vec<&str> = vec![r.icon];
            if let Unlock::Item(ids) = r.unlock {
                items.extend(ids);
            }
            for item in items {
                if let Some(name) = item.strip_prefix("biolab:") {
                    assert!(
                        root.join(format!("assets/biolab/items/{name}.json")).exists(),
                        "{}: нет предмета {item}",
                        r.id
                    );
                }
            }
        }
    }

    #[test]
    fn nodes_light_up_from_what_the_player_does() {
        let mut s = PlayerState::default();
        let items = vec!["biolab:mold".to_string()];
        let facts = ResearchFacts { items: &items, discovered: 0, identified: 0 };
        let now = research_now(&s, &facts);
        let mold = research_index("mold").unwrap();
        assert_ne!(now & research_bit(mold), 0);
        assert_ne!(now & research_bit(0), 0, "журнал изучен всегда");
        assert_eq!(now & research_bit(research_index("infection").unwrap()), 0);

        s.infection = 5;
        s.drugs[0].blood = 30;
        s.genes[0] = 1;
        let empty: Vec<String> = Vec::new();
        let facts = ResearchFacts { items: &empty, discovered: 1, identified: 10 };
        let now = research_now(&s, &facts);
        for id in ["infection", "window", "therapy", "bonds", "identify", "collection"] {
            assert_ne!(now & research_bit(research_index(id).unwrap()), 0, "{id}");
        }
        assert_eq!(now & research_bit(research_index("overdose").unwrap()), 0);
        s.drugs[0].blood = 500;
        assert_ne!(research_now(&s, &facts) & research_bit(research_index("overdose").unwrap()), 0);
        assert_eq!(research_now(&s, &facts) & research_bit(research_index("hardened").unwrap()), 0);
        s.strain = WILD.values().map(|v| v as u8);
        s.strain[3] += 1;
        assert_ne!(research_now(&s, &facts) & research_bit(research_index("hardened").unwrap()), 0);
    }

    #[test]
    fn research_is_saved_by_name_and_sent_as_eight_bytes() {
        let mask = research_bit(0) | research_bit(research_index("vat").unwrap());
        assert_eq!(research_to_text(mask), "manual,vat");
        assert_eq!(research_from_text("manual, vat,ghost"), mask);
        assert_eq!(research_from_packet(&research_packet(mask)), Some(mask));
        assert_eq!(research_from_packet(&[1, 2]), None);
        let mut s = PlayerState::default();
        s.research = mask;
        assert_eq!(state_from_text(&state_to_text(&s)).research, mask);
    }

    #[test]
    fn the_tick_reports_each_node_once() {
        let dir = std::env::temp_dir().join(format!("biolab-research-{}", std::process::id()));
        let items = vec!["biolab:still".to_string()];
        let facts = ResearchFacts { items: &items, discovered: 0, identified: 0 };
        let (mask, opened) = research_tick(&dir, 7, &facts);
        assert_eq!(opened, vec![0, research_index("still").unwrap()]);
        assert_eq!(research_message(opened[1]), "Журнал лаборанта: новая запись «Перегонный куб»");
        let (again, none) = research_tick(&dir, 7, &facts);
        assert_eq!(again, mask);
        assert!(none.is_empty());
        let saved = std::fs::read_to_string(state_path(&dir, 7)).unwrap();
        assert!(saved.contains("research=manual,still"), "{saved}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nodes_show_as_done_open_locked_or_hidden() {
        let bench = research_index("bench").unwrap();
        let flask = research_index("flask").unwrap();
        let analyzer = research_index("analyzer").unwrap();
        let mask = research_bit(0);
        assert_eq!(node_state(mask, 0), NodeState::Done);
        assert_eq!(node_state(mask, bench), NodeState::Open);
        assert_eq!(node_state(mask, flask), NodeState::Hidden);
        let mask = mask | research_bit(bench);
        assert_eq!(node_state(mask, flask), NodeState::Open);
        assert_eq!(node_state(mask, analyzer), NodeState::Hidden);
        assert_eq!(research_tooltip(mask, flask)[1], "Сделайте колбу.");
        assert_eq!(research_progress(mask), format!("Изучено 2 из {}", RESEARCH.len()));
        // Частично открытые предшественники: виден вопрос.
        let window = research_index("window").unwrap();
        assert_eq!(node_state(research_bit(research_index("pills").unwrap()), window), NodeState::Open);
        assert_eq!(research_tooltip(0, flask), vec!["???".to_string(), "Сначала: Лабораторный стол".to_string()]);
        // Забежал вперёд: бинт изучен, агар перед ним виден вопросом.
        let agar = research_index("agar").unwrap();
        let bandage = research_bit(research_index("bandage").unwrap());
        assert_eq!(node_state(research_bit(0) | bandage, agar), NodeState::Locked);
    }

    #[test]
    fn layout_fits_small_and_large_screens() {
        for (w, h) in [(320.0, 200.0), (427.0, 240.0), (640.0, 360.0), (960.0, 540.0)] {
            let l = research_layout(w, h);
            assert!(tab_rect(&l, 0)[0] >= 0.0, "{w}×{h}: вкладки за краем");
            assert!(l.frame[0] + l.frame[2] <= w + 1.0 && l.frame[1] >= 0.0, "{w}×{h}");
            for (i, r) in RESEARCH.iter().enumerate() {
                let rect = node_rect(&l, r);
                assert!(rect[0] >= l.grid[0] && rect[0] + rect[2] <= l.grid[0] + l.grid[2], "{w}×{h}: {i}");
                assert!(rect[1] >= l.field[1] && rect[1] + rect[3] <= l.field[1] + l.field[3], "{w}×{h}: {i}");
            }
            // Узлы одной науки не налезают друг на друга.
            for a in 0..RESEARCH.len() {
                for b in a + 1..RESEARCH.len() {
                    if RESEARCH[a].science == RESEARCH[b].science {
                        let (ra, rb) = (node_rect(&l, &RESEARCH[a]), node_rect(&l, &RESEARCH[b]));
                        let apart = ra[0] + ra[2] <= rb[0] || rb[0] + rb[2] <= ra[0] || ra[1] + ra[3] <= rb[1] || rb[1] + rb[3] <= ra[1];
                        assert!(apart, "{w}×{h}: {} и {}", RESEARCH[a].id, RESEARCH[b].id);
                    }
                }
            }
        }
    }

    #[test]
    fn clicks_open_records_switch_tabs_and_turn_pages() {
        let l = research_layout(427.0, 240.0);
        let mask = research_bit(0);
        let view = ResearchView { open: true, science: 0, page: None };
        let bench = research_index("bench").unwrap();
        let center = node_center(&l, &RESEARCH[bench]);
        assert_eq!(research_hit(&l, &view, mask, center), ResearchHit::Node(bench));
        let opened = research_click(view, ResearchHit::Node(bench), mask, 1);
        assert_eq!(opened.page, Some((bench, 0)));
        // Скрытый узел не ловит мышь.
        let flask = research_index("flask").unwrap();
        assert_eq!(research_hit(&l, &view, mask, node_center(&l, &RESEARCH[flask])), ResearchHit::Nothing);
        let tab = tab_rect(&l, 3);
        assert_eq!(research_hit(&l, &view, mask, [tab[0] + 2.0, tab[1] + 2.0]), ResearchHit::Tab(3));
        // Страницы листаются в пределах записи, щелчок мимо закрывает её.
        let [previous, next, back] = page_buttons(&l);
        assert_eq!(research_hit(&l, &opened, mask, [next[0] + 1.0, next[1] + 1.0]), ResearchHit::PageNext);
        assert_eq!(research_hit(&l, &opened, mask, [previous[0] + 1.0, previous[1] + 1.0]), ResearchHit::PagePrevious);
        assert_eq!(research_hit(&l, &opened, mask, [back[0] + 1.0, back[1] + 1.0]), ResearchHit::PageBack);
        assert_eq!(research_hit(&l, &opened, mask, [1.0, 1.0]), ResearchHit::PageBack);
        let turned = research_click(opened, ResearchHit::PageNext, mask, 3);
        assert_eq!(turned.page, Some((bench, 1)));
        let last = research_click(research_click(turned, ResearchHit::PageNext, mask, 2), ResearchHit::PageNext, mask, 2);
        assert_eq!(last.page, Some((bench, 1)));
        // Клавиши: Esc закрывает запись, потом экран; цифры — науки.
        let (back_to_map, close) = research_key(opened, "escape", 1);
        assert_eq!((back_to_map.page, close), (None, false));
        assert!(research_key(back_to_map, "escape", 1).1);
        assert_eq!(research_key(view, "5", 1).0.science, 4);
        assert_eq!(research_key(view, "9", 1).0, view);
    }

    #[test]
    fn records_are_read_by_node_and_split_into_pages() {
        let text = "= manual\n# Один\nтекст\n---\n# Два\n= bench\n# Стол\n";
        let manual = research_pages(text, 0);
        assert_eq!(manual, vec!["# Один\nтекст".to_string(), "# Два".to_string()]);
        assert_eq!(research_pages(text, research_index("bench").unwrap()), vec!["# Стол".to_string()]);
        let flask = research_index("flask").unwrap();
        assert_eq!(research_pages(text, flask), vec!["# Колба\nСделайте колбу.".to_string()]);
    }

    #[test]
    fn every_node_has_a_record_and_its_recipes_exist() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let text = std::fs::read_to_string(root.join(RESEARCH_PATH)).unwrap();
        for (i, r) in RESEARCH.iter().enumerate() {
            assert!(text.contains(&format!("= {}\n", r.id)), "нет записи {}", r.id);
            for page in research_pages(&text, i) {
                for line in page_lines(&page) {
                    if let Line::Craft(name) = line {
                        let json = std::fs::read_to_string(root.join(recipe_path(&name)))
                            .unwrap_or_else(|_| panic!("{}: нет рецепта {name}", r.id));
                        assert!(recipe_view(&json).is_some(), "{}: рецепт {name} не разобрался", r.id);
                    }
                }
            }
        }
    }

    #[test]
    fn every_recipe_of_the_mod_parses() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/recipe");
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(recipe_view(&text).is_some(), "{}", path.display());
        }
    }

    #[test]
    fn recipes_become_grids_and_machine_lines() {
        let shaped = r#"{"type":"minecraft:crafting_shaped","pattern":[" g "," i ","iii"],"key":{"g":"minecraft:glass_pane","i":{"item":"minecraft:iron_ingot"}},"result":{"id":"biolab:microscope","count":1}}"#;
        let Some(RecipeView::Grid { cells, result, count, shapeless }) = recipe_view(shaped) else {
            panic!("не сетка");
        };
        assert_eq!(cells[1].as_deref(), Some("minecraft:glass_pane"));
        assert_eq!(cells[0], None);
        assert_eq!(cells[8].as_deref(), Some("minecraft:iron_ingot"));
        assert_eq!((result.as_str(), count, shapeless), ("biolab:microscope", 1, false));
        let machine = "{\n \"type\": \"biolab:press\",\n \"ingredient\": \"biolab:extract_salicylate\",\n \"result\": {\"id\": \"biolab:pill_analgesic\", \"count\": 2},\n \"time\": 100\n}";
        assert_eq!(
            recipe_view(machine),
            Some(RecipeView::Machine {
                input: "biolab:extract_salicylate".into(),
                machine: "biolab:press".into(),
                result: "biolab:pill_analgesic".into(),
                count: 2,
                seconds: 5,
                byproduct: None,
            })
        );
        let spin = r#"{"type":"biolab:centrifuge","ingredient":"minecraft:gravel","result":{"id":"minecraft:flint","count":1},"byproduct":{"id":"minecraft:sand","count":2},"time":100}"#;
        let Some(RecipeView::Machine { byproduct, .. }) = recipe_view(spin) else {
            panic!("не машина");
        };
        assert_eq!(byproduct, Some(("minecraft:sand".to_string(), 2)));
        let bench = r#"{"type":"biolab:lab_bench","ingredients":["minecraft:iron_nugget","minecraft:glass_pane","minecraft:iron_nugget"],"result":{"id":"biolab:syringe","count":2},"time":40}"#;
        let Some(RecipeView::Bench { cells, result, count, seconds }) = recipe_view(bench) else {
            panic!("не стол");
        };
        assert_eq!(cells[1].as_deref(), Some("minecraft:glass_pane"));
        assert_eq!(cells[3], None);
        assert_eq!((result.as_str(), count, seconds), ("biolab:syringe", 2, 2));
        let crowded = r#"{"type":"biolab:lab_bench","ingredients":["a","a","a","a","a","a","a","a","a"],"result":"b"}"#;
        assert_eq!(recipe_view(crowded), None, "девять записей на восемь входов");
        let tagged = r##"{"type":"biolab:lab_bench","ingredients":["#minecraft:planks"],"result":"b"}"##;
        assert_eq!(recipe_view(tagged), None, "у тега нет одной иконки");
        let broken = r#"{"type":"biolab:centrifuge","ingredient":"minecraft:gravel","result":"minecraft:flint","byproduct":{"count":2}}"#;
        assert_eq!(recipe_view(broken), None, "битый побочный выход");
        assert_eq!(Json::parse(r#"{"a":"Ж\n","b":[1,-2.5e1,true,null]}"#).unwrap().get("a").and_then(Json::str), Some("Ж\n"));
        assert_eq!(Json::parse("{\"a\":1,}"), None);
        assert_eq!(recipe_view("{\"type\":\"minecraft:smelting\",\"result\":\"x\"}"), None);
    }
}

// ---- Экраны машин ----
//
// У каждой машины свой экран поверх её ванильного меню (GDD 4.16): раскладка
// слотов, лабораторная панель, прибор и шкала прогресса. Здесь только числа и
// тексты — раскладка, прямоугольники приборов, подсказки, кадры анимации, —
// а рисуют хуки `screen_layout.rs`, `screen_draw.rs`, `screen_tooltip.rs`.
// Всё, что прибор показывает, приходит слотами данных по контракту `DATA_*`;
// до первого пакета с сервера массив пуст, поэтому читаем через `screen_datum`.

/// Раскладка экрана машины: размер, где стоит каждый слот контейнера (None —
/// слот спрятан и выпадает из меню), где начинается инвентарь игрока, и два
/// прямоугольника: прибор машины и её шкала прогресса. Всё — в пикселях GUI
/// от левого верхнего угла панели; позиция слота — угол предмета, рамка на
/// пиксель шире.
#[derive(Clone, Debug, PartialEq)]
pub struct MachineScreen {
    pub width: f32,
    pub height: f32,
    pub slots: Vec<Option<[f32; 2]>>,
    /// Левая колонка и первый ряд инвентаря игрока; хотбар на 58 ниже ряда.
    pub inventory: [f32; 2],
    pub gauge: [f32; 4],
    pub progress: [f32; 4],
}

/// Ширина экрана, как у ванильных контейнеров.
pub const SCREEN_WIDTH: f32 = 176.0;
/// Высота и первый ряд инвентаря у экранов на меню печки и сетки 3×3.
pub const SCREEN_HEIGHT: f32 = 166.0;
pub const SCREEN_INVENTORY_Y: f32 = 84.0;
/// Экран на меню воронки ниже: так стоит инвентарь у ванильной воронки.
pub const HOPPER_HEIGHT: f32 = 133.0;
pub const HOPPER_INVENTORY_Y: f32 = 51.0;
/// Полоса заголовка: иконка машины и название. Приборы начинаются ниже.
pub const SCREEN_HEADER: f32 = 16.0;

/// Название машины на экране, если сервер не прислал своё, и что она делает —
/// для подсказки над шкалой.
pub const SCREEN_NAMES: [(&str, &str, &str); 11] = [
    ("biolab:still", "Перегонный куб", "Перегонка"),
    ("biolab:incubator", "Инкубатор", "Рост культуры"),
    ("biolab:fermenter", "Ферментер", "Брожение"),
    ("biolab:centrifuge", "Центрифуга", "Разделение"),
    ("biolab:synthesizer", "Синтезатор", "Сборка"),
    ("biolab:reactor", "Реактор", "Реакция"),
    ("biolab:press", "Пресс для таблеток", "Прессовка"),
    ("biolab:extractor", "Экстрактор ДНК", "Выделение ДНК"),
    ("biolab:cloning_vat", "Клонирующий чан", "Рост клона"),
    ("biolab:spectrometer", "Спектрометр", "Анализ"),
    ("biolab:lab_bench", "Лабораторный стол", "Сборка"),
];

/// Название машины для заголовка.
pub fn machine_title(id: &str) -> &'static str {
    SCREEN_NAMES.iter().find(|n| n.0 == id).map_or("", |n| n.1)
}

/// Что машина делает, одним словом.
pub fn machine_verb(id: &str) -> &'static str {
    SCREEN_NAMES.iter().find(|n| n.0 == id).map_or("Работа", |n| n.2)
}

/// Экран машины на меню печки: вход, «топливо», выход. Слот 1 нужен только
/// чану (питательный раствор), у остальных он спрятан — и из меню на сервере
/// тоже, так что shift-клик туда ничего не положит.
fn furnace_screen(
    input: [f32; 2],
    fuel: Option<[f32; 2]>,
    output: [f32; 2],
    gauge: [f32; 4],
    progress: [f32; 4],
) -> MachineScreen {
    MachineScreen {
        width: SCREEN_WIDTH,
        height: SCREEN_HEIGHT,
        slots: vec![Some(input), fuel, Some(output)],
        inventory: [8.0, SCREEN_INVENTORY_Y],
        gauge,
        progress,
    }
}

/// Раскладка экрана машины; None — не наша машина.
///
/// Синтезатор и центрифуга стоят ровно там, где ванильные сетка 3×3 и
/// воронка ставят свои слоты и инвентарь: Horizon раскладывает меню
/// `minecraft:generic_3x3` и `minecraft:hopper` по-своему раньше, чем
/// спрашивает мод (`machine_slot_position` в `game.rs`), и при другой
/// раскладке предметы легли бы мимо нарисованных рамок. У машин на меню
/// печки раскладка своя целиком.
pub fn machine_screen(id: &str) -> Option<MachineScreen> {
    let machine = machine(id)?;
    let screen = match machine.id {
        "biolab:still" => furnace_screen(
            [20.0, 35.0],
            None,
            [138.0, 35.0],
            [44.0, 18.0, 86.0, 44.0],
            [44.0, 65.0, 86.0, 5.0],
        ),
        "biolab:incubator" => furnace_screen(
            [22.0, 35.0],
            None,
            [138.0, 35.0],
            [46.0, 18.0, 84.0, 42.0],
            [46.0, 64.0, 84.0, 5.0],
        ),
        "biolab:fermenter" => furnace_screen(
            [26.0, 35.0],
            None,
            [134.0, 35.0],
            [60.0, 17.0, 56.0, 46.0],
            [60.0, 66.0, 56.0, 4.0],
        ),
        "biolab:reactor" => furnace_screen(
            [26.0, 22.0],
            None,
            [134.0, 22.0],
            [22.0, 44.0, 132.0, 27.0],
            [50.0, 25.0, 76.0, 10.0],
        ),
        "biolab:press" => furnace_screen(
            [26.0, 35.0],
            None,
            [134.0, 35.0],
            [58.0, 17.0, 60.0, 46.0],
            [58.0, 66.0, 60.0, 4.0],
        ),
        "biolab:extractor" => furnace_screen(
            [26.0, 35.0],
            None,
            [134.0, 35.0],
            [50.0, 18.0, 76.0, 34.0],
            [50.0, 58.0, 76.0, 8.0],
        ),
        "biolab:cloning_vat" => furnace_screen(
            [20.0, 20.0],
            Some([20.0, 50.0]),
            [146.0, 35.0],
            [44.0, 17.0, 84.0, 55.0],
            [133.0, 20.0, 6.0, 48.0],
        ),
        "biolab:spectrometer" => furnace_screen(
            [10.0, 20.0],
            None,
            [10.0, 50.0],
            [32.0, 17.0, 137.0, 55.0],
            [13.0, 39.0, 12.0, 9.0],
        ),
        // Сетка 3×3: входы 0–7, последний слот — выход.
        "biolab:synthesizer" => MachineScreen {
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            slots: (0..machine.slots)
                .map(|i| Some([62.0 + (i % 3) as f32 * 18.0, 17.0 + (i / 3) as f32 * 18.0]))
                .collect(),
            inventory: [8.0, SCREEN_INVENTORY_Y],
            gauge: [7.0, 17.0, 52.0, 54.0],
            progress: [118.0, 17.0, 50.0, 54.0],
        },
        // Стол на той же сетке 3×3: слева табло с тем, что соберётся,
        // справа лампа и шкала сборки.
        "biolab:lab_bench" => MachineScreen {
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            slots: (0..machine.slots)
                .map(|i| Some([62.0 + (i % 3) as f32 * 18.0, 17.0 + (i / 3) as f32 * 18.0]))
                .collect(),
            inventory: [8.0, SCREEN_INVENTORY_Y],
            gauge: [6.0, 17.0, 54.0, 54.0],
            progress: [118.0, 17.0, 50.0, 54.0],
        },
        // Воронка: вход 0 и четыре выхода в ряд.
        "biolab:centrifuge" => MachineScreen {
            width: SCREEN_WIDTH,
            height: HOPPER_HEIGHT,
            slots: (0..machine.slots).map(|i| Some([44.0 + i as f32 * 18.0, 20.0])).collect(),
            inventory: [8.0, HOPPER_INVENTORY_Y],
            gauge: [14.0, 18.0, 20.0, 20.0],
            progress: [138.0, 24.0, 30.0, 10.0],
        },
        _ => return None,
    };
    Some(screen)
}

/// Выход ли этот слот: у центрифуги выходы 1–4, у остальных — последний слот.
/// Выходы на экране в бирюзовой рамке.
pub fn is_output_slot(id: &str, slot: usize) -> bool {
    match machine(id) {
        Some(m) if m.id == "biolab:centrifuge" => slot >= 1 && slot < m.slots,
        Some(m) => slot + 1 == m.slots,
        None => false,
    }
}

/// Рамка слота: на пиксель шире предмета с каждой стороны.
pub fn slot_frame(at: [f32; 2]) -> [f32; 4] {
    [at[0] - 1.0, at[1] - 1.0, 18.0, 18.0]
}

/// Где стоят 36 слотов инвентаря игрока: сперва хотбар, потом три ряда,
/// как их нумерует Horizon.
pub fn inventory_slots(screen: &MachineScreen) -> Vec<[f32; 2]> {
    let [x, y] = screen.inventory;
    (0..36usize)
        .map(|i| match i {
            0..9 => [x + i as f32 * 18.0, y + 58.0],
            _ => [x + ((i - 9) % 9) as f32 * 18.0, y + ((i - 9) / 9) as f32 * 18.0],
        })
        .collect()
}

/// Всё, что машина ставит на панель выше инвентаря: рамки видимых слотов,
/// прибор и шкала. Друг на друга они не налезают.
pub fn screen_rects(screen: &MachineScreen) -> Vec<[f32; 4]> {
    let mut rects: Vec<[f32; 4]> = screen.slots.iter().flatten().map(|at| slot_frame(*at)).collect();
    rects.push(screen.gauge);
    rects.push(screen.progress);
    rects
}

/// Пересекаются ли два прямоугольника (касание — не пересечение).
pub fn rects_overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

/// Слот данных: до первого пакета с сервера массив пуст.
pub fn screen_datum(data: &[i32], index: usize, default: i32) -> i32 {
    data.get(index).copied().unwrap_or(default)
}

/// Прогресс 0–1 из общих слотов данных.
pub fn screen_progress(data: &[i32]) -> f32 {
    let total = screen_datum(data, DATA_TOTAL, 0);
    if total <= 0 {
        return 0.0;
    }
    (screen_datum(data, DATA_PROGRESS, 0) as f32 / total as f32).clamp(0.0, 1.0)
}

/// Работает ли машина сейчас: горит «пламя» или идёт прогресс.
pub fn screen_working(data: &[i32]) -> bool {
    screen_datum(data, DATA_TOTAL, 0) > 0
        && (screen_datum(data, DATA_LIT, 0) > 0 || screen_datum(data, DATA_PROGRESS, 0) > 0)
}

/// Кадр анимации от системного времени: стоящая машина замирает на нулевом.
pub fn anim_frame(working: bool, millis: u128, period_ms: u128, frames: u32) -> u32 {
    if !working || frames == 0 || period_ms == 0 {
        return 0;
    }
    ((millis / period_ms) % u128::from(frames)) as u32
}

/// Фаза 0–1 бегущей анимации (капли, пузыри): за `cycle_ms` проходит круг.
pub fn anim_phase(working: bool, millis: u128, cycle_ms: u128) -> f32 {
    if !working || cycle_ms == 0 {
        return 0.0;
    }
    (millis % cycle_ms) as f32 / cycle_ms as f32
}

/// Змеевик перегонного куба: четыре горизонтальных колена внутри прибора.
/// Точка на трубе при пройденной доле пути `s` (0–1): по ней бегут капли.
pub fn coil_point(gauge: [f32; 4], s: f32) -> [f32; 2] {
    let (left, right) = (gauge[0] + 6.0, gauge[0] + gauge[2] - 6.0);
    let s = s.clamp(0.0, 0.999) * COIL_RUNS as f32;
    let run = s as usize;
    let along = s - run as f32;
    // Колена идут змейкой: чётные слева направо, нечётные обратно.
    let px = if run % 2 == 0 {
        left + (right - left) * along
    } else {
        right - (right - left) * along
    };
    [px, coil_row(gauge, run)]
}

/// Сколько колен у змеевика.
pub const COIL_RUNS: usize = 4;

/// Высота `run`-го колена змеевика.
pub fn coil_row(gauge: [f32; 4], run: usize) -> f32 {
    let step = (gauge[3] - 10.0) / (COIL_RUNS - 1) as f32;
    (gauge[1] + 5.0 + run as f32 * step).floor()
}

/// Пузырь в баке ферментера: `k`-й из нескольких, всплывает за круг фазы.
pub fn bubble_point(gauge: [f32; 4], k: u32, phase: f32) -> [f32; 2] {
    let [x, y, w, h] = gauge;
    let lane = (k * 37 % 5) as f32;
    let t = (phase + k as f32 * 0.29).fract();
    [x + 8.0 + lane * (w - 18.0) / 4.0, y + h - 6.0 - t * (h - 14.0)]
}

/// Градусник реактора: шкала от 0 до этого числа.
pub const THERMO_MAX: i32 = 1000;

/// Трубка градусника внутри прибора реактора: слева колба, сверху отметка
/// цели нагрева, снизу полоса диапазона рецепта и подпись.
pub fn thermo_tube(gauge: [f32; 4]) -> [f32; 4] {
    [gauge[0] + 12.0, gauge[1] + 8.0, gauge[2] - 14.0, 6.0]
}

/// Где на трубке стоит эта температура.
pub fn thermo_x(tube: [f32; 4], temperature: i32) -> f32 {
    tube[0] + tube[2] * temperature.clamp(0, THERMO_MAX) as f32 / THERMO_MAX as f32
}

/// Показания реактора из слотов данных.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReactorReading {
    pub temperature: i32,
    /// None — под реактором нет источника нагрева.
    pub target: Option<i32>,
    /// None — текущему рецепту температура не важна.
    pub range: Option<(i32, i32)>,
}

pub fn reactor_reading(data: &[i32]) -> ReactorReading {
    let target = screen_datum(data, DATA_HEAT_TARGET, -1);
    let (min, max) = (screen_datum(data, DATA_RANGE_MIN, 0), screen_datum(data, DATA_RANGE_MAX, 0));
    ReactorReading {
        temperature: screen_datum(data, DATA_TEMPERATURE, ROOM_TEMPERATURE),
        target: (target >= 0).then_some(target),
        range: (min != 0 || max != 0).then_some((min, max)),
    }
}

/// Набор элементов из подряд идущих слотов данных, по порядку `ELEMENTS`.
fn formula_from_data(data: &[i32], first: usize) -> Formula {
    let mut formula = Formula::default();
    for (i, count) in formula.iter_mut().enumerate() {
        *count = screen_datum(data, first + i, 0).max(0) as u32;
    }
    formula
}

/// Что набрано в синтезаторе.
pub fn synth_formula(data: &[i32]) -> Formula {
    formula_from_data(data, DATA_SYNTH_ELEMENTS)
}

/// Состав колбы во входе спектрометра.
pub fn spectrum(data: &[i32]) -> Formula {
    formula_from_data(data, DATA_SPECTRUM)
}

/// Статус синтезатора словами.
pub fn synth_status_text(status: i32) -> &'static str {
    match status {
        SYNTH_UNSTABLE => "Неустойчиво: связи не сходятся",
        SYNTH_STABLE => "Устойчиво: соберётся",
        SYNTH_NOT_BLUEPRINT => "Не совпадает с чертежом",
        _ => "Пусто: положите элементы",
    }
}

/// Цвет статуса синтезатора (RGB): формула на табло горит им.
pub fn synth_status_colour(status: i32) -> u32 {
    match status {
        SYNTH_UNSTABLE => 0xE8_5A4A,
        SYNTH_STABLE => 0x5A_E8B4,
        SYNTH_NOT_BLUEPRINT => 0xE8_B04A,
        _ => 0x8A_9AA0,
    }
}

/// Запись формулы по элементам: `C6`, `H12`, `O6` — чтобы переносить
/// длинную формулу на табло синтезатора между элементами, а не посреди.
pub fn formula_tokens(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    for c in text.chars() {
        if c.is_ascii_uppercase() || tokens.is_empty() {
            tokens.push(String::new());
        }
        if let Some(last) = tokens.last_mut() {
            last.push(c);
        }
    }
    tokens
}

/// Полоса спектра элемента `i`: 18 полос по ширине прибора, под ними символы
/// в два ряда вперемежку, чтобы двухбуквенные не налезали друг на друга.
pub fn spectrum_bar(gauge: [f32; 4], i: usize) -> [f32; 4] {
    let pitch = ((gauge[2] - 8.0) / ELEMENTS.len() as f32).floor();
    let left = (gauge[0] + (gauge[2] - pitch * ELEMENTS.len() as f32) / 2.0).floor();
    [left + i as f32 * pitch + 1.0, gauge[1] + 3.0, pitch - 2.0, gauge[3] - 25.0]
}

/// Где подписан символ элемента `i`: центр строки под полосой, в верхнем или
/// нижнем ряду.
pub fn spectrum_label(gauge: [f32; 4], i: usize) -> [f32; 2] {
    let bar = spectrum_bar(gauge, i);
    [bar[0] + bar[2] / 2.0, bar[1] + bar[3] + 2.0 + (i % 2) as f32 * 9.0]
}

/// Над какой полосой спектра курсор.
pub fn spectrum_index(gauge: [f32; 4], at: [f32; 2]) -> Option<usize> {
    (0..ELEMENTS.len()).find(|&i| {
        let b = spectrum_bar(gauge, i);
        at[0] >= b[0] - 1.0
            && at[0] < b[0] + b[2] + 1.0
            && at[1] >= gauge[1]
            && at[1] < gauge[1] + gauge[3]
    })
}

/// Высота полосы спектра 0–1: шкала по самому большому числу, но не мельче 4
/// атомов, иначе одинокий атом рисовался бы во всю высоту.
pub fn spectrum_level(formula: &Formula, i: usize) -> f32 {
    let top = formula.iter().copied().max().unwrap_or(0).max(4);
    formula.get(i).copied().unwrap_or(0) as f32 / top as f32
}

/// Ген в чане или экстракторе: номер в `GENES` и сила; −1 — гена нет.
pub fn screen_gene(data: &[i32]) -> Option<(&'static GeneKind, i32)> {
    let index = usize::try_from(screen_datum(data, DATA_GENE, -1)).ok()?;
    GENES.get(index).map(|g| (g, screen_datum(data, DATA_GENE_STRENGTH, 0)))
}

/// Штамм культуры во входе инкубатора; нули — культуры нет.
pub fn screen_strain(data: &[i32]) -> Option<Strain> {
    let values: [i32; 4] = std::array::from_fn(|i| screen_datum(data, DATA_STRAIN + i, 0));
    values.iter().any(|v| *v > 0).then(|| Strain::of(values))
}

/// Силуэт клона в чане: голова, тело, две руки, две ноги — прямоугольники
/// по центру прибора, ногами на дне.
pub fn clone_parts(gauge: [f32; 4]) -> [[f32; 4]; 6] {
    let cx = (gauge[0] + gauge[2] / 2.0).floor();
    let floor = gauge[1] + gauge[3] - 14.0;
    [
        [cx - 4.0, floor - 30.0, 8.0, 8.0],
        [cx - 4.0, floor - 22.0, 8.0, 12.0],
        [cx - 7.0, floor - 22.0, 3.0, 11.0],
        [cx + 4.0, floor - 22.0, 3.0, 11.0],
        [cx - 4.0, floor - 10.0, 3.0, 10.0],
        [cx + 1.0, floor - 10.0, 3.0, 10.0],
    ]
}

/// Часть силуэта, которая уже выросла: клон растёт снизу вверх по прогрессу.
pub fn clone_grown(part: [f32; 4], gauge: [f32; 4], progress: f32) -> Option<[f32; 4]> {
    let parts = clone_parts(gauge);
    let top = parts.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
    let bottom = parts.iter().map(|p| p[1] + p[3]).fold(f32::MIN, f32::max);
    let cut = (bottom - (bottom - top) * progress.clamp(0.0, 1.0)).round();
    let y = part[1].max(cut);
    let h = part[1] + part[3] - y;
    (h > 0.0).then_some([part[0], y, part[2], h])
}

/// Плита пресса: опускается от верха прибора к таблетке по ходу прогресса.
pub fn press_plate(gauge: [f32; 4], progress: f32) -> [f32; 4] {
    let travel = gauge[3] - 24.0;
    let y = (gauge[1] + 4.0 + travel * progress.clamp(0.0, 1.0)).floor();
    [gauge[0] + 6.0, y, gauge[2] - 12.0, 6.0]
}

/// Атомы с правильным окончанием: 1 атом, 2 атома, 5 атомов.
pub fn atoms_word(n: u32) -> String {
    let word = match (n % 10, n % 100) {
        (_, 11..=14) => "атомов",
        (1, _) => "атом",
        (2..=4, _) => "атома",
        _ => "атомов",
    };
    format!("{n} {word}")
}

/// Сколько секунд осталось до конца рецепта; None — машина не работает.
pub fn seconds_left(data: &[i32]) -> Option<i32> {
    let total = screen_datum(data, DATA_TOTAL, 0);
    if total <= 0 {
        return None;
    }
    let progress = screen_datum(data, DATA_PROGRESS, 0).clamp(0, total);
    // Двадцать тиков в секунду; неполная секунда считается целой.
    Some((total - progress + 19) / 20)
}

/// Подсказка над шкалой прогресса: что идёт и сколько осталось.
pub fn progress_tooltip(id: &str, data: &[i32]) -> Vec<String> {
    let verb = machine_verb(id);
    let Some(seconds) = seconds_left(data) else {
        return vec![format!("{verb} не идёт")];
    };
    let total = screen_datum(data, DATA_TOTAL, 0);
    let percent = screen_datum(data, DATA_PROGRESS, 0).clamp(0, total) * 100 / total;
    vec![format!("{verb}: {percent}%, осталось {seconds} с")]
}

/// Что видно на табло стола.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchShown {
    /// Входы пусты.
    Empty,
    /// Набор не совпал ни с одним рецептом.
    NoRecipe,
    /// Соберётся эта вещь: id и имя.
    Item(&'static str, &'static str),
    /// Рецепт совпал, но его результата нет в таблице.
    Unlisted,
}

/// Табло стола из слота данных. До первого пакета — пусто.
pub fn bench_shown(data: &[i32]) -> BenchShown {
    match screen_datum(data, DATA_BENCH_RESULT, BENCH_EMPTY) {
        BENCH_NONE => BenchShown::NoRecipe,
        BENCH_UNLISTED => BenchShown::Unlisted,
        i => match usize::try_from(i).ok().and_then(|i| BENCH_RESULTS.get(i)) {
            Some(&(id, name)) => BenchShown::Item(id, name),
            None => BenchShown::Empty,
        },
    }
}

/// Строка состояния стола: над табло и над шкалой.
pub fn bench_status_text(shown: BenchShown) -> &'static str {
    match shown {
        BenchShown::Empty => "Пусто: выложите ингредиенты",
        BenchShown::NoRecipe => "Нет такого рецепта",
        BenchShown::Item(..) | BenchShown::Unlisted => "Рецепт найден",
    }
}

/// Цвет табло стола (RGB): лампа и подпись горят им.
pub fn bench_status_colour(shown: BenchShown) -> u32 {
    match shown {
        BenchShown::Empty => 0x8A_9AA0,
        BenchShown::NoRecipe => 0xE8_5A4A,
        BenchShown::Item(..) | BenchShown::Unlisted => 0x5A_E8B4,
    }
}

/// Бегущая строка: длинное имя, которое не влезает в узкое табло, едет
/// по кругу на шаг `step` символов. Хвост по ширине обрезает тот, кто
/// рисует, — ширину знает только шрифт.
pub fn ticker(text: &str, step: usize) -> String {
    let looped: Vec<char> = text.chars().chain("   ".chars()).collect();
    let shift = step % looped.len();
    looped[shift..].iter().chain(&looped[..shift]).collect()
}

/// Подсказка над табло стола.
pub fn bench_tooltip(data: &[i32]) -> Vec<String> {
    let shown = bench_shown(data);
    match shown {
        BenchShown::Item(_, name) => vec![format!("Соберётся: {name}")],
        BenchShown::NoRecipe => vec![
            bench_status_text(shown).to_string(),
            "§7Нужен ровно набор рецепта: по предмету на запись, без лишнего".to_string(),
        ],
        _ => vec![bench_status_text(shown).to_string()],
    }
}

/// Подсказка над градусником: «Температура 58 °C, рецепту нужно 55–65 °C».
pub fn reactor_tooltip(data: &[i32]) -> Vec<String> {
    let r = reactor_reading(data);
    let mut lines = vec![match r.range {
        Some((min, max)) => format!("Температура {} °C, рецепту нужно {min}–{max} °C", r.temperature),
        None => format!("Температура {} °C, рецепту она не важна", r.temperature),
    }];
    lines.push(match r.target {
        Some(target) => format!("Источник греет до {target} °C"),
        None => format!("Нагрева нет: остывает до {ROOM_TEMPERATURE} °C"),
    });
    // Источник, который греет выше предела, рано или поздно сорвёт реактор.
    if r.target.is_some_and(|t| t > OVERHEAT) || r.temperature > OVERHEAT {
        lines.push(format!("§cВыше {OVERHEAT} °C — перегрев, содержимое выбросит"));
    } else {
        lines.push(format!("§7Перегрев выше {OVERHEAT} °C"));
    }
    lines
}

/// Подсказка над табло синтезатора: формула, имя и статус.
pub fn synth_tooltip(data: &[i32]) -> Vec<String> {
    let formula = synth_formula(data);
    if formula.iter().all(|c| *c == 0) {
        return vec![synth_status_text(SYNTH_EMPTY).to_string()];
    }
    let status = screen_datum(data, DATA_SYNTH_STATUS, SYNTH_EMPTY);
    let text = formula_text(&formula);
    let name = compound_name_of(&formula);
    let first = if name == text { text } else { format!("{text} — {name}") };
    vec![first, synth_status_text(status).to_string()]
}

/// Подсказка над спектром: состав целиком и полоса под курсором.
pub fn spectrum_tooltip(gauge: [f32; 4], data: &[i32], at: [f32; 2]) -> Vec<String> {
    let formula = spectrum(data);
    if formula.iter().all(|c| *c == 0) {
        return vec!["Спектр пуст: положите колбу во вход".to_string()];
    }
    let mut lines = vec![format!("Состав: {}", formula_text(&formula))];
    if let Some(i) = spectrum_index(gauge, at) {
        lines.push(format!("{}: {}", ELEMENTS[i].symbol, atoms_word(formula[i])));
    }
    lines
}

/// Ген и сила словами.
pub fn gene_tooltip(prefix: &str, data: &[i32]) -> Vec<String> {
    match screen_gene(data) {
        Some((gene, strength)) => vec![
            format!("{prefix}: {}, сила {strength}", gene.title),
            format!("§7{}", gene.about),
        ],
        None => vec![format!("{prefix}: нет")],
    }
}

/// Подсказка над полосками штамма.
pub fn strain_tooltip(data: &[i32]) -> Vec<String> {
    match screen_strain(data) {
        Some(strain) => {
            let mut lines = vec!["Штамм культуры".to_string()];
            for (name, value) in STRAIN_NAMES.iter().zip(strain.values()) {
                lines.push(format!("{name} {value} из 5"));
            }
            lines
        }
        None => vec!["Культуры нет: положите чашку во вход".to_string()],
    }
}

/// Подсказка экрана машины в точке `at` (от левого верхнего угла панели):
/// над прибором — его числа словами, над шкалой — прогресс. Пусто, если
/// курсор не над ними.
pub fn screen_tooltip(id: &str, data: &[i32], at: [f32; 2]) -> Vec<String> {
    let Some(screen) = machine_screen(id) else {
        return Vec::new();
    };
    if inside(screen.progress, at) {
        let mut lines = Vec::new();
        if id == SYNTHESIZER {
            let status = screen_datum(data, DATA_SYNTH_STATUS, SYNTH_EMPTY);
            lines.push(synth_status_text(status).to_string());
        }
        if id == LAB_BENCH {
            lines.push(bench_status_text(bench_shown(data)).to_string());
        }
        lines.extend(progress_tooltip(id, data));
        return lines;
    }
    if !inside(screen.gauge, at) {
        return Vec::new();
    }
    let working = screen_working(data);
    let percent = (screen_progress(data) * 100.0) as i32;
    match id {
        REACTOR => reactor_tooltip(data),
        SYNTHESIZER => synth_tooltip(data),
        LAB_BENCH => bench_tooltip(data),
        SPECTROMETER => spectrum_tooltip(screen.gauge, data, at),
        CLONING_VAT => {
            let mut lines = gene_tooltip("Ген клона", data);
            lines.push(format!("Клон вырос на {percent}%"));
            lines
        }
        EXTRACTOR => gene_tooltip("Последний ген", data),
        "biolab:incubator" => strain_tooltip(data),
        "biolab:still" if working => vec!["Змеевик горячий: идёт перегонка".to_string()],
        "biolab:still" => vec!["Змеевик холодный: перегонять нечего".to_string()],
        "biolab:fermenter" if working => vec!["Бак бродит".to_string()],
        "biolab:fermenter" => vec!["Бак стоит: брожения нет".to_string()],
        "biolab:centrifuge" if working => vec!["Барабан крутится: идёт разделение".to_string()],
        "biolab:centrifuge" => vec!["Барабан стоит".to_string()],
        "biolab:press" => vec![format!("Плита опущена на {percent}%")],
        _ => Vec::new(),
    }
}

// Цвета экранов: ARGB для заливок, RGB для текста. Сталь с бирюзовой
// подсветкой — те же цвета, что в текстурах `gui/machine` (tools/mkgui_machine.py).
pub const SCREEN_EDGE: u32 = 0xFF_101619;
pub const SCREEN_STEEL: u32 = 0xFF_5B666E;
pub const SCREEN_STEEL_LIGHT: u32 = 0xFF_9AA8B1;
pub const SCREEN_STEEL_DARK: u32 = 0xFF_343D43;
pub const SCREEN_TEAL: u32 = 0xFF_3FD8C4;
pub const SCREEN_TEAL_DIM: u32 = 0xFF_1E6A62;
pub const SCREEN_GLASS: u32 = 0xFF_0E1A1E;
pub const SCREEN_SLOT: u32 = 0xFF_2C353B;
pub const SCREEN_RED: u32 = 0xFF_D8443A;
pub const SCREEN_AMBER: u32 = 0xFF_E0A040;
pub const SCREEN_GREEN: u32 = 0xFF_5AE8B4;
pub const SCREEN_TEXT: u32 = 0xE6_F4F2;
pub const SCREEN_TEXT_DIM: u32 = 0x9F_B8BC;

#[cfg(test)]
mod screen_tests {
    use super::*;

    /// Где можно рисовать прибор: под заголовком, над подписью инвентаря
    /// (она на 11 выше первого ряда), с полями по краям панели.
    fn work_area(s: &MachineScreen) -> [f32; 4] {
        [4.0, SCREEN_HEADER, s.width - 8.0, s.inventory[1] - 12.0 - SCREEN_HEADER]
    }

    fn contains(outer: [f32; 4], inner: [f32; 4]) -> bool {
        inner[0] >= outer[0]
            && inner[1] >= outer[1]
            && inner[0] + inner[2] <= outer[0] + outer[2]
            && inner[1] + inner[3] <= outer[1] + outer[3]
    }

    fn centre(r: [f32; 4]) -> [f32; 2] {
        [r[0] + r[2] / 2.0, r[1] + r[3] / 2.0]
    }

    #[test]
    fn every_machine_has_a_screen_for_its_slots() {
        for m in &MACHINES {
            let s = machine_screen(m.id).unwrap_or_else(|| panic!("{} без экрана", m.id));
            assert_eq!(s.slots.len(), m.slots, "{}", m.id);
            assert_eq!(s.width, SCREEN_WIDTH, "{}", m.id);
            assert!(!machine_title(m.id).is_empty(), "{} без названия", m.id);
            assert_ne!(machine_verb(m.id), "Работа", "{} без глагола", m.id);
        }
        assert_eq!(machine_screen("biolab:pen"), None);
        assert_eq!(machine_screen("minecraft:furnace"), None);
    }

    #[test]
    fn slots_and_gauges_fit_the_panel_and_do_not_overlap() {
        for m in &MACHINES {
            let s = machine_screen(m.id).unwrap();
            let area = work_area(&s);
            let rects = screen_rects(&s);
            for (i, r) in rects.iter().enumerate() {
                assert!(contains(area, *r), "{}: {r:?} вне панели {area:?}", m.id);
                for other in &rects[i + 1..] {
                    assert!(!rects_overlap(*r, *other), "{}: {r:?} налезает на {other:?}", m.id);
                }
            }
            let panel = [0.0, 0.0, s.width, s.height];
            for at in inventory_slots(&s) {
                let frame = slot_frame(at);
                assert!(contains([3.0, 3.0, s.width - 6.0, s.height - 6.0], frame), "{}: инвентарь {frame:?}", m.id);
                assert!(contains(panel, frame));
                for r in &rects {
                    assert!(!rects_overlap(*r, frame), "{}: {r:?} налезает на инвентарь", m.id);
                }
            }
        }
    }

    #[test]
    fn fuel_slot_shows_only_at_the_vat() {
        for m in MACHINES.iter().filter(|m| m.menu == "minecraft:furnace") {
            let s = machine_screen(m.id).unwrap();
            assert_eq!(s.slots[SLOT_FUEL].is_some(), m.id == CLONING_VAT, "{}", m.id);
            assert!(s.slots[SLOT_INPUT].is_some() && s.slots[SLOT_OUTPUT].is_some(), "{}", m.id);
            assert!(is_output_slot(m.id, SLOT_OUTPUT) && !is_output_slot(m.id, SLOT_FUEL), "{}", m.id);
        }
        let outputs: Vec<usize> = (0..6).filter(|i| is_output_slot("biolab:centrifuge", *i)).collect();
        assert_eq!(outputs, vec![1, 2, 3, 4]);
        assert!(is_output_slot(SYNTHESIZER, 8) && !is_output_slot(SYNTHESIZER, 7));
        assert!(is_output_slot(LAB_BENCH, 8) && !is_output_slot(LAB_BENCH, 0));
        assert!(!is_output_slot("biolab:pen", 0));
    }

    #[test]
    fn grid_and_hopper_keep_vanilla_places() {
        // Horizon ставит слоты этих меню сам (DispenserMenu, HopperMenu):
        // раскладка обязана совпасть, иначе рамки разойдутся с предметами.
        let synth = machine_screen(SYNTHESIZER).unwrap();
        assert_eq!(synth.slots[8], Some([98.0, 53.0]));
        assert_eq!(synth.slots[0], Some([62.0, 17.0]));
        assert_eq!((synth.height, synth.inventory), (166.0, [8.0, 84.0]));
        let bench = machine_screen(LAB_BENCH).unwrap();
        assert_eq!(bench.slots, synth.slots, "стол на той же сетке 3×3");
        assert_eq!((bench.height, bench.inventory), (166.0, [8.0, 84.0]));
        let centrifuge = machine_screen("biolab:centrifuge").unwrap();
        assert_eq!(centrifuge.slots[0], Some([44.0, 20.0]));
        assert_eq!(centrifuge.slots[4], Some([116.0, 20.0]));
        assert_eq!((centrifuge.height, centrifuge.inventory), (133.0, [8.0, 51.0]));
        assert_eq!(inventory_slots(&centrifuge)[0], [8.0, 109.0]);
        assert_eq!(inventory_slots(&centrifuge)[35], [152.0, 87.0]);
    }

    #[test]
    fn instrument_parts_stay_inside_their_gauges() {
        let still = machine_screen("biolab:still").unwrap();
        for k in 0..=20 {
            let p = coil_point(still.gauge, k as f32 / 20.0);
            assert!(inside(still.gauge, p), "капля {p:?}");
        }
        let fermenter = machine_screen("biolab:fermenter").unwrap();
        for k in 0..6 {
            for step in 0..10 {
                let p = bubble_point(fermenter.gauge, k, step as f32 / 10.0);
                assert!(inside(fermenter.gauge, p), "пузырь {p:?}");
            }
        }
        let reactor = machine_screen(REACTOR).unwrap();
        let tube = thermo_tube(reactor.gauge);
        assert!(contains(reactor.gauge, tube));
        assert!(thermo_x(tube, OVERHEAT) < tube[0] + tube[2]);
        assert_eq!(thermo_x(tube, -50), tube[0]);
        assert_eq!(thermo_x(tube, 5000), tube[0] + tube[2]);
        let spectro = machine_screen(SPECTROMETER).unwrap();
        for i in 0..ELEMENTS.len() {
            let bar = spectrum_bar(spectro.gauge, i);
            assert!(contains(spectro.gauge, bar), "полоса {i}: {bar:?}");
            assert_eq!(spectrum_index(spectro.gauge, centre(bar)), Some(i));
            let label = spectrum_label(spectro.gauge, i);
            // Символ шириной до 12 и высотой 8 вокруг центра строки.
            assert!(contains(spectro.gauge, [label[0] - 6.0, label[1], 12.0, 8.0]), "символ {i}");
            if i + 2 < ELEMENTS.len() {
                let next = spectrum_label(spectro.gauge, i + 2);
                assert!(next[0] - label[0] >= 12.0, "символы {i} и {} налезают", i + 2);
            }
            if i + 1 < ELEMENTS.len() {
                assert!(!rects_overlap(bar, spectrum_bar(spectro.gauge, i + 1)));
            }
        }
        let vat = machine_screen(CLONING_VAT).unwrap();
        for part in clone_parts(vat.gauge) {
            assert!(contains(vat.gauge, part), "клон {part:?}");
            assert_eq!(clone_grown(part, vat.gauge, 0.0), None);
            assert_eq!(clone_grown(part, vat.gauge, 1.0), Some(part));
        }
        // На середине ноги уже есть, головы ещё нет.
        let parts = clone_parts(vat.gauge);
        assert_eq!(clone_grown(parts[4], vat.gauge, 0.5), Some(parts[4]));
        assert_eq!(clone_grown(parts[0], vat.gauge, 0.5), None);
        let press = machine_screen("biolab:press").unwrap();
        let (up, down) = (press_plate(press.gauge, 0.0), press_plate(press.gauge, 1.0));
        assert!(contains(press.gauge, up) && contains(press.gauge, down));
        assert!(down[1] > up[1]);
    }

    #[test]
    fn animation_follows_time_only_while_working() {
        assert_eq!(anim_frame(false, 12_345, 100, 4), 0);
        let frames: Vec<u32> = [0, 100, 250, 399, 400].iter().map(|t| anim_frame(true, *t, 100, 4)).collect();
        assert_eq!(frames, vec![0, 1, 2, 3, 0]);
        assert_eq!(anim_frame(true, 500, 100, 0), 0);
        assert_eq!(anim_phase(false, 777, 1000), 0.0);
        assert_eq!(anim_phase(true, 1_250, 1000), 0.25);
        assert!(!screen_working(&[]));
        assert!(screen_working(&[1, 1, 0, 100]));
        assert!(!screen_working(&[0, 0, 0, 0]));
        assert_eq!(screen_progress(&[]), 0.0);
        assert_eq!(screen_progress(&[1, 1, 25, 100]), 0.25);
        assert_eq!(screen_progress(&[1, 1, 300, 100]), 1.0);
    }

    #[test]
    fn data_slots_read_by_the_contract() {
        assert_eq!(reactor_reading(&[]), ReactorReading { temperature: ROOM_TEMPERATURE, target: None, range: None });
        assert_eq!(
            reactor_reading(&[1, 1, 40, 100, 58, 300, 55, 65]),
            ReactorReading { temperature: 58, target: Some(300), range: Some((55, 65)) }
        );
        assert_eq!(screen_gene(&[]).map(|g| g.0.name), None);
        assert_eq!(screen_gene(&[0, 0, 0, 0, -1, 0]).map(|g| g.0.name), None);
        assert_eq!(screen_gene(&[0, 0, 0, 0, 1, 3]).map(|g| (g.0.name, g.1)), Some(("dwarfism", 3)));
        assert_eq!(screen_gene(&[0, 0, 0, 0, 999, 3]).map(|g| g.0.name), None);
        assert_eq!(screen_strain(&[0; 8]), None);
        assert_eq!(screen_strain(&[0, 0, 0, 0, 3, 2, 4, 1]), Some(Strain::of([3, 2, 4, 1])));
        let mut data = vec![0; machine_data_slots(SYNTHESIZER)];
        data[DATA_SYNTH_ELEMENTS] = 12;
        data[DATA_SYNTH_ELEMENTS + 1] = 6;
        data[DATA_SYNTH_ELEMENTS + 3] = 6;
        assert_eq!(formula_text(&synth_formula(&data)), "C6H12O6");
        assert_eq!(formula_tokens("C6H12O6"), vec!["C6", "H12", "O6"]);
        assert_eq!(formula_tokens("ClNa").concat(), "ClNa");
        assert_eq!(formula_tokens(""), Vec::<String>::new());
        let mut salt = Formula::default();
        salt[4] = 1;
        salt[11] = 1;
        assert_eq!(spectrum_level(&salt, 4), 0.25);
        assert_eq!(spectrum_level(&salt, 0), 0.0);
    }

    #[test]
    fn tooltips_say_numbers_in_words() {
        let reactor = machine_screen(REACTOR).unwrap();
        let typical = [1, 1, 40, 100, 58, 300, 55, 65];
        let lines = screen_tooltip(REACTOR, &typical, centre(reactor.gauge));
        assert_eq!(lines[0], "Температура 58 °C, рецепту нужно 55–65 °C");
        assert_eq!(lines[1], "Источник греет до 300 °C");
        assert_eq!(lines[2], "§7Перегрев выше 900 °C");
        let lava = [1, 1, 40, 100, 870, 950, 0, 0];
        let lines = screen_tooltip(REACTOR, &lava, centre(reactor.gauge));
        assert_eq!(lines[0], "Температура 870 °C, рецепту она не важна");
        assert!(lines[2].starts_with("§cВыше 900 °C"));
        let empty = screen_tooltip(REACTOR, &[], centre(reactor.gauge));
        assert_eq!(empty[0], "Температура 20 °C, рецепту она не важна");
        assert_eq!(empty[1], "Нагрева нет: остывает до 20 °C");

        let still = machine_screen("biolab:still").unwrap();
        assert_eq!(screen_tooltip("biolab:still", &[1, 1, 40, 100], centre(still.progress)), vec!["Перегонка: 40%, осталось 3 с"]);
        assert_eq!(screen_tooltip("biolab:still", &[], centre(still.progress)), vec!["Перегонка не идёт"]);
        assert_eq!(screen_tooltip("biolab:still", &[], centre(still.gauge)), vec!["Змеевик холодный: перегонять нечего"]);
        assert_eq!(screen_tooltip("biolab:still", &[1, 1, 40, 100], [1.0, 1.0]), Vec::<String>::new());

        let synth = machine_screen(SYNTHESIZER).unwrap();
        let mut data = vec![0; machine_data_slots(SYNTHESIZER)];
        assert_eq!(screen_tooltip(SYNTHESIZER, &data, centre(synth.gauge)), vec!["Пусто: положите элементы"]);
        assert_eq!(screen_tooltip(SYNTHESIZER, &[], centre(synth.gauge)), vec!["Пусто: положите элементы"]);
        data[DATA_SYNTH_STATUS] = SYNTH_STABLE;
        data[DATA_SYNTH_ELEMENTS] = 12;
        data[DATA_SYNTH_ELEMENTS + 1] = 6;
        data[DATA_SYNTH_ELEMENTS + 3] = 6;
        assert_eq!(
            screen_tooltip(SYNTHESIZER, &data, centre(synth.gauge)),
            vec!["C6H12O6 — глюкоза", "Устойчиво: соберётся"]
        );
        data[DATA_SYNTH_STATUS] = SYNTH_UNSTABLE;
        data[DATA_SYNTH_ELEMENTS + 3] = 7;
        assert_eq!(
            screen_tooltip(SYNTHESIZER, &data, centre(synth.gauge)),
            vec!["C6H12O7", "Неустойчиво: связи не сходятся"]
        );
        assert_eq!(screen_tooltip(SYNTHESIZER, &data, centre(synth.progress))[0], "Неустойчиво: связи не сходятся");

        let bench = machine_screen(LAB_BENCH).unwrap();
        assert_eq!(screen_tooltip(LAB_BENCH, &[], centre(bench.gauge)), vec!["Пусто: выложите ингредиенты"]);
        let mut data = vec![0; machine_data_slots(LAB_BENCH)];
        data[DATA_BENCH_RESULT] = BENCH_NONE;
        assert_eq!(screen_tooltip(LAB_BENCH, &data, centre(bench.gauge))[0], "Нет такого рецепта");
        data[DATA_BENCH_RESULT] = bench_result_index("biolab:microscope");
        assert_eq!(screen_tooltip(LAB_BENCH, &data, centre(bench.gauge)), vec!["Соберётся: Микроскоп"]);
        data[DATA_TOTAL] = 40;
        data[DATA_PROGRESS] = 10;
        assert_eq!(
            screen_tooltip(LAB_BENCH, &data, centre(bench.progress)),
            vec!["Рецепт найден", "Сборка: 25%, осталось 2 с"]
        );
        data[DATA_BENCH_RESULT] = BENCH_UNLISTED;
        assert_eq!(screen_tooltip(LAB_BENCH, &data, centre(bench.gauge)), vec!["Рецепт найден"]);

        let spectro = machine_screen(SPECTROMETER).unwrap();
        let mut data = vec![0; machine_data_slots(SPECTROMETER)];
        assert_eq!(screen_tooltip(SPECTROMETER, &data, centre(spectro.gauge)), vec!["Спектр пуст: положите колбу во вход"]);
        data[DATA_SPECTRUM + 4] = 1;
        data[DATA_SPECTRUM + 11] = 1;
        let na = centre(spectrum_bar(spectro.gauge, 4));
        assert_eq!(screen_tooltip(SPECTROMETER, &data, na), vec!["Состав: ClNa", "Na: 1 атом"]);

        let vat = machine_screen(CLONING_VAT).unwrap();
        let lines = screen_tooltip(CLONING_VAT, &[1, 1, 100, 400, 0, 2], centre(vat.gauge));
        assert_eq!(lines[0], "Ген клона: Гигантизм, сила 2");
        assert_eq!(lines[2], "Клон вырос на 25%");
        assert_eq!(screen_tooltip(CLONING_VAT, &[], centre(vat.gauge))[0], "Ген клона: нет");

        let extractor = machine_screen(EXTRACTOR).unwrap();
        assert_eq!(screen_tooltip(EXTRACTOR, &[0, 0, 0, 0, 1, 1], centre(extractor.gauge))[0], "Последний ген: Карликовость, сила 1");
        assert_eq!(screen_tooltip(EXTRACTOR, &[], centre(extractor.gauge)), vec!["Последний ген: нет"]);

        let incubator = machine_screen("biolab:incubator").unwrap();
        let lines = screen_tooltip("biolab:incubator", &[0, 0, 0, 0, 3, 2, 4, 1], centre(incubator.gauge));
        assert_eq!(lines, vec!["Штамм культуры", "Заразность 3 из 5", "Инкубация 2 из 5", "Тяжесть 4 из 5", "Стойкость 1 из 5"]);
        assert_eq!(screen_tooltip("biolab:incubator", &[], centre(incubator.gauge)), vec!["Культуры нет: положите чашку во вход"]);

        let press = machine_screen("biolab:press").unwrap();
        assert_eq!(screen_tooltip("biolab:press", &[1, 1, 50, 100], centre(press.gauge)), vec!["Плита опущена на 50%"]);
        let centrifuge = machine_screen("biolab:centrifuge").unwrap();
        assert_eq!(screen_tooltip("biolab:centrifuge", &[1, 1, 5, 100], centre(centrifuge.gauge)), vec!["Барабан крутится: идёт разделение"]);
        assert_eq!(screen_tooltip("biolab:pen", &[], [50.0, 50.0]), Vec::<String>::new());
    }

    #[test]
    fn the_bench_board_reads_its_data_slot() {
        assert_eq!(bench_shown(&[]), BenchShown::Empty);
        let mut data = vec![0; machine_data_slots(LAB_BENCH)];
        assert_eq!(bench_shown(&data), BenchShown::Item("biolab:syringe", "Шприц"), "номер 0 — первый результат");
        data[DATA_BENCH_RESULT] = BENCH_EMPTY;
        assert_eq!(bench_shown(&data), BenchShown::Empty);
        data[DATA_BENCH_RESULT] = BENCH_RESULTS.len() as i32;
        assert_eq!(bench_shown(&data), BenchShown::Empty, "номер за таблицей — как пусто");
        assert_eq!(seconds_left(&[]), None);
        assert_eq!(seconds_left(&[1, 1, 0, 40]), Some(2));
        assert_eq!(seconds_left(&[1, 1, 39, 40]), Some(1));
        // Бегущая строка проходит круг и возвращается к началу.
        assert_eq!(ticker("Яд", 0), "Яд   ");
        assert_eq!(ticker("Яд", 1), "д   Я");
        assert_eq!(ticker("Яд", 5), "Яд   ");
    }

    #[test]
    fn atoms_agree_with_their_number() {
        let words: Vec<String> = [1, 2, 4, 5, 11, 12, 21, 22, 25].iter().map(|n| atoms_word(*n)).collect();
        assert_eq!(
            words,
            vec!["1 атом", "2 атома", "4 атома", "5 атомов", "11 атомов", "12 атомов", "21 атом", "22 атома", "25 атомов"]
        );
    }
}
