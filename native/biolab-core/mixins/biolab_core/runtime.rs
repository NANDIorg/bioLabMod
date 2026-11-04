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

/// Подпись колбы: «Колба: вода, 250 мл». Вещество — каноническим именем из
/// `KNOWN`, незнакомое — формулой. Имя из журнала мира сюда не идёт (его
/// показывает подсказка): подпись обновляет тик инвентаря, и читать файл
/// журнала на каждый стек каждый тик — дорого.
pub fn flask_label(contents: &str, ml: i32) -> String {
    let name = match formula_from_text(contents) {
        Some(formula) => compound_name_of(&formula),
        // Старая колба со словом вместо формулы.
        None => compound_name(contents).to_string(),
    };
    format!("Колба: {name}, {ml} мл")
}

// ---- Имена и подсказки предметов (GDD 4.12, ревизия предметов) ----
//
// Имена — в языковых файлах, по одной схеме: препарат — «<вещество>, <форма>»
// («Пенициллин, таблетка», «Яд, дротик»), экстракт — «Экстракт <чего>»,
// элемент — названием элемента, машина и инструмент — существительным.
// Предмет со своими данными подписывается «<имя предмета>: <данные>»
// («Колба: вода, 250 мл», «Фрагмент ДНК: Хитин ++», «Культура: З2 И3 Т2 С1»),
// и подпись всегда выводится из данных.
//
// Подсказка — одна-две строки под строками данных: что предмет делает и как
// его применить. Применение пишется «ПКМ — …», «ПКМ по мобу — …», «ПКМ по
// блоку — …», машина — «вход → выход». С заглавной, без точки в конце.

/// Цвет строк-подсказок: серо-голубой, тусклее строк данных (`0xAAAAAA`),
/// чтобы не спорить с ними. Упакован, как строки подсказки ждёт Horizon
/// (`rgba` в `hud.rs`): `0xAABBGGRR`, то есть R 7F, G 8F, B 96, непрозрачный.
pub const HINT_INK: u32 = 0xFF96_8F7F;

/// Подсказки предметов. Элементам подсказку собирает `item_hint` из таблицы
/// `ELEMENTS`, здесь их нет. Тест сверяет таблицу с `content/assets/biolab/items/`.
pub const ITEM_HINTS: [(&str, &[&str]); 96] = [
    // Машины и блоки.
    ("biolab:lab_bench", &["Точная сборка: шприцы, приборы, препараты", "Выложите в сетку ровно набор рецепта"]),
    ("biolab:still", &["Неживое сырьё → элементы и агар"]),
    ("biolab:incubator", &["Мазок → культура, хлеб → плесень", "Культура на входе — селекция штамма"]),
    ("biolab:fermenter", &["Живое → экстракты и питательный раствор"]),
    ("biolab:centrifuge", &["Разделяет на основной и побочный выход", "Колбу раскладывает на элементы"]),
    ("biolab:synthesizer", &["Элементы в сетке → колба с веществом", "Чертёж в сетке задаёт нужный состав"]),
    ("biolab:reactor", &["Греется снизу: костёр, магма, огонь, лава", "Выше 900° — содержимое сгорит"]),
    ("biolab:press", &["Экстракт → таблетки, уголь → антидот"]),
    ("biolab:extractor", &["Достаёт ген: ткань, урожай, части животных"]),
    ("biolab:cloning_vat", &["Фрагмент ДНК + раствор → клон с геном"]),
    ("biolab:spectrometer", &["Опознаёт вещество в колбе, печатает чертёж", "Колба не тратится"]),
    ("biolab:pen", &["Держит подопытного моба на месте"]),
    ("biolab:console", &["ПКМ — отчёт о подопытных и опытах"]),
    ("biolab:gm_plot", &["ПКМ фрагментом ДНК или урожаем — посадить", "Спелую — ПКМ: собрать урожай"]),
    // Химия.
    ("biolab:flask", &["Центрифуга разложит, спектрометр опознает"]),
    ("biolab:blueprint", &["В синтезаторе задаёт состав, не тратится"]),
    ("biolab:acid_puddle_bucket", &["ПКМ по блоку — разлить: кислота жжёт", "Ведро остаётся пустым"]),
    ("biolab:sludge_bucket", &["ПКМ по блоку — разлить: шлам травит", "Ведро остаётся пустым"]),
    ("biolab:slime_bucket", &["ПКМ по блоку — разлить: слизь заражает", "Ведро остаётся пустым"]),
    // Болезни и штаммы.
    ("biolab:agar", &["Основа бинта, раствора и ГМ-грядки"]),
    ("biolab:swab", &["В инкубаторе вырастает в культуру"]),
    ("biolab:culture", &["ПКМ — заразить себя этим штаммом", "Инкубатор — селекция, реактор — ослабить"]),
    ("biolab:culture_weakened", &["Со шприцем на столе — вакцина"]),
    ("biolab:extract_raw", &["Центрифуга сделает из него сыворотку"]),
    ("biolab:remedy", &["ПКМ — снижает заражение"]),
    ("biolab:analyzer", &["В руке: заражение и кровь в углу экрана"]),
    // Экстракты.
    ("biolab:mold", &["Ферментер сделает экстракт пенициллина"]),
    ("biolab:extract_penicillin", &["Пресс — таблетки, со шприцем — инъекция"]),
    ("biolab:extract_salicylate", &["Пресс — таблетки; стол — мазь, аэрозоль"]),
    ("biolab:extract_caffeine", &["Пресс — таблетки; стол — шприц, аэрозоль"]),
    ("biolab:extract_venom", &["Со шприцем на столе — инъекция яда"]),
    ("biolab:extract_sedative", &["Со шприцем на столе — инъекция"]),
    // Препараты: первая строка — действие вещества, вторая — как принять.
    ("biolab:pill_penicillin", &["Лечит заражение, пока в крови", "ПКМ — принять"]),
    ("biolab:syringe_penicillin", &["Лечит заражение, пока в крови", "ПКМ — себе, по мобу — регенерация"]),
    ("biolab:pill_analgesic", &["Сопротивление урону", "ПКМ — принять"]),
    ("biolab:ointment_analgesic", &["Сопротивление урону", "ПКМ — нанести"]),
    ("biolab:spray_analgesic", &["Сопротивление урону, действует быстро", "ПКМ — вдохнуть"]),
    ("biolab:pill_stimulant", &["Скорость и спешка, потом ломка", "ПКМ — принять"]),
    ("biolab:syringe_stimulant", &["Скорость и спешка, потом ломка", "ПКМ — ввести себе"]),
    ("biolab:spray_stimulant", &["Скорость и спешка, потом ломка", "ПКМ — вдохнуть"]),
    ("biolab:pill_antidote", &["Выводит из крови остальные вещества", "ПКМ — принять"]),
    ("biolab:syringe_vaccine", &["Иммунитет к заражению", "ПКМ — ввести себе"]),
    ("biolab:syringe_sedative", &["Сила 2: зомби, скелет, свинья", "ПКМ — себе, по мобу — усыпить"]),
    ("biolab:syringe_venom", &["Токсикоз", "ПКМ — себе, по мобу — отравить"]),
    ("biolab:syringe", &["Наполняется экстрактом на столе"]),
    ("biolab:dart_sedative", &["ПКМ — бросить в моба, как снежок", "Сила 2: зомби, свинья; рукой — до 8 блоков"]),
    ("biolab:dart_venom", &["ПКМ — бросить в моба, как снежок", "Отравляет, рукой — до 8 блоков"]),
    ("biolab:bandage", &["ПКМ — остановить кровотечение"]),
    // Оружие для дротиков (GDD 4.20): дальность — из `DART_WEAPONS`, тест сверяет.
    ("biolab:blowpipe", &["ПКМ — дротик из инвентаря, до 16 блоков", "Тихая: сенсоры не слышат, перезарядка 1 с"]),
    ("biolab:tranq_crossbow", &["Держите ПКМ — зарядить, ПКМ — выстрел", "Дротик из инвентаря, до 32 блоков"]),
    ("biolab:air_rifle", &["ПКМ — магазин на 4, держите ПКМ — прицел", "Отпустите — выстрел до 48 блоков"]),
    ("biolab:co2_canister", &["Газ ружья: 12 выстрелов, сам подключится"]),
    ("biolab:co2_canister_empty", &["В ферментере заправится углекислым газом"]),
    ("biolab:scalpel", &["ПКМ по мутанту на операционном столе", "Срезает плоть, удар — кровоточащая рана"]),
    // Генетика.
    ("biolab:stretcher", &["ПКМ по усыплённому мобу — погрузить", "С пациентом — ПКМ по операционному столу"]),
    ("biolab:operating_table", &["Стол из секций в ряд: зомби — 2, гигант — 3+", "ПКМ носилками — уложить пациента"]),
    ("biolab:mutant_tissue", &["Срезают скальпелем с мутанта на столе", "Экстрактор достанет из неё ген"]),
    ("biolab:dna_fragment", &["ПКМ по мобу — вживить ген, по себе — генотерапия", "Сажается в грядку, в чане растит клона"]),
    ("biolab:microscope", &["ПКМ по мобу — прочитать геном"]),
    ("biolab:nutrient_broth", &["Корм клонирующего чана"]),
    ("biolab:gm_crop", &["ПКМ — съесть: действует по геному", "Сажается в грядку, экстрактор даст ген"]),
    ("biolab:mutagen", &["Со шприцем — инъекция, два в ведре — лужа"]),
    ("biolab:syringe_mutagen", &["ПКМ по мобу — копит мутацию генома"]),
    ("biolab:mutagen_bucket", &["ПКМ по блоку — разлить лужу мутагена", "Ведро остаётся пустым"]),
    // Книга.
    ("biolab:lab_manual", &["ПКМ — открыть карту исследований"]),
    // Свои растения (GDD 4.20, `plants.rs`): семена сажает `plant_seed.rs`,
    // крик мандрагоры — `mandrake.rs`.
    ("biolab:valerian_seeds", &["ПКМ по пашне — посадить", "Дико: луга, равнины, цветочный лес"]),
    ("biolab:belladonna_seeds", &["ПКМ по пашне — посадить", "Дико: леса и тёмный лес"]),
    ("biolab:mandrake_seeds", &["ПКМ по пашне — посадить", "Дико: болота и мангровые болота"]),
    ("biolab:valerian_root", &["Сырьё успокоительного отвара"]),
    ("biolab:belladonna_berries", &["Похожи на сладкие, но ядовиты", "Добавка к мандрагоровой настойке"]),
    ("biolab:mandrake_root", &["Сырьё мандрагоровой настойки", "Спелую рвите в шлеме: она кричит"]),
    // Снотворные (GDD 4.20): первой строкой — сила и кого берёт.
    ("biolab:spirit", &["Брожение сахара: основа настойки и газов", "Стол: с серой — эфир, с хлором — хлороформ"]),
    ("biolab:calming_brew", &["Сила 1: курица, кролик; прочих — вялость", "ПКМ — выпить, по мобу — напоить"]),
    ("biolab:syringe_calming", &["Сила 1: курица, кролик; прочих — вялость", "ПКМ — себе, по мобу — усыпить"]),
    ("biolab:mandrake_mash", &["Ферментер сделает настойку мандрагоры"]),
    ("biolab:tincture_mandrake", &["Со шприцем на столе — инъекция, сила 3", "В синтезаторе — основа транквилизатора"]),
    ("biolab:syringe_mandrake", &["Сила 3: корова, паук; себе — тошнота", "ПКМ — себе, по мобу — усыпить"]),
    ("biolab:dart_mandrake", &["ПКМ — бросить в моба, как снежок", "Сила 3: корова, паук; рукой — до 8 блоков"]),
    ("biolab:ether_mix", &["Реактор над костром, 250–320° — эфир"]),
    ("biolab:ether", &["Сила 3, газ: стол — аэрозоль, граната", "Огнеопасен: у огня облако вспыхнет"]),
    ("biolab:spray_ether", &["Сила 3: облачко газа перед собой", "ПКМ — распылить; огонь рядом — вспышка"]),
    ("biolab:grenade_ether", &["ПКМ — бросить: облако радиусом 3 на 5 с", "Сила 3, усыпляет стаю; огонь — вспышка"]),
    ("biolab:chloroform_mix", &["Реактор над магмой или костром душ"]),
    ("biolab:chloroform", &["Сила 4, газ: передозировка с массы + 2", "Стол — аэрозоль, тряпка, граната"]),
    ("biolab:spray_chloroform", &["Сила 4: облачко газа, мелочь убьёт", "ПКМ — распылить; без противогаза задевает"]),
    ("biolab:rag_chloroform", &["Сила 4: ПКМ по мобу вплотную — усыпить", "Мелочь и зомби не выдержат"]),
    ("biolab:grenade_chloroform", &["ПКМ — бросить: облако радиусом 3 на 5 с", "Сила 4, валит крупных; мелочь убьёт"]),
    ("biolab:curare_mash", &["Ферментер сделает сырец кураре"]),
    ("biolab:curare_raw", &["Перегонный куб выпарит экстракт кураре"]),
    ("biolab:extract_curare", &["Со шприцем на столе — инъекция кураре"]),
    ("biolab:syringe_curare", &["Сила 4: паралич, не сон; душит раз в 5 с", "ПКМ — себе, по мобу — парализовать"]),
    ("biolab:dart_curare", &["ПКМ — бросить в моба, как снежок", "Сила 4, паралич; рукой — до 8 блоков"]),
    ("biolab:tranquilizer", &["Сила 5: гигант, разоритель, мутанты", "Со шприцем на столе — инъекция"]),
    ("biolab:syringe_tranquilizer", &["Сила 5: берёт любого, уснёт до 8 с", "ПКМ — себе, по мобу — усыпить"]),
    ("biolab:dart_tranquilizer", &["ПКМ — бросить в моба, как снежок", "Сила 5, любого; рукой — до 8 блоков"]),
    ("biolab:syringe_wake", &["Снимает сон и паралич; себе — бодрость", "ПКМ — себе, по мобу — разбудить"]),
];

/// Предметы, которым подсказка осознанно не нужна. Сейчас пусто: каждому
/// нашлось что сказать. Предмет из `content/assets/biolab/items/` без
/// подсказки должен попасть сюда, иначе тест упадёт.
pub const HINTS_SKIPPED: [&str; 0] = [];

/// Строки подсказки предмета: что делает и как применить. Пусто — у
/// предмета подсказки нет.
pub fn item_hint(item: &str) -> Vec<String> {
    if let Some(e) = element(item) {
        return vec![format!("Символ {}: сырьё для синтезатора", e.symbol)];
    }
    ITEM_HINTS
        .iter()
        .find(|(id, _)| *id == item)
        .map_or_else(Vec::new, |(_, lines)| lines.iter().map(|l| (*l).to_string()).collect())
}

/// Подпись предмета со своими данными по его `custom_data`: `text(ключ)` —
/// строка под ключом. None — у предмета нет своей подписи или данных нет,
/// тогда остаётся имя из языкового файла. Колба подписывается отдельно
/// (`flask_label`: у неё ещё объём), чертёж — при печати (ему нужно имя из
/// журнала мира).
pub fn item_label(item: &str, text: impl Fn(&str) -> Option<String>) -> Option<String> {
    match item {
        FRAGMENT_ID => Some(fragment_label(&text(GENE_KEY)?)),
        CULTURE_ID => Some(culture_label(&strain_from_text(&text(STRAIN_KEY)?)?)),
        GM_CROP => Some(crop_label(&genome_from_text(&text(PLOT_GENOME_KEY)?))),
        STRETCHER_ID => stretcher_label(&text(PATIENT_KEY)?),
        _ => None,
    }
}

/// Состояние игрока в мире: болезни, кровь, прогресс лаборатории. Пока это
/// прототип механизма — счётчик тиков и уровень заражения; поля добавятся
/// вместе с подсистемами.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

// `Default` для массивов длиннее 32 сам не выводится, а генов теперь 40.
impl Default for PlayerState {
    fn default() -> Self {
        Self {
            ticks: 0,
            infection: 0,
            drugs: Default::default(),
            immunity: 0,
            withdrawal: 0,
            genes: [0; GENES.len()],
            mutagenesis: 0,
            strain: [0; 4],
            manual: false,
            research: 0,
        }
    }
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
/// `slots` — сколько у машины слотов, `menu` — ванильное меню, чью раскладку
/// слотов она повторяет (печка, воронка, сетка 3×3): по нему решаются роли
/// слотов. Открывается своё меню мода (`machine_menu`). Машины на рецептах
/// берут вход из слота 0 и кладут в последний; синтезатор читает все слоты,
/// кроме последнего.
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

/// Слоты, куда воронки кладут вход: входы и топливо (`slot_role`). Слот
/// топлива меню печки нужен только клонирующему чану (питательный раствор):
/// у остальных машин он пустует, и воронка, кладущая туда вход, просто
/// забивала бы его.
pub fn machine_inputs(machine: &Machine) -> Vec<usize> {
    (0..machine.slots)
        .filter(|slot| matches!(slot_role(machine, *slot), SlotRole::Input | SlotRole::Fuel))
        .collect()
}

/// Слоты, откуда воронки забирают выход. У центрифуги на меню воронки вход
/// один, а все остальные слоты — выходы.
pub fn machine_outputs(machine: &Machine) -> Vec<usize> {
    (0..machine.slots).filter(|slot| slot_role(machine, *slot) == SlotRole::Output).collect()
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

/// Какие входные слоты синтезатор опустошает, когда вещество готово: только
/// те, где лежат элементы — они и стали веществом. Чертёж остаётся в своём
/// слоте (по нему собирают сколько угодно раз), а чужой предмет, случайно
/// положенный в сетку, не пропадает. Раньше чистились все входы, и чертёж
/// уходил вместе с элементами. `items` — id предметов во входах по порядку.
pub fn synth_spent_slots(items: &[Option<&str>]) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.is_some_and(|id| element(id).is_some()))
        .map(|(slot, _)| slot)
        .collect()
}

// ---- Автоматизация машин (GDD 4.15) ----
//
// Как машина живёт рядом с воронками, компаратором и шифт-кликом. Здесь
// только решения: роль каждого слота, что слот принимает, куда шифт-клик
// кладёт стак, какой сигнал видит компаратор. Хуки — `machine_slots.rs`
// (стороны для воронок), `machine_quick_move.rs`, `menu_slot_rules.rs`,
// `hopper_filter.rs`, `machine_analog*.rs` и начало тика в `machine.rs`.

/// Роль слота машины.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotRole {
    /// Сырьё: то, что машина перерабатывает.
    Input,
    /// Слот топлива меню печки. Нужен только клонирующему чану — там лежит
    /// питательный раствор.
    Fuel,
    /// Готовое: игрок и воронки только забирают.
    Output,
    /// Слот есть в меню печки, но машине не нужен (топливо у всех, кроме
    /// чана): спрятан с экрана, ничего не принимает.
    Unused,
}

/// Роль слота по раскладке машины. Слот за пределами машины — `Unused`.
pub fn slot_role(machine: &Machine, slot: usize) -> SlotRole {
    if slot >= machine.slots {
        return SlotRole::Unused;
    }
    match machine.menu {
        "minecraft:furnace" => match slot {
            SLOT_INPUT => SlotRole::Input,
            SLOT_FUEL if machine.id == CLONING_VAT => SlotRole::Fuel,
            SLOT_FUEL => SlotRole::Unused,
            _ => SlotRole::Output,
        },
        // Центрифуга: вход и четыре выхода.
        "minecraft:hopper" if slot == 0 => SlotRole::Input,
        "minecraft:hopper" => SlotRole::Output,
        // Сетка 3×3: восемь входов, последний — выход.
        _ if slot + 1 == machine.slots => SlotRole::Output,
        _ => SlotRole::Input,
    }
}

/// Открывать у машин своё меню мода, а не ванильное. Только у своего типа
/// меню есть `results` (в выход нельзя положить руками) и правила слотов
/// (`native_menu_slot_rules`); ванильные меню печки, воронки и сетки для
/// контейнера мода пускают что угодно куда угодно. Если свои типы меню
/// подведут в игре, `false` возвращает ванильные: шифт-клик, воронки и
/// компаратор от этого не зависят.
pub const OWN_MENUS: bool = true;

/// Какое меню открывает машина: свой тип меню мода называется так же, как
/// машина (`data/biolab/horizon/menu/<машина>.json`).
pub fn machine_menu(machine: &Machine) -> &'static str {
    if OWN_MENUS { machine.id } else { machine.menu }
}

/// Машина, чей это тип меню; None — меню не наше.
pub fn machine_of_menu(menu: &str) -> Option<&'static Machine> {
    MACHINES.iter().find(|m| machine_menu(m) == menu)
}

/// Что принимает вход машин на рецептах: всё, что стоит в `ingredient`
/// (`ingredients` у стола) её рецептов в `content/`. Правилам слотов мир не
/// виден (ни рецептов, ни уровня), поэтому список свой; тест сверяет его с
/// рецептами, так что новый рецепт без строки здесь не пройдёт.
pub const MACHINE_INGREDIENTS: [(&str, &[&str]); 9] = [
    (
        "biolab:still",
        &[
            "biolab:curare_raw",
            "biolab:mutant_tissue",
            "minecraft:bone",
            "minecraft:bone_meal",
            "minecraft:charcoal",
            "minecraft:copper_ingot",
            "minecraft:dried_kelp",
            "minecraft:dried_kelp_block",
            "minecraft:glowstone_dust",
            "minecraft:gold_nugget",
            "minecraft:gunpowder",
            "minecraft:ice",
            "minecraft:iron_nugget",
            "minecraft:kelp",
            "minecraft:prismarine_crystals",
            "minecraft:prismarine_shard",
            "minecraft:redstone",
            "minecraft:rotten_flesh",
            "minecraft:sand",
            "minecraft:sea_pickle",
            "minecraft:seagrass",
            // Приходит из ветки растений (агент растений, M5).
            "biolab:valerian_root",
        ],
    ),
    ("biolab:incubator", &["biolab:culture", "biolab:swab", "minecraft:bread"]),
    (
        "biolab:fermenter",
        &[
            "biolab:co2_canister_empty",
            "biolab:culture",
            "biolab:curare_mash",
            "biolab:mandrake_mash",
            "biolab:mold",
            "minecraft:birch_leaves",
            "minecraft:cocoa_beans",
            "minecraft:poppy",
            "minecraft:spider_eye",
            "minecraft:sugar",
            "minecraft:wheat",
        ],
    ),
    (
        "biolab:centrifuge",
        &[
            "biolab:extract_raw",
            "minecraft:glistering_melon_slice",
            "minecraft:gravel",
            "minecraft:magma_cream",
            "minecraft:spider_eye",
        ],
    ),
    (
        "biolab:reactor",
        &["biolab:chloroform_mix", "biolab:culture", "biolab:element_glaurium", "biolab:ether_mix", "minecraft:calcite"],
    ),
    // Синтезатор по рецепту — только смесь транквилизатора (GDD 4.20).
    ("biolab:synthesizer", &["biolab:element_gold", "biolab:extract_sedative", "biolab:tincture_mandrake"]),
    (
        "biolab:press",
        &["biolab:extract_caffeine", "biolab:extract_penicillin", "biolab:extract_salicylate", "minecraft:charcoal"],
    ),
    (
        "biolab:extractor",
        &[
            "biolab:gm_crop",
            "biolab:mutant_tissue",
            "minecraft:bone",
            "minecraft:cocoa_beans",
            "minecraft:glow_ink_sac",
            "minecraft:goat_horn",
            "minecraft:ink_sac",
            "minecraft:magma_cream",
            "minecraft:phantom_membrane",
            "minecraft:poisonous_potato",
            "minecraft:rabbit_foot",
            "minecraft:rotten_flesh",
            "minecraft:spider_eye",
            "minecraft:turtle_scute",
        ],
    ),
    (
        "biolab:lab_bench",
        &[
            "biolab:agar",
            "biolab:co2_canister_empty",
            "biolab:calming_brew",
            "biolab:chloroform",
            "biolab:culture_weakened",
            "biolab:element_chlorine",
            "biolab:element_glaurium",
            "biolab:element_redstonium",
            "biolab:element_sulfur",
            "biolab:ether",
            "biolab:extract_caffeine",
            "biolab:extract_curare",
            "biolab:extract_penicillin",
            "biolab:extract_salicylate",
            "biolab:extract_sedative",
            "biolab:extract_venom",
            "biolab:flask",
            "biolab:mutagen",
            "biolab:mutant_tissue",
            "biolab:pill_antidote",
            "biolab:spirit",
            "biolab:syringe",
            "biolab:syringe_curare",
            "biolab:syringe_mandrake",
            "biolab:syringe_sedative",
            "biolab:syringe_tranquilizer",
            "biolab:syringe_venom",
            "minecraft:bamboo",
            "biolab:tincture_mandrake",
            "biolab:tranquilizer",
            "minecraft:compass",
            "minecraft:copper_ingot",
            "minecraft:crossbow",
            "minecraft:feather",
            "minecraft:glass",
            "minecraft:glass_bottle",
            "minecraft:glass_pane",
            "minecraft:iron_ingot",
            "minecraft:iron_nugget",
            "minecraft:slime_ball",
            "minecraft:spyglass",
            "minecraft:spider_eye",
            "minecraft:stick",
            "minecraft:string",
            "minecraft:sugar",
            "minecraft:vine",
            "minecraft:white_wool",
            // Приходят из ветки растений (агент растений, M5).
            "biolab:belladonna_berries",
            "biolab:mandrake_root",
        ],
    ),
];

/// Входы машины на рецептах; пусто — у машины своя логика.
pub fn machine_ingredients(id: &str) -> &'static [&'static str] {
    MACHINE_INGREDIENTS.iter().find(|(m, _)| *m == id).map_or(&[], |(_, items)| items)
}

/// Возьмёт ли слот машины этот предмет. Отвечает по одному id: правила слотов
/// зовутся по нескольку раз за клик без уровня и игрока — только стак и
/// быстро. Выход и пустующий слот топлива не берут ничего; топливо чана —
/// только раствор; вход — то, с чем машина работает: у машин со своей
/// логикой это их сырьё, у остальных — входы рецептов.
pub fn slot_accepts(machine: &Machine, slot: usize, item: &str) -> bool {
    match slot_role(machine, slot) {
        SlotRole::Output | SlotRole::Unused => false,
        SlotRole::Fuel => item == BROTH_ID,
        SlotRole::Input => match machine.id {
            CLONING_VAT => item == FRAGMENT_ID,
            // Синтезатор собирает формулу из элементов, а по рецепту — смесь
            // (транквилизатор зверолова, GDD 4.20): её входы тоже берёт.
            SYNTHESIZER => element(item).is_some() || item == BLUEPRINT_ID || machine_ingredients(SYNTHESIZER).contains(&item),
            SPECTROMETER => item == FLASK_ID,
            // Центрифуга разлагает колбы и вдобавок крутит свои рецепты.
            CENTRIFUGE => item == FLASK_ID || machine_ingredients(machine.id).contains(&item),
            id => machine_ingredients(id).contains(&item),
        },
    }
}

/// Возьмёт ли машина предмет хоть в один слот, куда кладут воронки. Слот
/// воронке не известен (`native_inventory_move_item` называет только блоки),
/// поэтому вопрос — про машину целиком: чужой предмет остаётся в воронке,
/// а не забивает вход.
pub fn hopper_accepts(machine: &Machine, item: &str) -> bool {
    machine_inputs(machine).into_iter().any(|slot| slot_accepts(machine, slot, item))
}

/// Правило слота своего меню машины: None — меню не наше (решает чужой мод
/// или игра), иначе можно ли положить предмет.
pub fn menu_slot_rule(menu: &str, slot: usize, item: &str) -> Option<bool> {
    machine_of_menu(menu).map(|machine| slot_accepts(machine, slot, item))
}

/// Стороны машины для воронок, по `Direction`: низ, верх, север, юг, запад,
/// восток. Снизу забирают выход. Сверху кладут сырьё, сбоку — топливо, как у
/// ванильной печки; у машин без топлива сбоку тоже сырьё. Раньше у чана все
/// стороны вели в оба слота, и раствор, пришедший в пустой чан, ложился во
/// вход фрагмента — чан вставал.
pub fn machine_faces(machine: &Machine) -> [Vec<usize>; 6] {
    let slots = |role: SlotRole| -> Vec<usize> {
        (0..machine.slots).filter(|slot| slot_role(machine, *slot) == role).collect()
    };
    let input = slots(SlotRole::Input);
    let fuel = slots(SlotRole::Fuel);
    let side = if fuel.is_empty() { input.clone() } else { fuel };
    [slots(SlotRole::Output), input, side.clone(), side.clone(), side.clone(), side]
}

/// Куда шифт-клик из инвентаря кладёт предмет: слоты, которые его примут,
/// по порядку (вход, у чана раствор — в топливо). Пусто — машине он не
/// нужен, решает игра (в своём меню она тоже спросит правила слотов, то
/// есть никуда не положит).
pub fn quick_move_targets(machine: &Machine, item: &str) -> Vec<usize> {
    (0..machine.slots).filter(|slot| slot_accepts(machine, *slot, item)).collect()
}

/// Раскладка шифт-клика, как `moveItemStackTo` ванильного меню: сперва
/// долить стаки того же предмета с теми же данными, потом занять пустые
/// слоты. `held` — слоты машины: пусто или (тот же ли предмет, сколько);
/// `targets` — куда можно (`quick_move_targets`); `max` — стопка предмета.
/// Вернёт, в какой слот сколько положить; сумма не больше `count`.
pub fn quick_move_plan(held: &[Option<(bool, u32)>], targets: &[usize], count: u32, max: u32) -> Vec<(usize, u32)> {
    let max = max.max(1);
    let mut left = count;
    let mut plan = Vec::new();
    for &slot in targets {
        if left == 0 {
            break;
        }
        if let Some(Some((true, have))) = held.get(slot)
            && *have < max
        {
            let put = (max - have).min(left);
            plan.push((slot, put));
            left -= put;
        }
    }
    for &slot in targets {
        if left == 0 {
            break;
        }
        if matches!(held.get(slot), Some(None)) {
            let put = left.min(max);
            plan.push((slot, put));
            left -= put;
        }
    }
    plan
}

/// Сигнал компаратора — прогресс работы: 0 — машина стоит, 1 — только
/// начала, 15 — почти готово. Не заполненность: воронки при автоматизации и
/// так держат выход пустым, а заполненность всех слотов (что дала бы
/// ванилла) смешивает сырьё с готовым. Прогресс же иначе не увидеть: по нему
/// понятно, работает ли машина, когда закончит и что она встала (нет
/// сырья, выход забит) — и это работает у чана, который ничего не выдаёт
/// в слот, и у спектрометра, который колбу не тратит. Сброс с 15 на 0 —
/// вещь готова: импульс для счётчика.
pub fn comparator_signal(data: &[i32]) -> i32 {
    let total = data.get(DATA_TOTAL).copied().unwrap_or(0);
    let progress = data.get(DATA_PROGRESS).copied().unwrap_or(0);
    if total <= 0 || progress <= 0 {
        return 0;
    }
    // Округление вверх: работающая машина — хотя бы 1, иначе компаратор
    // первые тики долгого рецепта не отличил бы её от стоящей.
    let signal = (u64::from(progress.unsigned_abs()) * 15).div_ceil(u64::from(total.unsigned_abs()));
    signal.clamp(1, 15) as i32
}

/// Сигналы машин для компаратора по месту. Компаратор спрашивает уровень,
/// а тикающая машина в это время вынута из мира — поэтому её сигнал лежит
/// здесь, а не читается из блок-сущности.
type Signals = std::collections::HashMap<(String, [i32; 3]), i32>;

fn machine_signals() -> &'static std::sync::Mutex<Signals> {
    static SIGNALS: std::sync::OnceLock<std::sync::Mutex<Signals>> = std::sync::OnceLock::new();
    SIGNALS.get_or_init(Default::default)
}

/// Запомнить сигнал машины. true — он изменился (или машина ещё не
/// отвечала с этого места): компараторам пора перечитать. Так соседей
/// будят только на смене ступени, а не каждый тик.
pub fn remember_signal(dimension: &str, pos: [i32; 3], signal: i32) -> bool {
    let mut signals = machine_signals().lock().unwrap_or_else(|e| e.into_inner());
    signals.insert((dimension.to_string(), pos), signal) != Some(signal)
}

/// Сигнал машины по месту; 0 — машина ещё не тикала.
pub fn known_signal(dimension: &str, pos: [i32; 3]) -> i32 {
    let signals = machine_signals().lock().unwrap_or_else(|e| e.into_inner());
    signals.get(&(dimension.to_string(), pos)).copied().unwrap_or(0)
}

#[cfg(test)]
mod automation_tests {
    use super::*;

    fn m(id: &str) -> &'static Machine {
        machine(id).unwrap_or_else(|| panic!("{id} — машина"))
    }

    #[test]
    fn every_slot_has_one_role() {
        for machine in &MACHINES {
            let roles: Vec<SlotRole> = (0..machine.slots).map(|s| slot_role(machine, s)).collect();
            assert!(roles.contains(&SlotRole::Input), "{}", machine.id);
            assert!(roles.contains(&SlotRole::Output), "{}", machine.id);
            assert_eq!(roles.contains(&SlotRole::Fuel), machine.id == CLONING_VAT, "{}", machine.id);
            assert_eq!(slot_role(machine, machine.slots), SlotRole::Unused, "за пределами");
        }
        assert_eq!(slot_role(m("biolab:still"), SLOT_FUEL), SlotRole::Unused);
        assert_eq!(slot_role(m(CLONING_VAT), SLOT_FUEL), SlotRole::Fuel);
        assert_eq!(slot_role(m(CENTRIFUGE), 3), SlotRole::Output);
        assert_eq!(slot_role(m(SYNTHESIZER), 7), SlotRole::Input);
        assert_eq!(slot_role(m(LAB_BENCH), 8), SlotRole::Output);
    }

    #[test]
    fn roles_agree_with_the_screen() {
        // Выход в бирюзовой рамке — тот же слот, что выход по роли; слот
        // без роли спрятан с экрана.
        for machine in &MACHINES {
            let screen = machine_screen(machine.id).unwrap();
            for slot in 0..machine.slots {
                let role = slot_role(machine, slot);
                assert_eq!(is_output_slot(machine.id, slot), role == SlotRole::Output, "{} {slot}", machine.id);
                assert_eq!(screen.slots[slot].is_none(), role == SlotRole::Unused, "{} {slot}", machine.id);
            }
        }
    }

    #[test]
    fn outputs_and_the_idle_fuel_slot_take_nothing() {
        for machine in &MACHINES {
            for slot in machine_outputs(machine) {
                for item in [FLASK_ID, "minecraft:bone", BROTH_ID, "biolab:element_carbon"] {
                    assert!(!slot_accepts(machine, slot, item), "{} {slot} {item}", machine.id);
                }
            }
        }
        assert!(!slot_accepts(m("biolab:still"), SLOT_FUEL, "minecraft:bone"));
        assert!(!slot_accepts(m("biolab:still"), SLOT_FUEL, "minecraft:coal"), "топлива у куба нет");
    }

    #[test]
    fn inputs_take_what_the_machine_works_with() {
        let vat = m(CLONING_VAT);
        assert!(slot_accepts(vat, SLOT_INPUT, FRAGMENT_ID));
        assert!(!slot_accepts(vat, SLOT_INPUT, BROTH_ID), "раствор — не во вход");
        assert!(slot_accepts(vat, SLOT_FUEL, BROTH_ID));
        assert!(!slot_accepts(vat, SLOT_FUEL, FRAGMENT_ID));
        assert!(!slot_accepts(vat, SLOT_FUEL, "minecraft:coal"), "уголь чану не топливо");

        let synth = m(SYNTHESIZER);
        assert!(slot_accepts(synth, 0, "biolab:element_hydrogen"));
        assert!(slot_accepts(synth, 7, BLUEPRINT_ID));
        assert!(!slot_accepts(synth, 0, "minecraft:dirt"));

        assert!(slot_accepts(m(SPECTROMETER), SLOT_INPUT, FLASK_ID));
        assert!(!slot_accepts(m(SPECTROMETER), SLOT_INPUT, BLUEPRINT_ID));

        let centrifuge = m(CENTRIFUGE);
        assert!(slot_accepts(centrifuge, 0, FLASK_ID), "колбу разлагает");
        assert!(slot_accepts(centrifuge, 0, "minecraft:gravel"), "и крутит рецепты");
        assert!(!slot_accepts(centrifuge, 0, "minecraft:dirt"));

        assert!(slot_accepts(m("biolab:still"), SLOT_INPUT, "minecraft:kelp"));
        assert!(!slot_accepts(m("biolab:still"), SLOT_INPUT, "minecraft:dirt"));
        assert!(slot_accepts(m(LAB_BENCH), 3, "minecraft:stick"));
        assert!(!slot_accepts(m(LAB_BENCH), 3, "minecraft:dirt"));
    }

    /// Входы рецептов машин в `content/`: машина → id предметов.
    fn recipe_inputs() -> std::collections::BTreeMap<String, std::collections::BTreeSet<String>> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/recipe");
        let mut out: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> = Default::default();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let json = Json::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let kind = json.get("type").and_then(Json::str).unwrap_or("");
            if !is_machine(kind) {
                continue;
            }
            let entries: Vec<&Json> = match (json.get("ingredient"), json.get("ingredients")) {
                (Some(one), _) => vec![one],
                (None, Some(list)) => list.arr().expect("ingredients — список").iter().collect(),
                (None, None) => panic!("{}: рецепт без входа", path.display()),
            };
            for entry in entries {
                // Тег или список вариантов правилам слотов не по зубам: им
                // виден только id предмета.
                let id = entry.str().unwrap_or_else(|| panic!("{}: вход не id предмета", path.display()));
                assert!(!id.starts_with('#'), "{}: тег во входе", path.display());
                out.entry(kind.to_string()).or_default().insert(id.to_string());
            }
        }
        out
    }

    #[test]
    fn the_ingredient_table_matches_the_recipes() {
        let recipes = recipe_inputs();
        let mut listed: Vec<&str> = MACHINE_INGREDIENTS.iter().map(|(id, _)| *id).collect();
        listed.sort();
        let with_recipes: Vec<&str> = recipes.keys().map(String::as_str).collect();
        assert_eq!(listed, with_recipes, "машины с рецептами");
        for (id, items) in &MACHINE_INGREDIENTS {
            let table: std::collections::BTreeSet<String> = items.iter().map(|s| s.to_string()).collect();
            assert_eq!(table.len(), items.len(), "{id}: предмет дважды");
            assert_eq!(&table, &recipes[*id], "{id}: таблица разошлась с рецептами");
        }
        // Каждый вход рецепта машина примет — и руками, и воронкой.
        for (id, items) in &recipes {
            let machine = m(id);
            for item in items {
                assert!(hopper_accepts(machine, item), "{id} {item}");
                assert!(!quick_move_targets(machine, item).is_empty(), "{id} {item}");
            }
        }
    }

    #[test]
    fn hoppers_follow_the_furnace_sides() {
        let vat = m(CLONING_VAT);
        let faces = machine_faces(vat);
        assert_eq!(faces[0], vec![SLOT_OUTPUT], "снизу — выход");
        assert_eq!(faces[1], vec![SLOT_INPUT], "сверху — фрагмент");
        for side in &faces[2..] {
            assert_eq!(side, &vec![SLOT_FUEL], "сбоку — раствор");
        }
        for machine in &MACHINES {
            let faces = machine_faces(machine);
            let inputs = machine_inputs(machine);
            assert_eq!(faces[0], machine_outputs(machine), "{}", machine.id);
            for face in &faces[1..] {
                assert!(!face.is_empty(), "{}", machine.id);
                assert!(face.iter().all(|s| inputs.contains(s)), "{}: выход в сторонах подачи", machine.id);
            }
        }
        let centrifuge = machine_faces(m(CENTRIFUGE));
        assert_eq!(centrifuge[0], vec![1, 2, 3, 4]);
        assert_eq!(centrifuge[3], vec![0]);
        assert_eq!(machine_faces(m(SYNTHESIZER))[4], (0..8).collect::<Vec<_>>());
    }

    #[test]
    fn hoppers_keep_foreign_items_out() {
        assert!(hopper_accepts(m(CLONING_VAT), FRAGMENT_ID));
        assert!(hopper_accepts(m(CLONING_VAT), BROTH_ID));
        assert!(!hopper_accepts(m(CLONING_VAT), "minecraft:coal"));
        assert!(!hopper_accepts(m("biolab:still"), "minecraft:dirt"));
        assert!(hopper_accepts(m(SYNTHESIZER), "biolab:element_oxygen"));
    }

    #[test]
    fn shift_click_routes_by_role() {
        assert_eq!(quick_move_targets(m(CLONING_VAT), BROTH_ID), vec![SLOT_FUEL]);
        assert_eq!(quick_move_targets(m(CLONING_VAT), FRAGMENT_ID), vec![SLOT_INPUT]);
        assert_eq!(quick_move_targets(m("biolab:press"), "minecraft:charcoal"), vec![SLOT_INPUT]);
        assert_eq!(quick_move_targets(m(CENTRIFUGE), FLASK_ID), vec![0]);
        assert_eq!(quick_move_targets(m(SYNTHESIZER), "biolab:element_carbon"), (0..8).collect::<Vec<_>>());
        assert!(quick_move_targets(m("biolab:still"), "minecraft:dirt").is_empty(), "не нужно — решает игра");
        assert!(quick_move_targets(m("biolab:still"), "minecraft:coal").is_empty(), "в пустующее топливо — нет");
    }

    #[test]
    fn shift_click_tops_up_then_fills_empty_slots() {
        // Слоты: 0 — чужой предмет, 1 — тот же (60), 2 — пусто, 3 — тот же (64).
        let held = [Some((false, 5)), Some((true, 60)), None, Some((true, 64))];
        assert_eq!(quick_move_plan(&held, &[0, 1, 2, 3], 10, 64), vec![(1, 4), (2, 6)]);
        // Влезло в доливку — пустой слот не трогаем.
        assert_eq!(quick_move_plan(&held, &[0, 1, 2, 3], 3, 64), vec![(1, 3)]);
        // Больше стопки — в пустой слот только стопка, остаток остаётся у игрока.
        assert_eq!(quick_move_plan(&[None, None], &[0], 100, 64), vec![(0, 64)]);
        assert_eq!(quick_move_plan(&[None, None], &[0, 1], 100, 64), vec![(0, 64), (1, 36)]);
        // Стопка в 1 (фрагмент, колба с данными): по одному в пустой слот.
        assert_eq!(quick_move_plan(&[None, None], &[0, 1], 3, 1), vec![(0, 1), (1, 1)]);
        // Некуда: всё занято чужим или полно.
        assert!(quick_move_plan(&held, &[0, 3], 10, 64).is_empty());
        // Слот за пределами машины пропускается.
        assert!(quick_move_plan(&held, &[9], 10, 64).is_empty());
    }

    #[test]
    fn own_menus_answer_slot_rules() {
        let still = m("biolab:still");
        assert_eq!(machine_of_menu(machine_menu(still)).map(|x| x.id), Some(still.id));
        assert_eq!(menu_slot_rule("minecraft:chest", 0, FLASK_ID), None, "чужое меню не трогаем");
        if OWN_MENUS {
            assert_eq!(menu_slot_rule("biolab:still", SLOT_OUTPUT, "minecraft:kelp"), Some(false));
            assert_eq!(menu_slot_rule("biolab:still", SLOT_INPUT, "minecraft:kelp"), Some(true));
            assert_eq!(menu_slot_rule("biolab:cloning_vat", SLOT_FUEL, BROTH_ID), Some(true));
        }
    }

    /// Свои типы меню в `content/`: по файлу на машину, слоты, данные и
    /// выходы — как у машины, иначе меню не откроется или экран недосчитается
    /// чисел.
    #[test]
    fn menu_types_match_the_machines() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/horizon/menu");
        for machine in &MACHINES {
            let name = machine.id.trim_start_matches("biolab:");
            let path = dir.join(format!("{name}.json"));
            let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("нет {}", path.display()));
            let json = Json::parse(&text).unwrap();
            let num = |key: &str| json.get(key).and_then(Json::num).map(|n| n as usize);
            assert_eq!(num("slots"), Some(machine.slots), "{name}");
            assert_eq!(num("data_slots"), Some(machine_data_slots(machine.id)), "{name}");
            let results: Vec<usize> = json
                .get("results")
                .and_then(Json::arr)
                .unwrap_or(&[])
                .iter()
                .filter_map(Json::num)
                .map(|n| n as usize)
                .collect();
            assert_eq!(results, machine_outputs(machine), "{name}");
        }
        let files = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(files, MACHINES.len(), "лишний тип меню");
    }

    #[test]
    fn comparator_reads_progress() {
        let data = |progress: i32, total: i32| {
            let mut d = vec![0; DATA_SLOTS];
            d[DATA_PROGRESS] = progress;
            d[DATA_TOTAL] = total;
            d
        };
        assert_eq!(comparator_signal(&[]), 0, "до первого тика");
        assert_eq!(comparator_signal(&data(0, 0)), 0, "стоит");
        assert_eq!(comparator_signal(&data(0, 100)), 0);
        assert_eq!(comparator_signal(&data(1, 400)), 1, "работает — хотя бы 1");
        assert_eq!(comparator_signal(&data(50, 100)), 8);
        assert_eq!(comparator_signal(&data(99, 100)), 15, "почти готово");
        assert_eq!(comparator_signal(&data(500, 100)), 15, "не выше 15");
        // Ступеней 15 на любой рецепт: соседей будят не чаще.
        let steps: std::collections::BTreeSet<i32> = (0..400).map(|p| comparator_signal(&data(p, 400))).collect();
        assert_eq!(steps.len(), 16);
    }

    #[test]
    fn neighbours_wake_only_on_a_new_step() {
        let pos = [123_457, -7, 99];
        assert!(remember_signal("test:dim", pos, 0), "первый ответ с места");
        assert!(!remember_signal("test:dim", pos, 0));
        assert!(remember_signal("test:dim", pos, 3));
        assert_eq!(known_signal("test:dim", pos), 3);
        assert_eq!(known_signal("test:other", pos), 0, "другое измерение");
        assert_eq!(known_signal("test:dim", [0, -64, 0]), 0, "не тикала");
    }
}

// ---- Звуки ----
//
// Выбор звука — здесь, одной таблицей: сменить звук значит поправить строку,
// хуки берут отсюда. Звуки двух видов:
//
// - Ванильные (`minecraft:…`) — там, где ванильный уместен (еда, бинт по
//   коже, набат перегрева). Имя должно быть в реестре `minecraft:sound_event`
//   версии 26.3: незнакомое сервер молча выбрасывает при рассылке
//   (`flush_native_sounds`), так что опечатка — это тишина в игре; тест
//   сверяет таблицу с ванильным списком.
// - Свои (`biolab:…`, API 61): событие чужого пространства имён сервер шлёт
//   по имени, а клиент ищет его в `assets/biolab/sounds.json` мода. Файлы
//   синтезирует `tools/mksounds.py` (он же пишет `sounds.json`); событие,
//   которого там нет, на клиенте просто не играет — тест сверяет таблицу с
//   `sounds.json` и наличие файлов. Свой звук — там, где ванильный звучал
//   чужим (центрифуга маяком, укол пчелой, скальпель ножницами по овце).
//
// Громкость держим низкой (до 0,8, фон машин — 0,2–0,4): лаборатория — это
// ряды машин, которые работают часами, и шум не должен утомлять. Высоту
// поднимаем, где ванильный звук слишком «большой» для пробирки, и опускаем,
// где нужна тяжесть (пресс, перегрев). Ванилла сама зажимает высоту в
// 0,5–2,0, поэтому за эти пределы не выходим.

/// Звук события мода (ванильный или свой). `source` — категория громкости, номер в
/// `SOUND_SOURCES` игры: её приглушает свой ползунок в настройках.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sound {
    pub event: &'static str,
    pub volume: f32,
    pub pitch: f32,
    pub source: usize,
}

/// Категории громкости — индексы в `SOUND_SOURCES` Horizon: master, music,
/// record, weather, block, hostile, neutral, player, ambient, voice, ui.
/// Машины звучат как блоки, всё, что делает сам игрок, — как игрок.
pub const SOURCE_BLOCK: usize = 4;
pub const SOURCE_PLAYER: usize = 7;

const fn block(event: &'static str, volume: f32, pitch: f32) -> Sound {
    Sound { event, volume, pitch, source: SOURCE_BLOCK }
}

const fn player(event: &'static str, volume: f32, pitch: f32) -> Sound {
    Sound { event, volume, pitch, source: SOURCE_PLAYER }
}

/// Фаза работы машины, у которой есть свой звук.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Взялась за работу из простоя.
    Start,
    /// Редкий фон, пока работает: раз в `MACHINE_HUM_EVERY` тиков.
    Work,
    /// Выдала результат.
    Done,
}

/// Звуки одной машины: старт, фон и готовый результат.
pub struct MachineSounds {
    pub machine: &'static str,
    pub start: Sound,
    pub work: Sound,
    pub done: Sound,
}

/// Машина → её звуки. Каждой — свой характер, чтобы по звуку было ясно, какая
/// из соседних машин закончила.
pub const MACHINE_SOUNDS: [MachineSounds; 11] = [
    // Перегонный куб: закипает, побулькивает, перелив готового в колбу.
    MachineSounds {
        machine: "biolab:still",
        start: block("minecraft:block.brewing_stand.brew", 0.5, 1.0),
        work: block("minecraft:block.bubble_column.bubble_pop", 0.3, 0.8),
        done: block("minecraft:item.bottle.fill", 0.6, 1.0),
    },
    // Инкубатор: включилась лампа, тёплый ровный гул (свой: капель улья
    // звучала протечкой), сигнал прибора — культура выросла.
    MachineSounds {
        machine: INCUBATOR,
        start: block("minecraft:block.copper_bulb.turn_on", 0.4, 1.0),
        work: block("biolab:block.incubator.hum", 0.25, 1.0),
        done: block("biolab:block.machine.ready", 0.45, 1.0),
    },
    // Ферментер: пузырь пошёл, бульканье брожения (своё: ванильная колонна
    // пузырей — это шум воды, а не густой бражки), слив в бутыль.
    MachineSounds {
        machine: "biolab:fermenter",
        start: block("minecraft:block.bubble_column.bubble_pop", 0.6, 1.0),
        work: block("biolab:block.fermenter.bubble", 0.35, 1.0),
        done: block("minecraft:item.bottle.fill", 0.6, 0.8),
    },
    // Центрифуга: раскрутка, ровный гул ротора, выбег с щелчком крышки —
    // все три свои: раньше это были точило и маяк.
    MachineSounds {
        machine: CENTRIFUGE,
        start: block("biolab:block.centrifuge.start", 0.45, 1.0),
        work: block("biolab:block.centrifuge.work", 0.3, 1.0),
        done: block("biolab:block.centrifuge.done", 0.45, 1.0),
    },
    // Синтезатор: энергия на сборку, кристальный звон атомов, готово.
    MachineSounds {
        machine: SYNTHESIZER,
        start: block("minecraft:block.beacon.power_select", 0.4, 1.4),
        work: block("minecraft:block.amethyst_block.chime", 0.4, 1.0),
        done: block("minecraft:block.enchantment_table.use", 0.6, 1.2),
    },
    // Реактор: шипение нагрева, треск топки, реакция прошла.
    MachineSounds {
        machine: REACTOR,
        start: block("minecraft:block.fire.extinguish", 0.3, 1.4),
        work: block("minecraft:block.blastfurnace.fire_crackle", 0.4, 1.0),
        done: block("minecraft:block.brewing_stand.brew", 0.5, 1.4),
    },
    // Пресс: поршень пошёл, возврат, удар — таблетка спрессована.
    MachineSounds {
        machine: "biolab:press",
        start: block("minecraft:block.piston.extend", 0.4, 1.0),
        work: block("minecraft:block.piston.contract", 0.3, 0.9),
        done: block("minecraft:block.anvil.land", 0.3, 1.2),
    },
    // Экстрактор: щелчки датчика, капли вытяжки, фрагмент в пробирке.
    MachineSounds {
        machine: EXTRACTOR,
        start: block("minecraft:block.sculk_sensor.clicking", 0.4, 1.2),
        work: block("minecraft:block.pointed_dripstone.drip_water", 0.5, 1.0),
        done: block("minecraft:item.bottle.fill_dragonbreath", 0.5, 1.2),
    },
    // Клонирующий чан: густое бульканье, ровный гул, рождение клона.
    MachineSounds {
        machine: CLONING_VAT,
        start: block("minecraft:block.brewing_stand.brew", 0.4, 0.7),
        work: block("minecraft:block.conduit.ambient", 0.4, 0.8),
        done: block("minecraft:block.sniffer_egg.hatch", 0.8, 1.0),
    },
    // Спектрометр: щелчок прибора, тиканье развёртки, сигнал — вещество
    // опознано. Сигнал тот же, что у инкубатора, но выше: прибор, а не нотный
    // блок, и по высоте ясно, который из двух закончил.
    MachineSounds {
        machine: SPECTROMETER,
        start: block("minecraft:block.comparator.click", 0.5, 1.0),
        work: block("minecraft:block.comparator.click", 0.3, 0.7),
        done: block("biolab:block.machine.ready", 0.5, 1.25),
    },
    // Лабораторный стол: щелчок, скрип инструмента, механизм собрал вещь.
    MachineSounds {
        machine: LAB_BENCH,
        start: block("minecraft:block.wooden_button.click_on", 0.4, 1.2),
        work: block("minecraft:ui.cartography_table.take_result", 0.3, 1.0),
        done: block("minecraft:block.crafter.craft", 0.5, 1.0),
    },
];

/// Раз в сколько тиков работающая машина подаёт фоновый звук. Четыре
/// секунды: слышно, что работа идёт, но звук не трещит над ухом. Рецепты
/// короче этого обходятся стартом и готовым результатом.
pub const MACHINE_HUM_EVERY: i32 = 80;

/// Отметка «машина занята» в сейве блока: 1 — работала в прошлом тике.
pub const BUSY_KEY: &str = "biolab_busy";

/// Звук машины в этой фазе, если машина наша.
pub fn machine_sound(machine: &str, phase: Phase) -> Option<Sound> {
    let sounds = MACHINE_SOUNDS.iter().find(|s| s.machine == machine)?;
    Some(match phase {
        Phase::Start => sounds.start,
        Phase::Work => sounds.work,
        Phase::Done => sounds.done,
    })
}

/// Какой звук дать машине по итогу тика и занята ли она теперь. `busy` —
/// работала ли она до этого тика. Старт звучит, только когда машина взялась
/// за работу из простоя: стопку входа она перерабатывает подряд, и иначе
/// после каждого готового предмета «готово» и «старт» шли бы парой.
pub fn machine_beat(busy: bool, step: Step) -> (Option<Phase>, bool) {
    match step {
        Step::Idle => (None, false),
        Step::Working(_) if !busy => (Some(Phase::Start), true),
        Step::Working(progress) if progress > 0 && progress % MACHINE_HUM_EVERY == 0 => {
            (Some(Phase::Work), true)
        }
        Step::Working(_) => (None, true),
        Step::Finish => (Some(Phase::Done), true),
    }
}

/// События мода вне тика машин: инструменты, препараты, генетика, журнал.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// Реактор перегрелся и сбросил содержимое.
    Overheat,
    /// Укол шприцем себе.
    SyringeSelf,
    /// Укол шприцем мобу.
    SyringeMob,
    /// Дротик выпущен.
    DartShot,
    /// Дротик попал в моба.
    DartHit,
    /// Бинт наложен.
    Bandage,
    /// Аэрозоль.
    Spray,
    /// Мазь.
    Ointment,
    /// Таблетка.
    Pill,
    /// Микстура от заражения.
    Remedy,
    /// Съеден урожай ГМ-грядки.
    Eat,
    /// Скальпель: с мутанта на операционном столе срезана ткань.
    Tissue,
    /// Моба погрузили на носилки или сняли со стола.
    Stretcher,
    /// Пациента уложили на стол и пристегнули.
    Strap,
    /// Ген вживлён мобу.
    GeneSplice,
    /// Генотерапия: ген вписан себе.
    GeneTherapy,
    /// Мутаген переписал геном (лужа под игроком, шприц мутагена мобу).
    Mutagen,
    /// В журнале открылся новый узел.
    Research,
    /// Выстрел арбалета-транквилизатора (`DartShot` — духовая трубка).
    CrossbowShot,
    /// Выстрел пневматического ружья.
    RifleShot,
    /// Натяжение арбалета: начало и середина.
    WeaponCharge,
    /// Дротик заряжен: арбалет натянут, магазин ружья полон.
    WeaponLoaded,
    /// Осечка: нет дротиков или газа.
    WeaponEmpty,
    /// К ружью подключён новый баллон CO₂.
    GasConnect,
    /// Газовая граната разбилась (GDD 4.20).
    GrenadeBreak,
    /// Облако эфира вспыхнуло от огня рядом.
    GasFlash,
}

impl Cue {
    pub const ALL: [Cue; 26] = [
        Cue::Overheat,
        Cue::SyringeSelf,
        Cue::SyringeMob,
        Cue::DartShot,
        Cue::DartHit,
        Cue::Bandage,
        Cue::Spray,
        Cue::Ointment,
        Cue::Pill,
        Cue::Remedy,
        Cue::Eat,
        Cue::Tissue,
        Cue::Stretcher,
        Cue::Strap,
        Cue::GeneSplice,
        Cue::GeneTherapy,
        Cue::Mutagen,
        Cue::Research,
        Cue::CrossbowShot,
        Cue::RifleShot,
        Cue::WeaponCharge,
        Cue::WeaponLoaded,
        Cue::WeaponEmpty,
        Cue::GasConnect,
        Cue::GrenadeBreak,
        Cue::GasFlash,
    ];
}

/// Событие → звук.
pub const SOUNDS: [(Cue, Sound); 26] = [
    // Набат: содержимое пропало, это надо услышать и из соседней комнаты.
    (Cue::Overheat, block("minecraft:block.bell.use", 1.0, 0.6)),
    // Укол — свой: щелчок поршня и «пфф» раствора (жало пчелы звучало
    // нападением, а не процедурой); себе чуть выше и тише, мобу ниже.
    (Cue::SyringeSelf, player("biolab:item.syringe.inject", 0.5, 1.1)),
    (Cue::SyringeMob, player("biolab:item.syringe.inject", 0.6, 0.9)),
    // Выстрелы оружия — свои (GDD 4.20). Рукой дротик бросают, и игра сама
    // играет бросок снежка (снаряд — копия снежка, `dart_hit.rs`). Трубка —
    // короткий выдох и шорох пролёта, и тихо: она тихое оружие. Арбалет —
    // щелчок спуска и гул тетивы. Ружьё — пневматический хлопок со свистом
    // пролёта и шипением остатка газа.
    // Попадание — ванильная стрела, выше обычного: дротик меньше, а «тук»
    // в цель у них одинаковый.
    (Cue::DartShot, player("biolab:item.blowpipe.shoot", 0.25, 1.0)),
    (Cue::DartHit, player("minecraft:entity.arrow.hit", 0.6, 1.4)),
    // Бинт — свой: два-три оборота ткани, а не надетая кожаная броня.
    (Cue::Bandage, player("biolab:item.bandage.apply", 0.6, 1.0)),
    // Аэрозоль — свой короткий «пшш» (тушение огня шипело углями).
    (Cue::Spray, player("biolab:item.spray.use", 0.4, 1.0)),
    // Мазь — втирание, как воск по меди.
    (Cue::Ointment, player("minecraft:item.honeycomb.wax_on", 0.6, 1.3)),
    // Таблетка — быстрый «хрум».
    (Cue::Pill, player("minecraft:entity.generic.eat", 0.5, 1.5)),
    (Cue::Remedy, player("minecraft:entity.generic.drink", 0.5, 1.2)),
    (Cue::Eat, player("minecraft:entity.generic.eat", 0.6, 1.0)),
    // Скальпель срезал ткань — свой «вжик» с влажным откликом (ножницы по
    // шерсти звучали стрижкой овцы).
    (Cue::Tissue, player("biolab:item.scalpel.cut", 0.6, 1.0)),
    // Носилки — тело мягко легло на полотно, ниже обычного: оно тяжёлое.
    (Cue::Stretcher, player("minecraft:block.wool.place", 0.8, 0.8)),
    // Стол — затянуты ремни, как привязанный поводок.
    (Cue::Strap, player("minecraft:item.lead.tied", 0.7, 0.9)),
    // Ген вживлён — заряд якоря возрождения, высоко.
    (Cue::GeneSplice, player("minecraft:block.respawn_anchor.charge", 0.5, 1.5)),
    // Генотерапия — своё восходящее мерцающее арпеджио: «внутри что-то
    // переписалось». Вживление мобу оставлено ванильным выше — по звуку
    // ясно, кому вписан ген.
    (Cue::GeneTherapy, player("biolab:item.gene_therapy.use", 0.5, 1.0)),
    // Мутаген — превращение жителя-зомби, ниже обычного: неприятно и чуждо.
    (Cue::Mutagen, player("minecraft:entity.zombie_villager.converted", 0.5, 0.8)),
    // Новый узел журнала — короткий звон повышения уровня, высоко и тихо.
    (Cue::Research, player("minecraft:entity.player.levelup", 0.35, 1.6)),
    (Cue::CrossbowShot, player("biolab:item.tranq_crossbow.shoot", 0.6, 1.0)),
    (Cue::RifleShot, player("biolab:item.air_rifle.shoot", 0.6, 1.0)),
    // Натяжение и заряд — ванильный арбалет, чуть выше: наш легче и меньше.
    (Cue::WeaponCharge, player("minecraft:item.crossbow.loading_middle", 0.5, 1.2)),
    (Cue::WeaponLoaded, player("minecraft:item.crossbow.loading_end", 0.6, 1.2)),
    // Осечка — сухой щелчок раздатчика, которому нечем стрелять.
    (Cue::WeaponEmpty, player("minecraft:block.dispenser.fail", 0.5, 1.6)),
    // Баллон подключён — короткое шипение газа, высоко.
    (Cue::GasConnect, player("minecraft:block.fire.extinguish", 0.3, 1.8)),
    // Колба-граната — звон разбитого стекла, как у брошенного зелья; газ
    // шипит уже облаком частиц.
    (Cue::GrenadeBreak, player("minecraft:entity.splash_potion.break", 0.8, 1.0)),
    // Вспышка эфира — хлопок огненного заряда, ниже обычного: облако, а не шар.
    (Cue::GasFlash, block("minecraft:item.firecharge.use", 1.0, 0.7)),
];

/// Свои события, которые объявлены в `sounds.json`, но код их пока не
/// играет: щелчок дозиметра ждёт заражённости чанков (M9, GDD 4.23 п. 12).
/// Без этого списка тест принял бы заготовку за забытый звук.
pub const RESERVED_SOUNDS: [&str; 1] = ["biolab:item.dosimeter.click"];

/// Звук события.
pub fn sound(cue: Cue) -> Option<Sound> {
    SOUNDS.iter().find(|(c, _)| *c == cue).map(|(_, s)| *s)
}

/// Звук приёма препарата по его форме (`FORMS`).
pub fn dose_cue(form: usize) -> Cue {
    match form {
        FORM_INJECTION => Cue::SyringeSelf,
        FORM_OINTMENT => Cue::Ointment,
        FORM_SPRAY => Cue::Spray,
        FORM_DRINK => Cue::Remedy,
        _ => Cue::Pill,
    }
}

/// Звук укола мобу: мутаген слышно отдельно — он переписывает геном.
pub fn mob_injection_cue(item: &str) -> Cue {
    if item == "biolab:syringe_mutagen" {
        Cue::Mutagen
    } else {
        Cue::SyringeMob
    }
}

// ---- Операционная (GDD 4.17) ----
//
// Ткань больше не падает с убитого мутанта: её срезают скальпелем с живого,
// уложенного на операционный стол. Цепочка: усыпить → погрузить на носилки →
// уложить на стол → срезать плоть. Здесь — чистая логика: таблица конечностей
// по видам, выбор следующего среза, запись пациента в носилки и подписи.
// Хуки (`inject.rs`, `place_fluid.rs`, `surgery.rs`) только читают и ставят
// метки на мобе.
//
// Метки — модификаторы `FollowRange` с нулевой прибавкой, как метки генов:
// их пишет в сейв ванилла и видит отрисовка (`biolab-deep`). Имена и номера
// конечностей — контракт с отрисовкой, менять только вместе с GDD 4.17.

pub const STRETCHER_ID: &str = "biolab:stretcher";
pub const OPERATING_TABLE: &str = "biolab:operating_table";
/// Ключ в `custom_data` носилок: пациент строкой (`patient_to_text`).
pub const PATIENT_KEY: &str = "biolab_patient";

/// Моб лежит на столе.
pub const SURGERY_TABLE: &str = "biolab:surgery/table";
/// Срезана конечность с этим номером.
pub const SURGERY_FLAYED: [&str; 4] = [
    "biolab:surgery/flayed/0",
    "biolab:surgery/flayed/1",
    "biolab:surgery/flayed/2",
    "biolab:surgery/flayed/3",
];
/// Модификатор скорости (`MovementSpeed`, `add_multiplied_base`) за срезанные
/// конечности. Пока моб на столе, его нет: стол и так держит телепортом.
pub const SURGERY_SLOW: &str = "biolab:surgery/slow";
/// Сколько скорости отнимает одна срезанная конечность: 10 % от базовой.
pub const SLOW_PER_LIMB: f64 = -0.1;
/// Урон от среза. Под наркозом хирург не добивает: здоровье не падает ниже
/// одного сердца (`after_cut`), а дальше решает кровотечение.
pub const CUT_DAMAGE: f32 = 4.0;
/// Ген регенерации отращивает одну конечность за столько тиков (минута).
pub const REGROW_EVERY: u64 = 1200;
/// Снотворное, которое стол подновляет пациенту каждый тик. Короче, чем от
/// шприца: снятый со стола моб просыпается быстро.
pub const TABLE_SEDATION_TICKS: i32 = 100;
/// Пациента дальше этого от точки над столом возвращают телепортом.
pub const TABLE_HOLD: f64 = 0.2;
/// Здоровье уложенного пациента выставляется ещё столько тиков: прибавку к
/// максимуму от генома `biolab-deep` ставит в тике моба, уже после появления.
pub const HEALTH_SETTLE_TICKS: u64 = 5;

/// Виды, у которых не четыре конечности. Остальные — четыре: гуманоиды
/// (руки, потом ноги) и четвероногие (передние, потом задние лапы). Номер
/// среза → часть модели решает отрисовка (GDD 4.17), сервер только считает.
/// 0 — конечностей нет: срезать можно один кусок тела. Пауку и нюхачу —
/// четыре, хоть ног у них больше: меток всего четыре.
pub const LIMBS: [(&str, u8); 34] = [
    // Без конечностей: слизни, гасты, рыбы, кальмары и прочие.
    ("slime", 0),
    ("magma_cube", 0),
    ("sulfur_cube", 0),
    ("ghast", 0),
    ("happy_ghast", 0),
    ("squid", 0),
    ("glow_squid", 0),
    ("cod", 0),
    ("salmon", 0),
    ("tropical_fish", 0),
    ("pufferfish", 0),
    ("tadpole", 0),
    ("dolphin", 0),
    ("guardian", 0),
    ("elder_guardian", 0),
    ("shulker", 0),
    ("phantom", 0),
    ("bat", 0),
    ("silverfish", 0),
    ("endermite", 0),
    ("blaze", 0),
    ("breeze", 0),
    ("nautilus", 0),
    ("zombie_nautilus", 0),
    // Две ноги или две руки.
    ("chicken", 2),
    ("parrot", 2),
    ("strider", 2),
    ("snow_golem", 2),
    ("allay", 2),
    ("vex", 2),
    // Руки одной частью и две ноги; у пчелы три пары лапок.
    ("villager", 3),
    ("wandering_trader", 3),
    ("witch", 3),
    ("bee", 3),
];

/// Гуманоиды: срез 0–1 — руки, 2–3 — ноги. Только для подписи в чате.
pub const HUMANOIDS: [&str; 23] = [
    "zombie", "husk", "drowned", "zombie_villager", "giant", "skeleton", "stray", "bogged",
    "parched", "wither_skeleton", "piglin", "piglin_brute", "zombified_piglin", "pillager",
    "vindicator", "evoker", "illusioner", "enderman", "iron_golem", "copper_golem", "warden",
    "creaking", "mannequin",
];

/// Вид без пространства имён: `minecraft:zombie` → `zombie`.
fn short_kind(kind: &str) -> &str {
    kind.strip_prefix("minecraft:").unwrap_or(kind)
}

/// Вид с пространством имён, как его ждёт `native_spawn_mob` и сейв.
pub fn full_kind(kind: &str) -> String {
    if kind.contains(':') {
        kind.to_string()
    } else {
        format!("minecraft:{kind}")
    }
}

/// Сколько конечностей у вида; 0 — нет ни одной.
pub fn limbs_of(kind: &str) -> u8 {
    let kind = short_kind(kind);
    match LIMBS.iter().find(|(k, _)| *k == kind) {
        Some((_, n)) => *n,
        None => 4,
    }
}

/// Сколько срезов можно сделать: по конечности, а у вида без них — один
/// кусок тела.
pub fn cuts_of(kind: &str) -> u8 {
    limbs_of(kind).clamp(1, SURGERY_FLAYED.len() as u8)
}

/// Маска срезанного по меткам на мобе: `has(метка)` — есть ли она.
pub fn flayed_mask(has: impl Fn(&str) -> bool) -> u8 {
    SURGERY_FLAYED
        .iter()
        .enumerate()
        .filter(|(_, marker)| has(marker))
        .fold(0, |mask, (i, _)| mask | 1 << i)
}

/// Следующий срез: первая несрезанная конечность из `cuts`. None — плоти
/// не осталось.
pub fn next_cut(mask: u8, cuts: u8) -> Option<u8> {
    (0..cuts.min(SURGERY_FLAYED.len() as u8)).find(|i| mask & (1 << i) == 0)
}

/// Сколько конечностей срезано.
pub fn flayed_count(mask: u8) -> u32 {
    (mask & 0b1111).count_ones()
}

/// Прибавка к скорости за срезанное (`add_multiplied_base`): −10 % за
/// конечность, но моб не встаёт совсем.
pub fn surgery_slow(mask: u8) -> f64 {
    (SLOW_PER_LIMB * f64::from(flayed_count(mask))).max(-0.9)
}

/// Какая конечность отрастёт: последняя срезанная, с большим номером.
pub fn regrown(mask: u8) -> Option<u8> {
    (0..SURGERY_FLAYED.len() as u8).rev().find(|i| mask & (1 << i) != 0)
}

/// Пора ли мобу `id` отрастить конечность. Сдвиг на id: мутанты в одной
/// лаборатории отращивают не все в один тик.
pub fn regrow_due(ticks: u64, id: u32) -> bool {
    (ticks + u64::from(id)) % REGROW_EVERY == 0
}

/// Здоровье после среза: урон, но не насмерть.
pub fn after_cut(health: f32) -> f32 {
    (health - CUT_DAMAGE).max(1.0)
}

/// Подпись среза: «правая рука», «левая задняя лапа», «кусок тела».
pub fn limb_name(kind: &str, index: u8) -> String {
    let short = short_kind(kind);
    if limbs_of(kind) == 0 {
        return "кусок тела".to_string();
    }
    let human = ["правая рука", "левая рука", "правая нога", "левая нога"];
    let beast = ["правая передняя лапа", "левая передняя лапа", "правая задняя лапа", "левая задняя лапа"];
    match (limbs_of(kind), HUMANOIDS.contains(&short)) {
        (4, true) => human.get(usize::from(index)).copied().map(str::to_string),
        (4, false) => beast.get(usize::from(index)).copied().map(str::to_string),
        _ => None,
    }
    .unwrap_or_else(|| format!("конечность {}", index + 1))
}

/// Можно ли унести моба на носилках. Боссов — нельзя.
pub fn can_carry(kind: &str) -> bool {
    !matches!(short_kind(kind), "wither" | "ender_dragon")
}

/// Русские имена видов для чата и подсказки носилок. Незнакомый вид
/// подписывается своим id.
pub const MOB_TITLES: [(&str, &str); 66] = [
    ("zombie", "Зомби"),
    ("husk", "Кадавр"),
    ("drowned", "Утопленник"),
    ("zombie_villager", "Зомби-житель"),
    ("skeleton", "Скелет"),
    ("stray", "Зимогор"),
    ("bogged", "Болотник"),
    ("wither_skeleton", "Скелет-иссушитель"),
    ("creeper", "Крипер"),
    ("spider", "Паук"),
    ("cave_spider", "Пещерный паук"),
    ("enderman", "Странник Края"),
    ("endermite", "Чешуйница Края"),
    ("silverfish", "Чешуйница"),
    ("witch", "Ведьма"),
    ("slime", "Слизень"),
    ("magma_cube", "Магмовый куб"),
    ("phantom", "Фантом"),
    ("pillager", "Разбойник"),
    ("vindicator", "Поборник"),
    ("evoker", "Заклинатель"),
    ("illusioner", "Иллюзор"),
    ("ravager", "Разоритель"),
    ("piglin", "Пиглин"),
    ("piglin_brute", "Пиглин-громила"),
    ("zombified_piglin", "Зомбифицированный пиглин"),
    ("hoglin", "Хоглин"),
    ("zoglin", "Зоглин"),
    ("ghast", "Гаст"),
    ("blaze", "Ифрит"),
    ("guardian", "Страж"),
    ("shulker", "Шалкер"),
    ("warden", "Хранитель"),
    ("pig", "Свинья"),
    ("cow", "Корова"),
    ("mooshroom", "Грибная корова"),
    ("sheep", "Овца"),
    ("chicken", "Курица"),
    ("rabbit", "Кролик"),
    ("horse", "Лошадь"),
    ("donkey", "Осёл"),
    ("mule", "Мул"),
    ("llama", "Лама"),
    ("goat", "Коза"),
    ("wolf", "Волк"),
    ("fox", "Лиса"),
    ("cat", "Кошка"),
    ("ocelot", "Оцелот"),
    ("panda", "Панда"),
    ("polar_bear", "Белый медведь"),
    ("camel", "Верблюд"),
    ("armadillo", "Броненосец"),
    ("frog", "Лягушка"),
    ("turtle", "Черепаха"),
    ("axolotl", "Аксолотль"),
    ("bee", "Пчела"),
    ("squid", "Спрут"),
    ("glow_squid", "Светящийся спрут"),
    ("dolphin", "Дельфин"),
    ("bat", "Летучая мышь"),
    ("parrot", "Попугай"),
    ("strider", "Лавомерка"),
    ("sniffer", "Нюхач"),
    ("villager", "Житель"),
    ("iron_golem", "Железный голем"),
    ("snow_golem", "Снежный голем"),
];

/// Имя вида по-русски.
pub fn mob_title(kind: &str) -> String {
    let short = short_kind(kind);
    MOB_TITLES
        .iter()
        .find(|(k, _)| *k == short)
        .map_or_else(|| short.to_string(), |(_, title)| (*title).to_string())
}

/// Пациент на носилках: всё, что нужно, чтобы вернуть моба в мир тем же.
/// Снаряжение, окрас и возраст носилки не переносят: у `native_spawn_mob`
/// таких настроек нет.
#[derive(Clone, Debug, PartialEq)]
pub struct Patient {
    /// `minecraft:zombie`.
    pub kind: String,
    pub genes: Vec<(&'static str, i32)>,
    pub health: f32,
    /// Маска срезанных конечностей: бит на номер `SURGERY_FLAYED`.
    pub flayed: u8,
    /// Имя, данное моба биркой.
    pub name: Option<String>,
    /// Чем усыплён (`SLEEPERS`): стол подновляет это вещество (GDD 4.20).
    /// None — запись старше M5 или вещество забыто: тогда седативное.
    pub sleeper: Option<&'static str>,
    /// Парализован кураре, а не спит: на стол такого не кладут.
    pub paralysed: bool,
}

/// Пациент строкой для `custom_data`: поля `ключ=значение` через `;`, имя
/// последним — в нём самом может оказаться `;`.
pub fn patient_to_text(p: &Patient) -> String {
    let mut text = format!(
        "kind={};health={};flayed={};genes={}",
        full_kind(&p.kind),
        p.health,
        p.flayed & 0b1111,
        genome_to_text(&p.genes)
    );
    if let Some(sleeper) = p.sleeper {
        text.push_str(";sleep=");
        text.push_str(sleeper);
    }
    if p.paralysed {
        text.push_str(";paralysed=1");
    }
    if let Some(name) = &p.name {
        text.push_str(";name=");
        text.push_str(name);
    }
    text
}

/// Пациент из строки носилок. None — носилки пусты или запись битая (без
/// вида некого возвращать в мир).
pub fn patient_from_text(text: &str) -> Option<Patient> {
    let mut p = Patient {
        kind: String::new(),
        genes: Vec::new(),
        health: 0.0,
        flayed: 0,
        name: None,
        sleeper: None,
        paralysed: false,
    };
    // Имя — хвост целиком, остальное — поля до него.
    let (fields, name) = match text.split_once(";name=") {
        Some((fields, name)) => (fields, Some(name)),
        None => (text, None),
    };
    for field in fields.split(';') {
        let Some((key, value)) = field.split_once('=') else {
            continue;
        };
        match key {
            "kind" => p.kind = value.trim().to_string(),
            "health" => p.health = value.trim().parse().unwrap_or(0.0),
            "flayed" => p.flayed = value.trim().parse::<u8>().unwrap_or(0) & 0b1111,
            "genes" => p.genes = genome_from_text(value.trim()),
            "sleep" => p.sleeper = sleeper(value.trim()).map(|s| s.id),
            "paralysed" => p.paralysed = value.trim() == "1",
            _ => {}
        }
    }
    p.name = name.filter(|n| !n.is_empty()).map(str::to_string);
    if p.kind.is_empty() {
        return None;
    }
    p.kind = full_kind(&p.kind);
    Some(p)
}

/// Кто на носилках: «Зомби» или «Зомби «Боб»».
pub fn patient_title(p: &Patient) -> String {
    match &p.name {
        Some(name) => format!("{} «{name}»", mob_title(&p.kind)),
        None => mob_title(&p.kind),
    }
}

/// Подпись занятых носилок: «Носилки: Зомби».
pub fn stretcher_label(text: &str) -> Option<String> {
    Some(format!("Носилки: {}", patient_title(&patient_from_text(text)?)))
}

/// Строки данных носилок для подсказки: кто на них и в каком он виде.
pub fn stretcher_tooltip(text: &str) -> Vec<String> {
    let Some(p) = patient_from_text(text) else {
        return vec!["Пусто".to_string()];
    };
    let mut lines = vec![
        format!("На носилках: {}", patient_title(&p)),
        format!("Здоровье: {:.0}", p.health),
    ];
    let genes: Vec<String> = p
        .genes
        .iter()
        .filter_map(|(name, strength)| Some(format!("{} {}", gene(name)?.title, "+".repeat(*strength as usize))))
        .collect();
    lines.push(if genes.is_empty() {
        "Геном: без мутаций".to_string()
    } else {
        format!("Геном: {}", genes.join(", "))
    });
    if p.paralysed {
        lines.push("Парализован: на стол — только спящего".to_string());
    }
    let cuts = cuts_of(&p.kind);
    let done = flayed_count(p.flayed).min(u32::from(cuts));
    if done > 0 {
        lines.push(format!("Срезано: {done} из {cuts}"));
    }
    lines
}

/// Что сказать после среза: какая конечность и что досталось.
pub fn cut_message(kind: &str, index: u8, given: u32, genes: &[(&'static str, i32)]) -> String {
    format!("Срезано: {}. {}", limb_name(kind, index), tissue_message(given, genes))
}

/// Точка над столом, где лежит пациент. Стол — полный блок: моб стоит на
/// его верхней грани.
pub fn table_spot(table: [i32; 3]) -> [f64; 3] {
    [f64::from(table[0]) + 0.5, f64::from(table[1]) + 1.0, f64::from(table[2]) + 0.5]
}

/// Свой стол пациента: ближайший стол под мобом — на блок вниз и вбок, на
/// случай если его сдвинуло. `is_table(блок)` — стоит ли там стол.
pub fn nearest_table(at: [f64; 3], is_table: impl Fn([i32; 3]) -> bool) -> Option<[i32; 3]> {
    let base = [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32];
    let mut best: Option<([i32; 3], f64)> = None;
    for dy in -2..=0 {
        for dx in -1..=1 {
            for dz in -1..=1 {
                let spot = [base[0] + dx, base[1] + dy, base[2] + dz];
                if !is_table(spot) {
                    continue;
                }
                let top = table_spot(spot);
                let away = (top[0] - at[0]).powi(2) + (top[1] - at[1]).powi(2) + (top[2] - at[2]).powi(2);
                if best.is_none_or(|(_, far)| away < far) {
                    best = Some((spot, away));
                }
            }
        }
    }
    best.map(|(spot, _)| spot)
}

/// Ушёл ли пациент от точки над столом настолько, что его пора вернуть.
pub fn off_table(at: [f64; 3], spot: [f64; 3]) -> bool {
    (at[0] - spot[0]).powi(2) + (at[1] - spot[1]).powi(2) + (at[2] - spot[2]).powi(2)
        > TABLE_HOLD * TABLE_HOLD
}

/// Блок, в котором можно лежать: над столом должно быть пусто.
pub fn is_air(block: &str) -> bool {
    matches!(block, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
}

// ---- Стол из секций ----
//
// Один блок стола — секция. Секции, поставленные в прямой ряд вдоль X или Z,
// — один стол: зомби в два блока ростом на одной секции лежал бы, свесившись
// с обеих сторон. Сколько секций нужно, решает пациент: его длина лёжа,
// умноженная на масштаб генома (гигантизм, карликовость), с округлением вверх.
//
// Соединение секций видно по модели. Шаблон блока — `minecraft:mushroom_stem`
// (`HugeMushroomBlock`): шесть булевых свойств сторон, `false` — там соседняя
// секция, `true` — сторона открыта. Ванилла ставит `false`, когда сосед — тот
// же блок, но обратно в `true` не возвращает: это делают хук `table_shape.rs`
// (на каждое обновление формы) и, для верности, укладка и удержание
// (`table_side_value` по соседям ряда).

/// Длиннее ряд не ищем: стол на 16 секций уложит и гиганта с гигантизмом.
pub const TABLE_MAX_SECTIONS: i32 = 16;

/// Длина тела лёжа в блоках для видов, у которых модель длиннее, чем выше:
/// корова, паук, лошадь. Хитбокс у них квадратный в плане, и его ширина
/// длины тела не знает. Числа сняты с ванильных моделей — от морды до
/// хвоста, без ног паука вбок. Остальные виды («высокие»: зомби, скелет,
/// игрокоподобные, курица) лежат на спине, и длина лёжа — их рост.
pub const LONG_BODIES: [(&str, f64); 30] = [
    ("cow", 1.5),
    ("mooshroom", 1.5),
    ("sheep", 1.5),
    ("pig", 1.45),
    ("spider", 1.6),
    ("cave_spider", 1.15),
    ("wolf", 1.25),
    ("fox", 1.3),
    ("cat", 1.3),
    ("ocelot", 1.3),
    ("horse", 2.0),
    ("donkey", 1.9),
    ("mule", 1.9),
    ("skeleton_horse", 2.0),
    ("zombie_horse", 2.0),
    ("camel", 2.4),
    ("camel_husk", 2.4),
    ("hoglin", 2.2),
    ("zoglin", 2.2),
    ("ravager", 2.6),
    ("sniffer", 2.4),
    ("polar_bear", 1.6),
    ("panda", 1.9),
    ("turtle", 1.3),
    ("armadillo", 0.8),
    ("axolotl", 0.9),
    ("dolphin", 1.6),
    ("salmon", 1.0),
    ("goat", 1.2),
    ("phantom", 1.3),
];

/// Прибавка к размеру от гена за единицу силы — та же, что рисует
/// `biolab-deep` (`mutants.rs`, `scale`): по таблице стол должен вместить
/// ровно ту модель, которую видит игрок. Сверяется тестом с исходником deep.
pub const GENE_SCALE: [(&str, f64); 12] = [
    ("gigantism", 0.22),
    ("dwarfism", -0.18),
    ("hypermetabolism", -0.04),
    ("chitin", 0.05),
    ("dense_bone", 0.04),
    ("brute_force", 0.06),
    ("thin_skin", -0.05),
    ("thorns", 0.06),
    ("predator", 0.08),
    ("rending_claws", 0.04),
    ("polymelia", 0.02),
    ("tumor", 0.04),
];

/// Во сколько раз мутант крупнее обычного моба: размеры генов перемножаются,
/// как в `biolab-deep`.
pub fn body_scale(genes: &[(&str, i32)]) -> f64 {
    genes.iter().fold(1.0, |acc, (name, strength)| {
        let step = GENE_SCALE.iter().find(|(g, _)| g == name).map_or(0.0, |(_, s)| *s);
        acc * (1.0 + step * f64::from(*strength))
    })
}

/// Длина моба лёжа в блоках. `width`, `height` — хитбокс вида, как его знает
/// Horizon (`MobType`, из `entity-types.json`). Длинным видам — длина тела по
/// таблице, остальным — рост; плоскому и широкому (черепаха, гаст) — ширина,
/// если она больше роста.
pub fn lying_length(kind: &str, width: f64, height: f64) -> f64 {
    let short = short_kind(kind);
    match LONG_BODIES.iter().find(|(k, _)| *k == short) {
        Some((_, length)) => *length,
        None => height.max(width),
    }
}

/// Сколько секций нужно, чтобы уложить моба длиной `length` с масштабом
/// `scale`: с округлением вверх, но не меньше одной. Сотая блока — допуск на
/// округление чисел хитбоксов (скелет в 1,99 — две секции, а не три).
pub fn sections_for_length(length: f64, scale: f64) -> i32 {
    let need = length * scale;
    if !need.is_finite() || need <= 0.0 {
        return 1;
    }
    ((need - 0.01).ceil() as i32).max(1)
}

/// Сколько секций нужно пациенту вида `kind` с хитбоксом вида и генами.
pub fn table_sections(kind: &str, width: f64, height: f64, genes: &[(&str, i32)]) -> i32 {
    sections_for_length(lying_length(kind, width, height), body_scale(genes))
}

/// Что сказать, когда ряд короче нужного.
pub fn table_too_short(need: i32, have: i32) -> String {
    format!("Пациент не помещается: нужен стол из {need} секций, здесь {have}")
}

/// Ряд секций стола: прямая линия вдоль X или Z.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableRow {
    /// Секция с наименьшей координатой по оси ряда — изголовье.
    pub start: [i32; 3],
    /// Шаг вдоль ряда: `[1, 0, 0]` — вдоль X, `[0, 0, 1]` — вдоль Z.
    pub step: [i32; 3],
    /// Сколько секций.
    pub len: i32,
}

impl TableRow {
    /// Вдоль X ли ряд.
    pub fn along_x(&self) -> bool {
        self.step[0] != 0
    }

    /// Секции по порядку от изголовья.
    pub fn sections(&self) -> Vec<[i32; 3]> {
        (0..self.len)
            .map(|i| [self.start[0] + self.step[0] * i, self.start[1], self.start[2] + self.step[2] * i])
            .collect()
    }

    /// Точка, где лежит пациент: середина ряда на верхней грани (стол —
    /// полный блок). У ряда из двух секций это стык между ними.
    pub fn center(&self) -> [f64; 3] {
        let half = f64::from(self.len) / 2.0;
        let along = |start: i32, step: i32| {
            if step == 0 {
                f64::from(start) + 0.5
            } else {
                f64::from(start) + half
            }
        };
        [
            along(self.start[0], self.step[0]),
            f64::from(self.start[1]) + 1.0,
            along(self.start[2], self.step[2]),
        ]
    }

    /// Куда пациент смотрит лёжа (градусы, 0 — на юг, −90 — на восток).
    /// Отрисовка (`lying` в biolab-deep) кладёт голову назад, против взгляда:
    /// смотрит к дальнему концу — голова у изголовья (`start`), на западе у
    /// ряда вдоль X и на севере у ряда вдоль Z. Там же модель ставит лампу.
    pub fn yaw(&self) -> f32 {
        if self.along_x() { -90.0 } else { 0.0 }
    }

    /// Стоит ли точка `at` над этим рядом (пациент на нём).
    pub fn covers(&self, at: [f64; 3]) -> bool {
        let end = [
            self.start[0] + self.step[0] * (self.len - 1),
            self.start[1],
            self.start[2] + self.step[2] * (self.len - 1),
        ];
        let inside = |v: f64, lo: i32, hi: i32| v >= f64::from(lo) - 0.25 && v <= f64::from(hi) + 1.25;
        inside(at[0], self.start[0], end[0])
            && inside(at[2], self.start[2], end[2])
            && at[1] >= f64::from(self.start[1]) + 0.5
            && at[1] <= f64::from(self.start[1]) + 2.5
    }
}

/// Ряд, в который входит секция `pos`: по обе стороны вдоль X и вдоль Z
/// считаются секции подряд, ряд — по оси, где их больше. Поровну (одна
/// секция или крест) — вдоль X. `is_table(блок)` — стоит ли там секция.
pub fn table_row(pos: [i32; 3], is_table: impl Fn([i32; 3]) -> bool) -> TableRow {
    let run = |step: [i32; 3], sign: i32| {
        (1..TABLE_MAX_SECTIONS)
            .take_while(|i| is_table([pos[0] + step[0] * sign * i, pos[1], pos[2] + step[2] * sign * i]))
            .count() as i32
    };
    let row = |step: [i32; 3]| {
        let back = run(step, -1);
        let ahead = run(step, 1);
        let len = (back + 1 + ahead).min(TABLE_MAX_SECTIONS);
        TableRow { start: [pos[0] - step[0] * back, pos[1], pos[2] - step[2] * back], step, len }
    };
    let x = row([1, 0, 0]);
    let z = row([0, 0, 1]);
    if z.len > x.len { z } else { x }
}

/// Свойство шаблона-гриба для стороны с нормалью `normal`.
pub fn table_side_key(normal: [i32; 3]) -> Option<&'static str> {
    match normal {
        [0, 0, -1] => Some("north"),
        [0, 0, 1] => Some("south"),
        [-1, 0, 0] => Some("west"),
        [1, 0, 0] => Some("east"),
        [0, 1, 0] => Some("up"),
        [0, -1, 0] => Some("down"),
        _ => None,
    }
}

/// Стороны секции: свойство и нормаль, все шесть, как у гриба.
pub const TABLE_SIDES: [[i32; 3]; 6] =
    [[0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0], [0, 1, 0], [0, -1, 0]];

/// Значение свойства стороны: к соседней секции — `false` (соединено, как
/// ставит ванилла гриба), иначе `true` — сторона открыта.
pub fn table_side_value(joined: bool) -> &'static str {
    if joined { "false" } else { "true" }
}

/// Какими должны быть стороны секции `pos` по её соседям: свойство и
/// значение на каждую из шести сторон. `is_table(блок)` — секция ли там.
pub fn table_sides(pos: [i32; 3], is_table: impl Fn([i32; 3]) -> bool) -> Vec<(&'static str, &'static str)> {
    TABLE_SIDES
        .iter()
        .filter_map(|n| {
            let key = table_side_key(*n)?;
            Some((key, table_side_value(is_table([pos[0] + n[0], pos[1] + n[1], pos[2] + n[2]]))))
        })
        .collect()
}

/// Пора ли пересчитать стороны ряда на тике удержания: раз в секунду —
/// хука `table_shape.rs` хватает, это страховка.
pub fn table_sides_due(ticks: u64) -> bool {
    ticks % 20 == 0
}

/// Хук операционной висит на тике игрока, а игроков бывает несколько: тик
/// мира обрабатываем один раз, остальные вызовы того же тика пропускаем.
fn last_surgery_pass() -> &'static std::sync::atomic::AtomicU64 {
    static AT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
    &AT
}

/// Первый ли это вызов в тике `ticks`.
pub fn surgery_pass_due(ticks: u64) -> bool {
    last_surgery_pass().swap(ticks, std::sync::atomic::Ordering::Relaxed) != ticks
}

fn settling() -> &'static std::sync::Mutex<std::collections::HashMap<u32, (f32, u64)>> {
    static AT: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u32, (f32, u64)>>> =
        std::sync::OnceLock::new();
    AT.get_or_init(Default::default)
}

/// Запомнить здоровье уложенного пациента до тика `until`: пока геном
/// поднимает ему максимум, здоровье держим тем, что было на носилках.
pub fn settle_health(id: u32, health: f32, until: u64) {
    if let Ok(mut table) = settling().lock() {
        table.insert(id, (health, until));
    }
}

/// Кому из уложенных ещё надо выставить здоровье в тике `ticks`: id и
/// здоровье с носилок. Чей срок вышел, тот забывается — и умерший тоже.
pub fn due_settles(ticks: u64) -> Vec<(u32, f32)> {
    let Ok(mut table) = settling().lock() else {
        return Vec::new();
    };
    table.retain(|_, (_, until)| ticks <= *until);
    table.iter().map(|(id, (health, _))| (*id, *health)).collect()
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
pub const BENCH_RESULTS: [(&str, &str); 40] = [
    ("biolab:syringe", "Шприц"),
    ("biolab:syringe_penicillin", "Пенициллин, инъекция"),
    ("biolab:syringe_sedative", "Седативное, инъекция"),
    ("biolab:syringe_stimulant", "Стимулятор, инъекция"),
    ("biolab:syringe_venom", "Яд, инъекция"),
    ("biolab:syringe_vaccine", "Вакцина, инъекция"),
    ("biolab:syringe_mutagen", "Мутаген, инъекция"),
    ("biolab:dart_sedative", "Седативное, дротик"),
    ("biolab:dart_venom", "Яд, дротик"),
    ("biolab:spray_analgesic", "Обезболивающее, аэрозоль"),
    ("biolab:spray_stimulant", "Стимулятор, аэрозоль"),
    ("biolab:ointment_analgesic", "Обезболивающее, мазь"),
    ("biolab:analyzer", "Анализатор крови"),
    ("biolab:microscope", "Микроскоп"),
    ("biolab:scalpel", "Скальпель"),
    ("biolab:swab", "Ватная палочка"),
    ("biolab:bandage", "Бинт"),
    ("biolab:mutagen", "Мутаген"),
    ("biolab:nutrient_broth", "Питательный раствор"),
    ("biolab:blowpipe", "Духовая трубка"),
    ("biolab:tranq_crossbow", "Арбалет-транквилизатор"),
    ("biolab:air_rifle", "Пневматическое ружьё"),
    ("biolab:co2_canister_empty", "Пустой баллон CO₂"),
    // Линейка снотворных (GDD 4.20, M5): дописаны в конец — номер уходит
    // экрану стола числом, старые номера не сдвигаются.
    ("biolab:syringe_calming", "Успокоительное, инъекция"),
    ("biolab:mandrake_mash", "Сусло мандрагоры"),
    ("biolab:syringe_mandrake", "Мандрагора, инъекция"),
    ("biolab:dart_mandrake", "Мандрагора, дротик"),
    ("biolab:ether_mix", "Смесь для эфира"),
    ("biolab:spray_ether", "Эфир, аэрозоль"),
    ("biolab:grenade_ether", "Эфир, граната"),
    ("biolab:chloroform_mix", "Смесь для хлороформа"),
    ("biolab:spray_chloroform", "Хлороформ, аэрозоль"),
    ("biolab:rag_chloroform", "Хлороформ, тряпка"),
    ("biolab:grenade_chloroform", "Хлороформ, граната"),
    ("biolab:curare_mash", "Сусло кураре"),
    ("biolab:syringe_curare", "Кураре, инъекция"),
    ("biolab:dart_curare", "Кураре, дротик"),
    ("biolab:syringe_tranquilizer", "Транквилизатор, инъекция"),
    ("biolab:dart_tranquilizer", "Транквилизатор, дротик"),
    ("biolab:syringe_wake", "Пробуждающее, инъекция"),
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
        // Новое дописывается в конец: номера старых результатов не сдвигаются.
        assert_eq!(bench_result_index("biolab:nutrient_broth"), 18);
        // Баллон дописан веткой оружия, снотворные — следом за ним.
        assert!(bench_result_index(CO2_CANISTER_EMPTY_ID) > 18);
        assert_eq!(bench_result_index("biolab:nutrient_broth"), 18, "старые номера не сдвинулись");
        assert_eq!(bench_result_index("biolab:syringe_wake"), BENCH_RESULTS.len() as i32 - 1);
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
/// мода (см. «Карта исследований»).
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

/// Пустое ведро — ванильное.
pub const EMPTY_BUCKET: &str = "minecraft:bucket";

/// Что остаётся в руке, когда ведро лужи вылито: в выживании пустое ведро,
/// в творческом — то же ведро, как у ванильных вёдер
/// (`BucketItem.getEmptySuccessItem`). None — рука не меняется: творческий
/// режим или это не наше ведро.
pub fn emptied_bucket(item: &str, creative: bool) -> Option<&'static str> {
    (fluid_of_bucket(item).is_some() && !creative).then_some(EMPTY_BUCKET)
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
pub const DRUG_COUNT: usize = 14;

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
    // Линейка снотворных (GDD 4.20, M5). На мобе каждое действует по силе
    // против массы (`SLEEPERS`), здесь — то, что оно делает человеку, если
    // уколоть себя или вдохнуть газ. Новые вещества — в конец: по номеру
    // идут байты пакета анализатора (в файле состояния — по имени).
    Drug {
        id: "calming",
        name: "Успокоительное",
        half_life: 30,
        window: (10, 80),
        effects: &[SEDATION_EFFECT],
        cures: 0,
        immunity: false,
        clears: false,
        withdrawal: false,
    },
    // Скополамин мандрагоры и белладонны: побочка — тошнота и «бред».
    Drug {
        id: "mandrake",
        name: "Мандрагора",
        half_life: 50,
        window: (10, 45),
        effects: &[SEDATION_EFFECT, "slowness", "nausea"],
        cures: 0,
        immunity: false,
        clears: false,
        withdrawal: false,
    },
    // Газы выветриваются быстро: облако задело — и через полминуты чисто.
    Drug {
        id: "ether",
        name: "Эфир",
        half_life: 15,
        window: (5, 40),
        effects: &["slowness", "nausea"],
        cures: 0,
        immunity: false,
        clears: false,
        withdrawal: false,
    },
    // Узкое окно — как у настоящего хлороформа: вдохнул лишнего — токсикоз.
    Drug {
        id: "chloroform",
        name: "Хлороформ",
        half_life: 25,
        window: (5, 20),
        effects: &["slowness", "blindness"],
        cures: 0,
        immunity: false,
        clears: false,
        withdrawal: false,
    },
    Drug {
        id: "curare",
        name: "Кураре",
        half_life: 40,
        window: (5, 40),
        effects: &[PARALYSIS_EFFECT, "slowness", "mining_fatigue"],
        cures: 0,
        immunity: false,
        clears: false,
        withdrawal: false,
    },
    Drug {
        id: "tranquilizer",
        name: "Транквилизатор",
        half_life: 60,
        window: (10, 40),
        effects: &[SEDATION_EFFECT, "slowness", "blindness"],
        cures: 0,
        immunity: false,
        clears: false,
        withdrawal: false,
    },
    // Кофеин + антидот: бодрит и, как антидот, выводит остальное из крови.
    Drug {
        id: "wake",
        name: "Пробуждающее",
        half_life: 20,
        window: (10, 100),
        effects: &["speed"],
        cures: 0,
        immunity: false,
        clears: true,
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

pub const FORMS: [Form; 5] = [
    Form { id: "pill", name: "таблетка", bioavailability: 60, direct: false },
    Form { id: "injection", name: "инъекция", bioavailability: 100, direct: true },
    Form { id: "ointment", name: "мазь", bioavailability: 40, direct: false },
    Form { id: "spray", name: "аэрозоль", bioavailability: 75, direct: true },
    // Отвар в бутылке (GDD 4.20): пьётся и всасывается, как таблетка, —
    // через желудок, но жидкость идёт чуть полнее.
    Form { id: "drink", name: "отвар", bioavailability: 70, direct: false },
];

pub const FORM_PILL: usize = 0;
pub const FORM_INJECTION: usize = 1;
pub const FORM_OINTMENT: usize = 2;
pub const FORM_SPRAY: usize = 3;
pub const FORM_DRINK: usize = 4;

/// Готовый препарат: предмет, вещество в нём, форма и доза.
pub struct Medicine {
    pub item: &'static str,
    pub drug: usize,
    pub form: usize,
    pub dose: u32,
}

pub const MEDICINES: [Medicine; 18] = [
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
    // Снотворные (GDD 4.20). Газы (эфир, хлороформ) себе не принимают: их
    // аэрозоль и граната дают облако (`gas_use.rs`, `grenade_hit.rs`), а
    // игрок в облаке без противогаза вдыхает дозу (`take_gas`).
    Medicine { item: "biolab:calming_brew", drug: 7, form: FORM_DRINK, dose: 60 },
    Medicine { item: "biolab:syringe_calming", drug: 7, form: FORM_INJECTION, dose: 40 },
    Medicine { item: "biolab:syringe_mandrake", drug: 8, form: FORM_INJECTION, dose: 35 },
    Medicine { item: "biolab:syringe_curare", drug: 11, form: FORM_INJECTION, dose: 25 },
    Medicine { item: "biolab:syringe_tranquilizer", drug: 12, form: FORM_INJECTION, dose: 30 },
    Medicine { item: "biolab:syringe_wake", drug: 13, form: FORM_INJECTION, dose: 50 },
];

/// Дротики: те же вещества, но вводятся издали — дротик бросают в моба.
pub const DARTS: [(&str, &str); 5] = [
    ("biolab:dart_sedative", "biolab:syringe_sedative"),
    ("biolab:dart_venom", "biolab:syringe_venom"),
    ("biolab:dart_mandrake", "biolab:syringe_mandrake"),
    ("biolab:dart_curare", "biolab:syringe_curare"),
    ("biolab:dart_tranquilizer", "biolab:syringe_tranquilizer"),
];

/// Чем снаряжён дротик: шприц с тем же веществом.
pub fn dart_syringe(item: &str) -> Option<&'static str> {
    DARTS
        .iter()
        .find(|(dart, _)| *dart == item)
        .map(|(_, syringe)| *syringe)
}

/// Дротик — настоящий снаряд (GDD 4.20): у каждого свой тип сущности мода,
/// `data/biolab/horizon/entity_type/<дротик>.json`, копия снежка. Летит, как
/// снежок, и рисуется моделью своего предмета. Тип назван так же, как
/// предмет, поэтому хук попадания (`dart_hit.rs`) узнаёт дротик по типу
/// снаряда. Типов два, а не один общий: движок разрешает предмету бросать
/// только свой тип, а своих данных на снаряде нет — вещество едет в типе.
pub fn dart_projectile(kind: &str) -> Option<&'static str> {
    DARTS.iter().find(|(dart, _)| *dart == kind).map(|(dart, _)| *dart)
}

/// Дальность броска рукой. Летит дротик дальше (баллистика снежка — до двух
/// десятков блоков), но дальше этой точки игла входит на излёте и вещество
/// не доходит. Дальний выстрел — дело оружия (`DART_WEAPONS`), у него своя
/// дальность.
pub const DART_THROW_RANGE: f64 = 8.0;

/// Сколько тиков помним бросок, если о дротике больше ничего не слышно
/// (улетел из прогруженных чанков, мир закрыли): минута полёта — с запасом.
pub const DART_FLIGHT_MAX: u64 = 1200;

/// Откуда и чем выпущен дротик: глаза бросавшего и дальность, до которой
/// игла ещё входит, — броска рукой или оружия.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DartLaunch {
    pub from: [f64; 3],
    pub range: u32,
}

/// Что делает попадание дротика.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DartImpact {
    /// Не дротик или не в моба: остальное игра делает сама — дротик
    /// ломается о блок, как снежок, крошками своего предмета.
    Nothing,
    /// Игла вошла: мобу — вещество этого шприца (`mob_injection`).
    Inject(&'static str),
    /// В моба попал, но на излёте: дальше дальности выстрела (в блоках).
    Spent(u32),
}

/// Попадание снаряда типа `kind`: в моба ли (`on_mob`), откуда и чем его
/// выпустили (`launch`; None — запуск не слышали: дротик из `/summon` или из
/// сохранения) и где он встретил цель (`at`). Без точки запуска дальность не
/// проверить, и такой дротик действует: лучше лишний укол, чем дротик,
/// который молча не работает.
pub fn dart_impact(kind: &str, on_mob: bool, launch: Option<DartLaunch>, at: [f64; 3]) -> DartImpact {
    let Some(dart) = dart_projectile(kind) else {
        return DartImpact::Nothing;
    };
    if !on_mob {
        return DartImpact::Nothing;
    }
    if let Some(DartLaunch { from, range }) = launch {
        let far = (0..3).map(|i| (at[i] - from[i]).powi(2)).sum::<f64>().sqrt();
        if far > f64::from(range) {
            return DartImpact::Spent(range);
        }
    }
    match dart_syringe(dart) {
        Some(syringe) => DartImpact::Inject(syringe),
        None => DartImpact::Nothing,
    }
}

/// Запуски дротиков, что ещё летят: id снаряда, запуск, тик запуска.
/// Хук попадания точку запуска не знает, а хук запуска знает игрока, —
/// поэтому запуск её запоминает, а попадание забирает.
fn dart_launches() -> &'static std::sync::Mutex<Vec<(u32, DartLaunch, u64)>> {
    static LAUNCHES: std::sync::Mutex<Vec<(u32, DartLaunch, u64)>> = std::sync::Mutex::new(Vec::new());
    &LAUNCHES
}

/// Запомнить запуск дротика `projectile` на тике `tick`. Заодно забываем
/// давние: о дротике не слышно дольше `DART_FLIGHT_MAX` — он уже не
/// прилетит. Разница по модулю, потому что после перезахода в мир тики идут
/// заново, а давний бросок не должен висеть вечно.
pub fn remember_dart_launch(projectile: u32, launch: DartLaunch, tick: u64) {
    let Ok(mut launches) = dart_launches().lock() else {
        return;
    };
    launches.retain(|&(id, _, at)| id != projectile && at.abs_diff(tick) <= DART_FLIGHT_MAX);
    launches.push((projectile, launch, tick));
}

/// Забрать запуск дротика `projectile`: попадание у снаряда одно.
pub fn take_dart_launch(projectile: u32) -> Option<DartLaunch> {
    let mut launches = dart_launches().lock().ok()?;
    let at = launches.iter().position(|&(id, _, _)| id == projectile)?;
    Some(launches.swap_remove(at).1)
}

/// Дальность оружия, которое прямо сейчас выпускает дротик. Хук запуска
/// (`dart_launch.rs`) зовётся изнутри `add_projectile` и знает только id и
/// тип снаряда — из чего выпущен, ему не сказать иначе. Оружие (`weapons.rs`)
/// взводит дальность перед `add_projectile` и снимает после; бросок рукой
/// её не взводит, и запуск берёт `DART_THROW_RANGE`. 0 — не взведено.
static ARMED_RANGE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Взвести дальность выстрела перед запуском дротика из оружия.
pub fn arm_dart_range(range: u32) {
    ARMED_RANGE.store(range, std::sync::atomic::Ordering::Relaxed);
}

/// Снять взведённую дальность: хук запуска забирает её, а оружие снимает
/// после `add_projectile` — на случай, если хук не позвали (мод выключен),
/// чтобы она не досталась следующему броску рукой.
pub fn take_armed_range() -> Option<u32> {
    match ARMED_RANGE.swap(0, std::sync::atomic::Ordering::Relaxed) {
        0 => None,
        range => Some(range),
    }
}

/// Запуск дротика из точки `from`: дальность — взведённая оружием, иначе
/// броска рукой.
pub fn dart_launch_from(from: [f64; 3]) -> DartLaunch {
    DartLaunch { from, range: take_armed_range().unwrap_or(DART_THROW_RANGE as u32) }
}

/// Сообщение, когда дротик долетел на излёте.
pub fn dart_spent_message(range: u32) -> String {
    if range == DART_THROW_RANGE as u32 {
        format!("Дротик на излёте: игла не вошла (рукой — до {range} блоков)")
    } else {
        format!("Дротик на излёте: игла не вошла (дальность оружия — {range} блоков)")
    }
}

// ---- Оружие для дротиков (GDD 4.20, «Оружие для снотворного») ----
//
// Оружие берёт дротики из инвентаря, как лук стрелы, и вещество задаёт
// дротик, а не оружие. Выстрел — тот же снаряд-дротик, что и бросок рукой
// (свой тип сущности, `dart_hit.rs`), но быстрее и с большей дальностью
// действия: её запоминает запуск (`dart_launch.rs`), и граница «на излёте»
// становится дальностью оружия. Снаряд выпускает `weapons.rs` (модуль игры):
// `Projectile::new` + `shoot_from_rotation` из `projectiles.rs` пина и
// `add_projectile`, как `throw_item` у снежка, только со своей скоростью.
//
// Ванильный арбалет взять шаблоном нельзя: его заряд, натяжение и выстрел в
// пине привязаны к id `minecraft:crossbow` (`game/crossbow.rs`,
// `use_crossbow`), берут только стрелы и фейерверки и выпускают стрелу.
// Поэтому натяжение — своё, на «использовании со временем» предмета мода
// (`native_use_duration` + `native_on_use_tick` + `native_release_using`).
//
// Здесь — чистая логика: таблица оружия, данные на стеке, решения клика,
// натяжения и выстрела. Хуки (`remedy.rs`, `weapon_*.rs`) только исполняют.

pub const BLOWPIPE_ID: &str = "biolab:blowpipe";
pub const TRANQ_CROSSBOW_ID: &str = "biolab:tranq_crossbow";
pub const AIR_RIFLE_ID: &str = "biolab:air_rifle";
/// Баллон сжатого CO₂ — газ ружья. Заправляется в ферментере (побочный
/// продукт брожения, GDD 4.2) из пустого.
pub const CO2_CANISTER_ID: &str = "biolab:co2_canister";
pub const CO2_CANISTER_EMPTY_ID: &str = "biolab:co2_canister_empty";

/// Скорость броска рукой: `throw_item` пина бросает снежок с 1,5.
pub const DART_THROW_SPEED: f32 = 1.5;

/// Как оружие заряжается.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loading {
    /// Трубка: каждый выстрел берёт дротик прямо из инвентаря, без натяжения.
    Each,
    /// Арбалет: зажатый ПКМ натягивает и заряжает один дротик, следующий
    /// клик стреляет — как ванильный арбалет.
    Charge,
    /// Ружьё: магазин заряжается из инвентаря кликом, зажатый ПКМ — прицел
    /// с приближением, отпустить — выстрел.
    Magazine,
}

/// Оружие для дротиков.
pub struct DartWeapon {
    pub item: &'static str,
    /// Дальность действия, блоков: дальше игла входит на излёте.
    pub range: u32,
    /// Скорость снаряда, блоков за тик (у броска рукой 1,5). Выше 3,9 не
    /// поднимаем: столько ванильный пакет скорости сущности передаёт клиенту.
    pub speed: f32,
    /// Разброс (`uncertainty` у `Projectile.shoot`): у броска рукой 1.
    pub spread: f32,
    /// Пауза после выстрела, тиков (кулдаун предмета).
    pub cooldown: i32,
    pub loading: Loading,
    /// Тихий выстрел: без игрового события `projectile_shoot`, которое
    /// слышат сенсоры скалка и страж, и почти без звука.
    pub quiet: bool,
    pub cue: Cue,
}

pub const DART_WEAPONS: [DartWeapon; 3] = [
    DartWeapon {
        item: BLOWPIPE_ID,
        range: 16,
        speed: 2.0,
        spread: 1.0,
        cooldown: 20,
        loading: Loading::Each,
        quiet: true,
        cue: Cue::DartShot,
    },
    // Ванильный арбалет стреляет с 3,15 — так же и наш.
    DartWeapon {
        item: TRANQ_CROSSBOW_ID,
        range: 32,
        speed: 3.15,
        spread: 0.5,
        cooldown: 5,
        loading: Loading::Charge,
        quiet: false,
        cue: Cue::CrossbowShot,
    },
    // Прицельное: разброс меньше всех.
    DartWeapon {
        item: AIR_RIFLE_ID,
        range: 48,
        speed: 3.8,
        spread: 0.2,
        cooldown: 10,
        loading: Loading::Magazine,
        quiet: false,
        cue: Cue::RifleShot,
    },
];

/// Оружие для дротиков с таким id.
pub fn dart_weapon(item: &str) -> Option<&'static DartWeapon> {
    DART_WEAPONS.iter().find(|w| w.item == item)
}

/// Использование со временем у арбалета и ружья: держится, пока зажат ПКМ,
/// как у ванильного лука (`use_duration` пина: 72000).
pub const WEAPON_USE_TICKS: i32 = 72_000;
/// Натяжение арбалета, тиков: ванильные 1,25 с.
pub const CROSSBOW_CHARGE_TICKS: u32 = 25;
/// Магазин ружья.
pub const MAGAZINE_SIZE: usize = 4;
/// Выстрелов на один баллон CO₂.
pub const CANISTER_SHOTS: u32 = 12;
/// Пауза после зарядки магазина, тиков.
pub const RIFLE_RELOAD_COOLDOWN: i32 = 30;
/// Приближение прицела ружья (у подзорной трубы — 10) и замедление мыши в
/// прицеле: при четырёхкратном приближении полная чувствительность дёргает.
pub const RIFLE_ZOOM: f32 = 4.0;
pub const RIFLE_ZOOM_TURN: f32 = 0.4;

/// `use_duration` оружия: у арбалета и ружья — держится до отпускания, у
/// трубки своего нет (клик и выстрел). Отвечает по одному id: игра спрашивает
/// его каждый кадр, без игрока.
pub fn weapon_use_duration(item: &str) -> Option<i32> {
    match dart_weapon(item)?.loading {
        Loading::Each => None,
        Loading::Charge | Loading::Magazine => Some(WEAPON_USE_TICKS),
    }
}

/// Ключ в `custom_data` оружия: что заряжено и сколько газа (`weapon_text`).
pub const WEAPON_KEY: &str = "biolab_weapon";

/// Данные на стеке оружия: заряженные дротики по порядку выстрела (у
/// арбалета — один, у ружья — магазин) и газ ружья в выстрелах.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeaponState {
    pub load: Vec<&'static str>,
    pub gas: u32,
}

/// Данные оружия из строки `load=<дротик>,<дротик>;gas=<n>`. Незнакомый
/// дротик (убранный из мода) выпадает, битое число — 0: оружие не ломается,
/// а разряжается.
pub fn weapon_state(text: &str) -> WeaponState {
    let mut state = WeaponState::default();
    for part in text.split(';') {
        match part.split_once('=') {
            Some(("load", darts)) => {
                state.load = darts.split(',').filter_map(dart_projectile).take(MAGAZINE_SIZE).collect();
            }
            Some(("gas", n)) => state.gas = n.trim().parse().unwrap_or(0).min(CANISTER_SHOTS),
            _ => {}
        }
    }
    state
}

pub fn weapon_text(state: &WeaponState) -> String {
    format!("load={};gas={}", state.load.join(","), state.gas)
}

/// Что делает клик (ПКМ) оружием.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponClick {
    /// Выстрелить этим дротиком: трубка берёт его из инвентаря, арбалет —
    /// заряженный.
    Shoot(&'static str),
    /// Клик не забираем: игра начинает использование — натяжение арбалета
    /// или прицел ружья.
    Use,
    /// Магазин ружья пуст: зарядить из инвентаря.
    Reload,
    /// Нечем: дротиков нет ни в оружии, ни в инвентаре.
    NoDarts,
}

/// Решение клика по оружию, его данным, первому дротику инвентаря и
/// творчеству (там дротик не нужен: берётся первый из `DARTS`, как
/// ванильный лук в творчестве стреляет без стрел).
pub fn weapon_click(
    weapon: &DartWeapon,
    state: &WeaponState,
    first_dart: Option<&'static str>,
    creative: bool,
) -> WeaponClick {
    let dart = first_dart.or(creative.then_some(DARTS[0].0));
    match weapon.loading {
        Loading::Each => dart.map_or(WeaponClick::NoDarts, WeaponClick::Shoot),
        Loading::Charge => match (state.load.first(), dart) {
            (Some(loaded), _) => WeaponClick::Shoot(loaded),
            (None, Some(_)) => WeaponClick::Use,
            (None, None) => WeaponClick::NoDarts,
        },
        Loading::Magazine => match (state.load.is_empty(), dart) {
            (false, _) => WeaponClick::Use,
            (true, Some(_)) => WeaponClick::Reload,
            (true, None) => WeaponClick::NoDarts,
        },
    }
}

/// Начать ли использование (`living_entity_use_item_start`). Сервер начинает
/// его, только если клик не забран, а клиент предсказывает начало сам, не
/// спрашивая `native_use_item`, — поэтому отказ здесь тот же, что у клика:
/// иначе клиент рисовал бы натяжение заряженного арбалета, пока сервер уже
/// выстрелил. Обе стороны решают по одному стеку и инвентарю.
pub fn weapon_use_may_start(
    weapon: &DartWeapon,
    state: &WeaponState,
    first_dart: Option<&'static str>,
    creative: bool,
) -> bool {
    weapon_click(weapon, state, first_dart, creative) == WeaponClick::Use
}

/// Шаг натяжения арбалета.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChargeStep {
    /// Звук начала: ванильный арбалет играет его на 20 % натяжения.
    Start,
    /// Звук середины, на 50 %.
    Middle,
    /// Натянут: дротик заряжен.
    Loaded,
}

/// Что происходит на тике натяжения `held` (0 — первый тик зажатого ПКМ).
/// Тики идут подряд, поэтому шаг — ровно в своей точке.
pub fn crossbow_charge(held: u32) -> Option<ChargeStep> {
    let point = |share: f32| (CROSSBOW_CHARGE_TICKS as f32 * share).round() as u32;
    match held {
        h if h == point(0.2) => Some(ChargeStep::Start),
        h if h == point(0.5) => Some(ChargeStep::Middle),
        h if h == CROSSBOW_CHARGE_TICKS => Some(ChargeStep::Loaded),
        _ => None,
    }
}

/// Сколько тиков держали ПКМ: `release_using` и `on_use_tick` пина дают
/// оставшиеся тики `WEAPON_USE_TICKS`.
pub fn weapon_held(remaining: i32) -> u32 {
    u32::try_from(i64::from(WEAPON_USE_TICKS) - i64::from(remaining)).unwrap_or(0)
}

/// Зарядить магазин: из стопок дротиков `stacks` (слот, дротик, сколько —
/// в порядке поиска) берём, пока магазин не полон. Вернёт, сколько взять из
/// какого слота. В творчестве ничего не берём: магазин полнится первым
/// дротиком инвентаря или первым из `DARTS`.
pub fn reload_magazine(
    state: &mut WeaponState,
    stacks: &[(i32, &'static str, u32)],
    creative: bool,
) -> Vec<(i32, u32)> {
    let mut take = Vec::new();
    if creative {
        let dart = first_dart(stacks).unwrap_or(DARTS[0].0);
        while state.load.len() < MAGAZINE_SIZE {
            state.load.push(dart);
        }
        return take;
    }
    for &(slot, dart, count) in stacks {
        let mut taken = 0;
        while taken < count && state.load.len() < MAGAZINE_SIZE {
            state.load.push(dart);
            taken += 1;
        }
        if taken > 0 {
            take.push((slot, taken));
        }
    }
    take
}

/// Выстрел ружья.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RifleShot {
    /// Выстрелить этим дротиком; `connected` — для выстрела подключён новый
    /// баллон (полный уходит из инвентаря, остаётся пустой).
    Fired { dart: &'static str, connected: bool },
    /// Магазин пуст.
    Empty,
    /// Газа нет, и баллона в инвентаре тоже.
    NoGas,
}

/// Отпущен прицел ружья: первый дротик магазина и один выстрел газа. Газ
/// кончился — подключаем полный баллон, если он есть (`canister`). В
/// творчестве газ не тратится.
pub fn rifle_fire(state: &mut WeaponState, canister: bool, creative: bool) -> RifleShot {
    if state.load.is_empty() {
        return RifleShot::Empty;
    }
    let mut connected = false;
    if state.gas == 0 && !creative {
        if !canister {
            return RifleShot::NoGas;
        }
        state.gas = CANISTER_SHOTS;
        connected = true;
    }
    let dart = state.load.remove(0);
    if !creative {
        state.gas -= 1;
    }
    RifleShot::Fired { dart, connected }
}

/// Первый дротик в стопках инвентаря.
pub fn first_dart(stacks: &[(i32, &'static str, u32)]) -> Option<&'static str> {
    stacks.first().map(|(_, dart, _)| *dart)
}

/// Имя дротика для подсказок и сообщений: как на табло стола (оно же имя
/// предмета, тест сверяет с языковым файлом).
pub fn dart_title(dart: &str) -> &str {
    BENCH_RESULTS.iter().find(|(id, _)| *id == dart).map_or(dart, |(_, name)| name)
}

/// Полоска под иконкой (`item_bar`): у ружья — сколько дротиков в магазине
/// (красная, когда кончился газ), у заряженного арбалета — полная. None —
/// полоски нет. Ширина `0..=13`, как у ванильной.
pub fn weapon_bar(item: &str, state: &WeaponState) -> Option<(i32, u32)> {
    match dart_weapon(item)?.loading {
        Loading::Each => None,
        Loading::Charge => (!state.load.is_empty()).then_some((13, 0x96_7AC8)),
        Loading::Magazine => {
            let width = (state.load.len() * 13 / MAGAZINE_SIZE) as i32;
            Some((width, if state.gas == 0 { 0xD2_4B4B } else { 0x8C_B4D2 }))
        }
    }
}

/// Строки данных в подсказке оружия.
pub fn weapon_tooltip(item: &str, state: &WeaponState) -> Vec<String> {
    let Some(weapon) = dart_weapon(item) else {
        return Vec::new();
    };
    match weapon.loading {
        Loading::Each => Vec::new(),
        Loading::Charge => vec![match state.load.first() {
            Some(dart) => format!("Заряжен: {}", dart_title(dart)),
            None => "Не заряжен".to_string(),
        }],
        Loading::Magazine => {
            let gas = if state.gas == 0 {
                "Газ: нет, нужен баллон CO₂".to_string()
            } else {
                format!("Газ: {} из {CANISTER_SHOTS} выстрелов", state.gas)
            };
            vec![magazine_message(state), gas]
        }
    }
}

/// Сообщения оружия.
pub fn no_darts_message() -> &'static str {
    "Нет дротиков: положите их в инвентарь"
}

pub fn no_gas_message() -> &'static str {
    "Нет газа: нужен баллон CO₂, заправка — в ферментере"
}

pub fn magazine_message(state: &WeaponState) -> String {
    format!("Магазин: {} из {MAGAZINE_SIZE}", state.load.len())
}

pub fn canister_message() -> String {
    format!("Подключён баллон CO₂: {CANISTER_SHOTS} выстрелов")
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

/// Что шприц делает мобу: эффекты, длительность и уровень. Снотворных здесь
/// нет: их действие решает сила против массы (`sleeper_of_item`, GDD 4.20).
pub fn mob_injection(item: &str) -> Option<(&'static [&'static str], i32, i32)> {
    match item {
        "biolab:syringe_venom" => Some((&[TOXICOSIS_EFFECT], 400, 1)),
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

/// Что остаётся в руке после приёма: от шприца — пустой шприц, от отвара —
/// пустая бутылка.
pub fn after_use(item: &str) -> Option<&'static str> {
    if item == CALMING_BREW {
        return Some("minecraft:glass_bottle");
    }
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
    // «Препарат» — общее слово, поэтому без рода: «Укол: Яд → Корова».
    // Снотворные говорят своё (`sleep_message`): засыпает, вял или слабо.
    format!("Укол: {what} → {}", mob_title(mob))
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

// ---- Снотворные: сила против массы (GDD 4.20, M5) ----
//
// Раньше снотворное было одно, и седативное валило любого моба — от курицы
// до разорителя. Теперь у вещества сила 1–5, у моба масса 1–5, и исход укола
// решает их разница: спит, только вялый, ничего или передозировка. Крупного
// мутанта поймать — отдельная задача: вывести вещество сильнее.
//
// Здесь только числа и правила без типов игры. Исполняют их модуль игры
// `biolab_sleep` (`sleep.rs`: доза мобу, газовое облако) и хуки: шприц,
// отвар и тряпка — `inject.rs`, дротик — `dart_hit.rs`, граната —
// `grenade_hit.rs`, аэрозоль — `gas_use.rs`, задержка засыпания, облака и
// вялость — `sleep_tick.rs`, стол — `surgery.rs`.

/// Паралич кураре: моб не двигается и не нападает, но не спит и не лежит.
/// Поведение — на хуках эффектов API 56 (`effects.rs`), неподвижность —
/// модификатор скорости в `biolab-deep` (`mob_tick.rs`, как у седации).
pub const PARALYSIS_EFFECT: &str = "biolab:paralysis";

/// Что вещество делает с мобом, который ему по силам.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SleepKind {
    /// Сон: эффект `biolab:sedation`, моб ложится. Годится для стола.
    Sleep,
    /// Паралич: эффект `biolab:paralysis`. На носилки можно, на стол — нет.
    Paralysis,
    /// Пробуждающее: снимает сон, паралич и вялость, силы не нужно.
    Wake,
}

/// Снотворное вещество.
#[derive(Debug)]
pub struct Sleeper {
    pub id: &'static str,
    /// Название — то же, что у вещества в `DRUGS`.
    pub name: &'static str,
    /// Сила 1–5; у пробуждающего 0 — масса ему не важна.
    pub strength: u8,
    pub kind: SleepKind,
    /// Газ: действует облаком, у игрока без противогаза — вдох (`take_gas`).
    pub gas: bool,
    /// С какого перевеса силы над массой начинается передозировка. У всех 3,
    /// у хлороформа 2: узкое окно.
    pub overdose_from: u8,
    /// Передозировка убивает (сильный газ), а не только травит.
    pub lethal: bool,
    /// Задержка засыпания, тики: моб ещё пробегает своё, как зверь в поле.
    /// У сильных веществ дольше; у транквилизатора растёт с массой.
    pub delay: u64,
    pub delay_per_mass: u64,
    /// Сколько тиков держится сон или паралич.
    pub ticks: i32,
}

/// Линейка снотворных (GDD 4.20). Порядок — от слабого к сильному.
pub const SLEEPERS: [Sleeper; 8] = [
    // Отвар валерианы: мелочь спит, прочие вялые. Передозировки не бывает:
    // сила 1 не перевешивает массу 1 на три.
    Sleeper { id: "calming", name: "Успокоительное", strength: 1, kind: SleepKind::Sleep, gas: false, overdose_from: 3, lethal: false, delay: 20, delay_per_mass: 0, ticks: 400 },
    // Прежнее седативное из мака — теперь сила 2: зомби, скелет, свинья.
    Sleeper { id: "sedative", name: "Седативное", strength: 2, kind: SleepKind::Sleep, gas: false, overdose_from: 3, lethal: false, delay: 40, delay_per_mass: 0, ticks: 600 },
    Sleeper { id: "mandrake", name: "Мандрагора", strength: 3, kind: SleepKind::Sleep, gas: false, overdose_from: 3, lethal: false, delay: 60, delay_per_mass: 0, ticks: 800 },
    // Эфир валит быстро, но ненадолго.
    Sleeper { id: "ether", name: "Эфир", strength: 3, kind: SleepKind::Sleep, gas: true, overdose_from: 3, lethal: false, delay: 20, delay_per_mass: 0, ticks: 240 },
    // Хлороформ: быстро и сильно, но с массы + 2 уже смерть — курицу и
    // зомби им не усыпляют.
    Sleeper { id: "chloroform", name: "Хлороформ", strength: 4, kind: SleepKind::Sleep, gas: true, overdose_from: 2, lethal: true, delay: 20, delay_per_mass: 0, ticks: 600 },
    Sleeper { id: "curare", name: "Кураре", strength: 4, kind: SleepKind::Paralysis, gas: false, overdose_from: 3, lethal: false, delay: 40, delay_per_mass: 0, ticks: 600 },
    // Полевой дротик зверолова: до 8 секунд задержки у самых тяжёлых.
    Sleeper { id: "tranquilizer", name: "Транквилизатор", strength: 5, kind: SleepKind::Sleep, gas: false, overdose_from: 3, lethal: false, delay: 60, delay_per_mass: 20, ticks: 1200 },
    Sleeper { id: "wake", name: "Пробуждающее", strength: 0, kind: SleepKind::Wake, gas: false, overdose_from: 3, lethal: false, delay: 0, delay_per_mass: 0, ticks: 0 },
];

/// Вещество по id.
pub fn sleeper(id: &str) -> Option<&'static Sleeper> {
    SLEEPERS.iter().find(|s| s.id == id)
}

/// Отвар валерианы в бутылке: пьют сами или поят моба (ПКМ по мобу).
pub const CALMING_BREW: &str = "biolab:calming_brew";
/// Тряпка с хлороформом: ПКМ по мобу вплотную.
pub const CHLOROFORM_RAG: &str = "biolab:rag_chloroform";
/// «Вплотную» — от глаз игрока до моба, в блоках.
pub const RAG_REACH: f64 = 2.5;

/// Чем снотворное дают мобу в упор: шприц, бутылка, тряпка. Дротик едет
/// через свой шприц (`DARTS`), газы — облаком (`GAS_ITEMS`).
pub const SLEEP_ITEMS: [(&str, &str); 8] = [
    (CALMING_BREW, "calming"),
    ("biolab:syringe_calming", "calming"),
    ("biolab:syringe_sedative", "sedative"),
    ("biolab:syringe_mandrake", "mandrake"),
    (CHLOROFORM_RAG, "chloroform"),
    ("biolab:syringe_curare", "curare"),
    ("biolab:syringe_tranquilizer", "tranquilizer"),
    ("biolab:syringe_wake", "wake"),
];

/// Снотворное в предмете, которым колют, поят или зажимают нос.
pub fn sleeper_of_item(item: &str) -> Option<&'static Sleeper> {
    SLEEP_ITEMS.iter().find(|(i, _)| *i == item).and_then(|(_, id)| sleeper(id))
}

/// Действует ли предмет на моба по ПКМ (`inject.rs`): лекарство, яд,
/// мутаген или снотворное.
pub fn acts_on_mobs(item: &str) -> bool {
    mob_injection(item).is_some() || sleeper_of_item(item).is_some()
}

/// Как газ выходит наружу.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GasForm {
    /// Аэрозоль: ПКМ — облачко перед собой.
    Spray,
    /// Колба-граната: бросок, как снежок; облако там, где разбилась.
    Grenade,
}

/// Газы в предметах. Граната — ещё и тип снаряда с тем же именем
/// (`entity_type/<граната>.json`, копия снежка).
pub const GAS_ITEMS: [(&str, &str, GasForm); 4] = [
    ("biolab:spray_ether", "ether", GasForm::Spray),
    ("biolab:spray_chloroform", "chloroform", GasForm::Spray),
    ("biolab:grenade_ether", "ether", GasForm::Grenade),
    ("biolab:grenade_chloroform", "chloroform", GasForm::Grenade),
];

/// Газ в предмете и как он выходит.
pub fn gas_of_item(item: &str) -> Option<(&'static Sleeper, GasForm)> {
    GAS_ITEMS
        .iter()
        .find(|(i, _, _)| *i == item)
        .and_then(|(_, id, form)| Some((sleeper(id)?, *form)))
}

/// Граната по типу снаряда (`p.kind.id()` в хуке попадания).
pub fn grenade_projectile(kind: &str) -> Option<&'static Sleeper> {
    gas_of_item(kind).filter(|(_, form)| *form == GasForm::Grenade).map(|(s, _)| s)
}

// Масса моба. Считается по хитбоксу вида тем же путём, что длина
// операционного стола (`lying_length`, `LONG_BODIES`): хитбокс в плане
// квадратный и длины тела коровы не знает, поэтому глубина у длинных видов —
// длина тела. Объём «ширина × рост × глубина» в блоках³ раскладывается по
// порогам: курица 0,11, зомби 0,70, свинья 1,17, корова 1,89, паук 2,02,
// железный голем 5,3, разоритель 11,2, гигант 155.

/// Верхние границы объёма для масс 1–4; больше последней — масса 5.
pub const MASS_BULK: [f64; 4] = [0.45, 1.5, 5.0, 30.0];

/// Боссы тяжелее своего хитбокса: страж с визером в хитбоксе как скелет,
/// а ловить их — дело последнего вещества.
pub const MASS_OVERRIDES: [(&str, u8); 4] =
    [("warden", 5), ("wither", 5), ("ender_dragon", 5), ("elder_guardian", 5)];

/// Ген устойчивости к снотворному (GDD 4.19): масса +1 за силу.
pub const RESISTANCE_GENE: &str = "sedation_resistance";

/// Объём тела вида в блоках³.
pub fn body_bulk(kind: &str, width: f64, height: f64) -> f64 {
    let short = short_kind(kind);
    let depth = LONG_BODIES
        .iter()
        .find(|(k, _)| *k == short)
        .map_or(width, |(_, length)| length.max(width));
    width * height * depth
}

/// Масса вида 1–5 по хитбоксу.
pub fn species_mass(kind: &str, width: f64, height: f64) -> u8 {
    let short = short_kind(kind);
    if let Some((_, mass)) = MASS_OVERRIDES.iter().find(|(k, _)| *k == short) {
        return *mass;
    }
    let bulk = body_bulk(kind, width, height);
    if !bulk.is_finite() {
        return 1;
    }
    1 + MASS_BULK.iter().filter(|limit| bulk >= **limit).count() as u8
}

/// Масса моба с генами: гигантизм +1, карликовость −1 (сила гена не
/// важна — меняется весовая категория), устойчивость +1…3 по силе. Итог —
/// 1–5: сильнейшее вещество (5) берёт любого, иначе выведенного мутанта
/// было бы нечем поймать.
pub fn mob_mass(kind: &str, width: f64, height: f64, genes: &[(&str, i32)]) -> u8 {
    let has = |name: &str| genes.iter().filter(|(g, _)| *g == name).map(|(_, s)| *s).max().unwrap_or(0);
    let mut mass = i32::from(species_mass(kind, width, height));
    if has("gigantism") > 0 {
        mass += 1;
    }
    if has("dwarfism") > 0 {
        mass -= 1;
    }
    mass += has(RESISTANCE_GENE).clamp(0, 3);
    mass.clamp(1, 5) as u8
}

/// Чем кончилась доза.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SleepOutcome {
    /// Сила ≥ массы: моб засыпает (или парализован) после задержки.
    Takes,
    /// Сила = масса − 1: только вялый — замедлен, издалека не нападает.
    Drowsy,
    /// Сила меньше: эффекта нет.
    TooWeak,
    /// Сила ≥ масса + перевес: урон (токсикоз), и всё же засыпает.
    Overdose,
    /// Передозировка сильного газа: смерть.
    Fatal,
    /// Пробуждающее сняло сон.
    Woken,
}

/// Повторная доза того же вещества тому же мобу в этом окне слабее на 1:
/// привыкание, как у игрока (4.3).
pub const REPEAT_WINDOW: u64 = 6000;

/// Сила дозы с поправкой на повтор.
pub fn dose_power(s: &Sleeper, repeated: bool) -> u8 {
    s.strength.saturating_sub(u8::from(repeated))
}

/// Исход дозы вещества `s` мобу массы `mass`; `repeated` — то же вещество
/// уже действовало на него в последние 5 минут.
pub fn sleep_outcome(s: &Sleeper, mass: u8, repeated: bool) -> SleepOutcome {
    if s.kind == SleepKind::Wake {
        return SleepOutcome::Woken;
    }
    let power = i32::from(dose_power(s, repeated));
    let mass = i32::from(mass);
    if power >= mass + i32::from(s.overdose_from) {
        return if s.lethal { SleepOutcome::Fatal } else { SleepOutcome::Overdose };
    }
    if power >= mass {
        SleepOutcome::Takes
    } else if power == mass - 1 {
        SleepOutcome::Drowsy
    } else {
        SleepOutcome::TooWeak
    }
}

/// Через сколько тиков моб уснёт.
pub fn sleep_delay(s: &Sleeper, mass: u8) -> u64 {
    s.delay + s.delay_per_mass * u64::from(mass)
}

/// Передозировка без смерти: токсикоз (урон, GDD 4.7) на 10 секунд.
pub const OVERDOSE_TOXICOSIS: (i32, i32) = (200, 1);

/// Вялость: замедление на половину срока сна и не нападать издалека — цель
/// дальше этого расстояния моб бросает (`sleep_tick.rs`).
pub const DROWSY_REACH: f64 = 4.0;
pub const DROWSY_SLOWNESS: i32 = 1;

/// Что сказать, когда вещество не по зверю: подсказка игроку.
pub const TOO_WEAK: &str = "Слишком слабое для такого зверя";

/// Как доза дошла до моба: от этого — первое слово сообщения.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via {
    Injection,
    Drink,
    Rag,
    Dart,
    Gas,
}

fn via_word(via: Via) -> &'static str {
    match via {
        Via::Injection => "Укол",
        Via::Drink => "Отвар",
        Via::Rag => "Тряпка",
        Via::Dart => "Дротик",
        Via::Gas => "Газ",
    }
}

/// Как дали вещество в упор этим предметом.
pub fn via_of_item(item: &str) -> Via {
    match item {
        CALMING_BREW => Via::Drink,
        CHLOROFORM_RAG => Via::Rag,
        _ => Via::Injection,
    }
}

/// Сообщение в чат после дозы: что, кому и чем кончилось. `mob` — вид моба.
pub fn sleep_message(s: &Sleeper, via: Via, mob: &str, mass: u8, outcome: SleepOutcome, repeated: bool) -> String {
    let head = format!("{}: {} → {}", via_word(via), s.name, mob_title(mob));
    let power = dose_power(s, repeated);
    let mut text = match outcome {
        SleepOutcome::Takes if s.kind == SleepKind::Paralysis => format!(
            "{head}: обмякнет через {} с — паралич, но не сон",
            sleep_delay(s, mass).div_ceil(20)
        ),
        SleepOutcome::Takes => format!("{head}: уснёт через {} с", sleep_delay(s, mass).div_ceil(20)),
        SleepOutcome::Drowsy => format!("{head}: только вялый — нужно сильнее (сила {power}, масса {mass})"),
        SleepOutcome::TooWeak => format!("{TOO_WEAK}: {} — сила {power}, масса {mass}", s.name),
        SleepOutcome::Overdose => format!("{head}: передозировка — отравлен"),
        SleepOutcome::Fatal => format!("{head}: передозировка — не выдержал"),
        SleepOutcome::Woken => format!("{head}: просыпается"),
    };
    if repeated && outcome != SleepOutcome::Woken {
        text.push_str(" (повтор за 5 минут — слабее на 1)");
    }
    text
}

/// Отложенный сон: моб уснёт в тик `due`.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingSleep {
    pub mob: u32,
    pub dimension: String,
    pub sleeper: &'static str,
    pub due: u64,
}

/// Газовое облако: где, какого радиуса, до какого тика, кого уже задело.
#[derive(Clone, Debug, PartialEq)]
pub struct Cloud {
    pub dimension: String,
    pub at: [f64; 3],
    pub radius: f64,
    pub until: u64,
    pub sleeper: &'static str,
    /// Мобы, которым облако уже дало дозу: одно облако — одна доза.
    pub dosed: Vec<u32>,
    /// Игроки, которые уже вдохнули.
    pub players: Vec<u128>,
}

/// Граната: облако радиусом 3 на 5 секунд (GDD 4.20).
pub const GRENADE_RADIUS: f64 = 3.0;
pub const GRENADE_TICKS: u64 = 100;
/// Аэрозоль: облачко поменьше в паре блоков перед лицом.
pub const SPRAY_RADIUS: f64 = 2.0;
pub const SPRAY_TICKS: u64 = 60;
pub const SPRAY_REACH: f64 = 2.5;
/// Рост глаз игрока над ногами.
pub const EYE_HEIGHT: f64 = 1.62;
/// Как часто облако дышит частицами и проверяет, кто в нём, тики.
pub const CLOUD_EVERY: u64 = 5;

/// Сколько вещества вдыхает игрок без противогаза за одно облако. У гена
/// устойчивости к снотворному доза меньше на четверть за силу.
pub fn gas_dose(s: &Sleeper, resistance: i32) -> u32 {
    let base: u32 = match s.id {
        "ether" => 30,
        "chloroform" => 18,
        _ => 0,
    };
    base * (4 - resistance.clamp(0, 3) as u32) / 4
}

/// Что на голове спасает от газа. Своего противогаза пока нет — его место
/// занимает черепаший панцирь: ванильный «дыхательный» шлем.
pub const GAS_MASKS: [&str; 1] = ["minecraft:turtle_helmet"];

/// Защищён ли игрок с этим предметом на голове.
pub fn gas_protected(head: Option<&str>) -> bool {
    head.is_some_and(|item| GAS_MASKS.contains(&item))
}

/// Внутри ли облака точка (середина тела).
pub fn in_cloud(cloud: &Cloud, at: [f64; 3]) -> bool {
    let d2: f64 = (0..3).map(|i| (at[i] - cloud.at[i]).powi(2)).sum();
    d2 <= cloud.radius * cloud.radius
}

/// Куда ложится облачко аэрозоля: `SPRAY_REACH` блоков от глаз по взгляду.
/// Поворот — как в Minecraft: `yaw` 0 — на юг (+Z), 90 — на запад (−X),
/// `pitch` больше нуля — вниз.
pub fn spray_point(feet: [f64; 3], yaw: f32, pitch: f32) -> [f64; 3] {
    let (yaw, pitch) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    let look = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()];
    [
        feet[0] + look[0] * SPRAY_REACH,
        feet[1] + EYE_HEIGHT + look[1] * SPRAY_REACH,
        feet[2] + look[2] * SPRAY_REACH,
    ]
}

/// Огонь рядом с облаком эфира даёт вспышку (GDD 4.20): какие блоки — огонь.
pub const FIRE_BLOCKS: [&str; 10] = [
    "minecraft:fire",
    "minecraft:soul_fire",
    "minecraft:lava",
    "minecraft:campfire",
    "minecraft:soul_campfire",
    "minecraft:torch",
    "minecraft:wall_torch",
    "minecraft:soul_torch",
    "minecraft:soul_wall_torch",
    "minecraft:magma_block",
];

/// Горит ли газ этого вещества.
pub fn flammable(s: &Sleeper) -> bool {
    s.id == "ether"
}

/// Даёт ли блок вспышку. Костёр — только зажжённый (`lit`).
pub fn ignites(block: &str, lit: Option<&str>) -> bool {
    if block.ends_with("campfire") {
        return lit != Some("false");
    }
    FIRE_BLOCKS.contains(&block)
}

/// Вспышка эфира: урон огнём всем в облаке и поджог.
pub const FLASH_DAMAGE: f32 = 5.0;
pub const FLASH_FIRE_TICKS: i32 = 100;

/// Сколько тиков помним отложенный сон, если моб пропал (выгружен чанк).
pub const PENDING_STALE: u64 = 600;

/// Память снотворных между хуками: кто когда уснёт, кому что давали, кто
/// вял, чем усыплён и какие облака висят. Только память процесса: после
/// перезахода в мир сон держит сам эффект (он сохраняется с мобом), а
/// забытое «чем усыплён» стол заменяет седативным.
#[derive(Debug, Default)]
pub struct SleepBook {
    pending: Vec<PendingSleep>,
    /// Моб, вещество, тик дозы — для привыкания.
    recent: Vec<(u32, &'static str, u64)>,
    /// Вялые: моб и до какого тика.
    drowsy: Vec<(u32, u64)>,
    /// Чем усыплён моб: это вещество подновляет операционный стол.
    slept: Vec<(u32, &'static str)>,
    clouds: Vec<Cloud>,
}

impl SleepBook {
    /// Давали ли мобу это вещество в последние 5 минут. Часы после
    /// перезахода идут заново — разница по модулю, как у бросков дротиков.
    pub fn repeated(&self, mob: u32, sleeper: &str, now: u64) -> bool {
        self.recent
            .iter()
            .any(|(m, s, at)| *m == mob && *s == sleeper && at.abs_diff(now) < REPEAT_WINDOW)
    }

    /// Запомнить дозу (для привыкания).
    pub fn note_dose(&mut self, mob: u32, sleeper: &'static str, now: u64) {
        self.recent.retain(|(m, s, at)| !(*m == mob && *s == sleeper) && at.abs_diff(now) < REPEAT_WINDOW);
        self.recent.push((mob, sleeper, now));
    }

    /// Моб уснёт в тик `due`. Новая доза заменяет прежнюю отложенную.
    pub fn schedule(&mut self, mob: u32, dimension: &str, sleeper: &'static str, due: u64) {
        self.pending.retain(|p| p.mob != mob || p.dimension != dimension);
        self.pending.push(PendingSleep { mob, dimension: dimension.to_string(), sleeper, due });
    }

    /// Забрать сон, чей тик настал, для уровня `dimension`. Давние — тех, о
    /// ком не слышно (`PENDING_STALE`), — выбрасываем.
    pub fn take_due(&mut self, now: u64, dimension: &str) -> Vec<PendingSleep> {
        self.pending.retain(|p| p.due.abs_diff(now) <= PENDING_STALE + 400);
        let mut due = Vec::new();
        self.pending.retain(|p| {
            if p.dimension == dimension && p.due <= now {
                due.push(p.clone());
                false
            } else {
                true
            }
        });
        due
    }

    /// Ждёт ли моб сна.
    pub fn is_pending(&self, mob: u32) -> bool {
        self.pending.iter().any(|p| p.mob == mob)
    }

    pub fn make_drowsy(&mut self, mob: u32, until: u64) {
        self.drowsy.retain(|(m, _)| *m != mob);
        self.drowsy.push((mob, until));
    }

    /// Вялые сейчас; заодно забываем тех, у кого вялость прошла.
    pub fn drowsy_now(&mut self, now: u64) -> Vec<u32> {
        self.drowsy.retain(|(_, until)| *until > now && until.abs_diff(now) <= REPEAT_WINDOW);
        self.drowsy.iter().map(|(m, _)| *m).collect()
    }

    /// Моб уснул этим веществом.
    pub fn fell_asleep(&mut self, mob: u32, sleeper: &'static str) {
        self.slept.retain(|(m, _)| *m != mob);
        self.slept.push((mob, sleeper));
    }

    /// Чем усыплён моб; None — не знаем (уснул до перезахода или командой).
    pub fn slept_with(&self, mob: u32) -> Option<&'static str> {
        self.slept.iter().find(|(m, _)| *m == mob).map(|(_, s)| *s)
    }

    /// Пробуждающее: ни сна в очереди, ни вялости, ни памяти о сне.
    pub fn wake(&mut self, mob: u32) {
        self.pending.retain(|p| p.mob != mob);
        self.drowsy.retain(|(m, _)| *m != mob);
        self.slept.retain(|(m, _)| *m != mob);
    }

    /// Облаку `at`/`until` (облако узнаём по месту и сроку: других полей
    /// хук не меняет) — моб или игрок получил свою дозу.
    pub fn mark_cloud(&mut self, at: [f64; 3], until: u64, mob: Option<u32>, player: Option<u128>) {
        if let Some(cloud) = self.clouds.iter_mut().find(|c| c.at == at && c.until == until) {
            cloud.dosed.extend(mob);
            cloud.players.extend(player);
        }
    }

    /// Облако вспыхнуло и выгорело.
    pub fn remove_cloud(&mut self, at: [f64; 3], until: u64) {
        self.clouds.retain(|c| !(c.at == at && c.until == until));
    }

    /// Новое облако.
    pub fn add_cloud(&mut self, cloud: Cloud) {
        // Без предела облака от сотни гранат копились бы, пока не кончатся.
        if self.clouds.len() >= 64 {
            self.clouds.remove(0);
        }
        self.clouds.push(cloud);
    }

    /// Облака сейчас: истёкшие (и «из будущего» — остаток прошлого мира)
    /// уходят.
    pub fn clouds_now(&mut self, now: u64) -> &mut Vec<Cloud> {
        self.clouds.retain(|c| c.until > now && c.until - now <= GRENADE_TICKS.max(SPRAY_TICKS));
        &mut self.clouds
    }
}

/// Общая память снотворных: хуки зовутся из разных мест тика.
pub fn sleep_book() -> &'static std::sync::Mutex<SleepBook> {
    static BOOK: std::sync::OnceLock<std::sync::Mutex<SleepBook>> = std::sync::OnceLock::new();
    BOOK.get_or_init(Default::default)
}

/// Тик снотворных — раз за тик на уровень, сколько бы игроков там ни было
/// (их тики зовут хук по очереди). Уровень свой у каждого прохода: игрок в
/// Незере ведёт сон и облака Незера, игрок в обычном мире — свои.
pub fn sleep_pass_due(ticks: u64, dimension: &str) -> bool {
    static LAST: std::sync::Mutex<Vec<(String, u64)>> = std::sync::Mutex::new(Vec::new());
    let Ok(mut last) = LAST.lock() else {
        return false;
    };
    match last.iter_mut().find(|(d, _)| d == dimension) {
        Some((_, at)) if *at == ticks => false,
        Some((_, at)) => {
            *at = ticks;
            true
        }
        None => {
            last.push((dimension.to_string(), ticks));
            true
        }
    }
}

/// Удержание на столе (GDD 4.20): стол подновляет то вещество, которым моба
/// усыпили, а не любое. Подновление — не новая доза, привыкания не даёт.
/// Ответ — каким веществом подновить; None — не подновлять: вещество
/// слабее массы (на столе вживили устойчивость), пациент проснётся.
/// Неизвестное вещество (уснул до перезахода, лёг на стол до M5) — стол
/// держит, как раньше, седативным: будить старых пациентов незачем.
pub fn table_sleeper(slept_with: Option<&str>, mass: u8) -> Option<&'static Sleeper> {
    let Some(s) = slept_with.and_then(sleeper) else {
        return sleeper("sedative");
    };
    // Кураре и пробуждающее столу не годятся: держим сном, если хватает силы.
    if s.kind != SleepKind::Sleep {
        return sleeper("sedative").filter(|sed| sed.strength >= mass);
    }
    (s.strength >= mass).then_some(s)
}

/// Что сказать, когда стол перестал держать пациента.
pub fn table_wakes_message(s: &Sleeper, mob: &str, mass: u8) -> String {
    format!(
        "{} просыпается на столе: {} слабее (сила {}, масса {mass}) — нужно сильнее",
        mob_title(mob),
        s.name,
        s.strength
    )
}

/// Сколько тиков пациент, выгруженный с носилок на землю, ещё спит или
/// парализован: успеть отойти или снова уколоть.
pub const UNLOAD_TICKS: i32 = 200;

/// Почему пациента нельзя уложить на стол; None — можно.
pub fn table_refuses(p: &Patient) -> Option<String> {
    p.paralysed.then(|| {
        format!("{} парализован, но не спит: на стол — только спящего. Усыпите снотворным", mob_title(&p.kind))
    })
}

/// Вдох газа игроком: доза вещества `drug` сразу в кровь (через лёгкие —
/// как аэрозоль), привыкание растёт. Состояние сразу пишется на диск.
pub fn take_gas(world: &std::path::Path, uuid: u128, drug: &str, dose: u32) -> Option<PlayerState> {
    let index = drug_index(drug)?;
    let mut table = states().lock().ok()?;
    let state = table
        .entry((world.to_path_buf(), uuid))
        .or_insert_with(|| {
            std::fs::read_to_string(state_path(world, uuid))
                .map(|t| state_from_text(&t))
                .unwrap_or_default()
        });
    let s = &mut state.drugs[index];
    s.blood += dose * FORMS[FORM_SPRAY].bioavailability / 100;
    s.tolerance = (s.tolerance + TOLERANCE_PER_DOSE).min(TOLERANCE_MAX);
    let saved = *state;
    let path = state_path(world, uuid);
    drop(table);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, state_to_text(&saved));
    Some(saved)
}

/// Сообщение игроку, вдохнувшему газ.
pub fn gas_message(s: &Sleeper, state: &PlayerState) -> String {
    let Some(index) = drug_index(s.id) else {
        return format!("Вы вдохнули: {}", s.name);
    };
    let level = level_of(&DRUGS[index], &state.drugs[index]);
    format!("Вы вдохнули: {} — {}, без противогаза", s.name, level_text(level))
}

// ---- Генетика ----
//
// Ген — признак с силой 1–3. У мобов он меняет характеристики, вид и
// поведение (это делает `biolab-deep`), у игрока — даёт постоянный эффект и
// свою побочку. Сам ген живёт в предмете-фрагменте, в мобе — данными
// `biolab:genome` (attachment, M11; до M11 — меткой-модификатором), а у
// игрока — в состоянии мира и копией в данных игрока для окна.
//
// Имена генов, их цена и форма данных — общий контракт с `biolab-deep`:
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

pub const GENES: [GeneKind; 41] = [
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
    GeneKind { name: "polymelia", title: "Полимелия", cost: 3, about: "третья рука: бьёт чаще", player: Some("haste"), side: None },
    GeneKind { name: "dicephaly", title: "Дицефалия", cost: 3, about: "вторая голова: со спины не подкрасться", player: None, side: None },
    GeneKind { name: "horns", title: "Рогатость", cost: 2, about: "разбегается и таранит с отбрасыванием", player: None, side: None },
    GeneKind { name: "tail", title: "Хвост", cost: 2, about: "бьёт хвостом тех, кто сзади", player: None, side: None },
    GeneKind { name: "tentacles", title: "Щупальца", cost: 3, about: "хватает и подтягивает к себе", player: None, side: None },
    GeneKind { name: "polyopia", title: "Многоглазие", cost: 2, about: "видит невидимых, замечает издалека", player: Some("night_vision"), side: None },
    GeneKind { name: "tumor", title: "Опухоль", cost: 1, about: "здоровее, но медленнее", player: Some("health_boost"), side: Some("slowness") },
    GeneKind { name: "membranes", title: "Перепонки", cost: 2, about: "медленно падает и планирует", player: Some("slow_falling"), side: None },
    // M5 (GDD 4.20): снотворное берёт такого хуже — масса +1 за силу.
    GeneKind { name: "sedation_resistance", title: "Устойчивость к снотворному", cost: 2, about: "снотворное берёт хуже: масса +1 за силу", player: None, side: None },
];

/// Природный предел: сколько генов бывает у мутанта, родившегося в мире, и
/// какую сумму цен такой геном выдерживает. С GDD 4.19 это правило только
/// природы — броска при рождении моба (`roll` в `biolab-deep`) и скрещивания
/// на грядке. Лаборатория — укол фрагмента, клонирующий чан, генотерапия,
/// носилки, мутаген — вживляет сколько угодно генов, без стабильности.
pub const MAX_GENES: usize = 3;
pub const STABILITY: i32 = 6;

/// Гены, которые приживаются в человеке без ванильного эффекта: их действие —
/// механика хуков (`thorns.rs`, `infection.rs`), а не статус. Для подсказок
/// и журнала: что ген даёт человеку.
pub const PLAYER_MECHANICS: [(&str, &str); 5] = [
    ("dicephaly", "подсвечивает мобов вокруг"),
    ("horns", "удар отбрасывает сильнее"),
    ("tail", "хвост бьёт тех, кто ударил сзади"),
    ("tentacles", "подтягивают предметы с земли"),
    // Вдох газа меньше на четверть за силу (`gas_dose`, `sleep_tick.rs`).
    ("sedation_resistance", "газ снотворного действует слабее"),
];

/// Что ген-механика даёт человеку; None — у гена ванильный эффект или он
/// в человеке не приживается.
pub fn player_mechanic(name: &str) -> Option<&'static str> {
    PLAYER_MECHANICS.iter().find(|(gene, _)| *gene == name).map(|(_, what)| *what)
}

/// Приживается ли ген в человеке: ванильным эффектом или механикой.
pub fn takes_in_humans(kind: &GeneKind) -> bool {
    kind.player.is_some() || player_mechanic(kind.name).is_some()
}

/// Метки генов старого формата (до M11): модификатор дальности преследования
/// с нулевой прибавкой, чьё имя и есть ген. Мод их больше не ставит и не
/// читает — старые миры переносит в данные `biolab-deep` (`data.rs`,
/// `migrate`). Список остаётся для подсказок и сверки двух бандлов.
pub const GENE_MARKERS: [&str; 123] = [
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
    "biolab:gene/polymelia/1",
    "biolab:gene/polymelia/2",
    "biolab:gene/polymelia/3",
    "biolab:gene/dicephaly/1",
    "biolab:gene/dicephaly/2",
    "biolab:gene/dicephaly/3",
    "biolab:gene/horns/1",
    "biolab:gene/horns/2",
    "biolab:gene/horns/3",
    "biolab:gene/tail/1",
    "biolab:gene/tail/2",
    "biolab:gene/tail/3",
    "biolab:gene/tentacles/1",
    "biolab:gene/tentacles/2",
    "biolab:gene/tentacles/3",
    "biolab:gene/polyopia/1",
    "biolab:gene/polyopia/2",
    "biolab:gene/polyopia/3",
    "biolab:gene/tumor/1",
    "biolab:gene/tumor/2",
    "biolab:gene/tumor/3",
    "biolab:gene/membranes/1",
    "biolab:gene/membranes/2",
    "biolab:gene/membranes/3",
    "biolab:gene/sedation_resistance/1",
    "biolab:gene/sedation_resistance/2",
    "biolab:gene/sedation_resistance/3",
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

// ---- Данные на сущностях (attachments, M11) ----
//
// Геном моба, состояние операционной и гены игрока хранятся своими данными
// мода на носителе (`docs/native-mods.md` пина, «Server attachments»), а не
// метками-модификаторами. Типы объявлены в модуле игры
// `crate::game::biolab_core_data` (`data.rs`): `Host` есть только у `Game`.
// Здесь — чистая часть контракта. Тот же ключ и тот же тип значения
// объявляет `biolab-deep` (`crate::game::biolab_deep_data`): ключ — пара
// «пространство имён, имя», и значение, прочитанное одним бандлом, второй
// видит как своё, только если тип совпадает (`String`, `u8`). Поэтому
// ключи, поля и форма строки здесь и в `biolab-deep` обязаны совпадать — это
// сверяет `biolab-deep/tests/modules.rs`.

/// Пространство имён наших данных.
pub const DATA_NAMESPACE: &str = "biolab";
/// Геном моба (`Holder::Entity`, `String`, с `sync`).
pub const GENOME_DATA: &str = "genome";
/// Операционная моба (`Holder::Entity`, `u8`, с `sync`).
pub const SURGERY_DATA: &str = "surgery";
/// «Бросок мутации уже был» (`Holder::Entity`, `bool`). Ставит `biolab-deep`.
pub const CHECKED_DATA: &str = "mutation_checked";
/// Гены игрока (`Holder::Player`, `String`, с `sync` и `copy_on_death`).
pub const PLAYER_GENES_DATA: &str = "player_genes";
/// Поле строки генов в сохранённой форме: `{genes: "chitin:2,horns:1"}`.
pub const GENES_FIELD: &str = "genes";
/// Поле отметки броска: `{checked: 1b}`.
pub const CHECKED_FIELD: &str = "checked";
/// Поля операционной: `{flayed: 5b, table: 1b, asleep: 0b}`.
pub const FLAYED_FIELD: &str = "flayed";
pub const TABLE_FIELD: &str = "table";
pub const ASLEEP_FIELD: &str = "asleep";
/// Биты операционной в значении: нижние четыре — срезанные конечности (как
/// `flayed_mask`), потом «на столе» и «спит». «Спит» ставит `biolab-deep` по
/// снотворному, мы его только сохраняем при записи.
pub const FLAYED_BITS: u8 = 0b1111;
pub const TABLE_BIT: u8 = 1 << 4;
pub const ASLEEP_BIT: u8 = 1 << 5;

/// Геном строкой данных: гены в порядке `GENES`, у каждого — сильнейшая из
/// названных сил; незнакомое и вне 1–3 отброшено. Один геном — одна строка:
/// «изменилось ли» решается сравнением строк, лишних записей (и рассылки
/// клиентам) нет.
pub fn genome_data(genes: &[(&str, i32)]) -> String {
    let mut best = [0i32; GENES.len()];
    for (name, strength) in genes {
        if let Some(i) = GENES.iter().position(|g| g.name == *name)
            && (1..=3).contains(strength)
        {
            best[i] = best[i].max(*strength);
        }
    }
    best.iter()
        .enumerate()
        .filter(|(_, s)| **s > 0)
        .map(|(i, s)| gene_text(GENES[i].name, *s))
        .collect::<Vec<_>>()
        .join(",")
}

/// Гены из строки данных, в порядке `GENES`. Незнакомое пропускаем: мир от
/// новой версии мода не должен ломать старую.
pub fn genes_of_data(text: &str) -> Vec<(&'static str, i32)> {
    let mut best = [0i32; GENES.len()];
    for (name, strength) in genome_from_text(text) {
        if let Some(i) = GENES.iter().position(|g| g.name == name) {
            best[i] = best[i].max(strength);
        }
    }
    best.iter()
        .enumerate()
        .filter(|(_, s)| **s > 0)
        .map(|(i, s)| (GENES[i].name, *s))
        .collect()
}

/// Операционная полями сохранённой формы.
pub fn surgery_fields(bits: u8) -> [(&'static str, i8); 3] {
    [
        (FLAYED_FIELD, (bits & FLAYED_BITS) as i8),
        (TABLE_FIELD, i8::from(bits & TABLE_BIT != 0)),
        (ASLEEP_FIELD, i8::from(bits & ASLEEP_BIT != 0)),
    ]
}

/// Операционная из полей; нет поля — ноль.
pub fn surgery_from_fields(field: impl Fn(&str) -> Option<i8>) -> u8 {
    let flag = |name: &str, bit: u8| if field(name).unwrap_or(0) != 0 { bit } else { 0 };
    (field(FLAYED_FIELD).unwrap_or(0) as u8 & FLAYED_BITS)
        | flag(TABLE_FIELD, TABLE_BIT)
        | flag(ASLEEP_FIELD, ASLEEP_BIT)
}

/// Биты операционной: стол и маска срезов. «Спит» — из `keep` (его ставит
/// `biolab-deep`, и запись стола или среза его не сбрасывает).
pub fn surgery_bits(on_table: bool, flayed: u8, keep: u8) -> u8 {
    (flayed & FLAYED_BITS) | if on_table { TABLE_BIT } else { 0 } | (keep & ASLEEP_BIT)
}

/// Радиус кислотной ауры мутанта; None — желёз нет. Тот же расчёт, что
/// `aura_radius` по метке и `mutants::acid_aura` в `biolab-deep`.
pub fn acid_radius(genes: &[(&'static str, i32)]) -> Option<f64> {
    genes
        .iter()
        .find(|(name, _)| *name == "acid_glands")
        .map(|(_, strength)| 2.0 + f64::from(*strength))
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
        // Цена — для природного броска: в лаборатории предела нет (GDD 4.19).
        format!("Цена: {} (природный мутант — до {STABILITY})", kind.cost * strength),
    ];
    lines.push(match (kind.player, kind.side, player_mechanic(kind.name)) {
        (None, _, Some(what)) => format!("Человеку: {what}"),
        (None, _, None) => "В человеке не приживается".to_string(),
        (Some(_), Some(side), _) => format!("Человеку: с побочкой ({side})"),
        (Some(_), None, _) => "Человеку: без побочек".to_string(),
    });
    lines
}

/// Какой ген достанется из образца. Детерминированно от зерна, чтобы прогон
/// повторялся: зерно даёт машина (позиция и тик).
pub fn roll_gene(seed: u64) -> (&'static str, i32) {
    let kind = &GENES[(seed % GENES.len() as u64) as usize];
    // Сила 1–3 у любого гена: раньше дорогой ген не бывал сильным (его
    // некуда было бы вживить), но с GDD 4.19 у лаборатории предела нет.
    let strength = 1 + ((seed / 7) % 3) as i32;
    (kind.name, strength)
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
    // Предел стабильности — у природных мутантов; геном сверх него — дело
    // рук лаборатории, и микроскоп так и пишет.
    lines.push(if total > STABILITY || genes.len() > MAX_GENES {
        format!("Цена генов: {total} — сверх природы ({STABILITY}), лабораторный")
    } else {
        format!("Цена генов: {total} из природных {STABILITY}")
    });
    lines
}

/// Чем закончилась попытка вживить ген.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Splice {
    /// Ген принят.
    Done,
    /// Ген в человеке не приживается (у мобов работает).
    NotForHumans,
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

/// Генотерапия: вживить себе ген. Лаборатория: ни числа генов, ни
/// стабильности не проверяем (GDD 4.19) — только приживается ли ген в
/// человеке и не слабее ли он уже вживлённого.
pub fn splice_player(state: &mut PlayerState, name: &str, strength: i32) -> Splice {
    let Some(index) = GENES.iter().position(|g| g.name == name) else {
        return Splice::NotForHumans;
    };
    let kind = &GENES[index];
    if !takes_in_humans(kind) {
        return Splice::NotForHumans;
    }
    let have = i32::from(state.genes[index]);
    if have >= strength {
        return Splice::Already;
    }
    state.genes[index] = strength.clamp(1, 3) as u8;
    Splice::Done
}

/// Сила гена у игрока; 0 — гена нет.
pub fn player_gene(state: &PlayerState, name: &str) -> i32 {
    GENES
        .iter()
        .position(|g| g.name == name)
        .map_or(0, |i| i32::from(state.genes[i]))
}

/// Сила гена у игрока этого мира, без тика: для хуков, которым состояние
/// нужно мимоходом (удар по мобу). 0 — гена нет или игрок ещё не загружен.
pub fn world_player_gene(world: &std::path::Path, uuid: u128, name: &str) -> i32 {
    states()
        .lock()
        .ok()
        .and_then(|table| table.get(&(world.to_path_buf(), uuid)).map(|s| player_gene(s, name)))
        .unwrap_or(0)
}

// ---- Гены игроков — клиенту, для наростов (GDD 4.19, этап «игрок») ----
//
// Гены игрока живут в состоянии мира на сервере (`PlayerState.genes`), а
// наросты рисует окно. С M11 своего пакета (`biolab_core:genes`) больше нет:
// раз в секунду гены кладутся в данные игрока `biolab:player_genes`
// (`infection.rs` → `biolab_core_data::set_player_genes`, строка
// `genome_data`), а данные с `sync` сами едут этому игроку и всем, кто его
// видит, и переживают смерть (`copy_on_death`). Окно раскладывает их по
// моделям игроков в `biolab-deep` (`view_sync.rs`).

/// Сообщение о генотерапии.
pub fn splice_message(result: Splice, name: &str, strength: i32) -> String {
    let title = gene(name).map_or("Ген", |k| k.title);
    match result {
        Splice::Done => format!("{title} вживлён, сила {strength}"),
        Splice::NotForHumans => format!("{title} в человеке не приживается"),
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

/// Примет ли моб ещё один ген уколом фрагмента. Лаборатория: числа генов и
/// стабильности не проверяем (GDD 4.19), мобу годится любой ген — и тот, что
/// в человеке не приживается. Отказ — только незнакомый ген или такой же уже
/// есть, и не слабее.
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
    Splice::Done
}

/// Уживётся ли ген в геноме по природным правилам: не больше `MAX_GENES`
/// генов и сумма цен не выше `STABILITY`. Ими живёт скрещивание на грядке —
/// это опыление, а не вживление, и линия набирает гены, как природа.
pub fn fits_naturally(have: &[(&'static str, i32)], name: &str, strength: i32) -> bool {
    let Some(kind) = gene(name) else {
        return false;
    };
    let current = have
        .iter()
        .find(|(id, _)| *id == kind.name)
        .map(|(_, s)| *s)
        .unwrap_or(0);
    if current >= strength {
        return false;
    }
    if current == 0 && have.len() >= MAX_GENES {
        return false;
    }
    let total: i32 = have
        .iter()
        .filter(|(id, _)| *id != kind.name)
        .filter_map(|(id, s)| Some(gene(id)?.cost * s))
        .sum();
    total + kind.cost * strength <= STABILITY
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

/// Радиус кислотной ауры по метке гена: железы жгут всех вокруг.
/// Тот же расчёт, что в `biolab-deep` (`mutants::acid_aura`).
pub fn aura_radius(marker: &str) -> Option<f64> {
    match gene_of_marker(marker) {
        Some(("acid_glands", strength)) => Some(2.0 + f64::from(strength)),
        _ => None,
    }
}

/// Что сказать игроку, когда с мутанта срезали ткань (`cut_message`).
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
        if !takes_in_humans(kind) {
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
        if !fits_naturally(&result, name, *strength) {
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

/// Подпись урожая: «ГМ-урожай: <гены>», по схеме «<предмет>: <данные>».
pub fn crop_label(genes: &[(&'static str, i32)]) -> String {
    if genes.is_empty() {
        return "ГМ-урожай: без генома".to_string();
    }
    let names: Vec<&str> = genes
        .iter()
        .filter_map(|(name, _)| Some(gene(name)?.title))
        .collect();
    format!("ГМ-урожай: {}", names.join(", "))
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

/// Подпись культуры: «Культура: З2 И3 Т2 С1» — первые буквы характеристик и
/// их значения, чтобы две линии различались прямо в инвентаре.
pub fn culture_label(strain: &Strain) -> String {
    let parts: Vec<String> = STRAIN_NAMES
        .iter()
        .zip(strain.values())
        .map(|(name, value)| format!("{}{value}", name.chars().next().unwrap_or('?')))
        .collect();
    format!("Культура: {}", parts.join(" "))
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

// ---- Уродства (GDD 4.19): механика генов ----
//
// Ванильных эффектов на эти гены нет, поэтому у моба их делает тик мира
// (`behaviour.rs`), а у игрока — тик игрока (`infection.rs`) и удар по мобу
// (`thorns.rs`). Здесь только числа и геометрия, чтобы их можно было
// проверить тестами. Все периоды — кратные `BEHAVIOUR_EVERY`: тик мира
// приходит раз в 10 тиков.

/// Ближний бой: на таком расстоянии (по горизонтали) рука, рог или хвост
/// достают игрока.
pub const MELEE_REACH: f64 = 2.5;

/// Полимелия у моба: как часто третья рука бьёт сама, в тиках. Цели
/// ближнего боя (`MeleeAttack`) закрыты внутри селектора целей, и их
/// интервал снаружи не поменять, поэтому «бьёт чаще» — это отдельный удар
/// третьей руки поверх обычных: раз в 2, 1,5 и 1 секунду.
pub fn extra_arm_period(strength: i32) -> u64 {
    match strength.clamp(1, 3) {
        1 => 40,
        2 => 30,
        _ => 20,
    }
}

/// Удар третьей руки: половина урона самого моба, но не меньше единицы.
pub fn extra_arm_damage(attack: f32) -> f32 {
    (attack * 0.5).max(1.0)
}

/// Рогатость у моба: с какого расстояния он берёт разбег (ускорение) и как
/// часто бьёт рогами вплотную.
pub const RAM_RUN: (f64, f64) = (4.0, 12.0);
pub const RAM_EVERY: u64 = 40;

/// Удар рогами: урон и добавочное отбрасывание поверх обычного.
pub fn ram_damage(strength: i32) -> f32 {
    1.0 + strength.clamp(1, 3) as f32
}
pub fn ram_push(strength: i32) -> f64 {
    0.4 + 0.3 * f64::from(strength.clamp(1, 3))
}

/// Хвост у моба: как часто он хлещет того, кто стоит сзади, урон и толчок.
pub const TAIL_EVERY: u64 = 20;
pub fn tail_damage(strength: i32) -> f32 {
    1.0 + strength.clamp(1, 3) as f32
}
pub fn tail_push(strength: i32) -> f64 {
    0.3 + 0.1 * f64::from(strength.clamp(1, 3))
}

/// Щупальца у моба: докуда достают и с какой силой подтягивают цель.
pub const GRAB_EVERY: u64 = 20;
pub fn grab_reach(strength: i32) -> f64 {
    3.0 + 1.5 * f64::from(strength.clamp(1, 3))
}
pub fn grab_pull(strength: i32) -> f64 {
    0.3 + 0.15 * f64::from(strength.clamp(1, 3))
}

/// Дицефалия у моба: в этом радиусе вторая голова замечает игрока, откуда бы
/// он ни подкрадывался (сзади, крадучись, за углом).
pub fn watch_radius(strength: i32) -> f64 {
    4.0 + 2.0 * f64::from(strength.clamp(1, 3))
}

/// Многоглазие у моба: на таком расстоянии он замечает игрока, даже
/// невидимого.
pub fn far_sight_radius(strength: i32) -> f64 {
    16.0 + 8.0 * f64::from(strength.clamp(1, 3))
}

/// Стоит ли `other` за спиной того, кто стоит в `at` и смотрит по `yaw`.
/// Углы — как у игры: градусы, 0 — на юг (+Z), 90 — на запад (−X). Спина —
/// всё, что дальше ~110° от взгляда: бок не считается.
pub fn behind(yaw: f32, at: [f64; 2], other: [f64; 2]) -> bool {
    let rad = f64::from(yaw).to_radians();
    let (fx, fz) = (-rad.sin(), rad.cos());
    let (dx, dz) = (other[0] - at[0], other[1] - at[1]);
    let len = (dx * dx + dz * dz).sqrt();
    if len < 1e-6 {
        return false;
    }
    (fx * dx + fz * dz) / len < -0.35
}

/// Дицефалия у игрока: в этом радиусе мобы подсвечиваются.
pub fn glow_radius(strength: i32) -> f64 {
    6.0 + 4.0 * f64::from(strength.clamp(1, 3))
}

/// Рогатость у игрока: добавочное отбрасывание моба при ударе.
pub fn player_ram_push(strength: i32) -> f64 {
    0.3 + 0.2 * f64::from(strength.clamp(1, 3))
}

/// Хвост у игрока: докуда достаёт и сколько бьёт ударившего сзади.
pub const PLAYER_TAIL_REACH: f64 = 3.0;
pub fn player_tail_damage(strength: i32) -> f32 {
    1.0 + strength.clamp(1, 3) as f32
}

/// Щупальца у игрока: радиус, в котором они тянут предметы с земли.
pub fn magnet_radius(strength: i32) -> f64 {
    3.0 + 2.0 * f64::from(strength.clamp(1, 3))
}

/// Скорость, с которой щупальца тянут предмет из `item` к игроку в `player`;
/// None — предмет вне досягаемости или уже у ног (подбирает ванилла).
pub fn magnet_velocity(item: [f64; 3], player: [f64; 3], strength: i32) -> Option<[f64; 3]> {
    if strength <= 0 {
        return None;
    }
    let d = [player[0] - item[0], player[1] + 0.5 - item[1], player[2] - item[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if len < 1.0 || len > magnet_radius(strength) {
        return None;
    }
    let speed = 0.15 + 0.05 * f64::from(strength.clamp(1, 3));
    Some([d[0] / len * speed, d[1] / len * speed, d[2] / len * speed])
}

/// Последнее `hurt_time` игроков: по скачку вверх видно новый удар.
fn hurt_times() -> &'static std::sync::Mutex<std::collections::HashMap<u128, i32>> {
    static TIMES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u128, i32>>> =
        std::sync::OnceLock::new();
    TIMES.get_or_init(Default::default)
}

/// Новый ли это удар: `hurt_time` сбрасывается на 10 при уроне и тает по
/// одному за тик, так что рост значит свежий удар.
pub fn fresh_hit(previous: i32, now: i32) -> bool {
    now > previous
}

/// Запомнить `hurt_time` игрока; true — его только что ударили.
pub fn note_hurt_time(uuid: u128, now: i32) -> bool {
    let Ok(mut table) = hurt_times().lock() else {
        return false;
    };
    let previous = table.insert(uuid, now).unwrap_or(0);
    fresh_hit(previous, now)
}

/// День ли сейчас: светобоязливые мутанты в это время не охотятся.
/// Сутки в игре — 24000 тиков, день начинается на нуле и кончается на 12000.
pub fn daylight(day_time: i64) -> bool {
    (0..12000).contains(&day_time.rem_euclid(24000))
}

/// Что ген делает, кроме чисел. Та же таблица, что в `biolab-deep`: обе
/// собираются из одного описания, а тест ниже следит, чтобы в core не
/// осталось гена без строки поведения.
pub const GENE_BEHAVIOUR: [&[&str]; 41] = [
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
    &["extra_arm"],
    &["watchful"],
    &["ram"],
    &["tail"],
    &["grab"],
    &["far_sight"],
    &[],
    &["glide"],
    // Устойчивость к снотворному — только масса (`mob_mass`), поведения нет.
    &[],
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
        // Колба хранит формулу: знакомое вещество — по имени, незнакомое — формулой.
        assert_eq!(flask_label("H2O", 250), "Колба: вода, 250 мл");
        assert_eq!(flask_label("CFe2", 10), "Колба: CFe2, 10 мл");
    }

    /// Папка `content/` мода.
    fn content_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
    }

    /// Предметы мода — по определениям в `assets/biolab/items/`.
    fn item_ids() -> Vec<String> {
        let mut ids: Vec<String> = std::fs::read_dir(content_dir().join("assets/biolab/items"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .map(|path| format!("biolab:{}", path.file_stem().unwrap().to_string_lossy()))
            .collect();
        ids.sort();
        ids
    }

    fn lang(name: &str) -> Json {
        let path = content_dir().join(format!("assets/biolab/lang/{name}.json"));
        Json::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// Имя предмета: у предмета-блока — имя блока.
    fn item_name(lang: &Json, id: &str) -> Option<String> {
        let short = id.trim_start_matches("biolab:");
        lang.get(&format!("item.biolab.{short}"))
            .or_else(|| lang.get(&format!("block.biolab.{short}")))
            .and_then(Json::str)
            .map(str::to_string)
    }

    #[test]
    fn every_item_has_a_hint_or_is_skipped_on_purpose() {
        let items = item_ids();
        assert!(items.len() > 50, "предметы не нашлись: {items:?}");
        for id in &items {
            let hinted = !item_hint(id).is_empty();
            let skipped = HINTS_SKIPPED.contains(&id.as_str());
            assert!(hinted != skipped, "{id}: подсказка должна быть либо в таблице, либо в HINTS_SKIPPED");
        }
        let mut table: Vec<&str> = ITEM_HINTS.iter().map(|(id, _)| *id).collect();
        for id in &table {
            assert!(items.iter().any(|item| item == id), "{id}: подсказка есть, а предмета нет");
            assert!(element(id).is_none(), "{id}: элементам подсказку собирает item_hint");
        }
        let before = table.len();
        table.sort();
        table.dedup();
        assert_eq!(table.len(), before, "предмет в таблице подсказок дважды");
        assert!(item_hint("minecraft:stick").is_empty());
    }

    #[test]
    fn hints_are_written_in_one_style() {
        for id in item_ids() {
            let lines = item_hint(&id);
            if lines.is_empty() {
                continue;
            }
            assert!(lines.len() <= 2, "{id}: подсказка длиннее двух строк");
            for line in &lines {
                let first = line.chars().next().expect("пустая строка подсказки");
                assert!(first.is_uppercase() || first.is_ascii_digit(), "{id}: «{line}» — с заглавной");
                assert!(!line.ends_with('.'), "{id}: «{line}» — без точки в конце");
                assert!(line.chars().count() <= 48, "{id}: «{line}» не влезет в подсказку");
                assert_eq!(line.trim(), line, "{id}: лишние пробелы");
            }
        }
        assert_eq!(item_hint("biolab:element_iron"), vec!["Символ Fe: сырьё для синтезатора"]);
    }

    #[test]
    fn hints_tell_what_the_code_does() {
        let hint = |id: &str| item_hint(id).join(" / ");
        // Препарат принимают правым кликом (`remedy.rs`).
        for med in &MEDICINES {
            assert!(hint(med.item).contains("ПКМ"), "{}: {}", med.item, hint(med.item));
        }
        // Шприц, который действует на моба (`inject.rs`), говорит об этом, остальные — нет.
        for item in item_ids() {
            if !item.starts_with("biolab:syringe_") {
                continue;
            }
            let on_mob = acts_on_mobs(&item);
            assert_eq!(hint(&item).contains("по мобу"), on_mob, "{item}: {}", hint(&item));
        }
        // Дротик бросают, и подсказка называет дальность броска рукой.
        for (dart, _) in &DARTS {
            assert!(hint(dart).contains("бросить"), "{dart}");
            assert!(hint(dart).contains(&format!("до {} блоков", DART_THROW_RANGE as i32)), "{dart}");
        }
        // Ведро разливает лужу по блоку (`place_fluid.rs`).
        // Вылитое ведро пустеет (`emptied_bucket`), и подсказка это говорит.
        for fluid in &FLUIDS {
            assert!(hint(fluid.bucket).contains("ПКМ по блоку"), "{}", fluid.bucket);
            assert!(hint(fluid.bucket).contains("пустым"), "{}", fluid.bucket);
        }
        // Чертёж в синтезаторе не тратится (`synth_spent_slots`).
        assert!(hint(BLUEPRINT_ID).contains("не тратится"));
        // Фрагмент: по мобу (`inject.rs`) и по себе (`remedy.rs`).
        assert!(hint(FRAGMENT_ID).contains("по мобу") && hint(FRAGMENT_ID).contains("по себе"));
        assert!(hint(MICROSCOPE_ID).contains("по мобу"));
        assert!(hint(REACTOR).contains(&format!("{OVERHEAT}°")));
        // Экстракт: «Пресс» в подсказке — ровно когда пресс его принимает.
        let mut pressed = Vec::new();
        for entry in std::fs::read_dir(content_dir().join("data/biolab/recipe")).unwrap() {
            let json = Json::parse(&std::fs::read_to_string(entry.unwrap().path()).unwrap()).unwrap();
            if json.get("type").and_then(Json::str) == Some("biolab:press")
                && let Some(input) = json.get("ingredient").and_then(Json::str)
            {
                pressed.push(input.to_string());
            }
        }
        for item in item_ids().iter().filter(|i| i.starts_with("biolab:extract_")) {
            assert_eq!(hint(item).contains("Пресс"), pressed.contains(item), "{item}: {}", hint(item));
        }
    }

    #[test]
    fn every_item_is_named_in_both_languages() {
        let (ru, en) = (lang("ru_ru"), lang("en_us"));
        for id in item_ids() {
            for (name, lang) in [("ru_ru", &ru), ("en_us", &en)] {
                let named = item_name(lang, &id).is_some_and(|n| !n.trim().is_empty());
                assert!(named, "{id}: нет имени в {name}");
            }
        }
        let keys = |lang: &Json| -> Vec<String> {
            match lang {
                Json::Obj(fields) => {
                    let mut keys: Vec<String> = fields.iter().map(|(k, _)| k.clone()).collect();
                    keys.sort();
                    keys
                }
                _ => panic!("языковой файл — не объект"),
            }
        };
        assert_eq!(keys(&ru), keys(&en), "русский и английский разошлись по ключам");
    }

    #[test]
    fn names_follow_the_scheme() {
        let ru = lang("ru_ru");
        let name = |id: &str| item_name(&ru, id).unwrap_or_default();
        // Препарат — «<вещество>, <форма>».
        for med in &MEDICINES {
            let want = format!("{}, {}", DRUGS[med.drug].name, FORMS[med.form].name);
            assert_eq!(name(med.item), want, "{}", med.item);
        }
        for (dart, syringe) in &DARTS {
            let drug = DRUGS[medicine(syringe).expect("дротик снаряжён препаратом").drug].name;
            assert_eq!(name(dart), format!("{drug}, дротик"));
        }
        // Экстракт — «Экстракт <чего>».
        for item in item_ids().iter().filter(|i| i.starts_with("biolab:extract_")) {
            assert!(name(item).starts_with("Экстракт "), "{item}: {}", name(item));
        }
        // Подпись предмета с данными начинается с его имени.
        let gene = gene_text(GENES[0].name, 2);
        let labelled = [
            (FLASK_ID, flask_label("H2O", 250)),
            (FRAGMENT_ID, fragment_label(&gene)),
            (CULTURE_ID, culture_label(&WILD)),
            (GM_CROP, crop_label(&[(GENES[0].name, 1)])),
            (BLUEPRINT_ID, blueprint_label("H2O", Some("вода"))),
            (STRETCHER_ID, stretcher_label("kind=minecraft:zombie;health=20").expect("пациент есть")),
        ];
        for (id, label) in labelled {
            assert!(label.starts_with(&format!("{}: ", name(id))), "{id}: «{label}»");
        }
    }

    #[test]
    fn data_items_are_labelled_from_their_data() {
        let gene = gene_text(GENES[0].name, 2);
        let fragment = item_label(FRAGMENT_ID, |key| (key == GENE_KEY).then(|| gene.clone()));
        assert_eq!(fragment, Some(fragment_label(&gene)));
        let culture = item_label(CULTURE_ID, |key| (key == STRAIN_KEY).then(|| "2,3,2,1".to_string()));
        assert_eq!(culture.as_deref(), Some("Культура: З2 И3 Т2 С1"));
        // Битый штамм и нет данных — подписи нет, остаётся имя предмета.
        assert_eq!(item_label(CULTURE_ID, |_| Some("9,9".to_string())), None);
        assert_eq!(item_label(GM_CROP, |_| None), None);
        assert_eq!(
            item_label(GM_CROP, |key| (key == PLOT_GENOME_KEY).then(|| "thermal:1".to_string())),
            Some(crop_label(&[("thermal", 1)]))
        );
        assert_eq!(item_label(FLASK_ID, |_| Some("H2O".to_string())), None);
    }

    #[test]
    fn stack_sizes_follow_the_rules() {
        let dir = content_dir().join("data/biolab/horizon/item");
        let single = [
            // Свои данные в стеке: меню сливает стопки по id, разные гены слиплись бы.
            FLASK_ID, FRAGMENT_ID, CULTURE_ID, BLUEPRINT_ID, GM_CROP, STRETCHER_ID,
            // Инструменты.
            SCALPEL_ID, ANALYZER_ID, MICROSCOPE_ID, MANUAL_ID,
            // Оружие: свои данные (заряд, магазин, газ) и стопкой не держится.
            BLOWPIPE_ID, TRANQ_CROSSBOW_ID, AIR_RIFLE_ID,
            // Полный баллон меняется на пустой прямо в слоте (`weapons.rs`):
            // выдача в инвентарь из хука ненадёжна, а стопку не подменить.
            CO2_CANISTER_ID,
        ];
        let mut checked = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let id = format!("biolab:{}", path.file_stem().unwrap().to_string_lossy());
            let json = Json::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let size = json.get("max_stack_size").and_then(Json::num).expect(&id) as u32;
            let want = if single.contains(&id.as_str()) || fluid_of_bucket(&id).is_some() {
                1
            } else if element(&id).is_some()
                || id == TISSUE_ID
                || id == "biolab:mold"
                || id.ends_with("_seeds")
                || ["biolab:valerian_root", "biolab:belladonna_berries", "biolab:mandrake_root"]
                    .contains(&id.as_str())
            {
                // Сырьё: и семена с урожаем своих растений, как ванильные.
                64
            } else {
                // Препараты, экстракты, шприцы, дротики и расходники лаборатории.
                16
            };
            assert_eq!(size, want, "{id}");
            checked += 1;
        }
        assert!(checked > 50);
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

    /// Рецепт реактора выполним, только если какой-то источник тепла держит
    /// температуру в его диапазоне сам: нагрев ползёт к потолку источника и
    /// стоит на нём, а выход из диапазона сбрасывает прогресс. Раньше
    /// ослабленная культура требовала 60–140 °C при самом слабом источнике в
    /// 300 °C, и её приходилось вымучивать огнивом.
    #[test]
    fn every_reactor_recipe_has_a_steady_heat_source() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/recipe");
        let mut checked = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let j = Json::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
            if j.get("type").and_then(Json::str) != Some(REACTOR) {
                continue;
            }
            let Some(range) = j.get("temperature").and_then(Json::arr) else {
                continue;
            };
            let (min, max) = (range[0].num().unwrap() as i32, range[1].num().unwrap() as i32);
            let time = j.get("time").and_then(Json::num).map_or(DEFAULT_TIME, |t| t as i32);
            let steady: Vec<&str> = HEAT_SOURCES
                .iter()
                .filter(|(_, ceiling)| (min..=max).contains(ceiling))
                .map(|(block, _)| *block)
                .collect();
            assert!(!steady.is_empty(), "{}: ни один источник не стоит в {min}–{max} °C", path.display());
            // И на деле: от комнатной температуры реактор доходит до диапазона
            // и держится в нём всё время рецепта, без рук игрока.
            for block in steady {
                let mut t = ROOM_TEMPERATURE;
                let mut inside = 0;
                for _ in 0..2000 {
                    t = next_temperature(t, heat_target(block));
                    if temperature_fits(t, Some((min, max))) {
                        inside += 1;
                    } else {
                        assert_eq!(inside, 0, "{}: на {block} вышел из диапазона", path.display());
                    }
                }
                assert!(inside >= time, "{}: на {block} в диапазоне {inside} тиков", path.display());
                assert!(t <= OVERHEAT);
            }
            checked += 1;
        }
        assert!(checked >= 3, "рецептов реактора с температурой: {checked}");
    }

    #[test]
    fn weakened_culture_cooks_on_a_campfire() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let view = recipe_view(&std::fs::read_to_string(root.join(recipe_path("culture_weakened"))).unwrap());
        assert!(matches!(view, Some(RecipeView::Machine { ref machine, .. }) if machine == REACTOR));
        let j = Json::parse(&std::fs::read_to_string(root.join(recipe_path("culture_weakened"))).unwrap()).unwrap();
        let range = j.get("temperature").and_then(Json::arr).unwrap();
        let (min, max) = (range[0].num().unwrap() as i32, range[1].num().unwrap() as i32);
        for block in ["minecraft:campfire", "minecraft:soul_campfire"] {
            assert!((min..=max).contains(&heat_target(block).unwrap()), "{block}");
        }
        assert!(!(min..=max).contains(&heat_target("minecraft:magma_block").unwrap()), "на магме культура сгорает");
    }

    #[test]
    fn synthesizer_spends_elements_and_keeps_the_blueprint() {
        let inputs = [
            Some("biolab:element_hydrogen"),
            Some(BLUEPRINT_ID),
            None,
            Some("biolab:element_oxygen"),
            Some("minecraft:dirt"),
            None,
            None,
            Some("biolab:element_hydrogen"),
        ];
        assert_eq!(synth_spent_slots(&inputs), vec![0, 3, 7], "чертёж и чужой предмет остаются");
        assert!(synth_spent_slots(&[Some(BLUEPRINT_ID), None]).is_empty());
        assert!(synth_spent_slots(&[]).is_empty());
    }

    #[test]
    fn poured_buckets_empty_in_survival_only() {
        for fluid in &FLUIDS {
            assert_eq!(emptied_bucket(fluid.bucket, false), Some(EMPTY_BUCKET), "{}", fluid.bucket);
            assert_eq!(emptied_bucket(fluid.bucket, true), None, "{}: в творческом ведро остаётся", fluid.bucket);
        }
        assert_eq!(emptied_bucket("minecraft:water_bucket", false), None, "ванильные вёдра — не наши");
        assert_eq!(emptied_bucket(EMPTY_BUCKET, false), None);
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
            // Газ (эфир, хлороформ) себе не принимают — его вдыхают в облаке.
            let inhaled = SLEEPERS.iter().any(|s| s.gas && s.id == drug.id);
            assert!(
                MEDICINES.iter().any(|m| m.drug == i) || inhaled,
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
        // Снотворное мобу — по силе против массы, не таблицей эффектов.
        assert!(mob_injection("biolab:syringe_sedative").is_none());
        assert_eq!(sleeper_of_item("biolab:syringe_sedative").map(|s| s.strength), Some(2));
        assert!(acts_on_mobs("biolab:syringe_sedative") && acts_on_mobs("biolab:syringe_venom"));
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

    /// Гены игрока уходят в окно данными `biolab:player_genes`: строкой по
    /// именам, в порядке таблицы. Пустая строка — генов нет, наросты снять.
    #[test]
    fn player_genes_become_data() {
        let mut me = PlayerState::default();
        assert_eq!(splice_player(&mut me, "tail", 2), Splice::Done);
        assert_eq!(splice_player(&mut me, "polymelia", 3), Splice::Done);
        let text = genome_data(&player_genes(&me));
        assert_eq!(text, "polymelia:3,tail:2");
        assert_eq!(genes_of_data(&text), vec![("polymelia", 3), ("tail", 2)]);
        assert_eq!(genome_data(&player_genes(&PlayerState::default())), "");
        // Весь геном влезает и читается обратно.
        let mut full = PlayerState::default();
        for i in 0..GENES.len() {
            full.genes[i] = 1 + (i % 3) as u8;
        }
        assert_eq!(genes_of_data(&genome_data(&player_genes(&full))), player_genes(&full));
    }

    /// Строка генома данных одна на геном: порядок таблицы, сильнейшая сила,
    /// мусор отброшен — иначе каждая запись уходила бы клиентам заново.
    #[test]
    fn genome_data_is_canonical() {
        let text = genome_data(&[("horns", 1), ("chitin", 2), ("horns", 3), ("ghost", 1), ("tail", 9)]);
        assert_eq!(text, "chitin:2,horns:3");
        assert_eq!(genome_data(&genes_of_data(&text)), text);
        assert_eq!(genes_of_data("ghost:1,horns:2,tail,chitin:x"), vec![("horns", 2)]);
        assert!(genes_of_data("").is_empty());
        assert_eq!(acid_radius(&genes_of_data("acid_glands:2")), Some(4.0));
        assert_eq!(acid_radius(&genes_of_data("horns:2")), None);
        // Кислотная аура по данным — та же, что раньше по метке.
        for strength in 1..=3 {
            let marker = marker_for("acid_glands", strength).unwrap();
            assert_eq!(acid_radius(&[("acid_glands", strength)]), aura_radius(marker));
        }
    }

    /// Операционная в данных: маска срезов, стол и «спит» — байтами; запись
    /// стола или среза не сбрасывает «спит», который ставит `biolab-deep`.
    #[test]
    fn surgery_data_round_trips() {
        for bits in 0..=(FLAYED_BITS | TABLE_BIT | ASLEEP_BIT) {
            let fields = surgery_fields(bits);
            let back = surgery_from_fields(|name| fields.iter().find(|(f, _)| *f == name).map(|(_, v)| *v));
            assert_eq!(back, bits);
        }
        assert_eq!(surgery_bits(true, 0b0101, ASLEEP_BIT), TABLE_BIT | ASLEEP_BIT | 0b0101);
        assert_eq!(surgery_bits(false, 0b1_0000, TABLE_BIT), 0, "чужие биты в маску не проходят");
        assert_eq!(flayed_mask(|m| m == SURGERY_FLAYED[1]) & FLAYED_BITS, 0b10);
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
        assert_eq!(text, "Укол: Яд → Корова");
        assert_eq!(injection_message("biolab:syringe_sedative", "minecraft:zombie"), "Укол: Седативное → Зомби");
        assert!(injection_message("biolab:syringe", "pig").starts_with("Укол: Препарат"));
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
            // Механика для человека — только у гена без ванильного эффекта.
            if player_mechanic(kind.name).is_some() {
                assert!(kind.player.is_none(), "{}: и эффект, и механика", kind.name);
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
        assert!(lines.iter().any(|l| l.contains("Цена: 4")), "{lines:?}");
        assert!(fragment_label(&text).contains("++"));
        assert_eq!(fragment_tooltip("мусор"), ["Ген не прочитан"]);

        // Ген, который человеку не вживить, честно об этом пишет.
        let lines = fragment_tooltip(&gene_text("volatile", 1));
        assert!(lines.iter().any(|l| l.contains("не приживается")), "{lines:?}");

        // Ген-механика пишет, что он даёт человеку, а не «не приживается».
        let lines = fragment_tooltip(&gene_text("tail", 2));
        assert!(lines.iter().any(|l| l.contains("Человеку: хвост")), "{lines:?}");
        let lines = fragment_tooltip(&gene_text("tumor", 1));
        assert!(lines.iter().any(|l| l.contains("с побочкой (slowness)")), "{lines:?}");
    }

    #[test]
    fn extracted_genes_are_usable() {
        let mut strong_expensive = false;
        for seed in 0..500 {
            let (name, strength) = roll_gene(seed);
            let kind = gene(name).expect(name);
            assert!((1..=3).contains(&strength));
            // Лаборатория без предела: любой ген любой силы есть куда вживить.
            assert_eq!(splice_mob(&[], name, strength), Splice::Done, "{name}");
            strong_expensive |= kind.cost * strength > STABILITY;
        }
        assert!(strong_expensive, "дорогие гены теперь бывают и сильными");
        assert_eq!(roll_gene(42), roll_gene(42), "зерно задаёт ген");
    }

    #[test]
    fn the_microscope_reads_a_genome() {
        assert_eq!(genome_lines(&[]), ["Обычный организм, мутаций нет"]);
        let lines = genome_lines(&[("gigantism", 2), ("bioluminescence", 1)]);
        assert!(lines.iter().any(|l| l.contains("Гигантизм")), "{lines:?}");
        assert!(lines.last().unwrap().contains("5 из природных 6"), "{lines:?}");
        // Лабораторный геном сверх природы микроскоп так и называет.
        let lab = [("gigantism", 2), ("chitin", 2), ("horns", 1), ("tail", 3)];
        let lines = genome_lines(&lab);
        assert_eq!(lines.len(), lab.len() + 2, "все гены видны: {lines:?}");
        assert!(lines.last().unwrap().contains("лабораторный"), "{lines:?}");
    }

    #[test]
    fn gene_therapy_has_no_limit() {
        let mut state = PlayerState::default();
        assert_eq!(splice_player(&mut state, "chitin", 2), Splice::Done);
        assert_eq!(player_genes(&state), [("chitin", 2)]);
        assert_eq!(player_stability(&state), 4);
        assert_eq!(splice_player(&mut state, "chitin", 1), Splice::Already);
        // Раньше здесь был перебор по стабильности; лаборатория его не знает.
        assert_eq!(splice_player(&mut state, "regeneration", 3), Splice::Done);
        assert_eq!(splice_player(&mut state, "volatile", 1), Splice::NotForHumans);
        assert_eq!(splice_player(&mut state, "bioluminescence", 1), Splice::Done);

        // Сколько угодно генов: все, что приживаются в человеке.
        let mut full = PlayerState::default();
        let mut taken = 0;
        for kind in &GENES {
            let result = splice_player(&mut full, kind.name, 3);
            if takes_in_humans(kind) {
                assert_eq!(result, Splice::Done, "{}", kind.name);
                taken += 1;
            } else {
                assert_eq!(result, Splice::NotForHumans, "{}", kind.name);
            }
        }
        assert_eq!(player_genes(&full).len(), taken);
        assert!(taken > MAX_GENES);
        assert!(player_stability(&full) > STABILITY);
        // Механика уродств приживается, хоть ванильного эффекта у неё и нет.
        assert_eq!(player_gene(&full, "tentacles"), 3);
        assert_eq!(player_gene(&full, "acid_glands"), 0);
        assert!(gene_effects(&full).iter().all(|(effect, _)| !effect.is_empty()));
        // И переживает сейв целиком.
        assert_eq!(state_from_text(&state_to_text(&full)), full);
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
    fn a_mob_takes_any_number_of_genes_in_the_lab() {
        assert_eq!(splice_mob(&[], "volatile", 1), Splice::Done, "мобу можно всё");
        assert_eq!(splice_mob(&[("chitin", 2)], "chitin", 1), Splice::Already);
        assert_eq!(splice_mob(&[("chitin", 1)], "chitin", 2), Splice::Done, "усиление");
        // Ни стабильность, ни число генов уколу не мешают.
        assert_eq!(splice_mob(&[("regeneration", 2)], "chitin", 1), Splice::Done);
        let full = [("bioluminescence", 1), ("hypermetabolism", 1), ("gigantism", 1)];
        assert_eq!(splice_mob(&full, "thermal", 1), Splice::Done);
        assert_eq!(splice_mob(&[], "telepathy", 1), Splice::NotForHumans);

        // Природные правила остались — для скрещивания на грядке.
        assert!(fits_naturally(&[], "chitin", 2));
        assert!(!fits_naturally(&[("regeneration", 2)], "chitin", 1), "перебор по стабильности");
        assert!(!fits_naturally(&full, "thermal", 1), "больше трёх генов");
        assert!(fits_naturally(&full, "bioluminescence", 2), "усилить свой можно");
        assert!(!fits_naturally(&[("chitin", 2)], "chitin", 1));
        assert!(!fits_naturally(&[], "telepathy", 1));
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

        // Предела нет: дорогой геном всё равно мутирует дальше.
        let mut heavy = PlayerState::default();
        splice_player(&mut heavy, "regeneration", 2);
        assert!(mutagen_tick(&mut heavy, MUTAGEN_LOAD, 0).is_some());
        assert_eq!(player_genes(&heavy).len(), 2);
        // Срыв проходит впустую, только когда меняться уже нечему.
        let mut full = PlayerState::default();
        for kind in &GENES {
            splice_player(&mut full, kind.name, 3);
        }
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
        assert_eq!(crop_label(&[]), "ГМ-урожай: без генома");
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
            "loyal", "toxin_proof", "rend", "extra_arm", "watchful", "ram", "tail",
            "grab", "far_sight", "glide",
        ] {
            assert!(
                GENE_BEHAVIOUR.iter().any(|flags| flags.contains(&flag)),
                "{flag} без гена"
            );
        }
    }

    #[test]
    fn deformity_genes_are_appended_and_complete() {
        // Новые гены — в конце таблицы: номера старых (сейвы, слоты данных
        // машин) не сдвинулись.
        assert_eq!(GENES[31].name, "rending_claws");
        let new = ["polymelia", "dicephaly", "horns", "tail", "tentacles", "polyopia", "tumor", "membranes"];
        for (i, name) in new.iter().enumerate() {
            assert_eq!(GENES[32 + i].name, *name);
            let kind = gene(name).expect(name);
            assert!(takes_in_humans(kind), "{name}: у игрока тоже работает (GDD 4.19)");
            assert!(marker_for(name, 3).is_some());
        }
        // Цены — по таблице GDD 4.19.
        let costs: Vec<i32> = new.iter().map(|n| gene(n).unwrap().cost).collect();
        assert_eq!(costs, [3, 3, 2, 2, 3, 2, 1, 2]);
        // Ванильных эффектов хватает там, где GDD их называет.
        assert_eq!(gene("polyopia").unwrap().player, Some("night_vision"));
        assert_eq!(gene("membranes").unwrap().player, Some("slow_falling"));
        assert_eq!(gene("polymelia").unwrap().player, Some("haste"));
        assert_eq!(gene("tumor").unwrap().player, Some("health_boost"));
        assert_eq!(gene("tumor").unwrap().side, Some("slowness"));
    }

    #[test]
    fn behind_means_the_back_half() {
        // Смотрит на юг (+Z): сзади — север (−Z).
        assert!(behind(0.0, [0.0, 0.0], [0.0, -2.0]));
        assert!(!behind(0.0, [0.0, 0.0], [0.0, 2.0]), "спереди");
        assert!(!behind(0.0, [0.0, 0.0], [2.0, 0.0]), "сбоку не спина");
        // Смотрит на запад (−X): сзади — восток.
        assert!(behind(90.0, [5.0, 5.0], [7.0, 5.0]));
        assert!(!behind(90.0, [5.0, 5.0], [3.0, 5.0]));
        assert!(!behind(0.0, [1.0, 1.0], [1.0, 1.0]), "в той же точке — не сзади");
    }

    #[test]
    fn deformities_grow_with_strength() {
        assert!(extra_arm_period(3) < extra_arm_period(1), "сильная рука бьёт чаще");
        for s in 1..=3 {
            assert_eq!(extra_arm_period(s) % BEHAVIOUR_EVERY, 0, "период ловится тиком мира");
        }
        assert_eq!(RAM_EVERY % BEHAVIOUR_EVERY, 0);
        assert_eq!(TAIL_EVERY % BEHAVIOUR_EVERY, 0);
        assert_eq!(GRAB_EVERY % BEHAVIOUR_EVERY, 0);
        assert_eq!(extra_arm_damage(0.0), 1.0, "без урона — хоть единица");
        assert_eq!(extra_arm_damage(6.0), 3.0);
        assert!(ram_push(3) > ram_push(1) && ram_damage(3) > ram_damage(1));
        assert!(grab_reach(3) > grab_reach(1) && grab_reach(1) > MELEE_REACH);
        assert!(far_sight_radius(1) > watch_radius(3), "многоглазие видит дальше");
        assert!(RAM_RUN.0 > MELEE_REACH, "разбег — не вплотную");
        assert!(glow_radius(3) > glow_radius(1));
        assert!(player_ram_push(2) > 0.0 && player_tail_damage(1) >= 2.0);
    }

    #[test]
    fn tentacles_pull_items_within_reach() {
        let player = [0.0, 64.0, 0.0];
        let v = magnet_velocity([4.0, 64.0, 0.0], player, 1).expect("в досягаемости");
        assert!(v[0] < 0.0, "тянут к игроку: {v:?}");
        assert_eq!(magnet_velocity([0.3, 64.0, 0.2], player, 3), None, "у ног подберёт ванилла");
        assert_eq!(magnet_velocity([20.0, 64.0, 0.0], player, 3), None, "далеко");
        assert_eq!(magnet_velocity([4.0, 64.0, 0.0], player, 0), None, "без гена");
        assert!(magnet_velocity([8.0, 64.0, 0.0], player, 3).is_some());
        assert!(magnet_velocity([8.0, 64.0, 0.0], player, 1).is_none());
    }

    #[test]
    fn a_new_hit_is_seen_once() {
        let uuid = 0xBADu128;
        assert!(note_hurt_time(uuid, 10), "только что ударили");
        assert!(!note_hurt_time(uuid, 9), "тот же удар тает");
        assert!(!note_hurt_time(uuid, 0));
        assert!(note_hurt_time(uuid, 10), "следующий удар");
        assert!(!fresh_hit(5, 5));
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
            assert!(acts_on_mobs(syringe), "{syringe} ничего не делает мобу");
        }
        assert_eq!(dart_syringe("biolab:syringe_venom"), None);
    }

    /// Каждый дротик бросается своим типом сущности — копией снежка, — и тип
    /// назван как предмет: по этому имени хук попадания узнаёт дротик.
    #[test]
    fn every_dart_flies_as_its_own_snowball() {
        let dir = content_dir().join("data/biolab/horizon/entity_type");
        let mut types = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("нет entity_type/") {
            let path = entry.unwrap().path();
            let id = format!("biolab:{}", path.file_stem().unwrap().to_string_lossy());
            let json = Json::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
            // Движок пина копирует только снежок (`ENTITY_TEMPLATES`).
            assert_eq!(json.get("template").and_then(Json::str), Some("minecraft:snowball"), "{id}");
            assert_eq!(json.get("item").and_then(Json::str), Some(id.as_str()), "{id}: бросает не свой предмет");
            // Газовая граната — тоже снаряд-снежок (GDD 4.20), но не дротик.
            if grenade_projectile(&id).is_some() {
                assert_eq!(json.get("damage").and_then(Json::num), Some(0.0), "{id}: граната не бьёт");
                continue;
            }
            assert_eq!(dart_projectile(&id), Some(DARTS.iter().find(|(d, _)| *d == id).expect("тип не дротик").0));
            types.push(id);
        }
        types.sort();
        let mut darts: Vec<String> = DARTS.iter().map(|(d, _)| d.to_string()).collect();
        darts.sort();
        assert_eq!(types, darts, "у дротика нет своего снаряда");
        // Летящий дротик назван так же, как дротик в руке.
        for (lang_name, lang) in [("ru_ru", lang("ru_ru")), ("en_us", lang("en_us"))] {
            for (dart, _) in DARTS {
                let short = dart.trim_start_matches("biolab:");
                let entity = lang.get(&format!("entity.biolab.{short}")).and_then(Json::str);
                let item = lang.get(&format!("item.biolab.{short}")).and_then(Json::str);
                assert!(entity.is_some() && entity == item, "{dart}: имя снаряда в {lang_name}");
            }
        }
        assert_eq!(dart_projectile("minecraft:snowball"), None);
        assert_eq!(dart_projectile("biolab:syringe_sedative"), None);
    }

    #[test]
    fn a_dart_injects_its_syringe_within_a_throw() {
        let hand = |from| Some(DartLaunch { from, range: DART_THROW_RANGE as u32 });
        let from = [0.0, 65.62, 0.0];
        let near = [5.0, 65.0, 3.0];
        let far = [DART_THROW_RANGE + 0.5, 65.62, 0.0];
        for (dart, syringe) in DARTS {
            // В моба на дальности броска — то же, что укол шприцем.
            assert_eq!(dart_impact(dart, true, hand(from), near), DartImpact::Inject(syringe));
            // Ровно на границе ещё входит.
            let edge = [DART_THROW_RANGE, 65.62, 0.0];
            assert_eq!(dart_impact(dart, true, hand(from), edge), DartImpact::Inject(syringe));
            // Дальше — на излёте.
            assert_eq!(dart_impact(dart, true, hand(from), far), DartImpact::Spent(8));
            // Бросок не слышали (`/summon`, сохранение) — дротик всё равно действует.
            assert_eq!(dart_impact(dart, true, None, far), DartImpact::Inject(syringe));
            // Мимо, в блок: дротик ломается сам, мы ничего не делаем.
            assert_eq!(dart_impact(dart, false, hand(from), near), DartImpact::Nothing);
            // Из оружия та же точка — не на излёте: граница — дальность оружия.
            for weapon in &DART_WEAPONS {
                let shot = Some(DartLaunch { from, range: weapon.range });
                let edge = [f64::from(weapon.range) - 0.5, 65.62, 0.0];
                let beyond = [f64::from(weapon.range) + 0.5, 65.62, 0.0];
                assert_eq!(dart_impact(dart, true, shot, far), DartImpact::Inject(syringe), "{}", weapon.item);
                assert_eq!(dart_impact(dart, true, shot, edge), DartImpact::Inject(syringe), "{}", weapon.item);
                assert_eq!(dart_impact(dart, true, shot, beyond), DartImpact::Spent(weapon.range), "{}", weapon.item);
            }
        }
        // Чужой снаряд — не наше дело.
        assert_eq!(dart_impact("minecraft:snowball", true, hand(from), near), DartImpact::Nothing);
        assert_eq!(dart_impact("minecraft:arrow", true, None, near), DartImpact::Nothing);
        // Сообщение называет ту дальность, которой не хватило.
        assert!(dart_spent_message(8).contains(&format!("рукой — до {} блоков", DART_THROW_RANGE as i32)));
        assert!(dart_spent_message(32).contains("32 блоков") && !dart_spent_message(32).contains("рукой"));
    }

    /// Одним тестом: таблица бросков и взведённая дальность общие на процесс,
    /// а тесты идут потоками.
    // ---- Снотворные: сила против массы (GDD 4.20) ----

    /// Масса — по хитбоксу вида, как длина стола: ориентиры GDD 4.20.
    #[test]
    fn mass_follows_the_hitbox_like_the_table() {
        let cases = [
            ("minecraft:chicken", 0.4, 0.7, 1),
            ("minecraft:rabbit", 0.4, 0.5, 1),
            ("minecraft:zombie", 0.6, 1.95, 2),
            ("minecraft:skeleton", 0.6, 1.99, 2),
            ("minecraft:pig", 0.9, 0.9, 2),
            ("minecraft:cow", 0.9, 1.4, 3),
            ("minecraft:spider", 1.4, 0.9, 3),
            ("minecraft:ravager", 1.95, 2.2, 4),
            ("minecraft:iron_golem", 1.4, 2.7, 4),
            ("minecraft:giant", 3.6, 12.0, 5),
            ("minecraft:warden", 0.9, 2.9, 5),
        ];
        for (kind, w, h, mass) in cases {
            assert_eq!(species_mass(kind, w, h), mass, "{kind}");
        }
        // Длина тела из таблицы стола: без неё корова весила бы как свинья.
        assert!(body_bulk("minecraft:cow", 0.9, 1.4) > 0.9 * 1.4 * 0.9);
        assert_eq!(species_mass("cow", 0.9, 1.4), 3, "вид без пространства имён");
        assert_eq!(species_mass("minecraft:zombie", f64::NAN, 1.0), 1);
    }

    #[test]
    fn genes_shift_the_mass() {
        let zombie = |genes: &[(&str, i32)]| mob_mass("minecraft:zombie", 0.6, 1.95, genes);
        assert_eq!(zombie(&[]), 2);
        assert_eq!(zombie(&[("gigantism", 1)]), 3);
        assert_eq!(zombie(&[("gigantism", 3)]), 3, "весовая категория, а не сила гена");
        assert_eq!(zombie(&[("dwarfism", 2)]), 1);
        assert_eq!(zombie(&[(RESISTANCE_GENE, 1)]), 3);
        assert_eq!(zombie(&[(RESISTANCE_GENE, 3)]), 5);
        assert_eq!(zombie(&[("gigantism", 1), (RESISTANCE_GENE, 3)]), 5, "не выше 5");
        assert_eq!(mob_mass("minecraft:chicken", 0.4, 0.7, &[("dwarfism", 1)]), 1, "не ниже 1");
        assert!(gene(RESISTANCE_GENE).is_some(), "ген в таблице генов");
    }

    /// Таблица GDD 4.20: что кого берёт.
    #[test]
    fn strength_against_mass() {
        use SleepOutcome::*;
        let s = |id: &str| sleeper(id).unwrap();
        let out = |id: &str, mass: u8| sleep_outcome(s(id), mass, false);
        // Отвар: мелочь спит, остальные вялые, крупным — ничего.
        assert_eq!(out("calming", 1), Takes);
        assert_eq!(out("calming", 2), Drowsy);
        assert_eq!(out("calming", 3), TooWeak);
        // Седативное: зомби спит, корова только вялая.
        assert_eq!(out("sedative", 2), Takes);
        assert_eq!(out("sedative", 3), Drowsy);
        assert_eq!(out("sedative", 5), TooWeak);
        assert_eq!(out("mandrake", 3), Takes);
        assert_eq!(out("mandrake", 4), Drowsy);
        // Хлороформ: окно узкое — курица и зомби не выживают.
        assert_eq!(out("chloroform", 1), Fatal);
        assert_eq!(out("chloroform", 2), Fatal);
        assert_eq!(out("chloroform", 3), Takes);
        assert_eq!(out("chloroform", 4), Takes);
        // Транквилизатор: гигант спит, зомби — передозировка без смерти.
        assert_eq!(out("tranquilizer", 5), Takes);
        assert_eq!(out("tranquilizer", 2), Overdose);
        assert_eq!(out("curare", 1), Overdose);
        assert_eq!(out("curare", 4), Takes);
        assert_eq!(out("wake", 5), Woken);
        // Слабые почти не передозируются: отвар, седативное, эфир — никогда.
        for mass in 1..=5 {
            for id in ["calming", "sedative", "ether", "mandrake"] {
                assert!(!matches!(out(id, mass), Overdose | Fatal), "{id} {mass}");
            }
        }
        // Повтор за 5 минут слабее на 1.
        assert_eq!(sleep_outcome(s("sedative"), 2, true), Drowsy);
        assert_eq!(sleep_outcome(s("chloroform"), 2, true), Takes, "повтор уводит из передозировки");
    }

    #[test]
    fn stronger_sleepers_take_longer_to_bite() {
        let s = |id: &str| sleeper(id).unwrap();
        assert!(sleep_delay(s("calming"), 1) < sleep_delay(s("sedative"), 2));
        assert!(sleep_delay(s("sedative"), 2) < sleep_delay(s("mandrake"), 3));
        assert!(sleep_delay(s("mandrake"), 3) <= sleep_delay(s("tranquilizer"), 1));
        assert_eq!(sleep_delay(s("tranquilizer"), 5), 160, "до 8 секунд");
        // Газы валят быстро.
        assert!(sleep_delay(s("ether"), 3) < sleep_delay(s("mandrake"), 3));
        for sl in &SLEEPERS {
            assert!(sl.strength <= 5);
            if sl.kind != SleepKind::Wake {
                assert!(sl.strength >= 1 && sl.ticks > 0, "{}", sl.id);
                assert!(sleep_delay(sl, 5) <= 160, "{}: задержка больше 8 с", sl.id);
            }
        }
    }

    /// Снотворные и их предметы сходятся с веществами и формами.
    #[test]
    fn sleepers_are_drugs_and_items() {
        for sl in &SLEEPERS {
            let drug = DRUGS.iter().find(|d| d.id == sl.id).unwrap_or_else(|| panic!("{}: нет в DRUGS", sl.id));
            assert_eq!(drug.name, sl.name, "{}", sl.id);
            // Каждое снотворное чем-то дают: в упор или газом.
            let contact = SLEEP_ITEMS.iter().any(|(_, id)| *id == sl.id);
            let gas = GAS_ITEMS.iter().any(|(_, id, _)| *id == sl.id);
            assert!(contact || gas, "{}: нечем дать", sl.id);
            assert_eq!(sl.gas, gas, "{}", sl.id);
        }
        for (item, id, _) in GAS_ITEMS {
            assert!(sleeper(id).is_some_and(|s| s.gas), "{item}");
            assert!(medicine(item).is_none(), "{item}: газ не принимают себе — remedy.rs проглотил бы клик");
        }
        assert_eq!(grenade_projectile("biolab:grenade_ether").map(|s| s.id), Some("ether"));
        assert!(grenade_projectile("biolab:spray_ether").is_none(), "аэрозоль не летит");
        // Дротик со снотворным — тот же шприц издали.
        for id in ["sedative", "mandrake", "curare", "tranquilizer"] {
            assert!(DARTS.iter().any(|(_, syringe)| sleeper_of_item(syringe).is_some_and(|s| s.id == id)), "{id}");
        }
        assert_eq!(via_of_item(CHLOROFORM_RAG), Via::Rag);
        assert_eq!(via_of_item(CALMING_BREW), Via::Drink);
        assert_eq!(after_use(CALMING_BREW), Some("minecraft:glass_bottle"));
    }

    #[test]
    fn sleep_messages_say_what_happened() {
        let s = |id: &str| sleeper(id).unwrap();
        let weak = sleep_message(s("calming"), Via::Dart, "minecraft:cow", 3, SleepOutcome::TooWeak, false);
        assert!(weak.starts_with(TOO_WEAK), "{weak}");
        let takes = sleep_message(s("tranquilizer"), Via::Dart, "minecraft:ravager", 4, SleepOutcome::Takes, false);
        assert_eq!(takes, "Дротик: Транквилизатор → Разоритель: уснёт через 7 с");
        let curare = sleep_message(s("curare"), Via::Dart, "minecraft:cow", 3, SleepOutcome::Takes, false);
        assert!(curare.contains("паралич, но не сон"), "{curare}");
        let again = sleep_message(s("sedative"), Via::Injection, "minecraft:zombie", 2, SleepOutcome::Drowsy, true);
        assert!(again.starts_with("Укол: Седативное → Зомби: только вялый"), "{again}");
        assert!(again.ends_with("слабее на 1)"), "{again}");
        let dead = sleep_message(s("chloroform"), Via::Rag, "minecraft:chicken", 1, SleepOutcome::Fatal, false);
        assert!(dead.starts_with("Тряпка: Хлороформ → Курица") && dead.contains("не выдержал"), "{dead}");
    }

    #[test]
    fn the_table_renews_what_put_the_patient_to_sleep() {
        let id = |s: Option<&Sleeper>| s.map(|s| s.id);
        assert_eq!(id(table_sleeper(Some("mandrake"), 3)), Some("mandrake"));
        assert_eq!(id(table_sleeper(Some("ether"), 2)), Some("ether"));
        // Вживили устойчивость — мандрагора больше не держит.
        assert_eq!(id(table_sleeper(Some("mandrake"), 4)), None);
        // Не знаем, чем усыплён, — держим седативным, как до M5.
        assert_eq!(id(table_sleeper(None, 5)), Some("sedative"));
        assert_eq!(id(table_sleeper(Some("moonshine"), 2)), Some("sedative"));
        // Паралич стол сном не держит, если седативного не хватает.
        assert_eq!(id(table_sleeper(Some("curare"), 2)), Some("sedative"));
        assert_eq!(id(table_sleeper(Some("curare"), 4)), None);
        let msg = table_wakes_message(sleeper("mandrake").unwrap(), "minecraft:cow", 4);
        assert!(msg.starts_with("Корова просыпается") && msg.contains("нужно сильнее"), "{msg}");
        let p = patient_from_text("kind=minecraft:cow;paralysed=1;sleep=curare").unwrap();
        assert!(table_refuses(&p).is_some_and(|m| m.contains("только спящего")));
        assert_eq!(table_refuses(&Patient { paralysed: false, ..p }), None);
    }

    /// Подсказка снотворного называет его силу — ту, что в `SLEEPERS`.
    #[test]
    fn sleep_hints_name_the_strength() {
        let hint = |id: &str| item_hint(id).join(" / ");
        let mut items: Vec<(&str, &Sleeper)> = SLEEP_ITEMS
            .iter()
            .filter_map(|(item, _)| Some((*item, sleeper_of_item(item)?)))
            .collect();
        items.extend(GAS_ITEMS.iter().filter_map(|(item, _, _)| Some((*item, gas_of_item(item)?.0))));
        items.extend(DARTS.iter().filter_map(|(dart, syringe)| Some((*dart, sleeper_of_item(syringe)?))));
        for (item, s) in items {
            if s.kind == SleepKind::Wake {
                assert!(hint(item).contains("сон"), "{item}: {}", hint(item));
                continue;
            }
            assert!(hint(item).contains(&format!("Сила {}", s.strength)), "{item}: {}", hint(item));
        }
        // Тряпка — вплотную, граната — бросок.
        assert!(hint(CHLOROFORM_RAG).contains("вплотную"));
        assert!(hint("biolab:grenade_ether").contains("бросить"));
    }

    /// Одним тестом: книга общая на процесс, но здесь — своя.
    #[test]
    fn the_sleep_book_remembers_doses_and_clouds() {
        let mut book = SleepBook::default();
        // Привыкание: то же вещество в окне — повтор, другое — нет.
        book.note_dose(7, "sedative", 100);
        assert!(book.repeated(7, "sedative", 100 + REPEAT_WINDOW - 1));
        assert!(!book.repeated(7, "sedative", 100 + REPEAT_WINDOW));
        assert!(!book.repeated(7, "mandrake", 200));
        assert!(!book.repeated(8, "sedative", 200));
        // Отложенный сон: новый заменяет прежний, отдаётся в свой тик и уровень.
        book.schedule(7, "minecraft:overworld", "sedative", 140);
        book.schedule(7, "minecraft:overworld", "mandrake", 160);
        book.schedule(9, "minecraft:the_nether", "ether", 120);
        assert!(book.is_pending(7));
        assert!(book.take_due(150, "minecraft:overworld").is_empty());
        let due = book.take_due(160, "minecraft:overworld");
        assert_eq!(due.len(), 1);
        assert_eq!((due[0].mob, due[0].sleeper), (7, "mandrake"));
        assert!(!book.is_pending(7) && book.is_pending(9));
        // Давнее (чанк выгружен) — выбрасывается.
        assert!(book.take_due(120 + PENDING_STALE + 401, "minecraft:overworld").is_empty());
        assert!(!book.is_pending(9));
        // Вялость проходит сама.
        book.make_drowsy(3, 500);
        assert_eq!(book.drowsy_now(499), vec![3]);
        assert!(book.drowsy_now(500).is_empty());
        // Чем усыплён — до пробуждения.
        book.fell_asleep(4, "chloroform");
        assert_eq!(book.slept_with(4), Some("chloroform"));
        book.schedule(4, "minecraft:overworld", "ether", 900);
        book.make_drowsy(4, 900);
        book.wake(4);
        assert_eq!(book.slept_with(4), None);
        assert!(!book.is_pending(4) && book.drowsy_now(800).is_empty());
        // Облака истекают; «из будущего» — остаток прошлого мира.
        let cloud = |until| Cloud {
            dimension: "minecraft:overworld".into(),
            at: [0.0, 64.0, 0.0],
            radius: GRENADE_RADIUS,
            until,
            sleeper: "ether",
            dosed: Vec::new(),
            players: Vec::new(),
        };
        book.add_cloud(cloud(200));
        book.add_cloud(cloud(100_000));
        assert_eq!(book.clouds_now(150).len(), 1);
        assert!(book.clouds_now(200).is_empty());
        for i in 0..100 {
            book.add_cloud(cloud(300 + i));
        }
        assert_eq!(book.clouds_now(299).len(), 64, "облаков не больше 64");
        // Отметки дозы и выгорание — по месту и сроку облака.
        let first = book.clouds_now(299)[0].clone();
        book.mark_cloud(first.at, first.until, Some(5), Some(77));
        book.mark_cloud(first.at, first.until, None, None);
        let marked = book.clouds_now(299)[0].clone();
        assert_eq!((marked.dosed, marked.players), (vec![5], vec![77]));
        book.remove_cloud(first.at, first.until);
        assert_eq!(book.clouds_now(299).len(), 63);
        // Проход — раз за тик на уровень.
        assert!(sleep_pass_due(10, "test:a"));
        assert!(!sleep_pass_due(10, "test:a"));
        assert!(sleep_pass_due(10, "test:b"), "другой уровень — свой проход");
        assert!(sleep_pass_due(11, "test:a"));
    }

    #[test]
    fn gas_reaches_whoever_breathes_without_a_mask() {
        let cloud = Cloud {
            dimension: "minecraft:overworld".into(),
            at: [0.0, 64.0, 0.0],
            radius: GRENADE_RADIUS,
            until: 100,
            sleeper: "ether",
            dosed: Vec::new(),
            players: Vec::new(),
        };
        assert!(in_cloud(&cloud, [2.0, 65.0, 2.0]));
        assert!(!in_cloud(&cloud, [3.0, 64.0, 1.0]));
        // Аэрозоль — перед лицом: взгляд на юг — к +Z, вниз — ниже глаз.
        let ahead = spray_point([0.0, 64.0, 0.0], 0.0, 0.0);
        assert!((ahead[2] - SPRAY_REACH).abs() < 1e-9 && ahead[0].abs() < 1e-9);
        assert!((ahead[1] - (64.0 + EYE_HEIGHT)).abs() < 1e-9);
        let west = spray_point([0.0, 64.0, 0.0], 90.0, 0.0);
        assert!((west[0] + SPRAY_REACH).abs() < 1e-9);
        assert!(spray_point([0.0, 64.0, 0.0], 0.0, 90.0)[1] < 64.0 + EYE_HEIGHT - 2.0);
        // Противогаз (пока — черепаший панцирь) спасает, остальное — нет.
        assert!(gas_protected(Some("minecraft:turtle_helmet")));
        assert!(!gas_protected(Some("minecraft:iron_helmet")) && !gas_protected(None));
        // Устойчивость к снотворному режет вдох на четверть за силу.
        let ether = sleeper("ether").unwrap();
        assert_eq!(gas_dose(ether, 0), 30);
        assert_eq!(gas_dose(ether, 2), 15);
        assert_eq!(gas_dose(ether, 9), 7, "сила не выше 3");
        assert_eq!(gas_dose(sleeper("sedative").unwrap(), 0), 0, "не газ — не вдыхается");
        // Хлороформ: одного облака хватает до побочек, двух — до передозировки.
        let chloroform = DRUGS.iter().find(|d| d.id == "chloroform").unwrap();
        let one = gas_dose(sleeper("chloroform").unwrap(), 0) * FORMS[FORM_SPRAY].bioavailability / 100;
        assert!(one >= chloroform.window.0 && one <= chloroform.window.1);
        assert!(one * 2 > chloroform.window.1);
        // Огонь: эфир вспыхивает, хлороформ — нет; потухший костёр не горит.
        assert!(flammable(ether) && !flammable(sleeper("chloroform").unwrap()));
        assert!(ignites("minecraft:fire", None) && ignites("minecraft:campfire", Some("true")));
        assert!(!ignites("minecraft:campfire", Some("false")) && !ignites("minecraft:stone", None));
    }

    /// Одним тестом: таблица бросков общая на процесс, а тесты идут потоками.
    #[test]
    fn a_dart_launch_is_remembered_until_its_hit() {
        let at = |from: [f64; 3]| DartLaunch { from, range: 8 };
        let base = 4_000_000;
        remember_dart_launch(base + 1, at([1.0, 2.0, 3.0]), 100);
        remember_dart_launch(base + 2, at([4.0, 5.0, 6.0]), 110);
        // Попадание забирает точку броска, второе попадание её уже не найдёт.
        assert_eq!(take_dart_launch(base + 1), Some(at([1.0, 2.0, 3.0])));
        assert_eq!(take_dart_launch(base + 1), None);
        // Тот же id снова брошен — помним последний бросок.
        remember_dart_launch(base + 2, at([7.0, 8.0, 9.0]), 120);
        assert_eq!(take_dart_launch(base + 2), Some(at([7.0, 8.0, 9.0])));
        assert_eq!(take_dart_launch(base + 2), None);
        // Давний бросок забывается при следующем — и вперёд по тикам, и назад
        // (после перезахода в мир тики идут заново).
        remember_dart_launch(base + 3, at([0.0; 3]), 200);
        remember_dart_launch(base + 4, at([0.0; 3]), 200 + DART_FLIGHT_MAX + 1);
        assert_eq!(take_dart_launch(base + 3), None);
        remember_dart_launch(base + 5, at([0.0; 3]), 10);
        assert_eq!(take_dart_launch(base + 4), None);
        assert_eq!(take_dart_launch(base + 5), Some(at([0.0; 3])));

        // Бросок рукой дальность не взводит — запуск берёт дальность руки.
        assert_eq!(dart_launch_from([0.0; 3]).range, DART_THROW_RANGE as u32);
        // Оружие взвело — запуск забирает её, и следующий бросок снова рукой.
        arm_dart_range(48);
        assert_eq!(dart_launch_from([1.0; 3]), DartLaunch { from: [1.0; 3], range: 48 });
        assert_eq!(dart_launch_from([1.0; 3]).range, DART_THROW_RANGE as u32);
        // Хук запуска не позвали — оружие снимает дальность само.
        arm_dart_range(16);
        assert_eq!(take_armed_range(), Some(16));
        assert_eq!(take_armed_range(), None);
    }

    /// Полёт дротика по физике снежка пина (`projectiles.rs`, `tick_inner`:
    /// гравитация 0,03, потом сопротивление 0,99, потом шаг): сколько блоков
    /// по горизонтали он пролетит из выстрела горизонтально, пока не опустится
    /// на `drop` блоков.
    fn flat_reach(speed: f32, drop: f64) -> f64 {
        let (mut x, mut y) = (0.0_f64, 0.0_f64);
        let (mut vx, mut vy) = (f64::from(speed), 0.0_f64);
        for _ in 0..400 {
            vy -= 0.03;
            vx *= f64::from(0.99_f32);
            vy *= f64::from(0.99_f32);
            x += vx;
            y += vy;
            if -y > drop {
                break;
            }
        }
        x
    }

    #[test]
    fn every_weapon_reaches_its_range() {
        // Бросок рукой долетает до своей границы, не уйдя ниже полутора блоков.
        assert!(flat_reach(DART_THROW_SPEED, 1.5) >= DART_THROW_RANGE);
        let mut last = DART_THROW_RANGE as u32;
        for weapon in &DART_WEAPONS {
            // Дальше и быстрее предыдущего: трубка → арбалет → ружьё (GDD 4.20).
            assert!(weapon.range > last, "{}", weapon.item);
            last = weapon.range;
            assert!(weapon.speed > DART_THROW_SPEED && weapon.speed <= 3.9, "{}", weapon.item);
            // На дальности оружия дротик ещё не упал ниже трёх блоков: им можно
            // попасть, целясь чуть выше, а не навесом.
            assert!(flat_reach(weapon.speed, 3.0) >= f64::from(weapon.range), "{}: {}", weapon.item, flat_reach(weapon.speed, 3.0));
            assert!(weapon.spread <= 1.0 && weapon.cooldown >= 0, "{}", weapon.item);
            assert_eq!(dart_weapon(weapon.item).map(|w| w.item), Some(weapon.item));
        }
        // GDD 4.20: 16, 32 и 48 блоков; тихая из них одна трубка, перезарядка 1 с.
        let range = |id| dart_weapon(id).map(|w| w.range);
        assert_eq!((range(BLOWPIPE_ID), range(TRANQ_CROSSBOW_ID), range(AIR_RIFLE_ID)), (Some(16), Some(32), Some(48)));
        assert!(DART_WEAPONS.iter().all(|w| w.quiet == (w.item == BLOWPIPE_ID)));
        assert_eq!(dart_weapon(BLOWPIPE_ID).map(|w| w.cooldown), Some(20));
        assert!(dart_weapon("biolab:dart_sedative").is_none());
        // У трубки нет использования со временем, у арбалета и ружья — до отпускания.
        assert_eq!(weapon_use_duration(BLOWPIPE_ID), None);
        assert_eq!(weapon_use_duration(TRANQ_CROSSBOW_ID), Some(WEAPON_USE_TICKS));
        assert_eq!(weapon_use_duration(AIR_RIFLE_ID), Some(WEAPON_USE_TICKS));
        assert_eq!(weapon_use_duration("minecraft:crossbow"), None);
    }

    #[test]
    fn weapon_data_round_trips_and_survives_junk() {
        let (sedative, venom) = (DARTS[0].0, DARTS[1].0);
        let state = WeaponState { load: vec![sedative, venom, sedative], gas: 7 };
        assert_eq!(weapon_state(&weapon_text(&state)), state);
        assert_eq!(weapon_state(""), WeaponState::default());
        // Убранный из мода дротик выпадает, лишний сверх магазина — тоже, газ не
        // больше баллона, битое число — 0.
        let junk = format!("load={sedative},biolab:gone,{venom},{venom},{venom},{venom};gas=99");
        assert_eq!(weapon_state(&junk), WeaponState { load: vec![sedative, venom, venom, venom], gas: CANISTER_SHOTS });
        assert_eq!(weapon_state("gas=много;load=").gas, 0);
    }

    #[test]
    fn a_click_shoots_charges_or_reloads_by_the_weapon() {
        let [blowpipe, crossbow, rifle] = [BLOWPIPE_ID, TRANQ_CROSSBOW_ID, AIR_RIFLE_ID].map(|id| dart_weapon(id).unwrap());
        let (sedative, venom) = (DARTS[0].0, DARTS[1].0);
        let empty = WeaponState::default();
        let loaded = WeaponState { load: vec![venom], gas: 3 };
        // Трубка стреляет первым дротиком инвентаря; без дротиков — осечка.
        assert_eq!(weapon_click(blowpipe, &empty, Some(venom), false), WeaponClick::Shoot(venom));
        assert_eq!(weapon_click(blowpipe, &empty, None, false), WeaponClick::NoDarts);
        // В творчестве дротик не нужен.
        assert_eq!(weapon_click(blowpipe, &empty, None, true), WeaponClick::Shoot(sedative));
        // Арбалет: заряженный стреляет своим, хоть в инвентаре другой; пустой —
        // натягивается, если есть чем зарядить.
        assert_eq!(weapon_click(crossbow, &loaded, Some(sedative), false), WeaponClick::Shoot(venom));
        assert_eq!(weapon_click(crossbow, &loaded, None, false), WeaponClick::Shoot(venom));
        assert_eq!(weapon_click(crossbow, &empty, Some(sedative), false), WeaponClick::Use);
        assert_eq!(weapon_click(crossbow, &empty, None, false), WeaponClick::NoDarts);
        // Ружьё: с магазином — прицел, пустое — зарядка, без дротиков — осечка.
        assert_eq!(weapon_click(rifle, &loaded, None, false), WeaponClick::Use);
        assert_eq!(weapon_click(rifle, &empty, Some(sedative), false), WeaponClick::Reload);
        assert_eq!(weapon_click(rifle, &empty, None, false), WeaponClick::NoDarts);
        // Использование начинается ровно когда клик его не забрал: заряженный
        // арбалет и пустое ружьё клиент не должен «натягивать».
        assert!(weapon_use_may_start(crossbow, &empty, Some(sedative), false));
        assert!(!weapon_use_may_start(crossbow, &loaded, Some(sedative), false));
        assert!(weapon_use_may_start(rifle, &loaded, None, false));
        assert!(!weapon_use_may_start(rifle, &empty, Some(sedative), false));
        assert!(!weapon_use_may_start(blowpipe, &empty, Some(sedative), false));
    }

    #[test]
    fn the_crossbow_charges_like_vanilla() {
        // Тики натяжения идут подряд с нуля; звуки — на 20 и 50 %, заряд — на
        // 1,25 с, как у ванильного арбалета.
        let steps: Vec<(u32, ChargeStep)> =
            (0..100).filter_map(|held| crossbow_charge(held).map(|step| (held, step))).collect();
        assert_eq!(steps, vec![(5, ChargeStep::Start), (13, ChargeStep::Middle), (25, ChargeStep::Loaded)]);
        assert_eq!(CROSSBOW_CHARGE_TICKS, 25);
        // Оставшиеся тики пина → сколько держали.
        assert_eq!(weapon_held(WEAPON_USE_TICKS), 0);
        assert_eq!(weapon_held(WEAPON_USE_TICKS - 25), 25);
        assert_eq!(weapon_held(WEAPON_USE_TICKS + 5), 0);
    }

    #[test]
    fn the_magazine_fills_from_the_inventory() {
        let (sedative, venom) = (DARTS[0].0, DARTS[1].0);
        // Левая рука (-106) раньше инвентаря: стопки идут в порядке поиска.
        let stacks = [(-106, venom, 1), (3, sedative, 16)];
        let mut state = WeaponState::default();
        assert_eq!(reload_magazine(&mut state, &stacks, false), vec![(-106, 1), (3, 3)]);
        assert_eq!(state.load, vec![venom, sedative, sedative, sedative]);
        // Полный магазин больше не берёт.
        assert!(reload_magazine(&mut state, &stacks, false).is_empty());
        // Доливает до полного.
        let mut half = WeaponState { load: vec![venom, venom], gas: 0 };
        assert_eq!(reload_magazine(&mut half, &[(0, sedative, 1)], false), vec![(0, 1)]);
        assert_eq!(half.load, vec![venom, venom, sedative]);
        // В творчестве ничего не тратится.
        let mut creative = WeaponState::default();
        assert!(reload_magazine(&mut creative, &[], true).is_empty());
        assert_eq!(creative.load, vec![sedative; MAGAZINE_SIZE]);
        assert_eq!(first_dart(&stacks), Some(venom));
        assert_eq!(first_dart(&[]), None);
    }

    #[test]
    fn the_rifle_spends_gas_and_connects_a_canister() {
        let (sedative, venom) = (DARTS[0].0, DARTS[1].0);
        let mut state = WeaponState { load: vec![venom, sedative], gas: 1 };
        // Последний выстрел газа.
        assert_eq!(rifle_fire(&mut state, false, false), RifleShot::Fired { dart: venom, connected: false });
        assert_eq!(state, WeaponState { load: vec![sedative], gas: 0 });
        // Газа нет и баллона нет — дротик остаётся в магазине.
        assert_eq!(rifle_fire(&mut state, false, false), RifleShot::NoGas);
        assert_eq!(state.load, vec![sedative]);
        // Баллон есть — подключается, выстрел уходит.
        assert_eq!(rifle_fire(&mut state, true, false), RifleShot::Fired { dart: sedative, connected: true });
        assert_eq!(state, WeaponState { load: vec![], gas: CANISTER_SHOTS - 1 });
        assert_eq!(rifle_fire(&mut state, true, false), RifleShot::Empty);
        // В творчестве газ не тратится и не нужен.
        let mut creative = WeaponState { load: vec![venom], gas: 0 };
        assert_eq!(rifle_fire(&mut creative, false, true), RifleShot::Fired { dart: venom, connected: false });
        assert_eq!(creative.gas, 0);
    }

    #[test]
    fn weapons_show_their_load() {
        let (sedative, venom) = (DARTS[0].0, DARTS[1].0);
        let empty = WeaponState::default();
        // Трубке показывать нечего.
        assert_eq!(weapon_bar(BLOWPIPE_ID, &empty), None);
        assert!(weapon_tooltip(BLOWPIPE_ID, &empty).is_empty());
        // Арбалет: полоска — только заряженному, в подсказке — каким дротиком.
        assert_eq!(weapon_bar(TRANQ_CROSSBOW_ID, &empty), None);
        let loaded = WeaponState { load: vec![venom], gas: 0 };
        assert_eq!(weapon_bar(TRANQ_CROSSBOW_ID, &loaded).map(|b| b.0), Some(13));
        assert_eq!(weapon_tooltip(TRANQ_CROSSBOW_ID, &loaded), vec!["Заряжен: Яд, дротик".to_string()]);
        assert_eq!(weapon_tooltip(TRANQ_CROSSBOW_ID, &empty), vec!["Не заряжен".to_string()]);
        // Ружьё: полоска по магазину, красная без газа.
        let rifle = |load: Vec<&'static str>, gas| WeaponState { load, gas };
        assert_eq!(weapon_bar(AIR_RIFLE_ID, &rifle(vec![sedative; 4], 5)), Some((13, 0x8C_B4D2)));
        assert_eq!(weapon_bar(AIR_RIFLE_ID, &rifle(vec![sedative; 2], 5)).map(|b| b.0), Some(6));
        assert_eq!(weapon_bar(AIR_RIFLE_ID, &rifle(vec![], 0)), Some((0, 0xD2_4B4B)));
        assert_eq!(
            weapon_tooltip(AIR_RIFLE_ID, &rifle(vec![sedative; 3], 7)),
            vec!["Магазин: 3 из 4".to_string(), "Газ: 7 из 12 выстрелов".to_string()]
        );
        assert!(weapon_tooltip(AIR_RIFLE_ID, &rifle(vec![], 0))[1].contains("баллон"));
        assert_eq!(weapon_bar("biolab:dart_sedative", &empty), None);
        // Подсказки называют дальность и способ — как делает код.
        for weapon in &DART_WEAPONS {
            let hint = item_hint(weapon.item).join(" / ");
            assert!(hint.contains(&format!("{} блоков", weapon.range)) || hint.contains(&format!("{} блока", weapon.range)), "{}: {hint}", weapon.item);
            assert!(hint.contains("ПКМ"), "{}: {hint}", weapon.item);
        }
        assert!(item_hint(AIR_RIFLE_ID).join(" ").contains(&MAGAZINE_SIZE.to_string()));
        assert!(item_hint(CO2_CANISTER_ID).join(" ").contains(&CANISTER_SHOTS.to_string()));
        assert!(item_hint(CO2_CANISTER_EMPTY_ID).join(" ").contains("ферментер"));
    }

    #[test]
    fn weapons_and_canisters_exist_in_content() {
        let root = content_dir();
        for id in [BLOWPIPE_ID, TRANQ_CROSSBOW_ID, AIR_RIFLE_ID, CO2_CANISTER_ID, CO2_CANISTER_EMPTY_ID] {
            let short = id.trim_start_matches("biolab:");
            assert!(root.join(format!("data/biolab/horizon/item/{short}.json")).exists(), "{id}: нет предмета");
            assert!(root.join(format!("assets/biolab/items/{short}.json")).exists(), "{id}: нет модели");
        }
        // Оружие и пустой баллон — на лабораторном столе, полный баллон — из
        // пустого в ферментере (CO₂ брожения, GDD 4.2).
        for id in [BLOWPIPE_ID, TRANQ_CROSSBOW_ID, AIR_RIFLE_ID, CO2_CANISTER_EMPTY_ID] {
            let path = root.join(format!("data/biolab/recipe/{}.json", id.trim_start_matches("biolab:")));
            let json = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{id}: нет рецепта"));
            assert!(
                matches!(recipe_view(&json), Some(RecipeView::Bench { result, .. }) if result == id),
                "{id}: рецепт не стола"
            );
        }
        let json = std::fs::read_to_string(root.join("data/biolab/recipe/co2_canister.json")).unwrap();
        let Some(RecipeView::Machine { input, machine, result, .. }) = recipe_view(&json) else {
            panic!("баллон не заправляется в машине");
        };
        assert_eq!((input.as_str(), machine.as_str(), result.as_str()), (CO2_CANISTER_EMPTY_ID, "biolab:fermenter", CO2_CANISTER_ID));
        // Ванильный арбалет — сырьё транквилизатора: GDD 4.20, «арбалет + шприц + железо».
        let crossbow = std::fs::read_to_string(root.join("data/biolab/recipe/tranq_crossbow.json")).unwrap();
        assert!(crossbow.contains("minecraft:crossbow") && crossbow.contains("biolab:syringe"));
        let blowpipe = std::fs::read_to_string(root.join("data/biolab/recipe/blowpipe.json")).unwrap();
        assert_eq!(blowpipe.matches("minecraft:bamboo").count(), 3);
        assert!(blowpipe.contains("minecraft:string"));
        // Ружьё собирается с баллоном.
        let rifle = std::fs::read_to_string(root.join("data/biolab/recipe/air_rifle.json")).unwrap();
        assert!(rifle.contains(CO2_CANISTER_EMPTY_ID) && rifle.contains("minecraft:copper_ingot"));
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
        assert_eq!(animals, 12, "шесть животных из GDD 4.15, пять уродств из 4.19 и устойчивость из 4.20");
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

    /// Все звуки мода: машин и событий.
    fn all_sounds() -> Vec<Sound> {
        let mut all: Vec<Sound> = SOUNDS.iter().map(|(_, s)| *s).collect();
        for m in &MACHINE_SOUNDS {
            all.extend([m.start, m.work, m.done]);
        }
        all
    }

    #[test]
    fn every_machine_and_event_has_a_sound() {
        for machine in &MACHINES {
            for phase in [Phase::Start, Phase::Work, Phase::Done] {
                assert!(machine_sound(machine.id, phase).is_some(), "{} без звука", machine.id);
            }
        }
        assert_eq!(MACHINE_SOUNDS.len(), MACHINES.len(), "звуки у чужой машины");
        assert_eq!(machine_sound("minecraft:furnace", Phase::Start), None);
        for cue in Cue::ALL {
            let found = SOUNDS.iter().filter(|(c, _)| *c == cue).count();
            assert_eq!(found, 1, "{cue:?} в таблице не ровно один раз");
            assert!(sound(cue).is_some());
        }
        assert_eq!(SOUNDS.len(), Cue::ALL.len());
    }

    #[test]
    fn sounds_are_ours_or_vanilla_and_gentle() {
        for s in all_sounds() {
            let ns = s.event.split_once(':').map(|(ns, _)| ns);
            assert!(matches!(ns, Some("minecraft" | "biolab")), "{} — чужое пространство имён", s.event);
            // `native_play_sound` отказывает имени не по правилам `Identifier`.
            let path = &s.event[s.event.find(':').unwrap() + 1..];
            assert!(
                !path.is_empty()
                    && path.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.-/".contains(c)),
                "{}",
                s.event
            );
            // Ванилла зажимает высоту в 0,5–2,0; громче 1 — только дальность.
            assert!((0.5..=2.0).contains(&s.pitch), "{}: высота {}", s.event, s.pitch);
            assert!(s.volume > 0.0 && s.volume <= 1.0, "{}: громкость {}", s.event, s.volume);
            assert!(s.source == SOURCE_BLOCK || s.source == SOURCE_PLAYER);
        }
        // Фон машины тише её старта и результата: он звучит чаще всех.
        for m in &MACHINE_SOUNDS {
            assert!(m.work.volume <= m.start.volume.max(m.done.volume), "{}", m.machine);
        }
        // Машины — категория блоков, остальное делает игрок.
        assert!(MACHINE_SOUNDS.iter().all(|m| m.done.source == SOURCE_BLOCK));
        assert_eq!(sound(Cue::Pill).map(|s| s.source), Some(SOURCE_PLAYER));
    }

    /// Ванильный список звуковых событий 26.3 из ресурсов лаунчера: сначала
    /// реестр сервера (`registries.json`, по нему сервер и пропускает звук),
    /// иначе `sounds.json` клиента. Нет лаунчера — `None`, проверку пропускаем.
    fn vanilla_sound_list() -> Option<(std::path::PathBuf, String)> {
        let home = std::env::var_os("HOME")?;
        let versions =
            std::path::Path::new(&home).join("Library/Application Support/horizon-launcher/versions");
        let mut dirs: Vec<std::path::PathBuf> = std::fs::read_dir(versions)
            .ok()?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        dirs.sort();
        for file in ["registries.json", "assets/minecraft/sounds.json"] {
            for dir in dirs.iter().rev() {
                let path = dir.join(".local/horizon-client/26.3").join(file);
                if let Ok(text) = std::fs::read_to_string(&path) {
                    return Some((path, text));
                }
            }
        }
        None
    }

    #[test]
    fn sound_events_exist_in_vanilla_26_3() {
        let Some((path, text)) = vanilla_sound_list() else {
            eprintln!("ванильный список звуков 26.3 не найден — проверка пропущена");
            return;
        };
        let registry = path.ends_with("registries.json");
        // Свои звуки сверяет `own_sounds_are_declared_and_present`.
        for s in all_sounds().into_iter().filter(|s| s.event.starts_with("minecraft:")) {
            // В реестре имя полное, в `sounds.json` — без `minecraft:`. Кавычки
            // с обеих сторон: имя события, а не кусок другого.
            let name = if registry { s.event } else { s.event.trim_start_matches("minecraft:") };
            assert!(
                text.contains(&format!("\"{name}\"")),
                "{} нет в {}",
                s.event,
                path.display()
            );
        }
    }

    /// Свой звук, которого нет в `sounds.json` мода, клиент молча не играет,
    /// а файл, которого нет на диске, движок выбрасывает из события
    /// (`sound_events.rs`, как ванильный `SoundManager`). Поэтому сверяем:
    /// всё сыгранное объявлено, всё объявленное сыграно или отложено
    /// (`RESERVED_SOUNDS`), у каждого события есть файл и субтитр на обоих
    /// языках. Пересобрать звуки — `python3 tools/mksounds.py`.
    #[test]
    fn own_sounds_are_declared_and_present() {
        let assets = content_dir().join("assets/biolab");
        let text = std::fs::read_to_string(assets.join("sounds.json")).expect("нет sounds.json");
        let Some(Json::Obj(events)) = Json::parse(&text) else { panic!("sounds.json — не объект") };
        let declared: Vec<String> = events.iter().map(|(name, _)| format!("biolab:{name}")).collect();

        let mut played: Vec<&str> =
            all_sounds().iter().map(|s| s.event).filter(|e| e.starts_with("biolab:")).collect();
        played.sort();
        played.dedup();
        assert!(!played.is_empty(), "свои звуки пропали из таблиц");
        for event in &played {
            assert!(declared.iter().any(|d| d == event), "{event} играется, но не объявлен в sounds.json");
        }
        for event in RESERVED_SOUNDS {
            assert!(!played.contains(&event), "{event} уже играется — убрать из RESERVED_SOUNDS");
            assert!(declared.iter().any(|d| d == event), "{event} отложен, но не объявлен");
        }
        for event in &declared {
            let used = played.contains(&event.as_str()) || RESERVED_SOUNDS.contains(&event.as_str());
            assert!(used, "{event} объявлен в sounds.json, но код его не играет");
        }

        let (ru, en) = (lang("ru_ru"), lang("en_us"));
        for (name, entry) in &events {
            let subtitle = entry.get("subtitle").and_then(Json::str).unwrap_or_else(|| panic!("{name}: нет субтитра"));
            for (lang_name, lang) in [("ru_ru", &ru), ("en_us", &en)] {
                let text = lang.get(subtitle).and_then(Json::str).unwrap_or("");
                assert!(!text.trim().is_empty(), "{subtitle}: нет текста в {lang_name}");
            }
            let sounds = entry.get("sounds").and_then(Json::arr).unwrap_or_else(|| panic!("{name}: нет sounds"));
            assert!(!sounds.is_empty(), "{name}: пустой список файлов");
            for sound in sounds {
                // Запись — строка `biolab:<путь>` или объект с полем `name`.
                let file = sound.str().or_else(|| sound.get("name").and_then(Json::str)).unwrap();
                let path = file.strip_prefix("biolab:").unwrap_or_else(|| panic!("{name}: {file} — не наш файл"));
                let ogg = assets.join("sounds").join(format!("{path}.ogg"));
                let bytes = std::fs::read(&ogg).unwrap_or_else(|_| panic!("{name}: нет {}", ogg.display()));
                assert!(bytes.starts_with(b"OggS"), "{}: не Ogg", ogg.display());
                // Звуки короткие: крупный файл — знак, что синтез пошёл не так.
                assert!(bytes.len() < 64 * 1024, "{}: {} байт", ogg.display(), bytes.len());
            }
        }
    }

    #[test]
    fn machine_starts_once_hums_rarely_and_rings_when_done() {
        // Из простоя — старт, дальше тишина до фона.
        assert_eq!(machine_beat(false, Step::Working(1)), (Some(Phase::Start), true));
        assert_eq!(machine_beat(true, Step::Working(2)), (None, true));
        assert_eq!(machine_beat(true, Step::Working(MACHINE_HUM_EVERY)), (Some(Phase::Work), true));
        assert_eq!(machine_beat(true, Step::Working(MACHINE_HUM_EVERY + 1)), (None, true));
        let hums = (1..=400).filter(|p| machine_beat(true, Step::Working(*p)).0.is_some()).count();
        assert_eq!(hums, 5, "фон раз в четыре секунды, не каждый тик");
        // Готово, и следующий предмет стопки идёт без нового старта.
        assert_eq!(machine_beat(true, Step::Finish), (Some(Phase::Done), true));
        assert_eq!(machine_beat(true, Step::Working(1)), (None, true));
        // Простой сбрасывает отметку: следующая работа снова со стартом.
        assert_eq!(machine_beat(true, Step::Idle), (None, false));
        assert_eq!(machine_beat(false, Step::Idle), (None, false));
        // Рецепт в один тик: сразу готово.
        assert_eq!(machine_beat(false, Step::Finish), (Some(Phase::Done), true));
    }

    #[test]
    fn doses_and_injections_pick_their_sound() {
        let cue = |item: &str| medicine(item).map(|m| dose_cue(m.form));
        assert_eq!(cue("biolab:pill_penicillin"), Some(Cue::Pill));
        assert_eq!(cue("biolab:syringe_vaccine"), Some(Cue::SyringeSelf));
        assert_eq!(cue("biolab:ointment_analgesic"), Some(Cue::Ointment));
        assert_eq!(cue("biolab:spray_stimulant"), Some(Cue::Spray));
        assert_eq!(mob_injection_cue("biolab:syringe_venom"), Cue::SyringeMob);
        assert_eq!(mob_injection_cue("biolab:syringe_mutagen"), Cue::Mutagen);
    }

    // ---- Операционная ----

    #[test]
    fn limbs_follow_the_kind_and_cuts_never_run_out_of_markers() {
        assert_eq!(limbs_of("minecraft:zombie"), 4);
        assert_eq!(limbs_of("cow"), 4);
        assert_eq!(limbs_of("minecraft:chicken"), 2);
        assert_eq!(limbs_of("minecraft:villager"), 3);
        assert_eq!(limbs_of("minecraft:slime"), 0);
        // Вид без конечностей — один кусок тела; незнакомый — четыре.
        assert_eq!(cuts_of("minecraft:slime"), 1);
        assert_eq!(cuts_of("minecraft:squid"), 1);
        assert_eq!(cuts_of("othermod:beast"), 4);
        let mut seen = std::collections::HashSet::new();
        for (kind, limbs) in &LIMBS {
            assert!(seen.insert(*kind), "{kind} в таблице дважды");
            assert!(!kind.contains(':'), "{kind}: вид без пространства имён");
            assert!(*limbs < 4, "{kind}: четыре — значение по умолчанию, в таблицу не пишется");
            assert!((1..=4).contains(&cuts_of(kind)), "{kind}");
        }
        for kind in &HUMANOIDS {
            assert_eq!(limbs_of(kind), 4, "{kind}: гуманоид с четырьмя конечностями");
        }
    }

    #[test]
    fn markers_match_the_render_contract() {
        // Контракт с отрисовкой (GDD 4.17): имена меток не меняются.
        assert_eq!(SURGERY_TABLE, "biolab:surgery/table");
        assert_eq!(SURGERY_SLOW, "biolab:surgery/slow");
        for (i, marker) in SURGERY_FLAYED.iter().enumerate() {
            assert_eq!(*marker, format!("biolab:surgery/flayed/{i}"));
        }
        let mask = flayed_mask(|m| m == SURGERY_FLAYED[0] || m == SURGERY_FLAYED[2]);
        assert_eq!(mask, 0b101);
        assert_eq!(flayed_mask(|_| false), 0);
    }

    #[test]
    fn cuts_go_limb_by_limb_until_no_flesh_is_left() {
        assert_eq!(next_cut(0, 4), Some(0));
        assert_eq!(next_cut(0b0001, 4), Some(1));
        assert_eq!(next_cut(0b0101, 4), Some(1), "пропуск после отращивания заполняется первым");
        assert_eq!(next_cut(0b1111, 4), None, "плоти не осталось");
        // У курицы две ноги: третий срез уже не нужен.
        assert_eq!(next_cut(0b0011, cuts_of("minecraft:chicken")), None);
        assert_eq!(next_cut(0, cuts_of("minecraft:slime")), Some(0));
        assert_eq!(next_cut(0b0001, cuts_of("minecraft:slime")), None);
        // Меток всего четыре, сколько бы ни попросили.
        assert_eq!(next_cut(0b1111, 9), None);
    }

    #[test]
    fn flayed_limbs_slow_the_mob_and_regrow_one_by_one() {
        assert_eq!(surgery_slow(0), 0.0);
        assert!((surgery_slow(0b0011) - -0.2).abs() < 1e-9);
        assert!((surgery_slow(0b1111) - -0.4).abs() < 1e-9);
        assert_eq!(regrown(0b0101), Some(2), "отрастает последняя срезанная");
        assert_eq!(regrown(0), None);
        // Раз в минуту, у разных мобов — в разные тики.
        let due = (0..REGROW_EVERY * 3).filter(|t| regrow_due(*t, 7)).count();
        assert_eq!(due, 3);
        assert_ne!(
            (0..REGROW_EVERY).find(|t| regrow_due(*t, 1)),
            (0..REGROW_EVERY).find(|t| regrow_due(*t, 2))
        );
        // Срез ранит, но не добивает.
        assert_eq!(after_cut(20.0), 16.0);
        assert_eq!(after_cut(3.0), 1.0);
    }

    #[test]
    fn a_patient_survives_the_trip_on_the_stretcher() {
        let p = Patient {
            kind: "minecraft:zombie".to_string(),
            genes: vec![("chitin", 2), ("volatile", 1)],
            health: 13.5,
            flayed: 0b0010,
            name: Some("Боб; доктор=нет".to_string()),
            sleeper: Some("mandrake"),
            paralysed: false,
        };
        let text = patient_to_text(&p);
        assert_eq!(patient_from_text(&text), Some(p.clone()));
        // Парализованный кураре — тоже (GDD 4.20); незнакомое вещество забыто.
        let stiff = Patient { sleeper: Some("curare"), paralysed: true, ..p.clone() };
        assert_eq!(patient_from_text(&patient_to_text(&stiff)), Some(stiff));
        assert_eq!(patient_from_text("kind=cow;sleep=moonshine").and_then(|p| p.sleeper), None);
        assert_eq!(patient_from_text("kind=cow").map(|p| (p.sleeper, p.paralysed)), Some((None, false)));
        // Без имени и без генов — тоже.
        let plain = Patient { name: None, genes: Vec::new(), ..p };
        assert_eq!(patient_from_text(&patient_to_text(&plain)), Some(plain));
        // Вид без пространства имён дополняется, незнакомые поля пропускаются.
        let short = patient_from_text("kind=cow;health=4;mystery=1;flayed=99").unwrap();
        assert_eq!(short.kind, "minecraft:cow");
        assert_eq!(short.flayed, 99 & 0b1111);
        // Пустые носилки и битая запись — пациента нет.
        assert_eq!(patient_from_text(""), None);
        assert_eq!(patient_from_text("health=5"), None);
    }

    #[test]
    fn the_stretcher_says_who_is_on_it() {
        let p = Patient {
            kind: "minecraft:zombie".to_string(),
            genes: vec![("chitin", 2)],
            health: 12.0,
            flayed: 0b0011,
            name: None,
            sleeper: None,
            paralysed: false,
        };
        let text = patient_to_text(&p);
        assert_eq!(stretcher_label(&text).as_deref(), Some("Носилки: Зомби"));
        let stiff = patient_to_text(&Patient { paralysed: true, ..p.clone() });
        assert!(stretcher_tooltip(&stiff).iter().any(|l| l.starts_with("Парализован")));
        assert_eq!(item_label(STRETCHER_ID, |key| (key == PATIENT_KEY).then(|| text.clone())), stretcher_label(&text));
        assert_eq!(item_label(STRETCHER_ID, |_| None), None, "пустые носилки — просто «Носилки»");
        let lines = stretcher_tooltip(&text);
        assert_eq!(lines[0], "На носилках: Зомби");
        assert!(lines.iter().any(|l| l.contains("Хитиновый панцирь ++")), "{lines:?}");
        assert!(lines.iter().any(|l| l == "Срезано: 2 из 4"), "{lines:?}");
        assert_eq!(stretcher_tooltip(""), vec!["Пусто".to_string()]);
        let named = Patient { name: Some("Боб".to_string()), kind: "minecraft:modded_thing".to_string(), ..p };
        assert_eq!(patient_title(&named), "modded_thing «Боб»");
    }

    #[test]
    fn cuts_are_named_by_the_body() {
        assert_eq!(limb_name("minecraft:zombie", 0), "правая рука");
        assert_eq!(limb_name("minecraft:zombie", 3), "левая нога");
        assert_eq!(limb_name("minecraft:cow", 2), "правая задняя лапа");
        assert_eq!(limb_name("minecraft:slime", 0), "кусок тела");
        assert_eq!(limb_name("minecraft:chicken", 1), "конечность 2");
        let text = cut_message("minecraft:zombie", 0, 1, &[("chitin", 1)]);
        assert!(text.starts_with("Срезано: правая рука. Мутантная ткань ×1"), "{text}");
        assert!(can_carry("minecraft:zombie") && !can_carry("minecraft:wither"));
        assert_eq!(mob_title("minecraft:cow"), "Корова");
        let mut kinds: Vec<&str> = MOB_TITLES.iter().map(|(k, _)| *k).collect();
        let before = kinds.len();
        kinds.sort();
        kinds.dedup();
        assert_eq!(kinds.len(), before, "вид в таблице имён дважды");
    }

    #[test]
    fn the_patient_keeps_to_its_own_table() {
        let tables = [[0, 64, 0], [3, 64, 0]];
        let is_table = |p: [i32; 3]| tables.contains(&p);
        // Лежит на столе — этот стол и находится.
        assert_eq!(nearest_table([0.5, 65.0, 0.5], is_table), Some([0, 64, 0]));
        // Сползло на соседний блок — стол всё ещё свой.
        assert_eq!(nearest_table([1.4, 65.0, 0.5], is_table), Some([0, 64, 0]));
        // Стола рядом нет — пациент свободен.
        assert_eq!(nearest_table([10.5, 65.0, 0.5], is_table), None);
        let spot = table_spot([0, 64, 0]);
        assert_eq!(spot, [0.5, 65.0, 0.5]);
        assert!(!off_table([0.55, 65.0, 0.5], spot));
        assert!(off_table([1.0, 65.0, 0.5], spot));
        assert!(is_air("minecraft:air") && !is_air("minecraft:stone"));
    }

    /// Хитбоксы ванильных видов (ширина, рост) — как в `entity-types.json`,
    /// откуда их берёт `MobType` Horizon.
    const HITBOXES: [(&str, f64, f64); 9] = [
        ("minecraft:chicken", 0.4, 0.7),
        ("minecraft:zombie", 0.6, 1.95),
        ("minecraft:skeleton", 0.6, 1.99),
        ("minecraft:cow", 0.9, 1.4),
        ("minecraft:spider", 1.4, 0.9),
        ("minecraft:ravager", 1.95, 2.2),
        ("minecraft:giant", 3.6, 12.0),
        ("minecraft:enderman", 0.6, 2.9),
        ("minecraft:turtle", 1.2, 0.4),
    ];

    fn sections(kind: &str, genes: &[(&str, i32)]) -> i32 {
        let (_, w, h) = HITBOXES.iter().find(|(k, _, _)| *k == kind).expect(kind);
        table_sections(kind, *w, *h, genes)
    }

    #[test]
    fn the_table_is_as_long_as_the_patient() {
        // Ориентиры GDD 4.17: курица — одна секция, зомби, скелет, корова и
        // паук — две, разоритель и гигант с гигантизмом — три и больше.
        assert_eq!(sections("minecraft:chicken", &[]), 1);
        assert_eq!(sections("minecraft:zombie", &[]), 2);
        assert_eq!(sections("minecraft:skeleton", &[]), 2, "1,99 — не повод для третьей секции");
        assert_eq!(sections("minecraft:cow", &[]), 2, "длинная: длина тела, а не рост");
        assert_eq!(sections("minecraft:spider", &[]), 2, "в высоту 0,9, в длину 1,6");
        assert_eq!(sections("minecraft:ravager", &[]), 3);
        assert_eq!(sections("minecraft:enderman", &[]), 3);
        assert_eq!(sections("minecraft:giant", &[]), 12);
        assert_eq!(sections("minecraft:turtle", &[]), 2, "плоский: длина по ширине");
        // Гигантизм растягивает, карликовость сжимает.
        assert!(sections("minecraft:zombie", &[("gigantism", 3)]) >= 3);
        assert_eq!(sections("minecraft:zombie", &[("gigantism", 1)]), 3, "1,95 × 1,22 = 2,38");
        assert_eq!(sections("minecraft:zombie", &[("dwarfism", 3)]), 1, "1,95 × 0,46 < 1");
        assert_eq!(sections("minecraft:chicken", &[("gigantism", 3)]), 2);
        // Гены без размера стол не меняют.
        assert_eq!(sections("minecraft:zombie", &[("bioluminescence", 3), ("regen", 2)]), 2);
        // Ровно в блок — одна секция, чуть больше — две; ноль и мусор — одна.
        assert_eq!(sections_for_length(1.0, 1.0), 1);
        assert_eq!(sections_for_length(1.05, 1.0), 2);
        assert_eq!(sections_for_length(0.0, 1.0), 1);
        assert_eq!(sections_for_length(f64::NAN, 1.0), 1);
        assert_eq!(
            table_too_short(3, 2),
            "Пациент не помещается: нужен стол из 3 секций, здесь 2"
        );
        let mut seen = std::collections::HashSet::new();
        for (kind, length) in &LONG_BODIES {
            assert!(seen.insert(*kind), "{kind} в таблице дважды");
            assert!(!kind.contains(':'), "{kind}: вид без пространства имён");
            assert!(*length > 0.3 && *length < 4.0, "{kind}: {length}");
        }
        // Подсказка и журнал называют те же числа, что считает стол.
        let zombie = format!("зомби — {}", sections("minecraft:zombie", &[]));
        assert!(item_hint(OPERATING_TABLE).join(" ").contains(&zombie), "{:?}", item_hint(OPERATING_TABLE));
        let journal = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/research/ru.txt"),
        )
        .unwrap();
        assert!(journal.contains(&format!("{zombie} секции")), "в журнале нет «{zombie} секции»");
    }

    /// Масштаб генома — тот же, что рисует `biolab-deep`: читаем его таблицу
    /// `STEPS` и имена генов прямо из исходника `mutants.rs`.
    #[test]
    fn the_body_scale_matches_the_renderer() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../biolab-deep/mixins/biolab_deep/mutants.rs");
        let source = std::fs::read_to_string(&path).expect("исходник biolab-deep");
        let names: Vec<&str> = {
            let start = source.find("pub const GENE_NAMES").expect("GENE_NAMES");
            let body = &source[start..];
            let body = &body[body.find('[').unwrap() + 1..];
            let body = &body[body.find('[').unwrap() + 1..body.find("];").unwrap()];
            body.split(',').map(|s| s.trim().trim_matches('"')).filter(|s| !s.is_empty()).collect()
        };
        let steps: Vec<f64> = {
            let start = source.find("const STEPS").expect("STEPS");
            let body = &source[start..];
            let body = &body[body.find("= [").unwrap() + 3..];
            let body = &body[..body.find("];").unwrap()];
            body.split(',').map(|s| s.trim().parse().unwrap()).collect()
        };
        assert_eq!(names.len(), steps.len(), "{names:?}");
        for (name, step) in names.iter().zip(&steps) {
            let ours = GENE_SCALE.iter().find(|(g, _)| g == name).map_or(0.0, |(_, s)| *s);
            assert!((ours - step).abs() < 1e-9, "{name}: у нас {ours}, в отрисовке {step}");
        }
        for (name, _) in &GENE_SCALE {
            assert!(names.contains(name), "{name}: такого гена нет в biolab-deep");
        }
        assert!((body_scale(&[("gigantism", 3)]) - 1.66).abs() < 1e-9);
        assert!((body_scale(&[("gigantism", 1), ("dwarfism", 1)]) - 1.22 * 0.82).abs() < 1e-9);
        assert_eq!(body_scale(&[]), 1.0);
    }

    #[test]
    fn sections_in_a_line_make_one_table() {
        // Три секции вдоль X и одна отдельная.
        let tables = [[0, 64, 0], [1, 64, 0], [2, 64, 0], [10, 64, 10]];
        let is_table = |p: [i32; 3]| tables.contains(&p);
        let row = table_row([1, 64, 0], is_table);
        assert_eq!(row, TableRow { start: [0, 64, 0], step: [1, 0, 0], len: 3 });
        // С какой секции ни кликни — тот же стол.
        assert_eq!(table_row([2, 64, 0], is_table), row);
        assert_eq!(row.sections(), vec![[0, 64, 0], [1, 64, 0], [2, 64, 0]]);
        // Пациент — по центру ряда, а не той секции, по которой кликнули.
        assert_eq!(row.center(), [1.5, 65.0, 0.5]);
        assert!(row.along_x());
        assert_eq!(row.yaw(), -90.0, "вдоль X смотрит на восток, голова на западе");
        // Две секции — стык между ними.
        let pair = [[5, 70, 5], [5, 70, 6]];
        let row = table_row([5, 70, 6], |p| pair.contains(&p));
        assert_eq!(row, TableRow { start: [5, 70, 5], step: [0, 0, 1], len: 2 });
        assert_eq!(row.center(), [5.5, 71.0, 6.0]);
        assert_eq!(row.yaw(), 0.0, "вдоль Z смотрит на юг, голова на севере");
        // Одинокая секция — ряд из одной, вдоль X.
        let one = table_row([10, 64, 10], is_table);
        assert_eq!(one, TableRow { start: [10, 64, 10], step: [1, 0, 0], len: 1 });
        assert_eq!(one.center(), [10.5, 65.0, 10.5]);
        // Крест: ось, где секций больше.
        let cross = [[0, 0, 0], [1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, 2], [0, 0, -1], [0, 0, -2]];
        let row = table_row([0, 0, 0], |p| cross.contains(&p));
        assert_eq!((row.step, row.len, row.start), ([0, 0, 1], 5, [0, 0, -2]));
        // Бесконечный ряд обрезается.
        let long = table_row([0, 0, 0], |p| p[1] == 0 && p[2] == 0);
        assert!(long.len <= TABLE_MAX_SECTIONS);
        // Пациент над рядом — на нём; рядом с рядом — нет.
        let row = table_row([1, 64, 0], is_table);
        assert!(row.covers(row.center()));
        assert!(row.covers([0.1, 65.0, 0.5]) && row.covers([2.9, 65.0, 0.5]));
        assert!(!row.covers([4.0, 65.0, 0.5]));
        assert!(!row.covers([1.5, 65.0, 2.5]));
        assert!(!row.covers([1.5, 70.0, 0.5]));
    }

    #[test]
    fn sides_open_where_the_neighbour_is_gone() {
        assert_eq!(table_side_key([0, 0, -1]), Some("north"));
        assert_eq!(table_side_key([1, 0, 0]), Some("east"));
        assert_eq!(table_side_key([0, -1, 0]), Some("down"));
        assert_eq!(table_side_key([1, 1, 0]), None);
        // Свойства — ровно шесть сторон гриба.
        let keys: std::collections::BTreeSet<_> =
            TABLE_SIDES.iter().filter_map(|n| table_side_key(*n)).collect();
        assert_eq!(keys.len(), 6);
        // Секция в середине ряда вдоль X: запад и восток соединены, остальное открыто.
        let tables = [[0, 0, 0], [1, 0, 0], [2, 0, 0]];
        let sides = table_sides([1, 0, 0], |p| tables.contains(&p));
        assert!(sides.contains(&("west", "false")) && sides.contains(&("east", "false")));
        assert!(sides.contains(&("north", "true")) && sides.contains(&("up", "true")));
        // Соседа сломали — сторона снова открыта (ванилла сама её не вернёт).
        let broken = [[1, 0, 0], [2, 0, 0]];
        let sides = table_sides([1, 0, 0], |p| broken.contains(&p));
        assert!(sides.contains(&("west", "true")) && sides.contains(&("east", "false")));
        assert_eq!(sides.len(), 6);
        assert!(table_sides_due(40) && !table_sides_due(41));
    }

    #[test]
    fn the_operating_room_runs_once_a_tick_and_settles_health() {
        assert!(surgery_pass_due(1_000_001));
        assert!(!surgery_pass_due(1_000_001), "второй игрок в том же тике ждёт");
        assert!(surgery_pass_due(1_000_002));
        settle_health(77, 9.5, 2_000_005);
        assert_eq!(due_settles(2_000_000).iter().find(|(id, _)| *id == 77), Some(&(77, 9.5)));
        assert_eq!(due_settles(2_000_006).iter().find(|(id, _)| *id == 77), None, "срок вышел");
    }

    #[test]
    fn surgery_sounds_exist() {
        for cue in [Cue::Stretcher, Cue::Strap, Cue::Tissue] {
            assert!(sound(cue).is_some(), "{cue:?}");
        }
    }
}

// ---- Карта исследований ----
//
// «Журнал лаборанта» — карта, как в Таумономиконе: пять наук, в каждой узлы,
// связанные линиями. Узел загорается сам, когда игрок сделал что-то новое:
// получил предмет, опознал вещество, заразился, принял лекарство. Здесь
// только данные и чистая логика: таблица узлов, условия, раскладка экрана,
// попадание мышью, клавиши, колесо и разбор страниц. Экран — свой экран мода
// (`native_mod_screen_open`, API 111) на стабильных хуках: открывает его
// `research_open.rs` / `research_net.rs`, рисует `research_draw.rs`, мышь,
// колесо и клавиши — `research_click.rs`, `research_release.rs`,
// `research_scroll.rs`, `research_key.rs`, отложенное (закрыть, щелчок) —
// `research_frame.rs`.

/// Канал, по которому сервер раз в секунду сообщает клиенту изученные узлы.
pub const NET_RESEARCH: &str = "biolab_core:research";
/// Канал «открой журнал»: сервер шлёт его игроку, применившему журнал. Решает
/// сервер, как у ванильной книги: блок под прицелом (машина, сундук) успевает
/// забрать щелчок раньше, и тогда журнал не открывается.
pub const NET_JOURNAL: &str = "biolab_core:journal";
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

pub const RESEARCH: [Research; 62] = [
    // Основы
    node("manual", 0, (0, 1), "biolab:lab_manual", "Журнал лаборанта", &[], Unlock::Start, "Есть с самого начала."),
    node("bench", 0, (1, 1), "biolab:lab_bench", "Лабораторный стол", &["manual"], Unlock::Item(&["biolab:lab_bench"]), "Соберите лабораторный стол на верстаке."),
    node("flask", 0, (2, 0), "biolab:flask", "Колба", &["bench"], Unlock::Item(&["biolab:flask"]), "Сделайте колбу на верстаке: пузырёк и глина."),
    node("analyzer", 0, (3, 0), "biolab:analyzer", "Анализатор крови", &["flask"], Unlock::Item(&["biolab:analyzer"]), "Соберите анализатор на лабораторном столе."),
    node("swab", 0, (2, 2), "biolab:swab", "Ватная палочка", &["bench"], Unlock::Item(&["biolab:swab"]), "Соберите ватную палочку на столе."),
    node("agar", 0, (3, 2), "biolab:agar", "Агар", &["swab"], Unlock::Item(&["biolab:agar"]), "Сушёную ламинарию — в перегонный куб."),
    node("bandage", 0, (4, 2), "biolab:bandage", "Бинт", &["agar"], Unlock::Item(&["biolab:bandage"]), "Соберите бинт на столе: шерсть и агар."),
    // Свои растения (GDD 4.20): нижний ряд Основ, каждое — само по себе. Сырьё
    // для снотворных, но ищут их в мире, ещё до лаборатории.
    node("valerian", 0, (0, 3), "biolab:valerian_root", "Валериана", &[], Unlock::Item(&["biolab:valerian_seeds", "biolab:valerian_root"]), "Сорвите валериану на лугу или равнине."),
    node("belladonna", 0, (1, 3), "biolab:belladonna_berries", "Белладонна", &[], Unlock::Item(&["biolab:belladonna_seeds", "biolab:belladonna_berries"]), "Сорвите белладонну в лесу."),
    node("mandrake", 0, (2, 3), "biolab:mandrake_root", "Мандрагора", &[], Unlock::Item(&["biolab:mandrake_seeds", "biolab:mandrake_root"]), "Сорвите мандрагору на болоте."),
    // Химия
    node("still", 1, (0, 1), "biolab:still", "Перегонный куб", &[], Unlock::Item(&["biolab:still"]), "Сделайте перегонный куб: медь и печь."),
    node("elements", 1, (1, 1), "biolab:element_carbon", "Элементы", &["still"], Unlock::Item(ELEMENT_ITEMS), "Положите песок или древесный уголь в куб."),
    node("synth", 1, (2, 1), "biolab:synthesizer", "Синтезатор", &["elements"], Unlock::Item(&["biolab:synthesizer"]), "Сделайте синтезатор на верстаке."),
    node("bonds", 1, (2, 0), "biolab:element_hydrogen", "Правило связей", &["synth"], Unlock::Discovered(1), "Соберите вещество в синтезаторе, хоть H2O."),
    node("spectro", 1, (3, 1), "biolab:spectrometer", "Спектрометр", &["synth"], Unlock::Item(&["biolab:spectrometer"]), "Сделайте спектрометр: нужен анализатор."),
    node("identify", 1, (4, 0), "biolab:flask", "Опознание", &["spectro"], Unlock::Identified(1), "Положите колбу с веществом в спектрометр."),
    node("blueprint", 1, (4, 1), "biolab:blueprint", "Чертежи", &["spectro"], Unlock::Item(&["biolab:blueprint"]), "Заберите чертёж из выхода спектрометра."),
    node("reactor", 1, (3, 2), "biolab:reactor", "Реактор", &["synth"], Unlock::Item(&["biolab:reactor"]), "Сделайте реактор: железо и доменная печь."),
    node("collection", 1, (5, 0), "biolab:element_gold", "Десять веществ", &["identify"], Unlock::Identified(10), "Опознайте спектрометром десять веществ."),
    node("separation", 1, (2, 2), "biolab:centrifuge", "Разделение", &["synth"], Unlock::Item(&["biolab:centrifuge"]), "Сделайте центрифугу на верстаке."),
    // Микробиология
    node("incubator", 2, (0, 1), "biolab:incubator", "Инкубатор", &[], Unlock::Item(&["biolab:incubator"]), "Сделайте инкубатор: медь, стекло, печь."),
    node("culture", 2, (1, 1), "biolab:culture", "Культура", &["incubator"], Unlock::Item(&["biolab:culture"]), "Положите ватную палочку в инкубатор."),
    node("strain", 2, (2, 0), "biolab:microscope", "Штамм", &["culture"], Unlock::Item(&["biolab:microscope"]), "Соберите микроскоп на лабораторном столе."),
    node("weakened", 2, (2, 1), "biolab:culture_weakened", "Ослабленная культура", &["culture"], Unlock::Item(&["biolab:culture_weakened"]), "Культуру — в реактор над костром, 250–400 °C."),
    node("vaccine", 2, (3, 1), "biolab:syringe_vaccine", "Вакцина", &["weakened"], Unlock::Item(&["biolab:syringe_vaccine"]), "Соберите на столе шприц и ослабленный штамм."),
    node("immunity", 2, (4, 1), "minecraft:golden_apple", "Иммунитет", &["vaccine"], Unlock::Immune, "Вакцину в руку и ПКМ в воздух."),
    node("infection", 2, (0, 3), "minecraft:rotten_flesh", "Заражение", &[], Unlock::Infected, "Съешьте гнилую плоть или подойдите к зомби."),
    node("remedy", 2, (1, 3), "biolab:remedy", "Сыворотка", &["infection"], Unlock::Item(&["biolab:remedy"]), "Прогоните культуру: ферментер, центрифуга."),
    node("hardened", 2, (2, 3), "minecraft:fermented_spider_eye", "Стойкий штамм", &["remedy"], Unlock::Hardened, "Бросьте курс пенициллина, не долечившись."),
    // Фармакология
    node("mold", 3, (0, 1), "biolab:mold", "Плесень", &[], Unlock::Item(&["biolab:mold"]), "Положите хлеб в инкубатор."),
    node("fermenter", 3, (1, 1), "biolab:fermenter", "Ферментер", &["mold"], Unlock::Item(&["biolab:fermenter"]), "Сделайте ферментер: медь, бочка, печь."),
    node("extracts", 3, (2, 1), "biolab:extract_penicillin", "Экстракты", &["fermenter"], Unlock::Item(&["biolab:extract_penicillin", "biolab:extract_salicylate", "biolab:extract_caffeine", "biolab:extract_sedative", "biolab:extract_venom", "biolab:extract_raw"]), "Положите плесень или какао в ферментер."),
    node("pills", 3, (3, 0), "biolab:press", "Таблетки", &["extracts"], Unlock::Item(&["biolab:pill_penicillin", "biolab:pill_analgesic", "biolab:pill_stimulant", "biolab:pill_antidote"]), "Сделайте пресс и положите в него экстракт."),
    node("injections", 3, (3, 1), "biolab:syringe", "Инъекции", &["extracts"], Unlock::Item(&["biolab:syringe_penicillin", "biolab:syringe_sedative", "biolab:syringe_stimulant", "biolab:syringe_venom"]), "Соберите на столе шприц с экстрактом."),
    node("topical", 3, (3, 2), "biolab:ointment_analgesic", "Мази и аэрозоли", &["extracts"], Unlock::Item(&["biolab:ointment_analgesic", "biolab:spray_analgesic", "biolab:spray_stimulant"]), "Соберите мазь или аэрозоль на столе."),
    node("antidote", 3, (2, 3), "biolab:pill_antidote", "Антидот", &["extracts"], Unlock::Item(&["biolab:pill_antidote"]), "Положите древесный уголь в пресс."),
    node("window", 3, (4, 0), "biolab:pill_analgesic", "Терапевтическое окно", &["pills"], Unlock::Medicine, "Примите лекарство: в руку и ПКМ в воздух."),
    node("overdose", 3, (5, 0), "biolab:pill_stimulant", "Передозировка", &["window"], Unlock::Overdose, "Примите двойную дозу или дождитесь ломки."),
    node("darts", 3, (4, 1), "biolab:dart_sedative", "Дротики", &["injections"], Unlock::Item(&["biolab:dart_sedative", "biolab:dart_venom"]), "Соберите дротик: шприц, перо и палка."),
    // Оружие для дротиков (GDD 4.20): трубка рано, арбалет в середине, ружьё
    // поздно — после баллона CO₂, из которого оно стреляет.
    node("blowpipe", 3, (5, 1), "biolab:blowpipe", "Духовая трубка", &["darts"], Unlock::Item(&["biolab:blowpipe"]), "Соберите трубку на столе: бамбук и нить."),
    node("tranq_crossbow", 3, (4, 2), "biolab:tranq_crossbow", "Арбалет-транквилизатор", &["darts"], Unlock::Item(&["biolab:tranq_crossbow"]), "Соберите на столе: арбалет, шприц, железо."),
    node("co2", 3, (4, 3), "biolab:co2_canister", "Баллон CO₂", &["tranq_crossbow"], Unlock::Item(&["biolab:co2_canister", "biolab:co2_canister_empty"]), "Соберите баллон на столе: железо и медь."),
    node("air_rifle", 3, (5, 3), "biolab:air_rifle", "Пневматическое ружьё", &["co2"], Unlock::Item(&["biolab:air_rifle"]), "Соберите ружьё на столе: нужен баллон."),
    // Генетика
    node("scalpel", 4, (0, 1), "biolab:scalpel", "Скальпель", &[], Unlock::Item(&["biolab:scalpel"]), "Соберите скальпель на лабораторном столе."),
    // Операционная (GDD 4.17): носилки и стол над скальпелем, ткань — там,
    // где сходятся обе ветки.
    node("stretcher", 4, (0, 0), "biolab:stretcher", "Носилки", &[], Unlock::Item(&["biolab:stretcher"]), "Сделайте носилки на верстаке: палки и кожа."),
    node("operating_table", 4, (1, 0), "biolab:operating_table", "Операционный стол", &["stretcher"], Unlock::Item(&["biolab:operating_table"]), "Сделайте стол на верстаке: железо и шерсть."),
    node("tissue", 4, (1, 1), "biolab:mutant_tissue", "Ткань мутанта", &["scalpel", "operating_table"], Unlock::Item(&["biolab:mutant_tissue"]), "Срежьте скальпелем с мутанта на столе."),
    node("extractor", 4, (2, 1), "biolab:extractor", "Экстрактор ДНК", &["tissue"], Unlock::Item(&["biolab:extractor"]), "Сделайте экстрактор ДНК: нужен микроскоп."),
    node("fragment", 4, (3, 1), "biolab:dna_fragment", "Фрагмент ДНК", &["extractor"], Unlock::Item(&["biolab:dna_fragment"]), "Положите мутантную ткань в экстрактор."),
    node("mutagen", 4, (2, 3), "biolab:mutagen", "Мутаген", &["extractor"], Unlock::Item(&["biolab:mutagen", "biolab:mutagen_bucket", "biolab:syringe_mutagen"]), "Соберите мутаген на лабораторном столе."),
    node("splicing", 4, (4, 0), "biolab:microscope", "Сплайсинг", &["fragment"], Unlock::Item(&["biolab:microscope"]), "Соберите микроскоп на лабораторном столе."),
    node("vat", 4, (5, 0), "biolab:cloning_vat", "Клонирующий чан", &["splicing"], Unlock::Item(&["biolab:cloning_vat"]), "Сделайте клонирующий чан: нужен ферментер."),
    node("pen", 4, (5, 1), "biolab:pen", "Вольер и пульт", &["vat"], Unlock::Item(&["biolab:pen", "biolab:console"]), "Сделайте вольер из решёток или пульт."),
    node("therapy", 4, (4, 2), "biolab:syringe", "Генотерапия", &["fragment"], Unlock::Gene, "Фрагмент ДНК в руку и ПКМ в воздух."),
    node("crops", 4, (5, 2), "biolab:gm_plot", "ГМ-грядка", &["therapy"], Unlock::Item(&["biolab:gm_plot", "biolab:gm_crop"]), "Сделайте ГМ-грядку: земля, агар, мука."),
    node("animal_genes", 4, (3, 2), "minecraft:rabbit_foot", "Гены животных", &["extractor"], Unlock::Item(&["minecraft:rabbit_foot", "minecraft:glow_ink_sac", "minecraft:turtle_scute", "minecraft:magma_cream"]), "Добудьте кроличью лапку или сгусток магмы."),
    // Уродства (GDD 4.19): восемь генов, которые видно на теле. Узел — под
    // генами животных: четыре из них тоже достаются из частей животных.
    node("deformities", 4, (3, 3), "minecraft:goat_horn", "Уродства", &["animal_genes"], Unlock::Item(&["minecraft:goat_horn", "minecraft:ink_sac", "minecraft:phantom_membrane", "minecraft:poisonous_potato"]), "Добудьте козий рог или мембрану фантома."),
    // Линейка снотворных (GDD 4.20, M5) — в фармакологии. В конце таблицы:
    // изученное хранится по имени, а предшественники стоят выше.
    node("herbs", 3, (1, 2), "biolab:calming_brew", "Травы: отвар и мандрагора", &["fermenter"], Unlock::Item(&["biolab:calming_brew", "biolab:syringe_calming", "biolab:mandrake_mash", "biolab:tincture_mandrake", "biolab:syringe_mandrake", "biolab:dart_mandrake"]), "Сварите корень валерианы в перегонном кубе."),
    node("sleep_scale", 3, (2, 0), "biolab:dart_tranquilizer", "Сила против массы", &["injections"], Unlock::Item(&["biolab:calming_brew", "biolab:syringe_calming", "biolab:syringe_mandrake", "biolab:dart_mandrake", "biolab:spray_ether", "biolab:grenade_ether", "biolab:rag_chloroform", "biolab:syringe_curare", "biolab:dart_curare", "biolab:syringe_tranquilizer", "biolab:dart_tranquilizer"]), "Сделайте снотворное, кроме седативного."),
    node("gases", 3, (5, 2), "biolab:grenade_ether", "Эфир и хлороформ", &["topical"], Unlock::Item(&["biolab:ether", "biolab:chloroform", "biolab:spray_ether", "biolab:spray_chloroform", "biolab:grenade_ether", "biolab:grenade_chloroform", "biolab:rag_chloroform"]), "Спирт с серой — на стол, смесь — в реактор."),
    node("tranquilizer", 3, (1, 0), "biolab:syringe_tranquilizer", "Транквилизатор зверолова", &["sleep_scale"], Unlock::Item(&["biolab:tranquilizer", "biolab:syringe_tranquilizer", "biolab:dart_tranquilizer", "biolab:syringe_wake"]), "Седативное, настойку и золото — в синтезатор."),
    node("curare", 3, (3, 3), "biolab:dart_curare", "Кураре", &["antidote"], Unlock::Item(&["biolab:curare_mash", "biolab:curare_raw", "biolab:extract_curare", "biolab:syringe_curare", "biolab:dart_curare"]), "Лианы, паучий глаз и колбу — на стол."),
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

// Вёрстка страницы записи. Одно правило на отрисовку (`research_draw.rs`) и
// на тест «страница влезает в экран»: отрисовка меряет текст шрифтом игры,
// тест — оценкой с запасом, а переносы, высоты строк и низ страницы общие.

/// Где на странице текст: левый край, верх первой строки, ширина строки и
/// низ, ниже которого рисовать нельзя — под ним кнопки «<», «к карте», «>».
pub fn page_area(l: &ResearchLayout) -> [f32; 4] {
    let [x, y, w, _] = l.page;
    [x + 10.0, y + 8.0, w - 20.0, page_buttons(l)[0][1] - 2.0]
}

/// Строка страницы на экране: заголовок, одна строка текста после переноса,
/// отбивка, ряд иконок или рецепт.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageRow {
    Title(String),
    Text(String),
    Gap,
    Items(Vec<String>),
    Recipe(RecipeView),
}

impl PageRow {
    /// Высота рисунка строки и шаг до следующей, в пикселях интерфейса:
    /// заголовок — текст и черта под ним, строка текста — 8, иконки — 16,
    /// сетка 3×3 из слотов по 18, у стола под сеткой ещё подпись, строка
    /// машины — один слот.
    pub fn height(&self) -> (f32, f32) {
        match self {
            PageRow::Title(_) => (12.0, 15.0),
            PageRow::Text(_) => (8.0, 10.0),
            PageRow::Gap => (0.0, 5.0),
            PageRow::Items(_) => (16.0, 19.0),
            PageRow::Recipe(RecipeView::Grid { .. }) => (54.0, 58.0),
            PageRow::Recipe(RecipeView::Bench { .. }) => (65.0, 70.0),
            PageRow::Recipe(RecipeView::Machine { .. }) => (18.0, 22.0),
        }
    }
}

/// Перенос по словам: строка растёт, пока влезает в ширину; слово длиннее
/// строки встаёт одно. Ширину меряет `measure`: в игре шрифт, в тестах оценка.
pub fn wrap_text(text: &str, width: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if line.is_empty() || measure(&candidate) <= width {
            line = candidate;
        } else {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Страница строками экрана: текст перенесён по ширине, рецепты разобраны.
/// Рецепт, который не нашёлся или не разобрался, пропускается.
pub fn page_rows(
    page: &str,
    width: f32,
    measure: impl Fn(&str) -> f32,
    recipe: impl Fn(&str) -> Option<RecipeView>,
) -> Vec<PageRow> {
    let mut rows = Vec::new();
    for line in page_lines(page) {
        match line {
            Line::Title(title) => rows.push(PageRow::Title(title)),
            Line::Text(text) => {
                rows.extend(wrap_text(&text, width, &measure).into_iter().map(PageRow::Text));
            }
            Line::Gap => rows.push(PageRow::Gap),
            Line::Items(items) => rows.push(PageRow::Items(items)),
            Line::Craft(name) => {
                if let Some(view) = recipe(&name) {
                    rows.push(PageRow::Recipe(view));
                }
            }
        }
    }
    rows
}

/// Сколько строк страницы влезет: строка рисуется, если её рисунок кончается
/// не ниже `bottom`. Первая невлезшая и всё после неё не рисуются.
pub fn rows_fitting(rows: &[PageRow], top: f32, bottom: f32) -> usize {
    let mut row = top;
    for (i, line) in rows.iter().enumerate() {
        let (drawn, step) = line.height();
        if row + drawn > bottom {
            return i;
        }
        row += step;
    }
    rows.len()
}

/// Ширина строки шрифтом Minecraft, оценка с запасом — для проверок вёрстки
/// без игры. Буквы и цифры по 6 пикселей, широкие кириллические — 8, пробел
/// 4, точки и запятые 3, тире, стрелка и «№» — 9. Оценка не меньше настоящей
/// ширины, поэтому переносов по ней выходит не меньше, чем в игре.
pub fn estimated_text_width(text: &str) -> f32 {
    text.chars()
        .map(|c| match c {
            ' ' => 4.0,
            '.' | ',' | ':' | ';' | '!' | '\'' | '|' | 'i' | 'l' => 3.0,
            'Ж' | 'Ш' | 'Щ' | 'Ы' | 'Ю' | 'Ф' | 'М' | 'ж' | 'ш' | 'щ' | 'ы' | 'ю' | 'ф' | 'м' => 8.0,
            '—' | '→' | '№' | '…' => 9.0,
            '–' => 7.0,
            _ => 6.0,
        })
        .sum()
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

/// Сколько страниц у открытой записи (1 — на карте): кнопкам, клавишам и
/// колесу, чтобы не листать за последнюю.
pub fn research_page_count(view: &ResearchView, text: &str) -> usize {
    view.page.map_or(1, |(node, _)| research_pages(text, node).len())
}

/// Имя клавиши для `research_key` из ванильного (`key.keyboard.e`, как его
/// отдаёт `native_screen_key_pressed_pre`). None — клавиша журналу не нужна:
/// её не глотаем, пусть делает своё (F2, F11 и прочее работают и над экраном).
pub fn research_key_name(vanilla: &str) -> Option<&'static str> {
    let key = vanilla.strip_prefix("key.keyboard.")?;
    // Цифры с цифрового блока — те же науки.
    let key = key.strip_prefix("keypad.").unwrap_or(key);
    Some(match key {
        "escape" => "escape",
        "backspace" => "backspace",
        "e" => "e",
        "left" => "left",
        "a" => "a",
        "right" => "right",
        "d" => "d",
        "1" => "1",
        "2" => "2",
        "3" => "3",
        "4" => "4",
        "5" => "5",
        _ => return None,
    })
}

/// Что сделать с нажатой клавишей: новый вид, отменить ли её дальше и
/// закрыть ли экран.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResearchKeyPress {
    pub view: ResearchView,
    /// Клавиша наша: дальше её не пускать.
    pub cancel: bool,
    /// Закрыть экран мода. Хук клавиш получает игру только на чтение, так что
    /// закрывает кадровый хук (`research_frame.rs`) по просьбе отсюда.
    pub close: bool,
}

pub fn research_key_press(view: ResearchView, vanilla: &str, pages: usize) -> ResearchKeyPress {
    let Some(name) = research_key_name(vanilla) else {
        return ResearchKeyPress { view, cancel: false, close: false };
    };
    let (next, close) = research_key(view, name, pages);
    // Escape на карте закрывает свой экран мода сама игра — не мешаем ей: так
    // экран закроется, даже если кадровый хук почему-то не сработает.
    if close && name == "escape" {
        return ResearchKeyPress { view: next, cancel: false, close: false };
    }
    ResearchKeyPress { view: next, cancel: true, close }
}

/// Шаг колеса: на записи листает страницы (вверх — назад), на карте —
/// науки, как вкладки сверху вниз. `steps` больше нуля — колесо вверх.
pub fn research_scroll(view: ResearchView, steps: i32, pages: usize) -> ResearchView {
    let mut view = view;
    for _ in 0..steps.unsigned_abs() {
        let up = steps > 0;
        view = match view.page {
            Some(_) => {
                let hit = if up { ResearchHit::PagePrevious } else { ResearchHit::PageNext };
                research_click(view, hit, 0, pages)
            }
            None => {
                let science = if up {
                    view.science.saturating_sub(1)
                } else {
                    (view.science + 1).min(SCIENCES.len() - 1)
                };
                ResearchView { science, ..view }
            }
        };
    }
    view
}

/// Ввод экрана журнала между хуками. Хуки экрана получают игру только на
/// чтение (`&self`): закрыть экран и щёлкнуть звуком они не могут, поэтому
/// просят об этом, а делает кадровый хук. Здесь же — какие кнопки мыши
/// нажаты над журналом и остаток колеса.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResearchInput {
    /// Кнопки, нажатие которых журнал отменил: только их отпускание и
    /// отменяем. Нажатие, начатое до экрана (тот самый правый щелчок, что
    /// открыл журнал), отпускается как обычно — иначе игра сочтёт кнопку
    /// зажатой.
    pub held: u8,
    /// Дробный остаток колеса: тачпад шлёт доли щелчка (пиксели / 30), и без
    /// остатка запись листалась бы на каждое касание.
    pub wheel: f64,
    /// Закрыть экран в следующем кадре.
    pub close: bool,
    /// Щёлкнуть звуком кнопки в следующем кадре.
    pub click: bool,
}

impl ResearchInput {
    pub fn press(&mut self, button: i32) {
        if (0..8).contains(&button) {
            self.held |= 1 << button;
        }
    }

    /// Отпущена кнопка: true — её нажатие было наше, отпускание тоже глотаем.
    pub fn release(&mut self, button: i32) -> bool {
        if !(0..8).contains(&button) {
            return false;
        }
        let bit = 1 << button;
        let ours = self.held & bit != 0;
        self.held &= !bit;
        ours
    }

    /// Колесо на `dy`: сколько целых шагов набралось (вверх — больше нуля).
    pub fn scroll(&mut self, dy: f64) -> i32 {
        if !dy.is_finite() {
            return 0;
        }
        // Колесо повернули в другую сторону — старый остаток не в счёт.
        if self.wheel * dy < 0.0 {
            self.wheel = 0.0;
        }
        self.wheel += dy;
        let steps = self.wheel.trunc();
        self.wheel -= steps;
        steps.clamp(-8.0, 8.0) as i32
    }

    /// Просьбы к кадру: (закрыть, щёлкнуть). Забирает их.
    pub fn take(&mut self) -> (bool, bool) {
        let asked = (self.close, self.click);
        self.close = false;
        self.click = false;
        asked
    }
}

fn research_input_cell() -> &'static std::sync::Mutex<ResearchInput> {
    static INPUT: std::sync::OnceLock<std::sync::Mutex<ResearchInput>> = std::sync::OnceLock::new();
    INPUT.get_or_init(Default::default)
}

/// Поменять ввод журнала под замком.
pub fn with_research_input<R>(f: impl FnOnce(&mut ResearchInput) -> R) -> Option<R> {
    research_input_cell().lock().ok().map(|mut input| f(&mut input))
}

/// Подпись внизу карты.
pub const RESEARCH_HINT: &str = "Щелчок — запись · 1–5, колесо — науки · Esc — закрыть";

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
    /// Так же рисуется смесь синтезатора (`machine` — синтезатор): у неё
    /// тоже список входов, а катализатор (`catalyst`) не тратится.
    Bench {
        cells: [Option<String>; 9],
        result: String,
        count: u32,
        seconds: u32,
        machine: String,
        catalyst: Option<String>,
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
        LAB_BENCH | SYNTHESIZER => {
            let ingredients = j.get("ingredients")?.arr()?;
            if ingredients.is_empty() || ingredients.len() > 8 {
                return None;
            }
            let mut cells: [Option<String>; 9] = Default::default();
            for (cell, ingredient) in cells.iter_mut().zip(ingredients) {
                *cell = Some(ingredient_id(ingredient)?);
            }
            let ticks = j.get("time").and_then(Json::num).unwrap_or(DEFAULT_TIME as f64).max(1.0) as u32;
            let catalyst = j.get("catalyst").and_then(Json::str).map(str::to_string);
            Some(RecipeView::Bench {
                cells,
                result,
                count,
                seconds: ticks.div_ceil(20),
                machine: kind.to_string(),
                catalyst,
            })
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
        assert_eq!(research_tooltip(mask, flask)[1], RESEARCH[flask].hint);
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

    /// Клавиши приходят ванильными именами (`native_screen_key_pressed_pre`):
    /// наши глотаем, чужие пропускаем, Escape на карте закрывает сама игра.
    #[test]
    fn keys_of_the_screen_come_by_vanilla_names() {
        let map = ResearchView { open: true, science: 0, page: None };
        let record = ResearchView { page: Some((1, 0)), ..map };
        assert_eq!(research_key_name("key.keyboard.e"), Some("e"));
        assert_eq!(research_key_name("key.keyboard.keypad.3"), Some("3"));
        assert_eq!(research_key_name("key.keyboard.left"), Some("left"));
        assert_eq!(research_key_name("key.keyboard.f2"), None);
        assert_eq!(research_key_name("key.mouse.left"), None);

        // Escape: на записи — назад к карте, глотаем; на карте — не мешаем
        // игре закрыть экран.
        let back = research_key_press(record, "key.keyboard.escape", 1);
        assert_eq!(back, ResearchKeyPress { view: map, cancel: true, close: false });
        let esc = research_key_press(map, "key.keyboard.escape", 1);
        assert!(!esc.cancel && !esc.close);
        // E и Backspace на карте закрывают экран нашими руками.
        let e = research_key_press(map, "key.keyboard.e", 1);
        assert!(e.cancel && e.close && !e.view.open);
        // Цифры и стрелки — наши; F2 (снимок) и прочее идут дальше.
        assert_eq!(research_key_press(map, "key.keyboard.2", 1).view.science, 1);
        assert!(research_key_press(map, "key.keyboard.2", 1).cancel);
        let turned = research_key_press(record, "key.keyboard.right", 3);
        assert_eq!((turned.view.page, turned.cancel), (Some((1, 1)), true));
        assert_eq!(research_key_press(map, "key.keyboard.f2", 1), ResearchKeyPress { view: map, cancel: false, close: false });
    }

    #[test]
    fn the_wheel_turns_pages_and_sciences() {
        let map = ResearchView { open: true, science: 0, page: None };
        // На карте: вниз — следующая наука, без выхода за края.
        assert_eq!(research_scroll(map, -1, 1).science, 1);
        assert_eq!(research_scroll(map, 1, 1).science, 0);
        assert_eq!(research_scroll(map, -20, 1).science, SCIENCES.len() - 1);
        // На записи: вниз — следующая страница, вверх — предыдущая.
        let record = ResearchView { page: Some((2, 0)), ..map };
        assert_eq!(research_scroll(record, -2, 3).page, Some((2, 2)));
        assert_eq!(research_scroll(record, -5, 3).page, Some((2, 2)));
        assert_eq!(research_scroll(research_scroll(record, -2, 3), 1, 3).page, Some((2, 1)));
        assert_eq!(research_scroll(record, 0, 3), record);

        // Тачпад шлёт доли щелчка: шаг набирается из них, разворот колеса
        // сбрасывает остаток.
        let mut input = ResearchInput::default();
        assert_eq!(input.scroll(0.4), 0);
        assert_eq!(input.scroll(0.4), 0);
        assert_eq!(input.scroll(0.4), 1);
        assert_eq!(input.scroll(-0.5), 0);
        assert_eq!(input.scroll(-0.6), -1);
        assert_eq!(input.scroll(-3.0), -3);
        assert_eq!(input.scroll(f64::NAN), 0);
    }

    #[test]
    fn only_the_buttons_pressed_on_the_screen_are_released_by_it() {
        let mut input = ResearchInput::default();
        // Правый щелчок открыл журнал до экрана: его отпускание — игры.
        assert!(!input.release(1));
        input.press(0);
        assert!(input.release(0));
        assert!(!input.release(0), "второй раз — уже не наше");
        input.press(40);
        assert!(!input.release(40), "кнопка за пределами маски");
        input.close = true;
        input.click = true;
        assert_eq!(input.take(), (true, true));
        assert_eq!(input.take(), (false, false));
    }

    #[test]
    fn the_hint_fits_under_the_map() {
        let l = research_layout(427.0, 240.0);
        assert!(estimated_text_width(RESEARCH_HINT) + 12.0 <= l.field[2], "{}", estimated_text_width(RESEARCH_HINT));
        let text = "= manual\n# Один\n---\n# Два\n";
        let map = ResearchView { open: true, science: 0, page: None };
        assert_eq!(research_page_count(&map, text), 1);
        assert_eq!(research_page_count(&ResearchView { page: Some((0, 0)), ..map }, text), 2);
    }

    #[test]
    fn records_are_read_by_node_and_split_into_pages() {
        let text = "= manual\n# Один\nтекст\n---\n# Два\n= bench\n# Стол\n";
        let manual = research_pages(text, 0);
        assert_eq!(manual, vec!["# Один\nтекст".to_string(), "# Два".to_string()]);
        assert_eq!(research_pages(text, research_index("bench").unwrap()), vec!["# Стол".to_string()]);
        let flask = research_index("flask").unwrap();
        assert_eq!(research_pages(text, flask), vec![format!("# Колба\n{}", RESEARCH[flask].hint)]);
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

    /// Записи журнала из контента.
    fn records() -> String {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        std::fs::read_to_string(root.join(RESEARCH_PATH)).unwrap()
    }

    /// Рецепт по имени файла, как его читает страница.
    fn content_recipe(name: &str) -> Option<RecipeView> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let json = std::fs::read_to_string(root.join(recipe_path(name))).ok()?;
        recipe_view(&json)
    }

    fn result_of(view: &RecipeView) -> String {
        match view {
            RecipeView::Grid { result, .. }
            | RecipeView::Bench { result, .. }
            | RecipeView::Machine { result, .. } => result.clone(),
        }
    }

    #[test]
    fn every_crafted_item_has_its_recipe_in_some_record() {
        let text = records();
        let mut shown = std::collections::HashSet::new();
        for i in 0..RESEARCH.len() {
            for page in research_pages(&text, i) {
                for line in page_lines(&page) {
                    if let Line::Craft(name) = line
                        && let Some(view) = content_recipe(&name)
                    {
                        shown.insert(result_of(&view));
                    }
                }
            }
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/recipe");
        let mut missing = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let view = recipe_view(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let result = result_of(&view);
            if !shown.contains(&result) {
                missing.push(format!("{result} ({})", path.file_stem().unwrap().to_string_lossy()));
            }
        }
        missing.sort();
        assert!(missing.is_empty(), "крафт не показан ни в одной записи: {missing:#?}");
    }

    #[test]
    fn text_wraps_by_width_and_a_long_page_is_cut() {
        let by_chars = |s: &str| s.chars().count() as f32 * 6.0;
        assert_eq!(wrap_text("один два три", 60.0, by_chars), vec!["один два", "три"]);
        assert_eq!(wrap_text("сверхдлинноеслово и", 30.0, by_chars), vec!["сверхдлинноеслово", "и"]);
        assert!(wrap_text("   ", 60.0, by_chars).is_empty());

        let machine = RecipeView::Machine {
            input: "a".into(),
            machine: "biolab:still".into(),
            result: "b".into(),
            count: 1,
            seconds: 4,
            byproduct: None,
        };
        let page = "# Заголовок\nстрока\n\n@item a b\n@craft нет\n@craft есть";
        let rows = page_rows(page, 100.0, by_chars, |name| (name == "есть").then(|| machine.clone()));
        assert_eq!(
            rows,
            vec![
                PageRow::Title("Заголовок".into()),
                PageRow::Text("строка".into()),
                PageRow::Gap,
                PageRow::Items(vec!["a".into(), "b".into()]),
                PageRow::Recipe(machine),
            ],
            "ненайденный рецепт места не занимает"
        );
        // 15 + 10 + 5 + 19 = 49: строка машины встаёт на 49 и рисуется до 67.
        assert_eq!(rows_fitting(&rows, 0.0, 67.0), 5);
        assert_eq!(rows_fitting(&rows, 0.0, 66.0), 4);
        assert_eq!(rows_fitting(&rows, 0.0, 11.0), 0);

        assert_eq!(estimated_text_width("Аб в"), 22.0);
        assert!(estimated_text_width("Ж") > estimated_text_width("А"));
        assert!(estimated_text_width("i.") < estimated_text_width("ab"));

        // Текст кончается над кнопками и не вылезает за пергамент.
        let l = research_layout(427.0, 240.0);
        let [left, top, width, bottom] = page_area(&l);
        assert!(bottom < page_buttons(&l)[0][1]);
        assert!(top > l.page[1] && left + width < l.page[0] + l.page[2]);
    }

    #[test]
    fn every_page_fits_the_smallest_screen() {
        let l = research_layout(427.0, 240.0);
        let [_, top, width, bottom] = page_area(&l);
        let text = records();
        let mut problems = Vec::new();
        for (i, r) in RESEARCH.iter().enumerate() {
            for (p, page) in research_pages(&text, i).iter().enumerate() {
                let at = format!("{} стр. {}", r.id, p + 1);
                let rows = page_rows(page, width, estimated_text_width, content_recipe);
                let fit = rows_fitting(&rows, top, bottom);
                if fit < rows.len() {
                    problems.push(format!("{at}: влезло {fit} строк из {}, обрезано с «{:?}»", rows.len(), rows[fit]));
                }
                // Справа вверху иконка узла: заголовок короче черты под ним.
                match rows.first() {
                    Some(PageRow::Title(title)) if estimated_text_width(title) <= width - 24.0 => {}
                    Some(PageRow::Title(title)) => problems.push(format!("{at}: заголовок «{title}» налезет на иконку")),
                    _ => problems.push(format!("{at}: страница начинается не с заголовка")),
                }
                for row in &rows {
                    match row {
                        PageRow::Text(line) if estimated_text_width(line) > width => {
                            problems.push(format!("{at}: слово шире строки: «{line}»"));
                        }
                        PageRow::Items(items) if items.len() as f32 * 19.0 - 3.0 > width => {
                            problems.push(format!("{at}: иконок больше, чем влезает в ряд"));
                        }
                        _ => {}
                    }
                }
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    #[test]
    fn hints_say_what_to_do_and_fit_beside_the_cursor() {
        let wide: Vec<String> = RESEARCH
            .iter()
            .filter(|r| !r.hint.ends_with('.') || estimated_text_width(r.hint) + 8.0 > 270.0)
            .map(|r| format!("{}: {} ({} пикс.)", r.id, r.hint, estimated_text_width(r.hint)))
            .collect();
        assert!(wide.is_empty(), "подсказка без точки или шире 270 пикселей:\n{}", wide.join("\n"));
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
        let Some(RecipeView::Bench { cells, result, count, seconds, .. }) = recipe_view(bench) else {
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
/// печки раскладка своя целиком. Со своими типами меню (`OWN_MENUS`)
/// раскладка наша у всех, но места те же — чтобы откат на ванильные меню
/// ничего не сдвинул.
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

// Выпадение блоков при поломке (M11). Таблицы и теги — данные контентного
// мода, а тест держит их в согласии с тем, какие блоки у нас есть: у каждого
// блока с предметом — своя таблица в формате движка 26.3, у луж — никакой.
// Подробнее и проверка шире — `tools/check_loot.py`.
#[cfg(test)]
mod drop_tests {
    use super::*;

    fn content(rel: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content").join(rel)
    }

    fn read(path: &std::path::Path) -> Option<Json> {
        Json::parse(&std::fs::read_to_string(path).ok()?)
    }

    /// Блоки мода: имя без пространства, есть ли предмет, шаблон.
    fn blocks() -> Vec<(String, bool, String)> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(content("data/biolab/horizon/block")).unwrap() {
            let path = entry.unwrap().path();
            let json = read(&path).expect("блок читается");
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let item = !matches!(json.get("item"), Some(Json::Bool(false)));
            let template = json.get("template").and_then(Json::str).unwrap_or("").to_string();
            out.push((name, item, template));
        }
        out.sort();
        out
    }

    /// Условие в виде, который читает движок (`items.rs`,
    /// `LootContext::condition`): строка или объект с `type`. Старый вид
    /// `{"condition": "..."}` без `type` движок считает «нет» — это и был баг
    /// операционного стола.
    fn engine_reads(condition: &Json) -> bool {
        match condition {
            Json::Str(_) => true,
            Json::Arr(list) => list.iter().all(engine_reads),
            Json::Obj(_) => condition.get("type").and_then(Json::str).is_some(),
            _ => false,
        }
    }

    #[test]
    fn the_old_condition_form_is_caught() {
        let old = Json::parse(r#"{"condition": "minecraft:survives_explosion"}"#).unwrap();
        let new = Json::parse(r#"{"type": "minecraft:survives_explosion"}"#).unwrap();
        assert!(!engine_reads(&old));
        assert!(engine_reads(&new));
        assert!(engine_reads(&Json::Str("minecraft:tool/can_silk_touch".into())));
    }

    #[test]
    fn every_block_with_an_item_drops_itself() {
        let blocks = blocks();
        assert_eq!(blocks.len(), 22, "блоков стало больше — решите, выпадают ли новые");
        for (name, item, template) in &blocks {
            let table = content(&format!("data/biolab/loot_table/blocks/{name}.json"));
            // Культуры (GDD 4.20) предмета не имеют — семена отдельный предмет,
            // — но роняют семена и урожай: их проверяет `tests/plants.rs`.
            if template == "minecraft:beetroots" {
                assert!(!item, "{name}: у культуры свой предмет — семена");
                assert!(table.exists(), "{name}: культура без таблицы ничего не уронит");
                continue;
            }
            if !item {
                assert!(!table.exists(), "{name}: у блока без предмета (лужа) не должно быть таблицы");
                continue;
            }
            let json = read(&table).unwrap_or_else(|| panic!("{name}: нет таблицы выпадения"));
            let pools = json.get("pools").and_then(Json::arr).unwrap_or(&[]);
            assert!(!pools.is_empty(), "{name}: пустая таблица");
            let id = format!("biolab:{name}");
            let mut drops_itself = false;
            for pool in pools {
                if let Some(c) = pool.get("condition") {
                    assert!(engine_reads(c), "{name}: условие пула движок не прочтёт");
                }
                for c in pool.get("conditions").and_then(Json::arr).unwrap_or(&[]) {
                    assert!(engine_reads(c), "{name}: условие пула движок не прочтёт");
                }
                for entry in pool.get("entries").and_then(Json::arr).unwrap_or(&[]) {
                    drops_itself |= entry.get("name").and_then(Json::str) == Some(id.as_str());
                }
            }
            assert!(drops_itself, "{name}: таблица не роняет сам блок");
            assert_eq!(
                json.get("random_sequence").and_then(Json::str),
                Some(format!("biolab:blocks/{name}").as_str()),
                "{name}: последовательность случайностей — по имени таблицы, как у ванили"
            );
        }
        // Все машины, стол, вольер, пульт и грядка выпадают; лужи — нет.
        let dropping: Vec<&str> = blocks.iter().filter(|b| b.1).map(|b| b.0.as_str()).collect();
        for machine in &MACHINES {
            assert!(dropping.contains(&machine.id.trim_start_matches("biolab:")), "{}", machine.id);
        }
        for id in [OPERATING_TABLE, PEN, CONSOLE, GM_PLOT] {
            assert!(dropping.contains(&id.trim_start_matches("biolab:")), "{id}");
        }
        for fluid in &FLUIDS {
            assert!(!dropping.contains(&fluid.block.trim_start_matches("biolab:")), "{}", fluid.block);
        }
    }

    #[test]
    fn blocks_whose_template_needs_a_tool_are_in_a_tool_tag() {
        // `requires_tool` шаблонов по gameplay-facts пина: печь и снег требуют
        // инструмент. Без тега ни один инструмент не подходит, и лут не
        // катится вовсе (`game.rs`, `break_block_as_player`: `harvest`).
        let tag = |tool: &str| -> Vec<String> {
            let json = read(&content(&format!("data/minecraft/tags/block/mineable/{tool}.json")))
                .unwrap_or_else(|| panic!("нет тега mineable/{tool}"));
            json.get("values")
                .and_then(Json::arr)
                .unwrap_or(&[])
                .iter()
                .filter_map(|v| v.str().map(str::to_string))
                .collect()
        };
        let pickaxe = tag("pickaxe");
        let shovel = tag("shovel");
        for (name, item, template) in blocks() {
            let id = format!("biolab:{name}");
            match template.as_str() {
                "minecraft:furnace" => assert!(pickaxe.contains(&id), "{id}: машина копается киркой"),
                "minecraft:snow" if item => assert!(shovel.contains(&id), "{id}: грядка копается лопатой"),
                _ => {}
            }
        }
        // Металлические лабораторный и операционный столы — тоже киркой
        // (быстрее), хотя ломаются и рукой.
        assert!(pickaxe.contains(&LAB_BENCH.to_string()));
        assert!(pickaxe.contains(&OPERATING_TABLE.to_string()));
    }
}
