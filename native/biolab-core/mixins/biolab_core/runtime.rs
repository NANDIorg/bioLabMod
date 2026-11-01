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

pub const MACHINES: [Machine; 10] = [
    Machine { id: "biolab:still", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:incubator", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:fermenter", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:centrifuge", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:synthesizer", slots: 9, menu: "minecraft:generic_3x3" },
    Machine { id: "biolab:reactor", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:press", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:extractor", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:cloning_vat", slots: 3, menu: "minecraft:furnace" },
    Machine { id: "biolab:spectrometer", slots: 3, menu: "minecraft:furnace" },
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
// дать. Поэтому при первом входе в мир игрок получает «Журнал лаборанта»:
// что для чего, из чего крафтится и с чего начинать. Текст лежит данными
// мода (`data/biolab/manual/ru.txt`), страницы разделены строкой `---`, так
// что правка текста не требует пересборки нативной части.

/// Книга — ванильная `written_book`: свой предмет с тем же шаблоном клиент
/// как книгу не открывает (проверено в игре — экран не появлялся), а имя
/// ей всё равно даёт заголовок внутри.
pub const MANUAL_ID: &str = "minecraft:written_book";
/// Где лежит текст книги внутри данных мода.
pub const MANUAL_PATH: &str = "data/biolab/manual/ru.txt";
pub const MANUAL_TITLE: &str = "Журнал лаборанта";
pub const MANUAL_AUTHOR: &str = "BioLab";
/// Сколько страниц книга вмещает и какой длины бывает страница.
pub const MANUAL_MAX_PAGES: usize = 100;
pub const MANUAL_MAX_PAGE: usize = 1024;

/// Страницы книги из текста: разделитель — строка из трёх дефисов.
pub fn manual_pages(text: &str) -> Vec<String> {
    text.split("\n---\n")
        .map(|page| page.trim().to_string())
        .filter(|page| !page.is_empty())
        .map(|mut page| {
            if page.len() > MANUAL_MAX_PAGE {
                page.truncate(MANUAL_MAX_PAGE);
            }
            page
        })
        .take(MANUAL_MAX_PAGES)
        .collect()
}

/// Запасная страница: если данные мода не прочитались, книга всё равно
/// должна объяснить, куда смотреть.
pub fn manual_fallback() -> Vec<String> {
    vec![format!(
        "{MANUAL_TITLE}\n\nТекст руководства не прочитался: проверьте файл\n{MANUAL_PATH}\n\nКоманда /biolab покажет, что умеет мод."
    )]
}

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
    fn the_manual_splits_into_pages() {
        let text = "Первая\n\nстрока\n---\nВторая\n---\n\n---\nТретья\n";
        let pages = manual_pages(text);
        assert_eq!(pages.len(), 3, "пустая страница не считается: {pages:?}");
        assert_eq!(pages[0], "Первая\n\nстрока");
        assert_eq!(pages[2], "Третья");
        assert!(manual_pages("").is_empty());

        // Слишком длинная страница обрезается, слишком длинная книга — тоже.
        let long = "а".repeat(MANUAL_MAX_PAGE * 2);
        assert_eq!(manual_pages(&long)[0].len(), MANUAL_MAX_PAGE);
        let many = (0..MANUAL_MAX_PAGES + 20)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("\n---\n");
        assert_eq!(manual_pages(&many).len(), MANUAL_MAX_PAGES);

        assert!(!manual_fallback().is_empty(), "без данных книга всё равно что-то говорит");
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
}
