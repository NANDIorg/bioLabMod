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
pub const ITEM_HINTS: [(&str, &[&str]); 114] = [
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
    // Многоблочные установки (GDD 4.18): каркас и колонна.
    ("biolab:casing", &["Стенки больших установок"]),
    ("biolab:casing_glass", &["Стенка установки, сквозь которую видно"]),
    ("biolab:input_port", &["Вход установки: воронки кладут сюда", "Ставится вплотную к контроллеру"]),
    ("biolab:output_port", &["Выход установки: воронки забирают отсюда", "Ставится вплотную к контроллеру"]),
    ("biolab:column", &["Низ колонны: огонь под ним, тарелки сверху", "Сырьё → до пяти элементов за раз"]),
    ("biolab:column_tray", &["Секция колонны: одна тарелка — одна фракция"]),
    ("biolab:column_condenser", &["Вершина колонны, над тарелками"]),
    // Карантинная камера (GDD 4.18 п. 4).
    ("biolab:quarantine", &["Фильтр чистой комнаты: ставится в её стену", "Экран: герметичность камеры в процентах"]),
    ("biolab:clean_panel", &["Стены, пол и потолок карантинной камеры"]),
    ("biolab:filter_cartridge", &["Расходник фильтра камеры: ~20 минут", "Без патрона камера течёт"]),
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
    // Генный секвенатор (GDD 4.18, п. 3).
    ("biolab:sequencer", &["Консоль секвенатора: установка 3×2×3", "Вектор, антисмысл, чтение генома"]),
    ("biolab:sequencer_rack", &["Стойка секвенатора: по бокам за консолью"]),
    ("biolab:sequencer_monitor", &["Монитор секвенатора: над консолью"]),
    ("biolab:gene_vector", &["ПКМ по мобу — все гены одним уколом", "ПКМ в воздух — себе"]),
    ("biolab:antisense_vector", &["ПКМ по мобу — удалить этот ген", "ПКМ в воздух — удалить у себя"]),
    // Биореактор (GDD 4.18, п. 2).
    ("biolab:bioreactor", &["Середина стенки куба 3×3×3, огонь под днищем", "Партия до 8: рецепты ферментера и инкубатора"]),
    // Криобанк и термобокс (GDD 4.23 п. 9).
    ("biolab:cryobank", &["Холодильник образцов: внутри не портятся", "Культуры, фрагменты, ткань, векторы"]),
    ("biolab:thermobox", &["В инвентаре образцы портятся вчетверо медленнее", "Для похода с культурами и фрагментами"]),
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
        GENE_VECTOR => Some(vector_label(&text(VECTOR_KEY)?)),
        ANTISENSE_VECTOR => Some(antisense_label(&text(ANTISENSE_KEY)?)),
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
    pub research: u128,
    /// Очки знания узлов и счёт повторов опытов (GDD 4.23 п. 7, «M9: лаборатория»).
    pub lab: LabProgress,
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
            lab: LabProgress::default(),
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
    text.push_str(&lab_to_text(&s.lab));
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
                // Очки знания (GDD 4.23 п. 7): `research_points`, `research_repeats`.
                if lab_from_line(&mut s.lab, key, value) {
                    continue;
                }
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

pub const MACHINES: [Machine; 15] = [
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
    // Контроллеры многоблочных установок (GDD 4.18, раздел «M8»): роли
    // слотов — из их строки `MULTIBLOCKS`, меню — всегда своё.
    Machine { id: COLUMN, slots: COLUMN_SLOTS, menu: MULTIBLOCK_MENU },
    Machine { id: SEQUENCER, slots: SEQUENCER_SLOTS, menu: MULTIBLOCK_MENU },
    Machine { id: BIOREACTOR, slots: BIOREACTOR_SLOTS, menu: MULTIBLOCK_MENU },
    // Фильтр карантинной камеры: один слот — фильтрующий патрон.
    Machine { id: QUARANTINE, slots: QUARANTINE_SLOTS, menu: MULTIBLOCK_MENU },
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
        // Генный секвенатор: режим, итог и гены на мониторе (раздел «M8»).
        SEQUENCER => DATA_SEQ_END,
        // Фильтр карантинной камеры: точка дыры, размер, патрон.
        QUARANTINE => DATA_QR_END,
        // Контроллер установки: общие числа сборки (раздел «M8»); свои
        // числа установки — после `DATA_MB_END`.
        BIOREACTOR => DATA_BIO_END,
        id if multiblock(id).is_some() => DATA_MB_END,
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
// `container_accepts.rs` (что берёт слот для воронки, раздел «M11-Г»),
// `machine_analog*.rs` и начало тика в `machine.rs`.

/// Роль слота машины.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotRole {
    /// Сырьё: то, что машина перерабатывает.
    Input,
    /// Слот топлива меню печки. Нужен только клонирующему чану — там лежит
    /// питательный раствор; у биореактора (GDD 4.18) — слот раствора.
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
    // Контроллер установки: роли — в строке `MULTIBLOCKS`.
    if let Some(mb) = multiblock(machine.id) {
        return mb.slots.get(slot).copied().unwrap_or(SlotRole::Unused);
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
    // У контроллера установки ванильной раскладки нет: меню всегда своё.
    if OWN_MENUS || machine.menu == MULTIBLOCK_MENU { machine.id } else { machine.menu }
}

/// Машина, чей это тип меню; None — меню не наше.
pub fn machine_of_menu(menu: &str) -> Option<&'static Machine> {
    MACHINES.iter().find(|m| machine_menu(m) == menu)
}

/// Что принимает вход машин на рецептах: всё, что стоит в `ingredient`
/// (`ingredients` у стола) её рецептов в `content/`. Правилам слотов мир не
/// виден (ни рецептов, ни уровня), поэтому список свой; тест сверяет его с
/// рецептами, так что новый рецепт без строки здесь не пройдёт.
pub const MACHINE_INGREDIENTS: [(&str, &[&str]); 10] = [
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
    // Ректификационная колонна (GDD 4.18): входы рецептов `biolab:column`.
    (
        COLUMN,
        &[
            "minecraft:bone",
            "minecraft:gunpowder",
            "minecraft:kelp",
            "minecraft:rotten_flesh",
            "minecraft:seagrass",
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
            // Секвенатор (GDD 4.18): фрагменты, фосфор — каждый в свой слот.
            SEQUENCER => sequencer_accepts(slot, item),
            // Биореактор варит рецепты ферментера и инкубатора (GDD 4.18).
            BIOREACTOR => BIOREACTOR_RECIPES.iter().any(|kind| machine_ingredients(kind).contains(&item)),
            // Фильтр карантинной камеры — только патрон.
            QUARANTINE => item == FILTER_CARTRIDGE,
            id => machine_ingredients(id).contains(&item),
        },
    }
}

/// Возьмёт ли машина предмет хоть в один слот, куда кладут воронки. Был
/// фильтром воронки «про машину целиком» (`native_inventory_move_item`
/// слота не называет); с M11 воронку спрашивают по слоту
/// (`container_slot_accepts`, раздел «M11-Г»), а это осталось сводкой для
/// тестов: каждый вход рецепта машина примет.
pub fn hopper_accepts(machine: &Machine, item: &str) -> bool {
    machine_inputs(machine).into_iter().any(|slot| slot_accepts(machine, slot, item))
}

/// Правило слота своего меню машины: None — меню не наше (решает чужой мод
/// или игра), иначе можно ли положить предмет.
pub fn menu_slot_rule(menu: &str, slot: usize, item: &str) -> Option<bool> {
    // Криобанк (GDD 4.23 п. 9) — не машина: берёт только образцы.
    if menu == CRYOBANK {
        return Some(slot < CRYOBANK_SLOTS && cryobank_accepts(item));
    }
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

/// Свойство `lit` блока машины по числам данных: работает — `"true"`.
/// От него зависит модель `_on` со светящейся лампой работы (GDD 4.12):
/// число `DATA_LIT` видит только экран, а модель выбирает состояние блока.
/// `None` — у машины такого свойства нет (стол, грядка), трогать нечего.
pub fn machine_lit(id: &str, data: &[i32]) -> Option<&'static str> {
    if !is_machine(id) || id == LAB_BENCH {
        return None;
    }
    Some(if data.get(DATA_LIT).copied().unwrap_or(0) > 0 { "true" } else { "false" })
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
            // Фильтр карантинной камеры ничего не выдаёт: только патрон.
            assert_eq!(roles.contains(&SlotRole::Output), machine.id != QUARANTINE, "{}", machine.id);
            assert_eq!(roles.contains(&SlotRole::Fuel), [CLONING_VAT, BIOREACTOR].contains(&machine.id), "{}", machine.id);
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
        // Порты установок (GDD 4.18): пять слотов, у выходного все — `results`.
        for p in &PORTS {
            let name = p.id.trim_start_matches("biolab:");
            let text = std::fs::read_to_string(dir.join(format!("{name}.json"))).unwrap_or_else(|_| panic!("нет меню {name}"));
            let json = Json::parse(&text).unwrap();
            assert_eq!(json.get("slots").and_then(Json::num), Some(PORT_SLOTS as f64), "{name}");
            let results = json.get("results").and_then(Json::arr).map_or(0, |r| r.len());
            assert_eq!(results, if p.kind == PortKind::Output { PORT_SLOTS } else { 0 }, "{name}");
        }
        // Криобанк (GDD 4.23 п. 9): девять слотов, выходов нет.
        let cryo = Json::parse(&std::fs::read_to_string(dir.join("cryobank.json")).expect("нет меню криобанка")).unwrap();
        assert_eq!(cryo.get("slots").and_then(Json::num), Some(CRYOBANK_SLOTS as f64));
        let files = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(files, MACHINES.len() + PORTS.len() + 1, "лишний тип меню");
    }

    #[test]
    fn machine_lamp_follows_lit_data() {
        let mut data = vec![0; 8];
        assert_eq!(machine_lit("biolab:incubator", &data), Some("false"));
        data[DATA_LIT] = 1;
        assert_eq!(machine_lit("biolab:incubator", &data), Some("true"));
        // Стол без свойства `lit`, не машина — не трогаем.
        assert_eq!(machine_lit(LAB_BENCH, &data), None);
        assert_eq!(machine_lit("minecraft:furnace", &data), None);
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
pub const MACHINE_SOUNDS: [MachineSounds; 15] = [
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
    // Ректификационная колонна: кипятильник разгорается, гудит топка под
    // башней, фракции стекают по тарелкам — ниже, чем у перегонного куба.
    MachineSounds {
        machine: COLUMN,
        start: block("minecraft:block.fire.extinguish", 0.3, 0.8),
        work: block("minecraft:block.blastfurnace.fire_crackle", 0.35, 0.8),
        done: block("minecraft:item.bottle.fill", 0.6, 0.7),
    },
    // Генный секвенатор: щелчок консоли, перезвон стоек при чтении цепи,
    // сигнал прибора — выше, чем у спектрометра.
    MachineSounds {
        machine: SEQUENCER,
        start: block("minecraft:block.comparator.click", 0.5, 1.4),
        work: block("minecraft:block.amethyst_block.chime", 0.3, 1.4),
        done: block("biolab:block.machine.ready", 0.5, 1.5),
    },
    // Биореактор: тот же характер, что у ферментера, но бак большой — ниже
    // и глуше; партия сливается ведром.
    MachineSounds {
        machine: BIOREACTOR,
        start: block("minecraft:block.copper_bulb.turn_on", 0.5, 0.7),
        work: block("biolab:block.fermenter.bubble", 0.45, 0.7),
        done: block("minecraft:item.bucket.empty", 0.6, 0.9),
    },
    // Фильтр карантинной камеры: камера закрылась (маяк включился), ровный
    // гул вытяжки, патрон износился — маяк гаснет.
    MachineSounds {
        machine: QUARANTINE,
        start: block("minecraft:block.beacon.activate", 0.4, 1.4),
        work: block("minecraft:block.beacon.ambient", 0.25, 1.6),
        done: block("minecraft:block.beacon.deactivate", 0.5, 1.2),
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
    /// Щелчок дозиметра: чем опаснее вокруг, тем чаще (M9, GDD 4.23 п. 12).
    DosimeterClick,
    /// Своё дыхание в маске или костюме (M9, GDD 4.23 п. 12, 4.25).
    Breath,
    /// Перед приступом болезни (M9, GDD 4.23 п. 2).
    SeizureWarning,
    /// Тревога: тяжёлый штамм утекает из машины вне карантина (M9).
    LeakAlarm,
}

impl Cue {
    pub const ALL: [Cue; 30] = [
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
        Cue::DosimeterClick,
        Cue::Breath,
        Cue::SeizureWarning,
        Cue::LeakAlarm,
    ];
}

/// Событие → звук.
pub const SOUNDS: [(Cue, Sound); 30] = [
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
    // Дозиметр — свой сухой щелчок; темп задаёт код (`click_interval`), тихо:
    // он щёлкает часто.
    (Cue::DosimeterClick, player("biolab:item.dosimeter.click", 0.35, 1.0)),
    // Дыхание в маске — ванильный вдох-выдох игрока, ниже и глуше обычного.
    (Cue::Breath, player("minecraft:entity.player.breath", 0.25, 0.8)),
    // Перед приступом — удар сердца (как у стража): тревожно, но не громко.
    (Cue::SeizureWarning, player("minecraft:entity.warden.heartbeat", 0.6, 1.0)),
    // Тревога утечки — высокий звон колокольчика нотного блока: его слышно
    // из соседней комнаты, и он не спутается с набатом перегрева.
    (Cue::LeakAlarm, block("minecraft:block.note_block.bell", 0.9, 1.8)),
];

/// Свои события, которые объявлены в `sounds.json`, но код их пока не
/// играет. Без этого списка тест принял бы заготовку за забытый звук. Щелчок
/// дозиметра здесь больше не лежит: его играет тик дозы (M9, GDD 4.23 п. 12).
pub const RESERVED_SOUNDS: [&str; 0] = [];

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

/// Вид без пространства имён: `minecraft:zombie` → `zombie`. Свой вид мода
/// (`biolab:infected`) — это тело его шаблона: конечности, рост, масса и
/// носилки у него зомби (`mod_template`, раздел «M11-А: Заражённый»).
fn short_kind(kind: &str) -> &str {
    if let Some(template) = mod_template(kind) {
        return template;
    }
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
    if let Some(title) = mod_title(kind) {
        return title.to_string();
    }
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

pub const GENES: [GeneKind; 50] = [
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
    // M9: пропорции (GDD 4.24) — гены формы частей тела: руки, голова, ноги,
    // корпус, асимметрия. Вид рисует `biolab-deep` (`biolab_proportions`),
    // числа и поведение — раздел «M9: пропорции» ниже.
    GeneKind { name: "long_arms", title: "Долгорукость", cost: 2, about: "длинные руки: достаёт издали, но замах реже", player: Some("haste"), side: Some("weakness") },
    GeneKind { name: "heavy_arms", title: "Тяжёлые руки", cost: 2, about: "толстые руки: бьёт сильнее, ходит медленнее", player: Some("strength"), side: Some("slowness") },
    GeneKind { name: "macrocephaly", title: "Большеголовость", cost: 1, about: "большая голова: замечает дальше, но сам на виду", player: Some("night_vision"), side: Some("glowing") },
    GeneKind { name: "microcephaly", title: "Микроцефалия", cost: 1, about: "маленькая голова: замечает хуже, но в неё не попасть", player: Some("resistance"), side: Some("blindness") },
    GeneKind { name: "short_legs", title: "Коротконогость", cost: 1, about: "короткие ноги: медленнее, но с ног не сбить", player: Some("resistance"), side: Some("slowness") },
    GeneKind { name: "stilt_legs", title: "Ходульные ноги", cost: 2, about: "длинные ноги: быстрее и прыгает выше, но хрупче", player: Some("jump_boost"), side: Some("weakness") },
    GeneKind { name: "bloated_torso", title: "Раздутый торс", cost: 2, about: "широкий корпус: живучее, но крупная мишень", player: Some("health_boost"), side: Some("hunger") },
    GeneKind { name: "sunken_chest", title: "Впалая грудь", cost: 1, about: "узкий корпус: хрупче, но в него трудно попасть", player: Some("speed"), side: Some("weakness") },
    GeneKind { name: "asymmetry", title: "Асимметрия", cost: 1, about: "правая сторона крупнее левой: ходит боком", player: None, side: None },
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
pub const PLAYER_MECHANICS: [(&str, &str); 6] = [
    ("dicephaly", "подсвечивает мобов вокруг"),
    ("horns", "удар отбрасывает сильнее"),
    ("tail", "хвост бьёт тех, кто ударил сзади"),
    ("tentacles", "подтягивают предметы с земли"),
    // Вдох газа меньше на четверть за силу (`gas_dose`, `sleep_tick.rs`).
    ("sedation_resistance", "газ снотворного действует слабее"),
    // M9: пропорции (GDD 4.24): ванильного эффекта у асимметрии нет — она
    // только видна на модели (F5) и в инвентаре.
    ("asymmetry", "только вид: правая сторона тела крупнее"),
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
pub const GENE_MARKERS: [&str; 150] = [
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
    // M9: пропорции (GDD 4.24)
    "biolab:gene/long_arms/1",
    "biolab:gene/long_arms/2",
    "biolab:gene/long_arms/3",
    "biolab:gene/heavy_arms/1",
    "biolab:gene/heavy_arms/2",
    "biolab:gene/heavy_arms/3",
    "biolab:gene/macrocephaly/1",
    "biolab:gene/macrocephaly/2",
    "biolab:gene/macrocephaly/3",
    "biolab:gene/microcephaly/1",
    "biolab:gene/microcephaly/2",
    "biolab:gene/microcephaly/3",
    "biolab:gene/short_legs/1",
    "biolab:gene/short_legs/2",
    "biolab:gene/short_legs/3",
    "biolab:gene/stilt_legs/1",
    "biolab:gene/stilt_legs/2",
    "biolab:gene/stilt_legs/3",
    "biolab:gene/bloated_torso/1",
    "biolab:gene/bloated_torso/2",
    "biolab:gene/bloated_torso/3",
    "biolab:gene/sunken_chest/1",
    "biolab:gene/sunken_chest/2",
    "biolab:gene/sunken_chest/3",
    "biolab:gene/asymmetry/1",
    "biolab:gene/asymmetry/2",
    "biolab:gene/asymmetry/3",
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
pub const GENE_BEHAVIOUR: [&[&str]; 50] = [
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
    // M9: пропорции (GDD 4.24). Числа (здоровье, скорость, урон, броня,
    // отбрасывание, дальность взгляда, прыжок) ставит `biolab-deep`
    // (`biolab_proportions`); здесь — то, что видно только всем мобам разом.
    &["long_reach"],
    &[],
    &["big_head"],
    &["small_head"],
    &[],
    &["stilts"],
    &[],
    &[],
    &["lopsided"],
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
            GENE_VECTOR => "Генный вектор",
            ANTISENSE_VECTOR => "Антисмысловой вектор",
            CULTURE_ID => "Культура",
            _ => "Препарат",
        });
    let target = kind_label(mob);
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
        let name = kind_label(kind);
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
            // Векторы секвенатора (GDD 4.18): гены в данных, как у фрагмента.
            GENE_VECTOR, ANTISENSE_VECTOR,
            // Инструменты.
            SCALPEL_ID, ANALYZER_ID, MICROSCOPE_ID, MANUAL_ID,
            // Оружие: свои данные (заряд, магазин, газ) и стопкой не держится.
            BLOWPIPE_ID, TRANQ_CROSSBOW_ID, AIR_RIFLE_ID,
            // Полный баллон меняется на пустой прямо в слоте (`weapons.rs`):
            // выдача в инвентарь из хука ненадёжна, а стопку не подменить.
            CO2_CANISTER_ID,
            // Фильтрующий патрон карантинной камеры: износ — прочность предмета.
            FILTER_CARTRIDGE,
            // Термобокс (GDD 4.23 п. 9): один на поход, стопка ему ни к чему.
            THERMOBOX,
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
            // Свой живой вид (заражённый, шаблон — зомби) — не снаряд: его
            // проверяет `infected_tests`.
            if mod_template(&id).is_some() {
                continue;
            }
            // Снаряд копирует снежок (`ENTITY_TEMPLATES`).
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
            assert!(!inputs.is_empty() && (!outputs.is_empty() || machine.id == QUARANTINE), "{}", machine.id);
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

pub const RESEARCH: [Research; 68] = [
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
    // Многоблочные установки (GDD 4.18, M8). Общие части — в Основах: из них
    // строят все установки. Каждая установка — узел своей науки: колонна —
    // химия (после элементов: она их и даёт).
    node("multiblock", 0, (4, 1), "biolab:casing", "Установки", &["bench"], Unlock::Item(&["biolab:casing", "biolab:casing_glass", "biolab:input_port", "biolab:output_port"]), "Сделайте корпус установки: железо и медь."),
    node("column", 1, (1, 2), "biolab:column", "Ректификационная колонна", &["elements"], Unlock::Item(&["biolab:column", "biolab:column_tray", "biolab:column_condenser"]), "Сделайте кипятильник: куб и доменная печь."),
    // Генный секвенатор (GDD 4.18, п. 3) — генетика, после генотерапии:
    // антисмысл — это она же, только обратная.
    node("sequencer", 4, (4, 3), "biolab:sequencer", "Генный секвенатор", &["therapy"], Unlock::Item(&["biolab:sequencer", "biolab:sequencer_rack", "biolab:sequencer_monitor", "biolab:gene_vector", "biolab:antisense_vector"]), "Сделайте консоль: экстрактор и микроскоп."),
    // Биореактор — микробиология: партия культур и экстрактов.
    node("bioreactor", 2, (2, 2), "biolab:bioreactor", "Биореактор", &["culture"], Unlock::Item(&["biolab:bioreactor"]), "Сделайте контроллер: ферментер и инкубатор."),
    // Карантинная камера — микробиология, после штамма: она нужна, когда
    // штамм выведен тяжёлым.
    node("quarantine", 2, (3, 0), "biolab:quarantine", "Карантинная камера", &["strain"], Unlock::Item(&["biolab:quarantine", "biolab:clean_panel", "biolab:filter_cartridge"]), "Сделайте фильтр: корпус, решётки, шерсть."),
    // Криобанк (GDD 4.23 п. 9) — микробиология, под культурой: её первой и
    // надо беречь.
    node("cryobank", 2, (1, 2), "biolab:cryobank", "Криобанк", &["culture"], Unlock::Item(&["biolab:cryobank", "biolab:thermobox"]), "Сделайте криобанк: железо, лёд, сундук."),
];

/// Номер узла по id.
pub fn research_index(id: &str) -> Option<usize> {
    RESEARCH.iter().position(|r| r.id == id)
}

/// Бит узла в маске изученного.
pub fn research_bit(index: usize) -> u128 {
    1u128 << index
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
pub fn research_now(s: &PlayerState, f: &ResearchFacts) -> u128 {
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
) -> (u128, Vec<usize>) {
    let Ok(mut table) = states().lock() else {
        return (0, Vec::new());
    };
    let state = table.entry((world.to_path_buf(), uuid)).or_default();
    // Узел открывает и его условие, и набранный порог очков (GDD 4.23 п. 7).
    let fresh = (research_now(state, facts) | points_mask(&state.lab)) & !state.research;
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
pub fn research_to_text(mask: u128) -> String {
    RESEARCH
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & research_bit(*i) != 0)
        .map(|(_, r)| r.id)
        .collect::<Vec<_>>()
        .join(",")
}

/// Маска из файла: незнакомые id пропускаем.
pub fn research_from_text(text: &str) -> u128 {
    text.split(',')
        .filter_map(|id| research_index(id.trim()))
        .fold(0, |mask, i| mask | research_bit(i))
}

/// Пакет для клиента: маска, шестнадцать байт. Узлов стало больше 64
/// (M8, многоблочные установки), и маска выросла до 128 бит; старый пакет
/// в восемь байт ещё читается — от сервера старой сборки.
pub fn research_packet(mask: u128) -> Vec<u8> {
    mask.to_le_bytes().to_vec()
}

pub fn research_from_packet(data: &[u8]) -> Option<u128> {
    if let Some(wide) = data.get(..16) {
        return Some(u128::from_le_bytes(wide.try_into().ok()?));
    }
    Some(u128::from(u64::from_le_bytes(data.get(..8)?.try_into().ok()?)))
}

/// Изученное на клиенте. 128-битного атомика в стабильном Rust нет —
/// мьютекс: читают его раз в кадр, пишут раз в секунду.
fn known_research() -> &'static std::sync::Mutex<u128> {
    static KNOWN: std::sync::Mutex<u128> = std::sync::Mutex::new(1);
    &KNOWN
}

/// Клиент запомнил, что изучено (пришло с сервера).
pub fn remember_research(mask: u128) {
    *known_research().lock().unwrap_or_else(|e| e.into_inner()) = mask;
}

/// Что изучено, как его знает клиент. Первый узел изучен всегда.
pub fn shown_research() -> u128 {
    *known_research().lock().unwrap_or_else(|e| e.into_inner()) | research_bit(0)
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

pub fn node_state(mask: u128, index: usize) -> NodeState {
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
pub fn research_progress(mask: u128) -> String {
    let done = (0..RESEARCH.len()).filter(|i| mask & research_bit(*i) != 0).count();
    format!("Изучено {done} из {}", RESEARCH.len())
}

/// Подсказка у курсора над узлом.
pub fn research_tooltip(mask: u128, index: usize) -> Vec<String> {
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
            PageRow::Recipe(RecipeView::Machine { .. } | RecipeView::Column { .. }) => (18.0, 22.0),
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

pub fn research_hit(l: &ResearchLayout, view: &ResearchView, mask: u128, at: [f32; 2]) -> ResearchHit {
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
pub fn research_click(view: ResearchView, hit: ResearchHit, mask: u128, pages: usize) -> ResearchView {
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
    /// Ректификационная колонна (GDD 4.18): вход, время и фракции сверху
    /// вниз — первая с верхней тарелки.
    Column {
        input: String,
        machine: String,
        fractions: Vec<(String, u32)>,
        seconds: u32,
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
    // У колонны вместо одного `result` — список фракций.
    if kind == COLUMN {
        let (input, fractions, ticks) = column_recipe(&j)?;
        return Some(RecipeView::Column {
            input,
            machine: kind.to_string(),
            fractions: fractions.into_iter().map(|f| (f.item, f.count)).collect(),
            seconds: (ticks.max(1) as u32).div_ceil(20),
        });
    }
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
        assert!(RESEARCH.len() <= 128, "маска изученного — 128 бит");
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
    fn research_is_saved_by_name_and_sent_as_sixteen_bytes() {
        let mask = research_bit(0) | research_bit(research_index("vat").unwrap());
        assert_eq!(research_to_text(mask), "manual,vat");
        assert_eq!(research_from_text("manual, vat,ghost"), mask);
        assert_eq!(research_packet(mask).len(), 16);
        assert_eq!(research_from_packet(&research_packet(mask)), Some(mask));
        assert_eq!(research_from_packet(&[1, 2]), None);
        // Узел за 64-м битом доходит до клиента.
        let far = research_bit(100);
        assert_eq!(research_from_packet(&research_packet(far)), Some(far));
        // Старый пакет в восемь байт.
        assert_eq!(research_from_packet(&5u64.to_le_bytes()), Some(5));
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
            // Колонна показана — показаны все её фракции; ключ — первая.
            RecipeView::Column { fractions, .. } => fractions.first().map(|f| f.0.clone()).unwrap_or_default(),
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

// ---- M11-Г: машины и анализатор (GDD 4.26) ----
//
// Две новые возможности пина `66f3981`; здесь только решения, хуки —
// `container_accepts.rs`, `analyzer_scan.rs`, `analyzer_marks.rs`.
//
// *Вход машины по слоту* (`native_container_accepts`, NeoForge
// `IItemHandler.isItemValid`). Воронка, выбрасыватель и `native_insert_item`
// спрашивают его про каждый слот, который им открывает `ModContainer`
// (`machine_faces`), и ответ тот же, что у правил слотов меню
// (`slot_accepts`): во вход — только сырьё машины, в топливо чана — только
// раствор, в выход — ничего. Отказ — воронка пробует следующий слот, а не
// найдя его — оставляет предмет у себя.
//
// Старый фильтр `hopper_filter.rs` (HEAD `native_inventory_move_item`) снят.
// Он спрашивал «про машину целиком» и в нашей сборке не срабатывал вовсе:
// игра зовёт `native_inventory_move_item` только при загруженном трейт-моде
// (`hopper_move_canceled` и ветка компостера в `game/machines.rs` пина
// выходят при пустом `mod_api::mods()`), а наши бандлы — миксины. Новый
// вопрос (`mod_container_accepts`, там же) задаётся всегда, и он точнее:
// старый фильтр пропустил бы фрагмент ДНК, пришедший в чан сбоку, — чан его
// «берёт», — и тот лёг бы в слот раствора. Забирает из машины воронка как
// раньше: спрашивают только при вкладывании, источник не проверяется.
//
// *Анализатор в мире* (`native_world_primitives`). С анализатором в руке
// вокруг мутантов и заражённых в 16 блоках — рамка, над ней короткая
// подпись. Это клиент, и он знает только то, что окну и так приходит:
// - мутант — геном моба (`biolab:genome`, данные с `sync`, по ним же рисует
//   наросты `biolab-deep`); спит ли и на столе ли — `biolab:surgery`;
// - заражённый — носитель болезни по виду: зомби (то же правило, по которому
//   заражает близость, `infection.rs`) и `biolab:infected` (GDD 4.26 А).
//   Мобы, больные штаммом (эффект `biolab:infection` от дротика, в вольере),
//   окну не видны: чужих эффектов клиенту не шлют (`remote_effect` в
//   `game/remote.rs` пина берёт только свои), а своих данных о болезни моба
//   у мода нет. Появятся — дописать в `analyzer_carrier`.
// Данные читаются раз в тик окна (`analyzer_scan.rs`: хуку кадра `Host` не
// достать — он `&self`) и лежат здесь; положение мобов кадр берёт живое, так
// что рамка не отстаёт.

/// Радиус, в котором анализатор подсвечивает мобов, в блоках.
pub const ANALYZER_RANGE: f64 = 16.0;

/// Виды-носители болезни помимо зомби (их узнают по `Species::Zombie`).
/// `biolab:infected` — свой вид заражённого (GDD 4.26 А).
pub const ANALYZER_CARRIERS: [&str; 1] = ["biolab:infected"];

/// Модель игрока в списке мобов окна: игроков анализатор не размечает.
pub const ANALYZER_PLAYER_KIND: &str = "minecraft:player";

/// Что анализатор видит в мобе.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalyzerMark {
    /// Носитель болезни.
    Carrier,
    /// Мутант: в геноме есть гены.
    Mutant,
    /// И то и другое — опаснее всего.
    CarrierMutant,
}

/// Цвета рамок, ARGB: болезнь — зелёный, мутация — фиолетовый, оба —
/// красный.
pub const ANALYZER_CARRIER_COLOR: u32 = 0xFF5C_E04A;
pub const ANALYZER_MUTANT_COLOR: u32 = 0xFFB4_6CFF;
pub const ANALYZER_BOTH_COLOR: u32 = 0xFFFF_5A3C;
/// Толщина рамки в пикселях.
pub const ANALYZER_LINE: f32 = 2.0;
/// Подпись: размер (1 — табличка с именем; с `scale_with_distance` такой,
/// как на 8 блоках) и зазор над рамкой в блоках.
pub const ANALYZER_TEXT_SCALE: f32 = 0.8;
pub const ANALYZER_TEXT_GAP: f64 = 0.3;

/// Носитель ли болезни моб такого вида. `zombie` — его вид в игре
/// `Species::Zombie` (так заражает близость, `infection.rs`).
pub fn analyzer_carrier(kind: &str, zombie: bool) -> bool {
    zombie || ANALYZER_CARRIERS.contains(&full_kind(kind).as_str())
}

/// Что показать над мобом; None — обычный моб, рамки нет.
pub fn analyzer_mark(carrier: bool, genes: &[(&str, i32)]) -> Option<AnalyzerMark> {
    match (carrier, !genes.is_empty()) {
        (true, true) => Some(AnalyzerMark::CarrierMutant),
        (true, false) => Some(AnalyzerMark::Carrier),
        (false, true) => Some(AnalyzerMark::Mutant),
        (false, false) => None,
    }
}

/// Цвет рамки и подписи.
pub fn analyzer_color(mark: AnalyzerMark) -> u32 {
    match mark {
        AnalyzerMark::Carrier => ANALYZER_CARRIER_COLOR,
        AnalyzerMark::Mutant => ANALYZER_MUTANT_COLOR,
        AnalyzerMark::CarrierMutant => ANALYZER_BOTH_COLOR,
    }
}

/// Подпись над рамкой: что с мобом, у мутанта — сильнейший ген (сила
/// плюсами) и сколько генов ещё, «спит» — если усыплён. «Заражён»,
/// «Мутант: Хитиновый панцирь ++, ещё 2», «Заражён · мутант: Гигантизм + ·
/// спит».
pub fn analyzer_label(mark: AnalyzerMark, genes: &[(&str, i32)], asleep: bool) -> String {
    let mutant = || {
        // Сильнейший ген; при равной силе — первый по порядку.
        let mut best: Option<(&str, i32)> = None;
        for (name, strength) in genes {
            if best.is_none_or(|(_, s)| *strength > s) {
                best = Some((*name, *strength));
            }
        }
        let Some((name, strength)) = best else {
            return "мутант".to_string();
        };
        let title = gene(name).map_or(name, |g| g.title);
        let mut text = format!("мутант: {title} {}", "+".repeat(strength.clamp(1, 3) as usize));
        if genes.len() > 1 {
            text.push_str(&format!(", ещё {}", genes.len() - 1));
        }
        text
    };
    let mut label = match mark {
        AnalyzerMark::Carrier => "заражён".to_string(),
        AnalyzerMark::Mutant => mutant(),
        AnalyzerMark::CarrierMutant => format!("заражён · {}", mutant()),
    };
    if asleep {
        label.push_str(" · спит");
    }
    // С заглавной.
    let mut chars = label.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => label,
    }
}

/// Дотягивается ли анализатор от `from` до моба у `feet`: шар радиуса
/// `ANALYZER_RANGE` плюс `margin` (запас снимка, который берётся раз в тик).
pub fn analyzer_reaches(from: [f64; 3], feet: [f64; 3], margin: f64) -> bool {
    let d: f64 = (0..3).map(|i| (feet[i] - from[i]).powi(2)).sum();
    let r = ANALYZER_RANGE + margin.max(0.0);
    d.is_finite() && d <= r * r
}

/// Рамка вокруг моба: `feet` — середина ног, `width` и `height` — хитбокс
/// вида, `scale` — во сколько раз мутант крупнее (`body_scale`: так его
/// рисует `biolab-deep`). Лежащего (спит, на столе) обводим низким квадратом:
/// куда он лёг головой, окну не известно. Рамка чуть шире моба, чтобы линии
/// не прятались в модели.
pub fn analyzer_box(feet: [f64; 3], width: f64, height: f64, scale: f64, lying: bool) -> ([f64; 3], [f64; 3]) {
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let (w, h) = if lying {
        (width.max(height) * scale, width.min(height) * scale)
    } else {
        (width * scale, height * scale)
    };
    let pad = 0.05;
    let half = w / 2.0 + pad;
    (
        [feet[0] - half, feet[1] - pad, feet[2] - half],
        [feet[0] + half, feet[1] + h + pad, feet[2] + half],
    )
}

/// Где моб в кадре: между прошлым и нынешним положением, `partial` — доля
/// тика (0–1).
pub fn analyzer_lerp(previous: [f64; 3], now: [f64; 3], partial: f32) -> [f64; 3] {
    let t = f64::from(partial.clamp(0.0, 1.0));
    [0, 1, 2].map(|i| previous[i] + (now[i] - previous[i]) * t)
}

/// Что окно знает о мобе из его данных (геном и операционная); снимает тик.
#[derive(Clone, Debug, PartialEq)]
pub struct AnalyzerSeen {
    pub id: u32,
    pub genes: Vec<(&'static str, i32)>,
    /// Бит «спит» (`ASLEEP_BIT`).
    pub asleep: bool,
    /// Лежит: спит или на столе.
    pub lying: bool,
}

impl AnalyzerSeen {
    /// Снимок по данным моба; None — рамке из них нечего взять.
    pub fn of(id: u32, genes: Vec<(&'static str, i32)>, surgery: u8) -> Option<Self> {
        let lying = surgery & (ASLEEP_BIT | TABLE_BIT) != 0;
        if genes.is_empty() && !lying {
            return None;
        }
        Some(Self { id, genes, asleep: surgery & ASLEEP_BIT != 0, lying })
    }
}

/// Снимок анализатора: None — анализатора в руке нет, рамки не рисуются.
fn analyzer_scan() -> &'static std::sync::Mutex<Option<Vec<AnalyzerSeen>>> {
    static SCAN: std::sync::OnceLock<std::sync::Mutex<Option<Vec<AnalyzerSeen>>>> = std::sync::OnceLock::new();
    SCAN.get_or_init(Default::default)
}

/// Тик окна: анализатор в руке, вот что видно рядом.
pub fn remember_analyzer_scan(seen: Vec<AnalyzerSeen>) {
    *analyzer_scan().lock().unwrap_or_else(|e| e.into_inner()) = Some(seen);
}

/// Тик окна: анализатора в руке нет.
pub fn forget_analyzer_scan() {
    *analyzer_scan().lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Для кадра: None — не рисовать ничего; иначе данные мобов по id.
pub fn shown_analyzer_scan() -> Option<std::collections::HashMap<u32, AnalyzerSeen>> {
    let scan = analyzer_scan().lock().unwrap_or_else(|e| e.into_inner());
    scan.as_ref().map(|seen| seen.iter().map(|s| (s.id, s.clone())).collect())
}

/// Слот машины для воронки, выбрасывателя и `native_insert_item`
/// (`native_container_accepts`): None — блок-сущность не наша машина (решает
/// не мод), иначе — берёт ли слот предмет. Правило то же, что у меню.
pub fn container_slot_accepts(entity: &str, slot: usize, item: &str) -> Option<bool> {
    if entity == CRYOBANK {
        return Some(slot < CRYOBANK_SLOTS && cryobank_accepts(item));
    }
    machine(entity).map(|machine| slot_accepts(machine, slot, item))
}

#[cfg(test)]
mod m11g_tests {
    use super::*;

    #[test]
    fn hoppers_ask_each_slot() {
        // Чан: фрагмент — только во вход, раствор — только в топливо. Раньше
        // фрагмент, пришедший сбоку, лёг бы в слот раствора: сбоку воронке
        // открыт только он, а «машина целиком» фрагмент берёт.
        assert_eq!(container_slot_accepts(CLONING_VAT, SLOT_INPUT, FRAGMENT_ID), Some(true));
        assert_eq!(container_slot_accepts(CLONING_VAT, SLOT_FUEL, FRAGMENT_ID), Some(false));
        assert_eq!(container_slot_accepts(CLONING_VAT, SLOT_FUEL, BROTH_ID), Some(true));
        assert_eq!(container_slot_accepts(CLONING_VAT, SLOT_INPUT, BROTH_ID), Some(false));
        // Выход не берёт ничего.
        for machine in &MACHINES {
            for slot in machine_outputs(machine) {
                for (_, items) in &MACHINE_INGREDIENTS {
                    for item in *items {
                        assert_eq!(container_slot_accepts(machine.id, slot, item), Some(false), "{} {slot} {item}", machine.id);
                    }
                }
            }
        }
        // Чужое — мимо, своё сырьё — во вход.
        assert_eq!(container_slot_accepts("biolab:still", SLOT_INPUT, "minecraft:dirt"), Some(false));
        assert_eq!(container_slot_accepts("biolab:still", SLOT_INPUT, "minecraft:kelp"), Some(true));
        assert_eq!(container_slot_accepts(SYNTHESIZER, 3, "biolab:element_oxygen"), Some(true));
        assert_eq!(container_slot_accepts(SYNTHESIZER, 8, "biolab:element_oxygen"), Some(false));
        // Не наша блок-сущность — решает не мод.
        assert_eq!(container_slot_accepts("minecraft:furnace", 0, "minecraft:kelp"), None);
        assert_eq!(container_slot_accepts(GM_PLOT, 0, "minecraft:kelp"), None);
    }

    #[test]
    fn every_face_a_hopper_fills_takes_something() {
        // Каждый слот, который `ModContainer` открывает воронке, что-то берёт
        // (иначе сторона мертва), а каждое сырьё машины находит свой слот
        // хоть с одной стороны.
        let known: Vec<&str> = MACHINE_INGREDIENTS
            .iter()
            .flat_map(|(_, items)| items.iter().copied())
            .chain([FRAGMENT_ID, BROTH_ID, FLASK_ID, "biolab:element_oxygen", PHOSPHORUS_ID, FILTER_CARTRIDGE])
            .collect();
        for machine in &MACHINES {
            let faces = machine_faces(machine);
            for slot in faces[1..].iter().flatten() {
                let takes = known.iter().any(|item| slot_accepts(machine, *slot, item));
                assert!(takes, "{}: слот {slot} ничего не берёт", machine.id);
            }
            for item in machine_ingredients(machine.id) {
                let reachable = faces[1..]
                    .iter()
                    .flatten()
                    .any(|slot| container_slot_accepts(machine.id, *slot, item) == Some(true));
                assert!(reachable, "{}: {item} воронкой не положить", machine.id);
            }
        }
    }

    #[test]
    fn analyzer_marks_carriers_and_mutants() {
        assert!(analyzer_carrier("minecraft:zombie", true), "вид зомби");
        assert!(analyzer_carrier("biolab:infected", false));
        assert!(!analyzer_carrier("minecraft:cow", false));
        assert!(!analyzer_carrier("cow", false));
        let horns = [("horns", 2)];
        assert_eq!(analyzer_mark(false, &[]), None);
        assert_eq!(analyzer_mark(true, &[]), Some(AnalyzerMark::Carrier));
        assert_eq!(analyzer_mark(false, &horns), Some(AnalyzerMark::Mutant));
        assert_eq!(analyzer_mark(true, &horns), Some(AnalyzerMark::CarrierMutant));
        let colors = [AnalyzerMark::Carrier, AnalyzerMark::Mutant, AnalyzerMark::CarrierMutant].map(analyzer_color);
        assert!(colors.iter().all(|c| c >> 24 == 0xFF), "непрозрачные");
        assert!(colors[0] != colors[1] && colors[1] != colors[2] && colors[0] != colors[2]);
    }

    #[test]
    fn analyzer_label_is_short() {
        assert_eq!(analyzer_label(AnalyzerMark::Carrier, &[], false), "Заражён");
        assert_eq!(analyzer_label(AnalyzerMark::Mutant, &[("chitin", 2)], false), "Мутант: Хитиновый панцирь ++");
        assert_eq!(
            analyzer_label(AnalyzerMark::Mutant, &[("gigantism", 1), ("chitin", 3), ("dwarfism", 3)], false),
            "Мутант: Хитиновый панцирь +++, ещё 2",
            "сильнейший, при равной силе — первый"
        );
        assert_eq!(
            analyzer_label(AnalyzerMark::CarrierMutant, &[("gigantism", 1)], true),
            "Заражён · мутант: Гигантизм + · спит"
        );
        // Каждый ген таблицы подписан своим именем, и подпись не разрастается.
        for kind in &GENES {
            let label = analyzer_label(AnalyzerMark::CarrierMutant, &[(kind.name, 3), ("gigantism", 1)], true);
            assert!(label.contains(kind.title), "{label}");
            assert!(label.chars().count() <= 64, "{label}");
        }
    }

    #[test]
    fn analyzer_reaches_sixteen_blocks() {
        let at = [0.0, 64.0, 0.0];
        assert!(analyzer_reaches(at, [16.0, 64.0, 0.0], 0.0));
        assert!(!analyzer_reaches(at, [12.0, 64.0, 12.0], 0.0), "по шару, а не по кубу");
        assert!(analyzer_reaches(at, [17.0, 64.0, 0.0], 2.0), "запас снимка");
        assert!(!analyzer_reaches(at, [f64::NAN, 64.0, 0.0], 2.0));
    }

    #[test]
    fn analyzer_box_wraps_the_mob() {
        // Зомби 0,6 × 1,95 стоит в (10, 64, 10).
        let (min, max) = analyzer_box([10.0, 64.0, 10.0], 0.6, 1.95, 1.0, false);
        assert!(min[0] < 9.7 && max[0] > 10.3 && min[1] < 64.0 && max[1] > 65.95);
        assert!((max[0] - min[0] - (max[2] - min[2])).abs() < 1e-9, "квадрат в плане");
        // Великан выше, лежащий — низкий и длинный.
        let (_, giant) = analyzer_box([0.0; 3], 0.6, 1.95, 1.5, false);
        assert!(giant[1] > 2.9);
        let (lmin, lmax) = analyzer_box([0.0; 3], 0.6, 1.95, 1.0, true);
        assert!(lmax[1] - lmin[1] < 0.8 && lmax[0] - lmin[0] > 1.9);
        // Битый масштаб — как у обычного моба.
        assert_eq!(analyzer_box([0.0; 3], 0.6, 1.95, f64::NAN, false), analyzer_box([0.0; 3], 0.6, 1.95, 1.0, false));
        assert_eq!(analyzer_lerp([0.0; 3], [2.0, 4.0, -2.0], 0.5), [1.0, 2.0, -1.0]);
        assert_eq!(analyzer_lerp([0.0; 3], [2.0, 4.0, -2.0], 7.0), [2.0, 4.0, -2.0]);
    }

    #[test]
    fn analyzer_scan_keeps_only_what_the_frame_needs() {
        assert_eq!(AnalyzerSeen::of(1, Vec::new(), 0), None);
        let seen = AnalyzerSeen::of(2, Vec::new(), ASLEEP_BIT).expect("спит");
        assert!(seen.asleep && seen.lying);
        let seen = AnalyzerSeen::of(3, vec![("horns", 1)], TABLE_BIT).expect("мутант на столе");
        assert!(!seen.asleep && seen.lying);
        // Срезы без стола и сна рамке не нужны.
        assert_eq!(AnalyzerSeen::of(4, Vec::new(), FLAYED_BITS), None);
        forget_analyzer_scan();
        assert_eq!(shown_analyzer_scan(), None);
        remember_analyzer_scan(vec![AnalyzerSeen::of(5, vec![("horns", 1)], 0).expect("мутант")]);
        assert_eq!(shown_analyzer_scan().expect("в руке")[&5].genes, vec![("horns", 1)]);
        forget_analyzer_scan();
    }
}

// ---- M11-А: Заражённый (GDD 4.26 А) ----
//
// Первый свой вид моба: `biolab:infected` — живой тип на шаблоне зомби
// (`data/biolab/horizon/entity_type/infected.json`). Тело, модель, звуки и
// цели — зомби; поверх — охота на скот (`infected_goals.rs`, хук
// `register_goals` lib-крейта) и болезнь в ударе. Здесь — чистая часть:
// таблица своих видов, штамм заражённого, очередь ударов и правила мазка.
//
// Кто ударил, хук урона (`native_living_incoming_damage`, API 50) знает только
// по номеру сущности и без доступа к игре, а удар моба по игроку приходит в
// пине вовсе без атакующего (`hurt_player_from` угадывает его, но хуку отдаёт
// 0). Поэтому хук урона только записывает удар в очередь
// (`infected_hurt.rs`), а разбирает её тик игрока (`infected_tick.rs`): там
// видны мобы, их виды и состояние игрока.

/// Вид «Заражённый».
pub const INFECTED_ID: &str = "biolab:infected";
/// Ватная палочка: ею берут мазок.
pub const SWAB_ID: &str = "biolab:swab";

/// Свои виды мода: id, шаблон (чьё у вида тело — без пространства имён) и
/// имя для чата. Таблицы операционной, носилок и массы (`short_kind`) видят
/// свой вид телом шаблона.
pub const MOD_MOBS: [(&str, &str, &str); 1] = [(INFECTED_ID, "zombie", "Заражённый")];

/// Шаблон своего вида мода; None — вид не наш.
pub fn mod_template(kind: &str) -> Option<&'static str> {
    MOD_MOBS.iter().find(|(id, _, _)| *id == kind).map(|(_, template, _)| *template)
}

/// Имя своего вида мода; None — вид не наш.
pub fn mod_title(kind: &str) -> Option<&'static str> {
    MOD_MOBS.iter().find(|(id, _, _)| *id == kind).map(|(_, _, title)| *title)
}

/// Подпись вида в журнале опытов и на пульте: ванильный — без
/// пространства имён (`cow`), свой — по имени («Заражённый»), а не
/// `biolab:infected`.
pub fn kind_label(kind: &str) -> &str {
    mod_title(kind).unwrap_or_else(|| kind.strip_prefix("minecraft:").unwrap_or(kind))
}

/// Заражённый ли это. Игра зовёт ванильные виды и с пространством имён, и
/// без; свой — всегда с ним.
pub fn is_infected(kind: &str) -> bool {
    kind == INFECTED_ID
}

/// Штамм заражённого: заразнее дикого (3 против 2), инкубация короче, курс
/// чуть длиннее. Его получает тот, кого заражённый ранил, и культура с
/// него — мазком или из добычи.
pub const INFECTED_STRAIN: Strain = Strain {
    infectivity: 3,
    incubation: 2,
    severity: 2,
    resistance: 2,
};

/// Насколько поднимает заражение игрока удар заражённого: втрое больше
/// близкого контакта с зомби (`CONTACT_STEP`).
pub const INFECTED_HIT_STEP: u32 = 10;

/// Сколько держится болезнь на мобе, которого ранил заражённый или заразил
/// сосед по стаду: десять минут. Болезнь моба — эффект `biolab:infection`
/// (его и видит пульт: «болен»).
pub const INFECTED_MOB_TICKS: i32 = 12_000;

/// На кого заражённый охотится, кроме игрока и жителей (цели зомби): скот.
/// Те же виды — в цели `infected_goals.rs` (там — `Species` игры).
pub const INFECTED_PREY: [&str; 4] =
    ["minecraft:cow", "minecraft:pig", "minecraft:sheep", "minecraft:chicken"];

/// Кто из мобов болеет от удара заражённого и передаёт болезнь соседям по
/// стаду: домашняя живность.
pub const CATCHES_INFECTION: [&str; 12] = [
    "minecraft:cow",
    "minecraft:pig",
    "minecraft:sheep",
    "minecraft:chicken",
    "minecraft:mooshroom",
    "minecraft:goat",
    "minecraft:rabbit",
    "minecraft:horse",
    "minecraft:donkey",
    "minecraft:mule",
    "minecraft:llama",
    "minecraft:camel",
];

/// Болеет ли моб этого вида от удара заражённого.
pub fn catches_infection(kind: &str) -> bool {
    CATCHES_INFECTION.contains(&full_kind(kind).as_str())
}

/// Тип урона ближнего удара моба (`DamageSource.mobAttack`).
pub const MOB_ATTACK: &str = "minecraft:mob_attack";
/// Как хук урона называет игрока.
pub const PLAYER_KIND: &str = "minecraft:player";

/// Удар моба, отложенный до тика игрока.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobHit {
    /// Номер сущности того, кого ударили.
    pub victim: i32,
    /// Ударили игрока.
    pub player: bool,
    /// Номер ударившего; 0 — хук его не знает (удар по игроку).
    pub causing: i32,
    /// Сколько раз удар уже переложили: он ждал тика своего игрока.
    pub age: u8,
}

/// Сколько ударов держит очередь: если её никто не разбирает (на сервере
/// нет игроков), она не растёт бесконечно.
pub const MOB_HITS_CAP: usize = 256;
/// Сколько раз удар по игроку перекладывают, пока не дойдёт тик того, кого
/// ударили (по гостю LAN — тик гостя). Дальше — забываем.
pub const MOB_HIT_MAX_AGE: u8 = 4;

/// Удар, который стоит запомнить; None — не ближний удар моба или вид,
/// которому болезнь от удара не страшна.
pub fn hit_worth_noting(victim: i32, kind: &str, damage_type: &str, causing: i32) -> Option<MobHit> {
    if damage_type != MOB_ATTACK {
        return None;
    }
    if kind == PLAYER_KIND {
        return Some(MobHit { victim, player: true, causing, age: 0 });
    }
    (causing > 0 && catches_infection(kind)).then_some(MobHit { victim, player: false, causing, age: 0 })
}

fn mob_hits() -> &'static std::sync::Mutex<Vec<MobHit>> {
    static HITS: std::sync::OnceLock<std::sync::Mutex<Vec<MobHit>>> = std::sync::OnceLock::new();
    HITS.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// Хук урона: запомнить удар (`infected_hurt.rs`). Зовётся на потоке
/// сервера посреди удара, поэтому — только запись.
pub fn note_mob_hit(victim: i32, kind: &str, damage_type: &str, causing: i32) {
    let Some(hit) = hit_worth_noting(victim, kind, damage_type, causing) else {
        return;
    };
    if let Ok(mut hits) = mob_hits().lock()
        && hits.len() < MOB_HITS_CAP
    {
        hits.push(hit);
    }
}

/// Тик игрока: забрать накопленные удары.
pub fn take_mob_hits() -> Vec<MobHit> {
    mob_hits().lock().map(|mut hits| std::mem::take(&mut *hits)).unwrap_or_default()
}

/// Удары по другим игрокам — обратно в очередь, на их тик; постаревшие
/// забываются.
pub fn requeue_mob_hits(rest: Vec<MobHit>) {
    let keep: Vec<MobHit> = rest
        .into_iter()
        .filter(|hit| hit.age < MOB_HIT_MAX_AGE)
        .map(|hit| MobHit { age: hit.age + 1, ..hit })
        .collect();
    if keep.is_empty() {
        return;
    }
    if let Ok(mut hits) = mob_hits().lock() {
        let room = MOB_HITS_CAP.saturating_sub(hits.len());
        hits.extend(keep.into_iter().take(room));
    }
}

/// Насколько далеко ищем ударившего игрока, если хук его не назвал: удар
/// рукой достаёт на 2–3 блока.
pub const MELEE_GUESS_RANGE: f64 = 3.0;

/// Кто ударил игрока: названный хуком, иначе ближайший из тех, кто охотится
/// на игрока, в пределах удара (так же гадает сама игра,
/// `hurt_player_from`). `near` — номер моба и квадрат расстояния до игрока.
pub fn guess_attacker(causing: i32, near: &[(u32, f64)]) -> Option<u32> {
    if causing > 0 {
        return Some(causing as u32);
    }
    near.iter()
        .filter(|(_, d2)| d2.is_finite() && *d2 <= MELEE_GUESS_RANGE * MELEE_GUESS_RANGE)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| *id)
}

/// Сообщение игроку, которого заражённый ранил впервые (до этого был здоров).
pub const INFECTED_HIT_MESSAGE: &str = "Заражённый поцарапал вас: в крови чужой штамм";

/// Штамм, которым заражает близкий контакт: рядом заражённый — его, иначе
/// дикий (зомби). `kinds` — виды мобов вплотную к игроку.
pub fn contact_strain<'a>(kinds: impl IntoIterator<Item = &'a str>) -> Strain {
    if kinds.into_iter().any(is_infected) {
        INFECTED_STRAIN
    } else {
        WILD
    }
}

/// Шанс за проверку, что больной моб заразит соседа по стаду: 5 % за
/// единицу заразности штамма.
pub const SPREAD_PERCENT_PER_INFECTIVITY: u64 = 5;

/// Заразит ли больной сосед здорового на этой проверке. `seed` — от
/// номеров обоих и тика: прогон повторяется.
pub fn spreads(strain: &Strain, seed: u64) -> bool {
    seed % 100 < SPREAD_PERCENT_PER_INFECTIVITY * strain.infectivity.clamp(0, 5) as u64
}

/// Пора ли проверять стадо: раз в шаг болезни (`INFECTION_EVERY`) и один
/// раз за тик сервера, сколько бы игроков ни было в мире.
pub fn herd_check_due(ticks: u64) -> bool {
    static LAST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
    ticks % INFECTION_EVERY == 0 && LAST.swap(ticks, std::sync::atomic::Ordering::Relaxed) != ticks
}

/// Зерно броска заражения соседа.
pub fn herd_seed(sick: u32, healthy: u32, ticks: u64) -> u64 {
    (u64::from(sick).wrapping_mul(2_654_435_761) ^ u64::from(healthy).wrapping_mul(40_503) ^ ticks)
        .wrapping_mul(6_364_136_223_846_793_005)
        >> 33
}

/// Зомби разных видов — носители дикого штамма.
pub const WILD_CARRIERS: [&str; 4] =
    ["minecraft:zombie", "minecraft:husk", "minecraft:drowned", "minecraft:zombie_villager"];

/// Штамм, который даёт мазок с моба: заражённый и больной моб (заболеть он
/// может только от заражённого) — штамм заражённого, зомби — дикий, со
/// здорового — ничего.
pub fn swab_strain(kind: &str, sick: bool) -> Option<Strain> {
    if is_infected(kind) || sick {
        return Some(INFECTED_STRAIN);
    }
    WILD_CARRIERS.contains(&full_kind(kind).as_str()).then_some(WILD)
}

/// Подпись палочки с мазком: «Мазок: З3 И2 Т2 С2».
pub fn swab_label(strain: &Strain) -> String {
    culture_label(strain).replacen("Культура", "Мазок", 1)
}

/// Что сказать, когда мазок взят.
pub fn swab_message(kind: &str, strain: &Strain) -> String {
    format!("Мазок взят: {} ({})", mob_title(kind), swab_label(strain))
}

/// Что сказать, когда мазок брать не с чего.
pub const SWAB_CLEAN: &str = "Мазок чистый: этот моб ничем не болен";
/// Палочка в руке уже с мазком.
pub const SWAB_USED: &str = "На палочке уже есть мазок: возьмите чистую";
/// Положить палочку с мазком некуда.
pub const SWAB_NO_ROOM: &str = "Некуда положить мазок: освободите место в инвентаре";

/// Штамм культуры, которую вырастил инкубатор. Мазок с моба даёт штамм
/// этого моба как есть; прогон готовой культуры — следующее поколение со
/// сдвигом (`select`); чистая палочка — дикую линию.
pub fn incubated_strain(input: Option<&str>, before: Option<Strain>, seed: u64) -> Strain {
    match (input, before) {
        (Some(SWAB_ID), Some(strain)) => strain,
        (_, Some(strain)) => select(&strain, seed),
        (_, None) => WILD,
    }
}

#[cfg(test)]
mod infected_tests {
    use super::*;

    #[test]
    fn infected_has_a_zombie_body_and_its_own_name() {
        assert_eq!(mod_template(INFECTED_ID), Some("zombie"));
        assert_eq!(mod_template("minecraft:zombie"), None);
        assert_eq!(mob_title(INFECTED_ID), "Заражённый");
        // Тело — зомби: четыре конечности по-человечьи, носилки, масса и
        // длина на столе как у зомби.
        assert_eq!(limbs_of(INFECTED_ID), limbs_of("minecraft:zombie"));
        assert_eq!(limb_name(INFECTED_ID, 0), "правая рука");
        assert!(can_carry(INFECTED_ID));
        assert_eq!(species_mass(INFECTED_ID, 0.6, 1.95), species_mass("minecraft:zombie", 0.6, 1.95));
        assert_eq!(table_sections(INFECTED_ID, 0.6, 1.95, &[]), table_sections("minecraft:zombie", 0.6, 1.95, &[]));
        // Носилки пишут вид полностью: `native_spawn_mob` вернёт его на стол
        // своим видом, а не зомби.
        assert_eq!(full_kind(INFECTED_ID), INFECTED_ID);
        let patient = Patient {
            kind: INFECTED_ID.to_string(),
            genes: vec![],
            health: 24.0,
            flayed: 0,
            name: None,
            sleeper: None,
            paralysed: false,
        };
        assert_eq!(patient_from_text(&patient_to_text(&patient)).map(|p| p.kind), Some(INFECTED_ID.to_string()));
        // Журнал и пульт подписывают его по имени, ванильных — как прежде.
        assert_eq!(kind_label(INFECTED_ID), "Заражённый");
        assert_eq!(kind_label("minecraft:cow"), "cow");
        assert_eq!(trial_record(CULTURE_ID, INFECTED_ID), "Заражённый ← Культура");
        let lines = console_lines(&[(INFECTED_ID.to_string(), 20.0, 24.0, vec![], true)], &[]);
        assert!(lines.iter().any(|l| l.starts_with("Заражённый: 20/24 HP, болен")), "{lines:?}");
    }

    #[test]
    fn only_mob_melee_on_players_and_livestock_is_queued() {
        let player = hit_worth_noting(7, PLAYER_KIND, MOB_ATTACK, 0);
        assert_eq!(player, Some(MobHit { victim: 7, player: true, causing: 0, age: 0 }));
        assert!(hit_worth_noting(9, "minecraft:cow", MOB_ATTACK, 3).is_some());
        assert!(hit_worth_noting(9, "cow", MOB_ATTACK, 3).is_some(), "вид без пространства имён");
        assert!(hit_worth_noting(9, "minecraft:cow", MOB_ATTACK, 0).is_none(), "без атакующего мобу не узнать, кто бил");
        assert!(hit_worth_noting(9, "minecraft:cow", "minecraft:arrow", 3).is_none());
        assert!(hit_worth_noting(7, PLAYER_KIND, "minecraft:fall", 0).is_none());
        assert!(hit_worth_noting(9, "minecraft:skeleton", MOB_ATTACK, 3).is_none(), "нежить не болеет");
        for prey in INFECTED_PREY {
            assert!(catches_infection(prey), "{prey}: добыча заражённого болеет от его удара");
        }
    }

    #[test]
    fn queue_is_capped_and_requeued_hits_age_out() {
        // Очередь общая для процесса: тест держит её сам от начала до конца.
        let _ = take_mob_hits();
        for i in 0..(MOB_HITS_CAP as i32 + 10) {
            note_mob_hit(i, PLAYER_KIND, MOB_ATTACK, 0);
        }
        let hits = take_mob_hits();
        assert_eq!(hits.len(), MOB_HITS_CAP);
        assert!(take_mob_hits().is_empty(), "забранные удары уходят из очереди");
        let mut hit = vec![hits[0]];
        for age in 1..=MOB_HIT_MAX_AGE {
            requeue_mob_hits(hit);
            hit = take_mob_hits();
            assert_eq!(hit.len(), 1);
            assert_eq!(hit[0].age, age);
        }
        requeue_mob_hits(hit);
        assert!(take_mob_hits().is_empty(), "удар, не дождавшийся своего игрока, забыт");
    }

    #[test]
    fn attacker_is_the_named_one_or_the_nearest_in_reach() {
        assert_eq!(guess_attacker(12, &[(3, 1.0)]), Some(12));
        assert_eq!(guess_attacker(0, &[(3, 4.0), (4, 1.0), (5, 0.5 + 100.0)]), Some(4));
        assert_eq!(guess_attacker(0, &[(3, 9.5)]), None, "дальше удара");
        assert_eq!(guess_attacker(0, &[(3, f64::NAN)]), None);
        assert_eq!(guess_attacker(0, &[]), None);
    }

    #[test]
    fn infected_strain_is_its_own_and_moderately_contagious() {
        assert_ne!(INFECTED_STRAIN, WILD);
        assert_eq!(Strain::of(INFECTED_STRAIN.values()), INFECTED_STRAIN, "в пределах 1–5");
        assert!(INFECTED_STRAIN.infectivity > WILD.infectivity && INFECTED_STRAIN.infectivity <= 3);
        assert!(INFECTED_HIT_STEP > CONTACT_STEP);
        assert_eq!(contact_strain(["minecraft:zombie", INFECTED_ID]), INFECTED_STRAIN);
        assert_eq!(contact_strain(["minecraft:zombie"]), WILD);
        assert_eq!(contact_strain([]), WILD);
    }

    #[test]
    fn herd_spread_follows_infectivity() {
        let share = |strain: &Strain| (0..10_000u64).filter(|s| spreads(strain, herd_seed(*s as u32, 77, *s))).count();
        let wild = share(&WILD);
        let infected = share(&INFECTED_STRAIN);
        assert!((800..1200).contains(&wild), "дикий — около 10 %: {wild}");
        assert!((1300..1700).contains(&infected), "заражённого — около 15 %: {infected}");
        assert!(!spreads(&Strain { infectivity: 0, ..WILD }, 0));
        // Один раз за тик, только на шаге болезни.
        assert!(herd_check_due(INFECTION_EVERY * 1_000_003));
        assert!(!herd_check_due(INFECTION_EVERY * 1_000_003), "второй игрок того же тика стадо не проверяет");
        assert!(!herd_check_due(INFECTION_EVERY * 1_000_003 + 1));
    }

    /// Данные вида: живой тип на шаблоне зомби с атрибутами, которые у зомби
    /// есть (иначе сборка корня контента остановится), своя таблица добычи,
    /// спавн в болотах и имя на обоих языках.
    #[test]
    fn infected_content_is_complete() {
        let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let read = |path: &str| {
            let text = std::fs::read_to_string(content.join(path)).unwrap_or_else(|_| panic!("нет {path}"));
            Json::parse(&text).unwrap_or_else(|| panic!("битый {path}"))
        };
        let short = INFECTED_ID.trim_start_matches("biolab:");
        let kind = read(&format!("data/biolab/horizon/entity_type/{short}.json"));
        assert_eq!(kind.get("template").and_then(Json::str), Some("minecraft:zombie"));
        assert_eq!(kind.get("item"), None, "у живого типа нет предмета");
        let attrs = kind.get("attributes").expect("свои атрибуты");
        assert_eq!(attrs.get("minecraft:max_health").and_then(Json::num), Some(24.0));
        let speed = attrs.get("minecraft:movement_speed").and_then(Json::num).unwrap();
        assert!(speed < 0.23 && speed > 0.15, "чуть медленнее зомби: {speed}");
        let loot = read(&format!("data/biolab/loot_table/entities/{short}.json"));
        assert_eq!(loot.get("type").and_then(Json::str), Some("minecraft:entity"));
        assert_eq!(loot.get("random_sequence").and_then(Json::str), Some("biolab:entities/infected"), "без неё моб ничего не роняет");
        let text = std::fs::read_to_string(content.join(format!("data/biolab/loot_table/entities/{short}.json"))).unwrap();
        assert!(text.contains(CULTURE_ID) && text.contains("minecraft:rotten_flesh"));
        assert!(!text.contains("\"functions\"") && !text.contains("\"conditions\""), "формат 26.3: modifier и condition");
        let spawns = read("data/biolab/neoforge/biome_modifier/infected_spawns.json");
        assert_eq!(spawns.get("type").and_then(Json::str), Some("neoforge:add_spawns"));
        let spawner = spawns.get("spawners").unwrap();
        assert_eq!(spawner.get("type").and_then(Json::str), Some(INFECTED_ID));
        let biomes: Vec<&str> = spawns.get("biomes").and_then(Json::arr).unwrap().iter().filter_map(Json::str).collect();
        assert_eq!(biomes, ["minecraft:swamp", "minecraft:mangrove_swamp"]);
        for (name, title) in [("ru_ru", "Заражённый"), ("en_us", "Infected")] {
            let lang = read(&format!("assets/biolab/lang/{name}.json"));
            assert_eq!(lang.get(&format!("entity.biolab.{short}")).and_then(Json::str), Some(title), "{name}");
        }
        assert_eq!(mob_title(INFECTED_ID), "Заражённый", "имя в чате — как в языке");
    }

    #[test]
    fn swab_takes_the_strain_of_the_mob() {
        assert_eq!(swab_strain(INFECTED_ID, false), Some(INFECTED_STRAIN));
        assert_eq!(swab_strain("minecraft:cow", true), Some(INFECTED_STRAIN), "заболевший скот");
        assert_eq!(swab_strain("minecraft:zombie", false), Some(WILD));
        assert_eq!(swab_strain("husk", false), Some(WILD));
        assert_eq!(swab_strain("minecraft:cow", false), None);
        assert_eq!(swab_label(&INFECTED_STRAIN), "Мазок: З3 И2 Т2 С2");
        assert!(swab_message(INFECTED_ID, &INFECTED_STRAIN).contains("Заражённый"));
        // Инкубатор: мазок — штамм как есть, культура — следующее поколение,
        // чистая палочка — дикая линия.
        assert_eq!(incubated_strain(Some(SWAB_ID), Some(INFECTED_STRAIN), 5), INFECTED_STRAIN);
        assert_eq!(incubated_strain(Some(CULTURE_ID), Some(INFECTED_STRAIN), 5), select(&INFECTED_STRAIN, 5));
        assert_eq!(incubated_strain(Some(SWAB_ID), None, 5), WILD);
        assert_eq!(incubated_strain(None, None, 5), WILD);
        // Штамм в данных палочки читается тем же разбором, что у культуры.
        assert_eq!(strain_from_text(&strain_to_text(&INFECTED_STRAIN)), Some(INFECTED_STRAIN));
    }
}

// ---- M8: многоблочные установки (GDD 4.18) ----
//
// Общий каркас для всех установок: ректификационной колонны (первая, на ней
// каркас отлажен), биореактора, карантинной камеры и генного секвенатора.
// Здесь только решения — схемы, проверка сборки, тексты нехватки, раскладка
// чисел для экрана, раскладка фракций колонны. Исполнение на игре — модули
// `crate::game::biolab_multiblock` (`multiblock.rs`: раз в секунду проверить
// схему, склеить или расклеить блоки, носить предметы через порты) и
// `crate::game::biolab_column` (`column.rs`: работа колонны). Тик контроллера
// зовёт их из `machine.rs`.
//
// *Контроллер* — обычная машина из `MACHINES` (блок-сущность, слоты, своё
// меню и экран, компаратор, лампа `lit`, воронки на нём самом), у которой
// есть строка в `MULTIBLOCKS`: схема, тексты нехватки и роли слотов.
//
// *Схема* (`Shape`) — ярусы снизу вверх. Ярусы до `controller` стоят под
// контроллером по одному, ярус `controller` — его собственный (сам
// контроллер — точка [0, 0] этого яруса, клеткой не пишется), ярусы выше
// могут повторяться (`min..=max`: тарелки колонны). Клетка — место
// относительно контроллера: `[вправо, назад]`, где «назад» — вглубь
// установки от лица контроллера, «вправо» — направо для игрока, который
// смотрит на это лицо (`Facing`). На клетке может стоять любая из частей
// её `Need`; необязательная клетка (`optional`) нехватки не даёт — так
// стоят порты колонны.
//
// *Проверка* — чистая функция: на вход замыкание «какой блок в точке», на
// выход `Verdict`: собрано (`Assembly`: секции, найденные части и порты)
// или нехватка (`Shortage`: что, сколько, ярус, сторона), которую
// `shortage_text` пишет человеческим текстом. Проверка другого рода
// (карантинная камера — замкнутость комнаты заливкой) встаёт через
// `Check::Custom`: своя функция с той же подписью, отвечает тем же
// `Verdict`.

/// Корпус установки: стенки, пол и потолок больших установок.
pub const CASING: &str = "biolab:casing";
/// Стекло установки: то же, что корпус, но видно внутрь.
pub const CASING_GLASS: &str = "biolab:casing_glass";
/// Порт входа: контроллер забирает из него сырьё.
pub const INPUT_PORT: &str = "biolab:input_port";
/// Порт выхода: контроллер кладёт в него готовое.
pub const OUTPUT_PORT: &str = "biolab:output_port";
/// Ректификационная колонна: контроллер — куб-кипятильник внизу башни.
pub const COLUMN: &str = "biolab:column";
/// Тарелка колонны — секция башни.
pub const COLUMN_TRAY: &str = "biolab:column_tray";
/// Конденсатор — вершина колонны.
pub const COLUMN_CONDENSER: &str = "biolab:column_condenser";

/// Свойство «склеенного» вида: свой `state_properties` блока (boolean), его
/// ваниль не трогает — ставит только контроллер (`native_set_block_property`).
pub const FORMED: &str = "formed";
/// Блоки со свойством `formed` (данные блоков объявляют его в
/// `state_properties`, тест сверяет).
pub const FORMABLE: [&str; 7] = [CASING, CASING_GLASS, COLUMN_TRAY, COLUMN_CONDENSER, SEQUENCER_RACK, SEQUENCER_MONITOR, BIOREACTOR];

/// Есть ли у блока «склеенный» вид.
pub fn is_formable(id: &str) -> bool {
    FORMABLE.contains(&id)
}

/// Значение свойства `formed`.
pub fn formed_value(formed: bool) -> &'static str {
    if formed { "true" } else { "false" }
}

/// Порт: вход или выход.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortKind {
    Input,
    Output,
}

/// Порт установки: блок со слотами, к которому цепляют воронки.
pub struct Port {
    pub id: &'static str,
    pub kind: PortKind,
}

/// Сколько слотов у порта: как у воронки.
pub const PORT_SLOTS: usize = 5;

pub const PORTS: [Port; 2] = [
    Port { id: INPUT_PORT, kind: PortKind::Input },
    Port { id: OUTPUT_PORT, kind: PortKind::Output },
];

/// Порт с таким id.
pub fn port(id: &str) -> Option<&'static Port> {
    PORTS.iter().find(|p| p.id == id)
}

/// Слоты порта для воронок: стороны, куда кладут, откуда забирают.
/// Входной — воронка кладёт с любой стороны, забрать нельзя; выходной —
/// наоборот. Руками из меню забирают из обоих, кладут только во входной
/// (у выходного все слоты — `results` его меню).
pub fn port_faces(port: &Port) -> ([Vec<usize>; 6], Vec<usize>, Vec<usize>) {
    let all: Vec<usize> = (0..PORT_SLOTS).collect();
    let faces = std::array::from_fn(|_| all.clone());
    match port.kind {
        PortKind::Input => (faces, all, Vec::new()),
        PortKind::Output => (faces, Vec::new(), all),
    }
}

/// Нужна ли блоку блок-сущность мода: машинам, грядке и портам.
pub fn has_block_entity(id: &str) -> bool {
    is_machine(id) || id == GM_PLOT || port(id).is_some() || id == CRYOBANK
}

/// Куда смотрит лицо контроллера (`facing` шаблона печи: сторона, где лицо).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    North,
    South,
    West,
    East,
}

impl Facing {
    pub fn from_name(name: &str) -> Option<Facing> {
        match name {
            "north" => Some(Facing::North),
            "south" => Some(Facing::South),
            "west" => Some(Facing::West),
            "east" => Some(Facing::East),
            _ => None,
        }
    }

    /// Шаг «вглубь установки» по X и Z — от лица контроллера назад.
    pub fn back(self) -> [i32; 2] {
        match self {
            Facing::North => [0, 1],
            Facing::South => [0, -1],
            Facing::West => [1, 0],
            Facing::East => [-1, 0],
        }
    }

    /// Шаг «вправо» для игрока, который стоит перед лицом и смотрит на него
    /// (то есть смотрит «назад»): у взгляда на север правая рука — восток.
    pub fn right(self) -> [i32; 2] {
        let [x, z] = self.back();
        [-z, x]
    }
}

/// Точка установки: контроллер в `origin`, клетка `[вправо, вверх, назад]`.
pub fn place(origin: [i32; 3], facing: Facing, at: [i32; 3]) -> [i32; 3] {
    let (r, b) = (facing.right(), facing.back());
    [
        origin[0] + at[0] * r[0] + at[2] * b[0],
        origin[1] + at[1],
        origin[2] + at[0] * r[1] + at[2] * b[1],
    ]
}

/// Что может стоять на клетке схемы.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// Блок с этим id.
    Block(&'static str),
    /// Источник тепла — те же блоки, что греют реактор (`heat_target`).
    Heat,
    /// Любой порт (`PORTS`).
    Port,
    /// Пусто: воздух (внутренность биореактора).
    Air,
}

impl Part {
    pub fn matches(self, block: &str) -> bool {
        match self {
            Part::Block(id) => id == block,
            Part::Heat => heat_target(block).is_some(),
            Part::Port => port(block).is_some(),
            Part::Air => is_air(block),
        }
    }
}

/// Что нужно на клетку и как это назвать в нехватке: «Нет тарелки», «Нет 3
/// блоков корпуса» — родительный падеж, единственное и множественное.
pub struct Need {
    pub parts: &'static [Part],
    pub one: &'static str,
    pub many: &'static str,
}

impl Need {
    pub fn matches(&self, block: &str) -> bool {
        self.parts.iter().any(|p| p.matches(block))
    }
}

/// Клетка яруса: `[вправо, назад]` от контроллера, номер нужды в `needs`
/// установки, необязательна ли.
pub struct Cell {
    pub at: [i32; 2],
    pub need: usize,
    pub optional: bool,
}

/// Обязательная клетка.
pub const fn cell(right: i32, back: i32, need: usize) -> Cell {
    Cell { at: [right, back], need, optional: false }
}

/// Необязательная клетка: если там то, что нужно, — часть установки (порт),
/// если нет — не беда.
pub const fn maybe(right: i32, back: i32, need: usize) -> Cell {
    Cell { at: [right, back], need, optional: true }
}

/// Ярус: клетки, сколько раз подряд повторяется (`min..=max`, обычно 1) и
/// имя для нехватки. Пустое имя — «N-й ярус», счёт снизу по ярусам без
/// имени; с именем («под кипятильником») — номер ярус не занимает.
pub struct Layer {
    pub cells: &'static [Cell],
    pub min: u32,
    pub max: u32,
    pub name: &'static str,
}

/// Схема из ярусов снизу вверх. `controller` — номер яруса контроллера:
/// ниже него ярусы стоят по одному, выше — могут повторяться.
pub struct Shape {
    pub layers: &'static [Layer],
    pub controller: usize,
}

/// Где стоит контроллер установки.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Site {
    pub origin: [i32; 3],
    pub facing: Facing,
}

/// «Какой блок в точке»: id без свойств, воздух — `minecraft:air`.
pub type BlockAt<'a> = &'a dyn Fn([i32; 3]) -> String;

/// Своя проверка установки (карантинная камера — заливкой комнаты): та же
/// подпись и тот же ответ, что у проверки по схеме.
pub type CustomCheck = fn(&Multiblock, Site, BlockAt) -> Verdict;

/// Вид проверки установки.
pub enum Check {
    /// По схеме ярусов.
    Shape(Shape),
    /// Своей функцией.
    Custom(CustomCheck),
}

/// Установка: её контроллер (машина из `MACHINES`), название, нужды для
/// текстов нехватки, проверка и роли слотов контроллера.
pub struct Multiblock {
    pub controller: &'static str,
    pub title: &'static str,
    pub needs: &'static [Need],
    pub check: Check,
    pub slots: &'static [SlotRole],
}

/// Сторона клетки в ярусе — для текста нехватки.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Ярус в одну клетку — сторона не нужна.
    None,
    Left,
    Right,
    Front,
    Back,
    /// Не у стенки: внутри или в середине пола и потолка.
    Middle,
}

impl Side {
    pub const ALL: [Side; 6] = [Side::None, Side::Left, Side::Right, Side::Front, Side::Back, Side::Middle];

    pub fn word(self) -> &'static str {
        match self {
            Side::None => "",
            Side::Left => "слева",
            Side::Right => "справа",
            Side::Front => "спереди",
            Side::Back => "сзади",
            Side::Middle => "в середине",
        }
    }
}

/// Ярус в тексте нехватки.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Без яруса (своя проверка).
    None,
    /// «N-й ярус», счёт снизу с 1.
    Number(u32),
    /// Ярус с именем: номер яруса в схеме.
    Named(usize),
}

/// Нехватка: чего (номер в `needs`), сколько, где.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortage {
    pub need: usize,
    pub count: u32,
    pub tier: Tier,
    pub side: Side,
}

/// Собранная установка.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Assembly {
    /// Сколько раз повторился первый повторяющийся ярус (тарелки колонны).
    pub sections: u32,
    /// Найденные части, кроме контроллера, портов и воздуха: им склеивать вид.
    pub parts: Vec<[i32; 3]>,
    /// Найденные порты.
    pub ports: Vec<([i32; 3], PortKind)>,
}

impl Assembly {
    pub fn inputs(&self) -> impl Iterator<Item = [i32; 3]> + '_ {
        self.ports.iter().filter(|(_, k)| *k == PortKind::Input).map(|(p, _)| *p)
    }

    pub fn outputs(&self) -> impl Iterator<Item = [i32; 3]> + '_ {
        self.ports.iter().filter(|(_, k)| *k == PortKind::Output).map(|(p, _)| *p)
    }
}

/// Итог проверки.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Formed(Assembly),
    Broken(Vec<Shortage>),
}

/// Сторона клетки по крайним клеткам яруса: у края по «вправо» — слева или
/// справа (углы тоже), иначе у края по «назад» — спереди или сзади, иначе в
/// середине. У яруса в одну клетку стороны нет.
pub fn cell_side(layer: &Layer, at: [i32; 2]) -> Side {
    let span = |axis: usize| {
        let values = layer.cells.iter().map(|c| c.at[axis]);
        (values.clone().min().unwrap_or(0), values.max().unwrap_or(0))
    };
    let ((rmin, rmax), (bmin, bmax)) = (span(0), span(1));
    if rmax > rmin {
        if at[0] == rmin {
            return Side::Left;
        }
        if at[0] == rmax {
            return Side::Right;
        }
    }
    if bmax > bmin {
        if at[1] == bmin {
            return Side::Front;
        }
        if at[1] == bmax {
            return Side::Back;
        }
    }
    if rmax > rmin || bmax > bmin { Side::Middle } else { Side::None }
}

/// Проверка по схеме. Ярусы ниже контроллера — по одному вниз, выше —
/// повторяющийся ярус берётся жадно, пока он целиком собран (не больше
/// `max`); если вышло меньше `min`, нехватка — в первом несобранном, а
/// следующие ярусы ждут выше, как будто повторов `min`. Поэтому ярус после
/// повторяющегося должен от него отличаться (конденсатор — не тарелка).
pub fn check_shape(mb: &Multiblock, shape: &Shape, site: Site, at: BlockAt) -> Verdict {
    let block_at = |cell: &Cell, y: i32| at(place(site.origin, site.facing, [cell.at[0], y, cell.at[1]]));
    let complete = |layer: &Layer, y: i32| {
        layer.cells.iter().all(|c| c.optional || mb.needs.get(c.need).is_some_and(|n| n.matches(&block_at(c, y))))
    };

    // Ярусы по местам: (номер яруса в схеме, высота над контроллером).
    let mut placed: Vec<(usize, i32)> = Vec::new();
    for (i, _) in shape.layers.iter().enumerate().take(shape.controller).rev() {
        placed.push((i, i as i32 - shape.controller as i32));
    }
    placed.push((shape.controller, 0));
    let mut sections = None;
    let mut y = 1;
    for (i, layer) in shape.layers.iter().enumerate().skip(shape.controller + 1) {
        let mut found = 0;
        while found < layer.max && complete(layer, y + found as i32) {
            found += 1;
        }
        if layer.min != layer.max && sections.is_none() {
            sections = Some(found);
        }
        for k in 0..found.max(layer.min) {
            placed.push((i, y + k as i32));
        }
        y += found.max(layer.min) as i32;
    }
    placed.sort_by_key(|(_, y)| *y);

    let mut assembly = Assembly { sections: sections.unwrap_or(0), ..Default::default() };
    let mut short: Vec<Shortage> = Vec::new();
    let mut number = 0;
    for (i, y) in placed {
        let layer = &shape.layers[i];
        let tier = if layer.name.is_empty() {
            number += 1;
            Tier::Number(number)
        } else {
            Tier::Named(i)
        };
        for c in layer.cells {
            let Some(need) = mb.needs.get(c.need) else {
                continue;
            };
            let pos = place(site.origin, site.facing, [c.at[0], y, c.at[1]]);
            let block = at(pos);
            if need.matches(&block) {
                match port(&block) {
                    Some(p) => assembly.ports.push((pos, p.kind)),
                    None if !is_air(&block) => assembly.parts.push(pos),
                    None => {}
                }
                continue;
            }
            if c.optional {
                continue;
            }
            let side = cell_side(layer, c.at);
            match short.iter_mut().find(|s| s.need == c.need && s.tier == tier && s.side == side) {
                Some(s) => s.count += 1,
                None => short.push(Shortage { need: c.need, count: 1, tier, side }),
            }
        }
    }
    if short.is_empty() { Verdict::Formed(assembly) } else { Verdict::Broken(short) }
}

/// Проверка установки её видом проверки.
pub fn check_multiblock(mb: &Multiblock, site: Site, at: BlockAt) -> Verdict {
    match &mb.check {
        Check::Shape(shape) => check_shape(mb, shape, site, at),
        Check::Custom(check) => check(mb, site, at),
    }
}

/// Слово после числа: 1, 21 — «блока», остальное — «блоков».
pub fn count_form<'a>(n: u32, one: &'a str, many: &'a str) -> &'a str {
    if n % 10 == 1 && n % 100 != 11 { one } else { many }
}

/// «Нет 3 блоков корпуса: 2-й ярус, слева», «Нет тарелки: 2-й ярус», «Нет
/// источника тепла: под кипятильником».
pub fn shortage_text(mb: &Multiblock, s: &Shortage) -> String {
    let Some(need) = mb.needs.get(s.need) else {
        return "Не хватает части".to_string();
    };
    let what = if s.count == 1 {
        need.one.to_string()
    } else {
        format!("{} {}", s.count, count_form(s.count, need.one, need.many))
    };
    let mut place = Vec::new();
    match s.tier {
        Tier::Number(n) => place.push(format!("{n}-й ярус")),
        Tier::Named(i) => {
            if let Check::Shape(shape) = &mb.check
                && let Some(layer) = shape.layers.get(i)
            {
                place.push(layer.name.to_string());
            }
        }
        Tier::None => {}
    }
    if s.side != Side::None {
        place.push(s.side.word().to_string());
    }
    if place.is_empty() { format!("Нет {what}") } else { format!("Нет {what}: {}", place.join(", ")) }
}

// Числа экрана контроллера (общие для всех установок, после четырёх общих
// чисел машины). Нехватку экрану не послать строкой: слоты данных — 16-битные
// числа. Поэтому строка нехватки идёт четырьмя числами (нужда, сколько,
// ярус, сторона), а текст собирает экран той же `shortage_text`.

/// Собрана ли установка: `MB_UNKNOWN` — ещё не проверяли (первая секунда
/// после загрузки), `MB_FORMED`, `MB_BROKEN`.
pub const DATA_MB_STATE: usize = 4;
/// Секций (тарелок колонны).
pub const DATA_MB_SECTIONS: usize = 5;
/// Найдено портов входа и выхода.
pub const DATA_MB_INPUTS: usize = 6;
pub const DATA_MB_OUTPUTS: usize = 7;
/// Сколько всего строк нехватки (на экран идут первые `MB_ROWS`).
pub const DATA_MB_SHORTAGES: usize = 8;
/// Первая строка нехватки: по `MB_ROW` чисел.
pub const DATA_MB_ROWS: usize = 9;
pub const MB_ROWS: usize = 4;
pub const MB_ROW: usize = 4;
/// Конец общих чисел установки: свои числа установки — отсюда.
pub const DATA_MB_END: usize = DATA_MB_ROWS + MB_ROWS * MB_ROW;

pub const MB_UNKNOWN: i32 = 0;
pub const MB_FORMED: i32 = 1;
pub const MB_BROKEN: i32 = 2;

/// Раз в сколько тиков контроллер сверяет схему: раз в секунду.
pub const MB_CHECK_EVERY: i64 = 20;
/// Раз в сколько тиков контроллер носит предметы через порты: как воронка.
pub const MB_PORT_EVERY: i64 = 8;

/// Пора ли проверять схему: раз в секунду, разнесено по месту, чтобы десять
/// установок не проверялись в один тик; сразу — если ещё не проверяли.
pub fn check_due(ticks: i64, pos: [i32; 3], state: i32) -> bool {
    let spread = (pos[0] as i64 * 7 + pos[2] as i64 * 13 + pos[1] as i64).rem_euclid(MB_CHECK_EVERY);
    state == MB_UNKNOWN || (ticks + spread).rem_euclid(MB_CHECK_EVERY) == 0
}

/// Строка нехватки числами.
pub fn encode_shortage(s: &Shortage) -> [i32; MB_ROW] {
    let tier = match s.tier {
        Tier::None => 0,
        Tier::Number(n) => n as i32,
        Tier::Named(i) => -(i as i32) - 1,
    };
    let side = Side::ALL.iter().position(|x| *x == s.side).unwrap_or(0) as i32;
    [s.need as i32, s.count.min(i16::MAX as u32) as i32, tier, side]
}

/// Строка нехватки из чисел; None — пустая строка или битые числа.
pub fn decode_shortage(row: &[i32]) -> Option<Shortage> {
    let [need, count, tier, side] = <[i32; MB_ROW]>::try_from(row.get(..MB_ROW)?).ok()?;
    if count <= 0 || need < 0 {
        return None;
    }
    let tier = match tier {
        0 => Tier::None,
        n if n > 0 => Tier::Number(n as u32),
        n => Tier::Named((-n - 1) as usize),
    };
    Some(Shortage {
        need: need as usize,
        count: count as u32,
        tier,
        side: *Side::ALL.get(usize::try_from(side).ok()?)?,
    })
}

/// Записать итог проверки в числа экрана контроллера (`data` длиной не
/// меньше `DATA_MB_END`).
pub fn write_verdict(data: &mut [i32], verdict: &Verdict) {
    if data.len() < DATA_MB_END {
        return;
    }
    for v in &mut data[DATA_MB_STATE..DATA_MB_END] {
        *v = 0;
    }
    match verdict {
        Verdict::Formed(a) => {
            data[DATA_MB_STATE] = MB_FORMED;
            data[DATA_MB_SECTIONS] = a.sections as i32;
            data[DATA_MB_INPUTS] = a.inputs().count() as i32;
            data[DATA_MB_OUTPUTS] = a.outputs().count() as i32;
        }
        Verdict::Broken(short) => {
            data[DATA_MB_STATE] = MB_BROKEN;
            data[DATA_MB_SHORTAGES] = short.len() as i32;
            for (k, s) in short.iter().take(MB_ROWS).enumerate() {
                let at = DATA_MB_ROWS + k * MB_ROW;
                data[at..at + MB_ROW].copy_from_slice(&encode_shortage(s));
            }
        }
    }
}

/// Что экран знает о сборке.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MultiblockShown {
    /// Ещё не проверяли.
    Unknown,
    /// Собрано: секций, портов входа и выхода.
    Formed { sections: u32, inputs: u32, outputs: u32 },
    /// Не собрано: первые строки нехватки и сколько их всего.
    Broken { rows: Vec<Shortage>, total: u32 },
}

pub fn multiblock_shown(data: &[i32]) -> MultiblockShown {
    let n = |i: usize| screen_datum(data, i, 0).max(0) as u32;
    match screen_datum(data, DATA_MB_STATE, MB_UNKNOWN) {
        MB_FORMED => MultiblockShown::Formed {
            sections: n(DATA_MB_SECTIONS),
            inputs: n(DATA_MB_INPUTS),
            outputs: n(DATA_MB_OUTPUTS),
        },
        MB_BROKEN => {
            let rows = (0..MB_ROWS)
                .filter_map(|k| {
                    let at = DATA_MB_ROWS + k * MB_ROW;
                    decode_shortage(data.get(at..at + MB_ROW)?)
                })
                .collect();
            MultiblockShown::Broken { rows, total: n(DATA_MB_SHORTAGES) }
        }
        _ => MultiblockShown::Unknown,
    }
}

/// Тексты нехватки для экрана: строки и «…и ещё N».
pub fn shortage_lines(mb: &Multiblock, rows: &[Shortage], total: u32) -> Vec<String> {
    let mut lines: Vec<String> = rows.iter().map(|s| shortage_text(mb, s)).collect();
    let more = total.saturating_sub(rows.len() as u32);
    if more > 0 {
        lines.push(format!("…и ещё {more}"));
    }
    lines
}

/// Позиции в сейве: `[x, y, z, …]` в `IntArray` блок-сущности.
pub fn positions_to_ints(list: &[[i32; 3]]) -> Vec<i32> {
    list.iter().flat_map(|p| p.iter().copied()).collect()
}

pub fn positions_from_ints(ints: &[i32]) -> Vec<[i32; 3]> {
    ints.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect()
}

/// Порты в сейве: `[x, y, z, вид, …]`, вид 0 — вход, 1 — выход.
pub fn ports_to_ints(list: &[([i32; 3], PortKind)]) -> Vec<i32> {
    list.iter()
        .flat_map(|(p, k)| [p[0], p[1], p[2], i32::from(*k == PortKind::Output)])
        .collect()
}

pub fn ports_from_ints(ints: &[i32]) -> Vec<([i32; 3], PortKind)> {
    ints.chunks_exact(4)
        .map(|c| ([c[0], c[1], c[2]], if c[3] == 1 { PortKind::Output } else { PortKind::Input }))
        .collect()
}

/// Ключи сейва контроллера.
pub const MB_FORMED_KEY: &str = "biolab_mb_formed";
pub const MB_SECTIONS_KEY: &str = "biolab_mb_sections";
pub const MB_PARTS_KEY: &str = "biolab_mb_parts";
pub const MB_PORTS_KEY: &str = "biolab_mb_ports";

/// Склейка: что поставить `formed=true` (новые части), что вернуть в
/// `false` (были склеены, а в установке их больше нет). Расклеивается всё
/// старое, если установка разобрана (`now` пусто).
pub fn glue_changes(before: &[[i32; 3]], now: &[[i32; 3]]) -> (Vec<[i32; 3]>, Vec<[i32; 3]>) {
    let unglue = before.iter().filter(|p| !now.contains(p)).copied().collect();
    (now.to_vec(), unglue)
}

/// Сколько из стака влезет в слот: пустой — до стопки, тот же предмет с
/// теми же данными (`same`) — до полной стопки, чужой — ничего. `held` —
/// слот: пусто или (тот же ли, сколько).
pub fn stack_room(held: Option<(bool, u32)>, max: u32) -> u32 {
    match held {
        None => max.max(1),
        Some((true, count)) => max.max(1).saturating_sub(count),
        Some((false, _)) => 0,
    }
}

/// Куда в слоты порта (`held`) лечь стаку из `count` штук: сперва долить
/// такие же, потом пустые. Вернёт (слот, сколько); сумма — сколько влезло.
pub fn port_plan(held: &[Option<(bool, u32)>], count: u32, max: u32) -> Vec<(usize, u32)> {
    let mut left = count;
    let mut plan = Vec::new();
    for pass in [true, false] {
        for (slot, h) in held.iter().enumerate() {
            if left == 0 {
                return plan;
            }
            let fits = match h {
                Some((true, _)) if pass => stack_room(*h, max),
                None if !pass => stack_room(None, max),
                _ => 0,
            };
            let put = fits.min(left);
            if put > 0 {
                plan.push((slot, put));
                left -= put;
            }
        }
    }
    plan
}

// ---- Ректификационная колонна (GDD 4.18, п. 1) ----
//
// Башня 1×1: источник тепла, над ним куб-кипятильник (контроллер), выше
// тарелки (1–5), на вершине конденсатор — высота от 3 до 7 блоков. Порты
// входа и выхода — по бокам кипятильника (слева, справа, сзади), не
// обязательны: без них сырьё кладут и готовое забирают руками или воронками
// прямо у кипятильника.
//
// Рецепт `biolab:column`: `ingredient`, `fractions` — список выходов
// сверху вниз (первая — с верхней тарелки: самая лёгкая), `time`. Тарелок
// столько, сколько фракций выходит за раз: колонна из одной тарелки даёт
// только первую фракцию, из пяти — до пяти. Фракция k лежит в слоте выхода
// k + 1 кипятильника: слоты на экране идут в ряд слева направо — сверху
// башни вниз. Выход ждёт, если хоть одна фракция не влезает в свой слот.

/// Тарелок в колонне: от одной до пяти (высота от 3 до 7 с кипятильником и
/// конденсатором).
pub const COLUMN_TRAYS: (u32, u32) = (1, 5);
/// Слоты кипятильника: вход и по слоту на фракцию.
pub const COLUMN_SLOTS: usize = 1 + COLUMN_TRAYS.1 as usize;

pub const COLUMN_NEEDS: [Need; 4] = [
    Need { parts: &[Part::Heat], one: "источника тепла", many: "источников тепла" },
    Need { parts: &[Part::Block(COLUMN_TRAY)], one: "тарелки", many: "тарелок" },
    Need { parts: &[Part::Block(COLUMN_CONDENSER)], one: "конденсатора", many: "конденсаторов" },
    Need { parts: &[Part::Port], one: "порта", many: "портов" },
];

pub const COLUMN_SHAPE: Shape = Shape {
    layers: &[
        Layer { cells: &[cell(0, 0, 0)], min: 1, max: 1, name: "под кипятильником" },
        // Кипятильник: порты по бокам и сзади, если есть.
        Layer { cells: &[maybe(-1, 0, 3), maybe(1, 0, 3), maybe(0, 1, 3)], min: 1, max: 1, name: "" },
        Layer { cells: &[cell(0, 0, 1)], min: COLUMN_TRAYS.0, max: COLUMN_TRAYS.1, name: "" },
        Layer { cells: &[cell(0, 0, 2)], min: 1, max: 1, name: "" },
    ],
    controller: 1,
};

/// Роли слотов кипятильника: вход и пять фракций.
pub const COLUMN_SLOT_ROLES: [SlotRole; COLUMN_SLOTS] = [
    SlotRole::Input,
    SlotRole::Output,
    SlotRole::Output,
    SlotRole::Output,
    SlotRole::Output,
    SlotRole::Output,
];

/// Установки мода. Новая установка — строка здесь и машина в `MACHINES`.
pub const MULTIBLOCKS: [Multiblock; 4] = [
    Multiblock {
        controller: COLUMN,
        title: "Ректификационная колонна",
        needs: &COLUMN_NEEDS,
        check: Check::Shape(COLUMN_SHAPE),
        slots: &COLUMN_SLOT_ROLES,
    },
    Multiblock {
        controller: SEQUENCER,
        title: "Генный секвенатор",
        needs: &SEQUENCER_NEEDS,
        check: Check::Shape(SEQUENCER_SHAPE),
        slots: &SEQUENCER_SLOT_ROLES,
    },
    Multiblock {
        controller: BIOREACTOR,
        title: "Биореактор",
        needs: &BIOREACTOR_NEEDS,
        check: Check::Shape(BIOREACTOR_SHAPE),
        slots: &BIOREACTOR_SLOT_ROLES,
    },
    // Карантинная камера (GDD 4.18 п. 4): своя проверка — заливкой.
    QUARANTINE_MULTIBLOCK,
];

/// Установка, чей это контроллер.
pub fn multiblock(controller: &str) -> Option<&'static Multiblock> {
    MULTIBLOCKS.iter().find(|m| m.controller == controller)
}

/// Тип меню контроллеров в строке `MACHINES`: своей ванильной раскладки у
/// них нет, открывается всегда своё меню мода (`machine_menu`), роли слотов
/// — из строки `MULTIBLOCKS`.
pub const MULTIBLOCK_MENU: &str = "biolab:multiblock";

/// Фракция рецепта колонны.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fraction {
    pub item: String,
    pub count: u32,
}

/// Рецепт колонны из JSON: вход, фракции сверху вниз, время. None — битый.
pub fn column_recipe(json: &Json) -> Option<(String, Vec<Fraction>, i32)> {
    if json.get("type").and_then(Json::str) != Some(COLUMN) {
        return None;
    }
    let input = json.get("ingredient")?.str()?.to_string();
    let list = json.get("fractions")?.arr()?;
    if list.is_empty() || list.len() > COLUMN_TRAYS.1 as usize {
        return None;
    }
    let fractions = list
        .iter()
        .map(|f| {
            let (item, count) = stack_of(f)?;
            Some(Fraction { item, count })
        })
        .collect::<Option<Vec<_>>>()?;
    let time = json.get("time").and_then(Json::num).unwrap_or(DEFAULT_TIME as f64) as i32;
    Some((input, fractions, time.max(1)))
}

/// Сколько фракций выходит за раз: по фракции на тарелку.
pub fn column_yield(fractions: usize, trays: u32) -> usize {
    fractions.min(trays as usize)
}

/// Влезут ли фракции, каждая в свой слот: `held` — слоты фракций по
/// порядку (пусто или что лежит), `piles` — первые `column_yield` фракций.
pub fn column_fits(held: &[Option<Pile>], piles: &[Pile], max_stack: impl Fn(&str) -> u32) -> bool {
    piles.iter().enumerate().all(|(k, pile)| match held.get(k) {
        Some(None) => pile.count <= max_stack(pile.item).max(1),
        Some(Some(h)) => {
            h.plain && pile.plain && h.item == pile.item && h.count + pile.count <= max_stack(pile.item)
        }
        None => false,
    })
}

/// Форма слова после числа: 1 тарелка, 3 тарелки, 5 тарелок.
pub fn plural<'a>(n: u32, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    match (n % 10, n % 100) {
        (_, 11..=14) => many,
        (1, _) => one,
        (2..=4, _) => few,
        _ => many,
    }
}

/// Сводка собранной колонны: высота и сколько фракций за раз.
pub fn column_status(sections: u32) -> String {
    format!("{sections} {}, фракций за раз: {sections}", plural(sections, "тарелка", "тарелки", "тарелок"))
}

#[cfg(test)]
mod multiblock_tests {
    use super::*;

    /// Мир для проверки: блоки по точкам, остальное — воздух.
    fn world(blocks: &[([i32; 3], &str)]) -> impl Fn([i32; 3]) -> String + use<> {
        let map: std::collections::HashMap<[i32; 3], String> =
            blocks.iter().map(|(p, b)| (*p, b.to_string())).collect();
        move |p| map.get(&p).cloned().unwrap_or_else(|| "minecraft:air".to_string())
    }

    fn column() -> &'static Multiblock {
        multiblock(COLUMN).expect("колонна — установка")
    }

    /// Колонна из `trays` тарелок над (0, 64, 0), лицом на юг.
    fn tower(trays: i32, heat: &'static str) -> Vec<([i32; 3], &'static str)> {
        let mut out = vec![([0, 63, 0], heat), ([0, 64, 0], COLUMN)];
        for k in 1..=trays {
            out.push(([0, 64 + k, 0], COLUMN_TRAY));
        }
        out.push(([0, 65 + trays, 0], COLUMN_CONDENSER));
        out
    }

    const SOUTH: Site = Site { origin: [0, 64, 0], facing: Facing::South };

    #[test]
    fn facing_turns_right_and_back() {
        // Лицо на юг: игрок стоит южнее и смотрит на север — вглубь
        // установки север, справа от него восток.
        assert_eq!(Facing::South.back(), [0, -1]);
        assert_eq!(Facing::South.right(), [1, 0]);
        assert_eq!(Facing::North.right(), [-1, 0]);
        assert_eq!(Facing::East.back(), [-1, 0]);
        assert_eq!(Facing::East.right(), [0, -1]);
        assert_eq!(Facing::West.right(), [0, 1]);
        for f in [Facing::North, Facing::South, Facing::West, Facing::East] {
            let (r, b) = (f.right(), f.back());
            assert_eq!(r[0] * b[0] + r[1] * b[1], 0, "вправо поперёк «назад»");
            assert_eq!(r[0] * b[1] - r[1] * b[0], -1, "правая тройка у всех сторон одна");
        }
        assert_eq!(place([10, 64, 10], Facing::South, [1, 2, 3]), [11, 66, 7]);
        assert_eq!(Facing::from_name("east"), Some(Facing::East));
        assert_eq!(Facing::from_name("up"), None);
    }

    #[test]
    fn a_column_of_any_height_from_three_to_seven_is_formed() {
        for trays in 1..=5 {
            let at = world(&tower(trays, "minecraft:campfire"));
            let Verdict::Formed(a) = check_multiblock(column(), SOUTH, &at) else {
                panic!("{trays} тарелок — не собрано");
            };
            assert_eq!(a.sections, trays as u32);
            // Тарелки и конденсатор — склеиваемые части; источник тепла тоже
            // в списке, но склеивать его нечем (не `formed`).
            assert_eq!(a.parts.len(), trays as usize + 2);
            assert!(a.ports.is_empty());
        }
        for heat in ["minecraft:magma_block", "minecraft:lava", "minecraft:soul_campfire"] {
            assert!(matches!(check_multiblock(column(), SOUTH, &world(&tower(2, heat))), Verdict::Formed(_)), "{heat}");
        }
    }

    #[test]
    fn what_is_missing_is_said_in_words() {
        let mb = column();
        // Без огня.
        let mut blocks = tower(2, "minecraft:stone");
        let Verdict::Broken(short) = check_multiblock(mb, SOUTH, &world(&blocks)) else { panic!() };
        assert_eq!(short.iter().map(|s| shortage_text(mb, s)).collect::<Vec<_>>(), vec!["Нет источника тепла: под кипятильником"]);
        // Без тарелок: конденсатор прямо на кипятильнике — тарелки нет на
        // 2-м ярусе, а конденсатор ждут выше.
        blocks = vec![([0, 63, 0], "minecraft:lava"), ([0, 64, 0], COLUMN), ([0, 65, 0], COLUMN_CONDENSER)];
        let Verdict::Broken(short) = check_multiblock(mb, SOUTH, &world(&blocks)) else { panic!() };
        let texts: Vec<String> = short.iter().map(|s| shortage_text(mb, s)).collect();
        assert_eq!(texts, vec!["Нет тарелки: 2-й ярус", "Нет конденсатора: 3-й ярус"]);
        // Три тарелки без конденсатора.
        blocks = tower(3, "minecraft:lava");
        blocks.pop();
        let Verdict::Broken(short) = check_multiblock(mb, SOUTH, &world(&blocks)) else { panic!() };
        assert_eq!(shortage_text(mb, &short[0]), "Нет конденсатора: 5-й ярус");
        // Шесть тарелок — конденсатор ждут на седьмом ярусе, там тарелка.
        let blocks = tower(6, "minecraft:lava");
        let Verdict::Broken(short) = check_multiblock(mb, SOUTH, &world(&blocks)) else { panic!() };
        assert_eq!(shortage_text(mb, &short[0]), "Нет конденсатора: 7-й ярус");
    }

    #[test]
    fn ports_beside_the_boiler_are_found_whatever_the_facing() {
        let mut blocks = tower(1, "minecraft:campfire");
        // Лицо на юг: слева (запад), справа (восток), сзади (север).
        blocks.push(([-1, 64, 0], INPUT_PORT));
        blocks.push(([0, 64, -1], OUTPUT_PORT));
        // Спереди порт не считается.
        blocks.push(([0, 64, 1], OUTPUT_PORT));
        let Verdict::Formed(a) = check_multiblock(column(), SOUTH, &world(&blocks)) else { panic!() };
        assert_eq!(a.inputs().collect::<Vec<_>>(), vec![[-1, 64, 0]]);
        assert_eq!(a.outputs().collect::<Vec<_>>(), vec![[0, 64, -1]]);
        assert!(!a.parts.iter().any(|p| port_by_pos(&a, *p)), "порты не склеиваются");
        // Повернули кипятильник лицом на север: задний порт оказался спереди.
        let north = Site { facing: Facing::North, ..SOUTH };
        let Verdict::Formed(a) = check_multiblock(column(), north, &world(&blocks)) else { panic!() };
        assert_eq!(a.outputs().collect::<Vec<_>>(), vec![[0, 64, 1]]);
    }

    fn port_by_pos(a: &Assembly, p: [i32; 3]) -> bool {
        a.ports.iter().any(|(q, _)| *q == p)
    }

    /// Куб 3×3×3 с контроллером в середине передней стенки — так будет
    /// стоять биореактор. Проверяет каркас на большой схеме: стороны, ярусы
    /// и подсчёт нехватки.
    const BOX_NEEDS: [Need; 2] = [
        Need { parts: &[Part::Block(CASING), Part::Block(CASING_GLASS), Part::Port], one: "блока корпуса", many: "блоков корпуса" },
        Need { parts: &[Part::Air], one: "пустоты", many: "пустот" },
    ];
    const WALL: [Cell; 9] = [
        cell(-1, 0, 0), cell(0, 0, 0), cell(1, 0, 0),
        cell(-1, 1, 0), cell(0, 1, 0), cell(1, 1, 0),
        cell(-1, 2, 0), cell(0, 2, 0), cell(1, 2, 0),
    ];
    const MIDDLE: [Cell; 8] = [
        cell(-1, 0, 0), cell(1, 0, 0),
        cell(-1, 1, 0), cell(0, 1, 1), cell(1, 1, 0),
        cell(-1, 2, 0), cell(0, 2, 0), cell(1, 2, 0),
    ];
    const BOX: Multiblock = Multiblock {
        controller: "biolab:test_box",
        title: "Куб",
        needs: &BOX_NEEDS,
        check: Check::Shape(Shape {
            layers: &[
                Layer { cells: &WALL, min: 1, max: 1, name: "" },
                Layer { cells: &MIDDLE, min: 1, max: 1, name: "" },
                Layer { cells: &WALL, min: 1, max: 1, name: "" },
            ],
            controller: 1,
        }),
        slots: &[SlotRole::Input, SlotRole::Output],
    };

    fn cube(site: Site) -> Vec<([i32; 3], &'static str)> {
        let mut out = Vec::new();
        for y in -1..=1 {
            for r in -1..=1 {
                for b in 0..=2 {
                    let at = [r, y, b];
                    let block = match at {
                        [0, 0, 0] => "biolab:test_box",
                        [0, 0, 1] => "minecraft:air",
                        [-1, 0, 1] => INPUT_PORT,
                        [_, 0, _] if r == 1 => CASING_GLASS,
                        _ => CASING,
                    };
                    out.push((place(site.origin, site.facing, at), block));
                }
            }
        }
        out
    }

    #[test]
    fn a_cube_reports_walls_by_tier_and_side() {
        let site = Site { origin: [100, 70, -20], facing: Facing::East };
        let Verdict::Formed(a) = check_multiblock(&BOX, site, &world(&cube(site))) else { panic!("куб не собран") };
        assert_eq!(a.sections, 0, "повторов нет");
        assert_eq!(a.ports.len(), 1);
        assert_eq!(a.parts.len(), 27 - 3, "без контроллера, порта и пустоты");
        // Сломали левую стенку второго яруса (три блока) и один угол сверху.
        let mut blocks = cube(site);
        for b in 0..=2 {
            let p = place(site.origin, site.facing, [-1, 0, b]);
            blocks.retain(|(q, _)| *q != p);
        }
        let corner = place(site.origin, site.facing, [1, 1, 2]);
        blocks.retain(|(q, _)| *q != corner);
        let Verdict::Broken(short) = check_multiblock(&BOX, site, &world(&blocks)) else { panic!() };
        let texts: Vec<String> = short.iter().map(|s| shortage_text(&BOX, s)).collect();
        assert_eq!(texts, vec!["Нет 3 блоков корпуса: 2-й ярус, слева", "Нет блока корпуса: 3-й ярус, справа"]);
        // Внутри не пусто.
        let mut blocks = cube(site);
        let inside = place(site.origin, site.facing, [0, 0, 1]);
        blocks.retain(|(q, _)| *q != inside);
        blocks.push((inside, "minecraft:dirt"));
        let Verdict::Broken(short) = check_multiblock(&BOX, site, &world(&blocks)) else { panic!() };
        assert_eq!(shortage_text(&BOX, &short[0]), "Нет пустоты: 2-й ярус, в середине");
    }

    /// Своя проверка — так встанет карантинная камера (заливкой).
    fn always_short(mb: &Multiblock, _: Site, _: BlockAt) -> Verdict {
        let _ = mb;
        Verdict::Broken(vec![Shortage { need: 0, count: 2, tier: Tier::None, side: Side::None }])
    }

    #[test]
    fn a_custom_check_plugs_in() {
        const NEEDS: [Need; 1] = [Need { parts: &[], one: "двери шлюза", many: "дверей шлюза" }];
        let room = Multiblock { controller: "biolab:test_room", title: "Комната", needs: &NEEDS, check: Check::Custom(always_short), slots: &[] };
        let Verdict::Broken(short) = check_multiblock(&room, SOUTH, &world(&[])) else { panic!() };
        assert_eq!(shortage_text(&room, &short[0]), "Нет 2 дверей шлюза");
    }

    #[test]
    fn counts_take_the_right_word() {
        let forms: Vec<&str> = [1, 2, 5, 11, 21, 22].iter().map(|n| count_form(*n, "блока", "блоков")).collect();
        assert_eq!(forms, vec!["блока", "блоков", "блоков", "блоков", "блока", "блоков"]);
        assert_eq!(column_status(1), "1 тарелка, фракций за раз: 1");
        assert_eq!(column_status(3), "3 тарелки, фракций за раз: 3");
        assert_eq!(column_status(5), "5 тарелок, фракций за раз: 5");
    }

    #[test]
    fn the_screen_gets_the_verdict_through_numbers() {
        let mb = column();
        let at = world(&[([0, 64, 0], COLUMN)]);
        let verdict = check_multiblock(mb, SOUTH, &at);
        let Verdict::Broken(short) = &verdict else { panic!() };
        assert_eq!(short.len(), 3, "нет огня, тарелки и конденсатора");
        let mut data = vec![0; DATA_MB_END];
        write_verdict(&mut data, &verdict);
        let MultiblockShown::Broken { rows, total } = multiblock_shown(&data) else { panic!() };
        assert_eq!((rows.as_slice(), total), (short.as_slice(), 3));
        assert_eq!(
            shortage_lines(mb, &rows, total),
            vec!["Нет источника тепла: под кипятильником", "Нет тарелки: 2-й ярус", "Нет конденсатора: 3-й ярус"]
        );
        // Строк больше, чем влезает в числа, — «и ещё».
        let many: Vec<Shortage> = (1..=6).map(|n| Shortage { need: 1, count: 1, tier: Tier::Number(n), side: Side::None }).collect();
        write_verdict(&mut data, &Verdict::Broken(many));
        let MultiblockShown::Broken { rows, total } = multiblock_shown(&data) else { panic!() };
        assert_eq!(rows.len(), MB_ROWS);
        assert_eq!(shortage_lines(mb, &rows, total).last().unwrap(), "…и ещё 2");
        // Собрано.
        let verdict = check_multiblock(mb, SOUTH, &world(&tower(4, "minecraft:lava")));
        write_verdict(&mut data, &verdict);
        assert_eq!(multiblock_shown(&data), MultiblockShown::Formed { sections: 4, inputs: 0, outputs: 0 });
        assert_eq!(multiblock_shown(&[]), MultiblockShown::Unknown, "до первого пакета");
        // Строка числами туда и обратно, ярус с именем — отрицательным.
        for s in [
            Shortage { need: 3, count: 40, tier: Tier::Named(0), side: Side::Back },
            Shortage { need: 0, count: 1, tier: Tier::None, side: Side::Middle },
        ] {
            assert_eq!(decode_shortage(&encode_shortage(&s)), Some(s));
        }
        assert_eq!(decode_shortage(&[0, 0, 0, 0]), None, "пустая строка");
        assert_eq!(decode_shortage(&[0, 1, 0, 99]), None, "битая сторона");
    }

    #[test]
    fn checks_spread_over_the_second() {
        let pos = [5, 64, -3];
        let due: Vec<i64> = (0..40).filter(|t| check_due(*t, pos, MB_FORMED)).collect();
        assert_eq!(due.len(), 2, "раз в секунду");
        assert_eq!(due[1] - due[0], MB_CHECK_EVERY);
        assert!(check_due(7, pos, MB_UNKNOWN), "не проверяли — сразу");
        let other = (0..20).find(|t| check_due(*t, [6, 64, -3], MB_FORMED));
        assert_ne!(other, Some(due[0]), "соседние установки — в разные тики");
    }

    #[test]
    fn the_save_keeps_positions_and_ports() {
        let parts = vec![[1, 2, 3], [-4, 70, 9]];
        assert_eq!(positions_from_ints(&positions_to_ints(&parts)), parts);
        let ports = vec![([0, 64, 1], PortKind::Input), ([1, 64, 0], PortKind::Output)];
        assert_eq!(ports_from_ints(&ports_to_ints(&ports)), ports);
        assert!(positions_from_ints(&[1, 2]).is_empty(), "обрывок не позиция");
        let (glue, unglue) = glue_changes(&[[0, 1, 0], [0, 2, 0]], &[[0, 1, 0]]);
        assert_eq!((glue, unglue), (vec![[0, 1, 0]], vec![[0, 2, 0]]));
        let (glue, unglue) = glue_changes(&[[0, 1, 0]], &[]);
        assert!(glue.is_empty() && unglue == vec![[0, 1, 0]], "разобрали — расклеить всё");
    }

    #[test]
    fn ports_fill_like_a_hopper() {
        assert_eq!(stack_room(None, 64), 64);
        assert_eq!(stack_room(Some((true, 60)), 64), 4);
        assert_eq!(stack_room(Some((false, 1)), 64), 0);
        assert_eq!(stack_room(None, 0), 1, "стопка не меньше 1");
        let held = [Some((false, 3)), Some((true, 62)), None, Some((true, 64)), None];
        assert_eq!(port_plan(&held, 10, 64), vec![(1, 2), (2, 8)]);
        assert_eq!(port_plan(&held, 1, 64), vec![(1, 1)]);
        assert_eq!(port_plan(&held, 200, 64), vec![(1, 2), (2, 64), (4, 64)]);
        assert_eq!(port_plan(&[Some((false, 1)); 5], 3, 64), vec![]);
        // Колба (стопка 1): по одной в пустые слоты.
        assert_eq!(port_plan(&[None, None, Some((true, 1))], 2, 1), vec![(0, 1), (1, 1)]);
    }

    #[test]
    fn ports_open_to_hoppers_one_way() {
        let (faces, inputs, outputs) = port_faces(port(INPUT_PORT).unwrap());
        assert_eq!(inputs.len(), PORT_SLOTS);
        assert!(outputs.is_empty(), "из входного порта воронка не тянет");
        assert!(faces.iter().all(|f| f.len() == PORT_SLOTS));
        let (_, inputs, outputs) = port_faces(port(OUTPUT_PORT).unwrap());
        assert!(inputs.is_empty(), "в выходной порт воронка не кладёт");
        assert_eq!(outputs.len(), PORT_SLOTS);
        assert!(has_block_entity(INPUT_PORT) && has_block_entity(COLUMN) && !has_block_entity(CASING));
    }

    #[test]
    fn the_column_shares_its_fractions_by_trays() {
        let json = Json::parse(
            r#"{"type":"biolab:column","ingredient":"minecraft:rotten_flesh","fractions":[{"id":"biolab:element_nitrogen","count":2},"biolab:element_carbon",{"id":"biolab:element_phosphorus","count":1}],"time":160}"#,
        )
        .unwrap();
        let (input, fractions, time) = column_recipe(&json).expect("рецепт");
        assert_eq!((input.as_str(), time), ("minecraft:rotten_flesh", 160));
        assert_eq!(fractions[0], Fraction { item: "biolab:element_nitrogen".into(), count: 2 });
        assert_eq!(fractions[1].count, 1, "строка — одна штука");
        assert_eq!(column_yield(fractions.len(), 1), 1, "одна тарелка — одна фракция");
        assert_eq!(column_yield(fractions.len(), 5), 3, "фракций не больше, чем в рецепте");
        let piles: Vec<Pile> = fractions.iter().map(|f| Pile { item: &f.item, count: f.count, plain: true }).collect();
        let max = |_: &str| 64;
        assert!(column_fits(&[None, None, None], &piles, max));
        let carbon = Pile { item: "biolab:element_carbon", count: 64, plain: true };
        assert!(!column_fits(&[None, Some(carbon), None], &piles, max), "углерод не влезет");
        assert!(column_fits(&[None, Some(carbon), None], &piles[..1], max), "одна тарелка — второй слот не нужен");
        let alien = Pile { item: "minecraft:dirt", count: 1, plain: true };
        assert!(!column_fits(&[Some(alien)], &piles[..1], max), "чужое в слоте фракции");
        assert!(column_recipe(&Json::parse(r#"{"type":"biolab:still","ingredient":"a","result":"b"}"#).unwrap()).is_none());
        let six = r#"{"type":"biolab:column","ingredient":"a","fractions":["b","c","d","e","f","g"]}"#;
        assert!(column_recipe(&Json::parse(six).unwrap()).is_none(), "тарелок только пять");
    }

    /// Рецепты колонны в `content/`: не меньше четырёх, все разбираются,
    /// фракции — существующие предметы.
    #[test]
    fn column_recipes_are_in_content() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let mut found = 0;
        for entry in std::fs::read_dir(root.join("data/biolab/recipe")).unwrap() {
            let path = entry.unwrap().path();
            let json = Json::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
            if json.get("type").and_then(Json::str) != Some(COLUMN) {
                continue;
            }
            let (input, fractions, _) = column_recipe(&json).unwrap_or_else(|| panic!("{}: битый рецепт колонны", path.display()));
            assert!(machine_ingredients(COLUMN).contains(&input.as_str()), "{input}: нет во входах колонны");
            for f in &fractions {
                if let Some(name) = f.item.strip_prefix("biolab:") {
                    assert!(root.join(format!("assets/biolab/items/{name}.json")).exists(), "{}", f.item);
                }
            }
            found += 1;
        }
        assert!(found >= 4, "рецептов колонны {found}, а в GDD — не меньше четырёх");
    }

    /// Блоки каркаса и колонны в `content/`: со свойством `formed` —
    /// ровно `FORMABLE`, у портов и кипятильника шаблон, открывающий меню.
    #[test]
    fn multiblock_blocks_are_declared() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/horizon/block");
        for id in [CASING, CASING_GLASS, INPUT_PORT, OUTPUT_PORT, COLUMN, COLUMN_TRAY, COLUMN_CONDENSER] {
            let name = id.trim_start_matches("biolab:");
            let text = std::fs::read_to_string(root.join(format!("{name}.json"))).unwrap_or_else(|_| panic!("нет блока {id}"));
            let json = Json::parse(&text).unwrap();
            let formed = json.get("state_properties").and_then(|p| p.get(FORMED));
            assert_eq!(formed.is_some(), is_formable(id), "{id}: свойство formed");
            if let Some(f) = formed {
                assert_eq!(f.get("type").and_then(Json::str), Some("boolean"), "{id}");
            }
            if port(id).is_some() || id == COLUMN {
                assert_eq!(json.get("template").and_then(Json::str), Some("minecraft:furnace"), "{id}: меню открывает шаблон");
            }
        }
        // Каждый контроллер установки — машина, и его меню — своё.
        for mb in &MULTIBLOCKS {
            let machine = machine(mb.controller).unwrap_or_else(|| panic!("{} не машина", mb.controller));
            assert_eq!(machine.menu, MULTIBLOCK_MENU);
            assert_eq!(machine.slots, mb.slots.len(), "{}: роли слотов", mb.controller);
            assert!(machine_data_slots(mb.controller) >= DATA_MB_END);
            if let Check::Shape(shape) = &mb.check {
                assert!(shape.controller < shape.layers.len());
                for (i, layer) in shape.layers.iter().enumerate() {
                    assert!(layer.min <= layer.max && layer.max >= 1, "{}: ярус {i}", mb.controller);
                    if i <= shape.controller {
                        assert_eq!((layer.min, layer.max), (1, 1), "{}: ниже контроллера ярусы не повторяются", mb.controller);
                    }
                    for c in layer.cells {
                        assert!(c.need < mb.needs.len(), "{}: ярус {i}", mb.controller);
                        if i == shape.controller {
                            assert_ne!(c.at, [0, 0], "{}: клетка на месте контроллера", mb.controller);
                        }
                    }
                }
            }
        }
    }
}

// ---- M8: генный секвенатор (GDD 4.18, п. 3) ----
//
// Установка 3×2×3: в середине передней стенки нижнего яруса — консоль
// (контроллер), по бокам среднего ряда — две стойки секвенаторов, над
// консолью — монитор, остальные клетки — корпус, стекло или порты. Вглубь от
// лица консоли три ряда, вширь три, в высоту два.
//
// Три работы; режим выбирают три кнопки слева на экране консоли: окно ловит
// нажатие (`native_screen_click`, `sequencer_click.rs`) и шлёт его серверу
// кнопкой меню, сервер меняет режим в сейве консоли (`native_menu_button`,
// `sequencer_button.rs`).
// - *Сборка вектора*: 2–3 фрагмента ДНК → генный вектор со всеми их генами;
//   вживляется одним уколом — мобу (`inject.rs`), себе (`remedy.rs`). Вектор
//   держит не больше `MAX_GENES` генов при сумме цен до `STABILITY`: в
//   лаборатории предела генома нет (GDD 4.19), а у конструкта — есть.
// - *Антисмысловой вектор*: фрагмент → вектор, который уколом удаляет этот
//   ген у моба или у игрока — обратная генотерапия. Прибавки к атрибутам
//   моба снимает сам `biolab-deep`: проход мутантов раз в `SCAN_EVERY` тиков
//   ставит их по геному и снимает те, которых по геному больше нет
//   (`biolab_bridge::apply_genome`). У игрока эффекты генов подновляются раз
//   в секунду на 3 секунды (`gene_effects`, `PHARMA_EFFECT_TICKS`): снятый
//   ген перестаёт их подновлять, наросты уходят с данными игрока.
// - *Чтение генома*: моб на операционном столе не дальше `SEQ_TABLE_REACH`
//   блоков от консоли — все гены, сила и цена на мониторе. Прочитанное
//   держится, пока на столе тот же моб с тем же геномом.
// Расходники — фосфор (нуклеотиды: штука на каждый ген вектора, на
// антисмысл и на чтение) и питательный раствор (порция на любой вектор).
// Исполнение на игре — `crate::game::biolab_sequencer` (`sequencer.rs`).

/// Консоль секвенатора — контроллер.
pub const SEQUENCER: &str = "biolab:sequencer";
/// Стойка секвенатора: по одной в левой и правой стенке среднего ряда.
pub const SEQUENCER_RACK: &str = "biolab:sequencer_rack";
/// Монитор — над консолью.
pub const SEQUENCER_MONITOR: &str = "biolab:sequencer_monitor";
/// Генный вектор: несколько генов, один укол.
pub const GENE_VECTOR: &str = "biolab:gene_vector";
/// Антисмысловой вектор: удаляет ген.
pub const ANTISENSE_VECTOR: &str = "biolab:antisense_vector";
/// Гены вектора в `custom_data`: строка генома (`genome_data`, `chitin:2,horns:1`).
pub const VECTOR_KEY: &str = "biolab_vector";
/// Ген антисмыслового вектора в `custom_data`: имя гена (`chitin`).
pub const ANTISENSE_KEY: &str = "biolab_antisense";
/// Фосфор — нуклеотиды секвенатора.
pub const PHOSPHORUS_ID: &str = "biolab:element_phosphorus";

/// Слоты консоли: три фрагмента, фосфор, раствор, выход.
pub const SEQ_FRAGMENT_SLOTS: usize = 3;
pub const SEQ_SLOT_PHOSPHORUS: usize = 3;
pub const SEQ_SLOT_BROTH: usize = 4;
pub const SEQ_SLOT_OUTPUT: usize = 5;
pub const SEQUENCER_SLOTS: usize = 6;

/// Роли слотов консоли: пять входов и выход. Раствор — тоже вход, а не
/// топливо: каждый вход берёт только своё (`sequencer_accepts`), так что
/// воронка с любой стороны кладёт фрагмент, фосфор и раствор по местам.
pub const SEQUENCER_SLOT_ROLES: [SlotRole; SEQUENCER_SLOTS] = [
    SlotRole::Input,
    SlotRole::Input,
    SlotRole::Input,
    SlotRole::Input,
    SlotRole::Input,
    SlotRole::Output,
];

pub const SEQUENCER_NEEDS: [Need; 3] = [
    Need {
        parts: &[Part::Block(CASING), Part::Block(CASING_GLASS), Part::Port],
        one: "блока корпуса",
        many: "блоков корпуса",
    },
    Need { parts: &[Part::Block(SEQUENCER_RACK)], one: "стойки секвенатора", many: "стоек секвенатора" },
    Need { parts: &[Part::Block(SEQUENCER_MONITOR)], one: "монитора", many: "мониторов" },
];

/// Нижний ярус: консоль в середине переднего ряда (клеткой не пишется),
/// стойки — по бокам среднего, остальное — стенка.
const SEQ_BOTTOM: [Cell; 8] = [
    cell(-1, 0, 0),
    cell(1, 0, 0),
    cell(-1, 1, 1),
    cell(0, 1, 0),
    cell(1, 1, 1),
    cell(-1, 2, 0),
    cell(0, 2, 0),
    cell(1, 2, 0),
];
/// Верхний ярус: монитор над консолью, остальное — стенка.
const SEQ_TOP: [Cell; 9] = [
    cell(-1, 0, 0),
    cell(0, 0, 2),
    cell(1, 0, 0),
    cell(-1, 1, 0),
    cell(0, 1, 0),
    cell(1, 1, 0),
    cell(-1, 2, 0),
    cell(0, 2, 0),
    cell(1, 2, 0),
];

pub const SEQUENCER_SHAPE: Shape = Shape {
    layers: &[
        Layer { cells: &SEQ_BOTTOM, min: 1, max: 1, name: "" },
        Layer { cells: &SEQ_TOP, min: 1, max: 1, name: "" },
    ],
    controller: 0,
};

/// Что берёт слот консоли: фрагменты — в первые три, фосфор и раствор — в
/// свои. Зовёт `slot_accepts`.
pub fn sequencer_accepts(slot: usize, item: &str) -> bool {
    match slot {
        s if s < SEQ_FRAGMENT_SLOTS => item == FRAGMENT_ID,
        SEQ_SLOT_PHOSPHORUS => item == PHOSPHORUS_ID,
        SEQ_SLOT_BROTH => item == BROTH_ID,
        _ => false,
    }
}

/// Работа секвенатора.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeqMode {
    Vector,
    Antisense,
    Read,
}

impl SeqMode {
    pub const ALL: [SeqMode; 3] = [SeqMode::Vector, SeqMode::Antisense, SeqMode::Read];

    /// Номер режима: в сейве, в числах экрана и кнопкой меню.
    pub fn id(self) -> i32 {
        SeqMode::ALL.iter().position(|m| *m == self).unwrap_or(0) as i32
    }

    pub fn from_id(id: i32) -> Option<SeqMode> {
        SeqMode::ALL.get(usize::try_from(id).ok()?).copied()
    }

    pub fn title(self) -> &'static str {
        match self {
            SeqMode::Vector => "Сборка вектора",
            SeqMode::Antisense => "Антисмысловой вектор",
            SeqMode::Read => "Чтение генома",
        }
    }

    /// Знак на кнопке.
    pub fn mark(self) -> &'static str {
        match self {
            SeqMode::Vector => "+",
            SeqMode::Antisense => "-",
            SeqMode::Read => "?",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            SeqMode::Vector => "2–3 фрагмента → вектор: все гены одним уколом",
            SeqMode::Antisense => "Фрагмент → вектор, который удаляет этот ген",
            SeqMode::Read => "Геном моба на операционном столе рядом",
        }
    }

    /// Сколько тиков идёт работа.
    pub fn time(self) -> i32 {
        match self {
            SeqMode::Vector => 200,
            SeqMode::Antisense => 120,
            SeqMode::Read => 60,
        }
    }
}

/// Режим консоли в её сейве.
pub const SEQ_MODE_KEY: &str = "biolab_seq_mode";
/// Последнее чтение: id моба и его геном строкой (`genome_data`).
pub const SEQ_PATIENT_KEY: &str = "biolab_seq_patient";
pub const SEQ_READ_KEY: &str = "biolab_seq_read";

/// Как далеко от консоли ищут операционный стол с пациентом: по горизонтали
/// (от центра консоли до моба) и по высоте.
pub const SEQ_TABLE_REACH: f64 = 6.0;
pub const SEQ_TABLE_HEIGHT: f64 = 3.0;

/// Что с работой секвенатора — для монитора.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeqStatus {
    /// Установка не собрана или ещё не тикала.
    Idle,
    /// Фрагментов нет.
    Empty,
    /// Вектору мало одного фрагмента.
    One,
    /// Во фрагменте нет прочитанного гена.
    Blank,
    /// Один ген в двух фрагментах.
    Twice,
    /// Генов больше `MAX_GENES`.
    TooMany,
    /// Сумма цен выше `STABILITY`.
    Unstable,
    NoPhosphorus,
    NoBroth,
    /// В выходе лежит вектор.
    OutputFull,
    /// Всё есть, идёт сборка.
    Working,
    /// На столе рядом никого.
    NoPatient,
    /// Идёт чтение.
    Reading,
    /// Прочитано: геном на мониторе.
    Read,
}

impl SeqStatus {
    pub const ALL: [SeqStatus; 14] = [
        SeqStatus::Idle,
        SeqStatus::Empty,
        SeqStatus::One,
        SeqStatus::Blank,
        SeqStatus::Twice,
        SeqStatus::TooMany,
        SeqStatus::Unstable,
        SeqStatus::NoPhosphorus,
        SeqStatus::NoBroth,
        SeqStatus::OutputFull,
        SeqStatus::Working,
        SeqStatus::NoPatient,
        SeqStatus::Reading,
        SeqStatus::Read,
    ];

    pub fn id(self) -> i32 {
        SeqStatus::ALL.iter().position(|s| *s == self).unwrap_or(0) as i32
    }

    pub fn from_id(id: i32) -> SeqStatus {
        usize::try_from(id).ok().and_then(|i| SeqStatus::ALL.get(i).copied()).unwrap_or(SeqStatus::Idle)
    }
}

/// Сумма цен генома: цена гена за единицу силы × сила.
pub fn genome_cost(genes: &[(&str, i32)]) -> i32 {
    genes.iter().filter_map(|(name, strength)| Some(gene(name)?.cost * strength)).sum()
}

/// Что собирается из входов: гены (для монитора — и при отказе) и итог.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeqPlan {
    pub genes: Vec<(&'static str, i32)>,
    pub status: SeqStatus,
}

/// Сборка вектора из фрагментов во входах: `fragments` — по слоту, None —
/// пусто, Some(строка гена) — фрагмент (пустая или битая строка — фрагмент
/// без гена). Нужно 2–3 разных гена, не больше `MAX_GENES`, сумма цен — не
/// выше `STABILITY`.
pub fn vector_plan(fragments: &[Option<&str>]) -> SeqPlan {
    let present: Vec<&str> = fragments.iter().flatten().copied().collect();
    let genes: Vec<(&'static str, i32)> = present.iter().filter_map(|t| gene_from_text(t)).collect();
    let status = if present.is_empty() {
        SeqStatus::Empty
    } else if genes.len() < present.len() {
        SeqStatus::Blank
    } else if present.len() == 1 {
        SeqStatus::One
    } else if genes.iter().enumerate().any(|(i, (a, _))| genes[..i].iter().any(|(b, _)| a == b)) {
        SeqStatus::Twice
    } else if genes.len() > MAX_GENES {
        SeqStatus::TooMany
    } else if genome_cost(&genes) > STABILITY {
        SeqStatus::Unstable
    } else {
        SeqStatus::Working
    };
    SeqPlan { genes, status }
}

/// Антисмысл: ген первого фрагмента во входах (по порядку слотов) — его
/// вектор и будет удалять. Остальные фрагменты ждут своей очереди.
pub fn antisense_plan(fragments: &[Option<&str>]) -> SeqPlan {
    match fragments.iter().flatten().next() {
        None => SeqPlan { genes: Vec::new(), status: SeqStatus::Empty },
        Some(text) => match gene_from_text(text) {
            Some(g) => SeqPlan { genes: vec![g], status: SeqStatus::Working },
            None => SeqPlan { genes: Vec::new(), status: SeqStatus::Blank },
        },
    }
}

/// Сколько уйдёт фосфора и раствора: фосфор — штука на ген вектора, на
/// антисмысл и на чтение; раствор — порция на любой вектор, чтение без него.
pub fn seq_supplies(mode: SeqMode, genes: usize) -> (u32, u32) {
    match mode {
        SeqMode::Vector => (genes.max(1) as u32, 1),
        SeqMode::Antisense => (1, 1),
        SeqMode::Read => (1, 0),
    }
}

/// Хватает ли всего, чтобы начать: план готов, расходники есть, выход пуст
/// (вектор не стакается). Иначе — чего не хватает, по порядку.
pub fn seq_ready(plan: SeqStatus, need: (u32, u32), phosphorus: u32, broth: u32, output_free: bool) -> SeqStatus {
    if plan != SeqStatus::Working {
        return plan;
    }
    if phosphorus < need.0 {
        return SeqStatus::NoPhosphorus;
    }
    if broth < need.1 {
        return SeqStatus::NoBroth;
    }
    if !output_free {
        return SeqStatus::OutputFull;
    }
    SeqStatus::Working
}

/// Пациент для чтения: ближайший моб на операционном столе (`mobs` — id и
/// позиция тех, у кого бит «на столе») не дальше `SEQ_TABLE_REACH` от центра
/// консоли по горизонтали и `SEQ_TABLE_HEIGHT` по высоте.
pub fn nearest_patient(origin: [i32; 3], mobs: &[(u32, [f64; 3])]) -> Option<u32> {
    let c = [f64::from(origin[0]) + 0.5, f64::from(origin[1]), f64::from(origin[2]) + 0.5];
    mobs.iter()
        .filter(|(_, p)| (p[1] - c[1]).abs() <= SEQ_TABLE_HEIGHT)
        .map(|(id, p)| (*id, (p[0] - c[0]).powi(2) + (p[2] - c[2]).powi(2)))
        .filter(|(_, d)| *d <= SEQ_TABLE_REACH * SEQ_TABLE_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// Действует ли прочитанное: тот же пациент с тем же геномом (строка
/// `genome_data`). Иначе — читать заново.
pub fn read_still_valid(patient: (u32, &str), saved: Option<(u32, &str)>) -> bool {
    saved.is_some_and(|(id, text)| id == patient.0 && text == patient.1)
}

/// Ген одной строкой: «Хитиновый панцирь ++».
pub fn gene_short(name: &str, strength: i32) -> String {
    let title = gene(name).map_or(name, |k| k.title);
    format!("{title} {}", "+".repeat(strength.clamp(0, 3) as usize))
}

/// Гены вектора строкой для предмета: та же форма, что геном моба.
pub fn vector_text(genes: &[(&str, i32)]) -> String {
    genome_data(genes)
}

/// Гены вектора из строки предмета.
pub fn vector_genes(text: &str) -> Vec<(&'static str, i32)> {
    genes_of_data(text)
}

/// Подпись вектора: «Генный вектор: Хитиновый панцирь ++, Рогатость +».
pub fn vector_label(text: &str) -> String {
    let genes = vector_genes(text);
    if genes.is_empty() {
        return "Генный вектор: пустой".to_string();
    }
    let list: Vec<String> = genes.iter().map(|(n, s)| gene_short(n, *s)).collect();
    format!("Генный вектор: {}", list.join(", "))
}

/// Подсказка вектора: гены, что они дают, цена и как вводить.
pub fn vector_tooltip(text: &str) -> Vec<String> {
    let genes = vector_genes(text);
    if genes.is_empty() {
        return vec!["Генов нет".to_string()];
    }
    let mut lines: Vec<String> = genes
        .iter()
        .filter_map(|(n, s)| Some(format!("{}: {}", gene_short(n, *s), gene(n)?.about)))
        .collect();
    lines.push(format!("Цена {} из {STABILITY}, генов {} из {MAX_GENES}", genome_cost(&genes), genes.len()));
    let human = genes.iter().filter(|(n, _)| gene(n).is_some_and(takes_in_humans)).count();
    lines.push(if human == genes.len() {
        "Человеку приживаются все".to_string()
    } else {
        format!("Человеку приживаются {human} из {}", genes.len())
    });
    lines
}

/// Подпись антисмыслового вектора: «Антисмысловой вектор: Хитиновый панцирь».
pub fn antisense_label(name: &str) -> String {
    match gene(name) {
        Some(kind) => format!("Антисмысловой вектор: {}", kind.title),
        None => "Антисмысловой вектор: пустой".to_string(),
    }
}

pub fn antisense_tooltip(name: &str) -> Vec<String> {
    match gene(name) {
        Some(kind) => vec![
            format!("Удаляет ген: {}", kind.title),
            "Уходят и прибавки, и эффекты гена".to_string(),
        ],
        None => vec!["Ген не прочитан".to_string()],
    }
}

/// Вектор мобу: каждый ген — как укол фрагментом (`splice_mob`, без предела
/// генома). Вернёт новый геном и исход по каждому гену вектора.
pub fn splice_vector_mob(
    have: &[(&'static str, i32)],
    genes: &[(&'static str, i32)],
) -> (Vec<(&'static str, i32)>, Vec<Splice>) {
    let mut out = have.to_vec();
    let mut results = Vec::with_capacity(genes.len());
    for (name, strength) in genes {
        let result = splice_mob(&out, name, *strength);
        if result == Splice::Done {
            out.retain(|(n, _)| n != name);
            out.push((name, *strength));
        }
        results.push(result);
    }
    (genes_of_data(&genome_data(&out)), results)
}

/// Вектор себе: каждый ген — как генотерапия (`splice_player`).
pub fn splice_vector_player(state: &mut PlayerState, genes: &[(&'static str, i32)]) -> Vec<Splice> {
    genes.iter().map(|(name, strength)| splice_player(state, name, *strength)).collect()
}

/// Что сказать после укола вектором: что вжилось, что нет и почему.
pub fn vector_message(genes: &[(&'static str, i32)], results: &[Splice]) -> String {
    let took: Vec<&str> = genes
        .iter()
        .zip(results)
        .filter(|(_, r)| **r == Splice::Done)
        .filter_map(|((n, _), _)| Some(gene(n)?.title))
        .collect();
    let refused = |want: Splice| -> Vec<&str> {
        genes
            .iter()
            .zip(results)
            .filter(|(_, r)| **r == want)
            .filter_map(|((n, _), _)| Some(gene(n)?.title))
            .collect()
    };
    let (already, alien) = (refused(Splice::Already), refused(Splice::NotForHumans));
    let mut parts = Vec::new();
    if took.is_empty() {
        parts.push("Вектор не вжился".to_string());
    } else {
        parts.push(format!("Вектор вжился: {}", took.join(", ")));
    }
    if !already.is_empty() {
        parts.push(format!("уже есть: {}", already.join(", ")));
    }
    if !alien.is_empty() {
        parts.push(format!("не приживается: {}", alien.join(", ")));
    }
    parts.join("; ")
}

/// Антисмысл мобу: геном без гена `name`; None — такого гена нет.
pub fn remove_gene(have: &[(&'static str, i32)], name: &str) -> Option<Vec<(&'static str, i32)>> {
    if !have.iter().any(|(n, _)| *n == name) {
        return None;
    }
    Some(have.iter().filter(|(n, _)| *n != name).copied().collect())
}

/// Антисмысл себе: снять ген из генома игрока. Ложь — гена не было.
pub fn unsplice_player(state: &mut PlayerState, name: &str) -> bool {
    match GENES.iter().position(|g| g.name == name) {
        Some(i) if state.genes[i] > 0 => {
            state.genes[i] = 0;
            true
        }
        _ => false,
    }
}

/// Что сказать после укола антисмыслом.
pub fn antisense_message(name: &str, removed: bool) -> String {
    let title = gene(name).map_or("Ген", |k| k.title);
    if removed {
        format!("{title}: ген выключен и удалён из генома")
    } else {
        format!("{title}: такого гена нет — удалять нечего")
    }
}

/// Поменять состояние игрока этого мира и, если поменялось, сразу записать
/// на диск — как `splice_world_player`. `f` отвечает итогом и «поменялось ли».
pub fn edit_world_player<R>(
    world: &std::path::Path,
    uuid: u128,
    f: impl FnOnce(&mut PlayerState) -> (R, bool),
) -> Option<R> {
    let mut table = states().lock().ok()?;
    let state = table.entry((world.to_path_buf(), uuid)).or_insert_with(|| {
        std::fs::read_to_string(state_path(world, uuid)).map(|t| state_from_text(&t)).unwrap_or_default()
    });
    let (result, changed) = f(state);
    let saved = *state;
    drop(table);
    if changed {
        let path = state_path(world, uuid);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, state_to_text(&saved));
    }
    Some(result)
}

/// Вектор себе в этом мире.
pub fn splice_vector_world_player(
    world: &std::path::Path,
    uuid: u128,
    genes: &[(&'static str, i32)],
) -> Option<Vec<Splice>> {
    edit_world_player(world, uuid, |state| {
        let results = splice_vector_player(state, genes);
        let changed = results.contains(&Splice::Done);
        (results, changed)
    })
}

/// Антисмысл себе в этом мире. Some(false) — гена не было.
pub fn unsplice_world_player(world: &std::path::Path, uuid: u128, name: &str) -> Option<bool> {
    edit_world_player(world, uuid, |state| {
        let removed = unsplice_player(state, name);
        (removed, removed)
    })
}

// Числа экрана консоли — после общих чисел установки: режим, итог (`SeqStatus`),
// сумма цен, сколько генов всего и первые `SEQ_SHOWN_GENES` генов числом
// «номер в `GENES` × 4 + сила» (0 — пусто).
pub const DATA_SEQ_MODE: usize = DATA_MB_END;
pub const DATA_SEQ_STATUS: usize = DATA_MB_END + 1;
pub const DATA_SEQ_COST: usize = DATA_MB_END + 2;
pub const DATA_SEQ_COUNT: usize = DATA_MB_END + 3;
pub const DATA_SEQ_GENES: usize = DATA_MB_END + 4;
pub const SEQ_SHOWN_GENES: usize = 8;
pub const DATA_SEQ_END: usize = DATA_SEQ_GENES + SEQ_SHOWN_GENES;

/// Записать в числа экрана режим, итог и гены (`data` длиной не меньше
/// `DATA_SEQ_END`).
pub fn write_seq(data: &mut [i32], mode: SeqMode, status: SeqStatus, genes: &[(&str, i32)]) {
    if data.len() < DATA_SEQ_END {
        return;
    }
    data[DATA_SEQ_MODE] = mode.id();
    data[DATA_SEQ_STATUS] = status.id();
    data[DATA_SEQ_COST] = genome_cost(genes);
    data[DATA_SEQ_COUNT] = genes.len() as i32;
    for k in 0..SEQ_SHOWN_GENES {
        data[DATA_SEQ_GENES + k] = genes
            .get(k)
            .and_then(|(name, s)| Some(GENES.iter().position(|g| g.name == *name)? as i32 * 4 + (*s).clamp(1, 3)))
            .unwrap_or(0);
    }
}

/// Что монитор знает о работе.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeqShown {
    pub mode: SeqMode,
    pub status: SeqStatus,
    pub cost: i32,
    /// Генов всего; на экран пришли первые `SEQ_SHOWN_GENES`.
    pub count: u32,
    pub genes: Vec<(&'static str, i32)>,
}

pub fn seq_shown(data: &[i32]) -> SeqShown {
    let genes = (0..SEQ_SHOWN_GENES)
        .filter_map(|k| {
            let v = screen_datum(data, DATA_SEQ_GENES + k, 0);
            let (index, s) = (v / 4, v % 4);
            (v > 0 && s > 0).then(|| GENES.get(index as usize).map(|g| (g.name, s)))?
        })
        .collect();
    SeqShown {
        mode: SeqMode::from_id(screen_datum(data, DATA_SEQ_MODE, 0)).unwrap_or(SeqMode::Vector),
        status: SeqStatus::from_id(screen_datum(data, DATA_SEQ_STATUS, 0)),
        cost: screen_datum(data, DATA_SEQ_COST, 0),
        count: screen_datum(data, DATA_SEQ_COUNT, 0).max(0) as u32,
        genes,
    }
}

/// Итог коротко — нижней строкой монитора.
pub fn seq_status_text(shown: &SeqShown) -> String {
    let (phosphorus, _) = seq_supplies(shown.mode, shown.count as usize);
    match shown.status {
        SeqStatus::Idle => "Ждёт".to_string(),
        SeqStatus::Empty if shown.mode == SeqMode::Vector => "Нет фрагментов".to_string(),
        SeqStatus::Empty => "Нет фрагмента".to_string(),
        SeqStatus::One => "Нужно 2–3 фрагмента".to_string(),
        SeqStatus::Blank => "Пустой фрагмент".to_string(),
        SeqStatus::Twice => "Ген дважды".to_string(),
        SeqStatus::TooMany => format!("Больше {MAX_GENES} генов"),
        SeqStatus::Unstable => format!("Цена {} > {STABILITY}", shown.cost),
        SeqStatus::NoPhosphorus => format!("Нужен фосфор ×{phosphorus}"),
        SeqStatus::NoBroth => "Нужен раствор".to_string(),
        SeqStatus::OutputFull => "Выход занят".to_string(),
        SeqStatus::Working if shown.mode == SeqMode::Antisense => "Синтез антисмысла".to_string(),
        SeqStatus::Working => format!("Цена {} из {STABILITY}", shown.cost),
        SeqStatus::NoPatient => "Нет пациента".to_string(),
        SeqStatus::Reading => "Чтение…".to_string(),
        SeqStatus::Read if shown.count == 0 => "Мутаций нет".to_string(),
        SeqStatus::Read => format!("Цена генов: {}", shown.cost),
    }
}

/// Итог подробно — в подсказке монитора.
pub fn seq_status_long(shown: &SeqShown) -> String {
    match shown.status {
        SeqStatus::Empty if shown.mode == SeqMode::Read => "Положите фосфор: чтение тратит штуку".to_string(),
        SeqStatus::Empty => "Положите фрагменты ДНК в три верхних слота".to_string(),
        SeqStatus::One => "Вектор собирают из 2–3 фрагментов".to_string(),
        SeqStatus::Blank => "Во фрагменте нет прочитанного гена".to_string(),
        SeqStatus::Twice => "Один ген в двух фрагментах: оставьте один".to_string(),
        SeqStatus::TooMany => format!("Вектор держит не больше {MAX_GENES} генов"),
        SeqStatus::Unstable => {
            format!("Вектор нестабилен: сумма цен {} выше {STABILITY} — уберите дорогой ген", shown.cost)
        }
        SeqStatus::NoPatient => format!(
            "Уложите усыплённого моба на операционный стол не дальше {} блоков от консоли",
            SEQ_TABLE_REACH as i32
        ),
        SeqStatus::Read if shown.cost > STABILITY || shown.count as usize > MAX_GENES => {
            format!("Сверх природы ({MAX_GENES} гена, цена {STABILITY}): геном лабораторный")
        }
        SeqStatus::Read => format!("В пределах природы: до {MAX_GENES} генов, цена до {STABILITY}"),
        _ => seq_status_text(shown),
    }
}

/// Цвет нижней строки монитора.
pub fn seq_status_ink(status: SeqStatus) -> u32 {
    match status {
        SeqStatus::Working | SeqStatus::Reading | SeqStatus::Read => 0x5A_E8B4,
        SeqStatus::Idle | SeqStatus::Empty | SeqStatus::NoPatient => SCREEN_TEXT_DIM,
        _ => 0xE8_5A4A,
    }
}

/// Строки монитора (текст и цвет): гены, «…и ещё N», внизу — итог. Не
/// больше `rows` строк: итог всегда виден.
pub fn seq_monitor(shown: &SeqShown, rows: usize) -> Vec<(String, u32)> {
    let room = rows.saturating_sub(1);
    let total = (shown.count as usize).max(shown.genes.len());
    let show = if total > room { room.saturating_sub(1) } else { total };
    let mut out: Vec<(String, u32)> = shown
        .genes
        .iter()
        .take(show)
        .map(|(name, s)| {
            let line = gene_short(name, *s);
            if shown.mode == SeqMode::Antisense { (format!("- {line}"), SCREEN_TEXT) } else { (line, SCREEN_TEXT) }
        })
        .collect();
    if total > out.len() {
        out.push((format!("…и ещё {}", total - out.len()), SCREEN_TEXT_DIM));
    }
    out.push((seq_status_text(shown), seq_status_ink(shown.status)));
    out
}

/// Подсказка монитора: режим, все пришедшие гены с тем, что они дают, итог.
pub fn seq_tooltip(shown: &SeqShown) -> Vec<String> {
    let mut lines = vec![shown.mode.title().to_string()];
    for (name, s) in &shown.genes {
        if let Some(kind) = gene(name) {
            lines.push(format!("§7{}: {}", gene_short(name, *s), kind.about));
        }
    }
    let more = (shown.count as usize).saturating_sub(shown.genes.len());
    if more > 0 {
        lines.push(format!("§7…и ещё {more}"));
    }
    lines.push(seq_status_long(shown));
    let (phosphorus, broth) = seq_supplies(shown.mode, shown.count as usize);
    lines.push(if broth > 0 {
        format!("§7Расход: фосфор ×{phosphorus}, раствор ×{broth}")
    } else {
        format!("§7Расход: фосфор ×{phosphorus}")
    });
    lines
}

/// Кнопки режимов слева на экране консоли: сверху вниз — вектор, антисмысл,
/// чтение.
pub fn seq_buttons() -> [[f32; 4]; 3] {
    [[5.0, 17.0, 16.0, 16.0], [5.0, 35.0, 16.0, 16.0], [5.0, 53.0, 16.0, 16.0]]
}

/// Кнопка под курсором.
pub fn seq_button_at(at: [f32; 2]) -> Option<SeqMode> {
    seq_buttons().iter().position(|r| inside(*r, at)).and_then(|i| SeqMode::ALL.get(i).copied())
}

/// Нажатие кнопки меню на сервере: новый режим, если кнопка наша и режим
/// другой. Иначе None — ничего не менять.
pub fn seq_press(current: i32, button: i32) -> Option<SeqMode> {
    let mode = SeqMode::from_id(button)?;
    (mode.id() != current).then_some(mode)
}

/// Подсказка кнопки режима.
pub fn seq_button_tooltip(mode: SeqMode, current: SeqMode) -> Vec<String> {
    let mut lines = vec![mode.title().to_string(), format!("§7{}", mode.hint())];
    if mode == current {
        lines.push("§aВыбрано".to_string());
    } else {
        lines.push("§7ЛКМ — выбрать".to_string());
    }
    lines
}

#[cfg(test)]
mod sequencer_tests {
    use super::*;

    fn world(blocks: &[([i32; 3], &str)]) -> impl Fn([i32; 3]) -> String + use<> {
        let map: std::collections::HashMap<[i32; 3], String> =
            blocks.iter().map(|(p, b)| (*p, b.to_string())).collect();
        move |p| map.get(&p).cloned().unwrap_or_else(|| "minecraft:air".to_string())
    }

    fn sequencer() -> &'static Multiblock {
        multiblock(SEQUENCER).expect("секвенатор — установка")
    }

    /// Собранный секвенатор у `site`: консоль, стойки, монитор, корпус; на
    /// месте `[вправо, вверх, назад]` из `swap` — другой блок.
    fn built(site: Site, swap: &[([i32; 3], &'static str)]) -> Vec<([i32; 3], &'static str)> {
        let mut out = Vec::new();
        for y in 0..=1 {
            for r in -1..=1 {
                for b in 0..=2 {
                    let at = [r, y, b];
                    let block = match at {
                        [0, 0, 0] => SEQUENCER,
                        [-1 | 1, 0, 1] => SEQUENCER_RACK,
                        [0, 1, 0] => SEQUENCER_MONITOR,
                        [_, 1, _] if b == 0 => CASING_GLASS,
                        _ => CASING,
                    };
                    let block = swap.iter().find(|(p, _)| *p == at).map_or(block, |(_, s)| *s);
                    out.push((place(site.origin, site.facing, at), block));
                }
            }
        }
        out
    }

    #[test]
    fn the_sequencer_is_formed_in_any_facing() {
        for facing in [Facing::North, Facing::South, Facing::West, Facing::East] {
            let site = Site { origin: [10, 64, -4], facing };
            let at = world(&built(site, &[([1, 0, 0], INPUT_PORT), ([0, 1, 2], OUTPUT_PORT)]));
            let Verdict::Formed(a) = check_multiblock(sequencer(), site, &at) else {
                panic!("{facing:?}: не собран");
            };
            assert_eq!(a.parts.len(), 18 - 1 - 2, "{facing:?}: без консоли и портов");
            assert_eq!(a.inputs().count(), 1);
            assert_eq!(a.outputs().count(), 1);
            assert!(a.parts.iter().all(|p| is_formable(&at(*p))), "склеиваются все части");
        }
    }

    #[test]
    fn missing_racks_and_monitor_are_named() {
        let site = Site { origin: [0, 64, 0], facing: Facing::South };
        let mb = sequencer();
        let at = world(&built(site, &[([-1, 0, 1], CASING), ([0, 1, 0], "minecraft:air"), ([1, 0, 2], "minecraft:dirt")]));
        let Verdict::Broken(short) = check_multiblock(mb, site, &at) else { panic!() };
        let texts: Vec<String> = short.iter().map(|s| shortage_text(mb, s)).collect();
        assert_eq!(
            texts,
            vec!["Нет стойки секвенатора: 1-й ярус, слева", "Нет блока корпуса: 1-й ярус, справа", "Нет монитора: 2-й ярус, спереди"]
        );
        // Голая консоль: 14 стенок, 2 стойки, монитор.
        let Verdict::Broken(short) = check_multiblock(mb, site, &world(&[([0, 64, 0], SEQUENCER)])) else { panic!() };
        let walls: u32 = short.iter().filter(|s| s.need == 0).map(|s| s.count).sum();
        let racks: u32 = short.iter().filter(|s| s.need == 1).map(|s| s.count).sum();
        assert_eq!((walls, racks), (14, 2));
    }

    #[test]
    fn sequencer_slots_take_their_items() {
        let m = machine(SEQUENCER).expect("машина");
        for slot in 0..SEQ_FRAGMENT_SLOTS {
            assert!(slot_accepts(m, slot, FRAGMENT_ID));
            assert!(!slot_accepts(m, slot, PHOSPHORUS_ID));
        }
        assert!(slot_accepts(m, SEQ_SLOT_PHOSPHORUS, PHOSPHORUS_ID));
        assert!(!slot_accepts(m, SEQ_SLOT_PHOSPHORUS, FRAGMENT_ID));
        assert!(slot_accepts(m, SEQ_SLOT_BROTH, BROTH_ID));
        assert!(!slot_accepts(m, SEQ_SLOT_OUTPUT, GENE_VECTOR), "в выход не кладут");
        assert_eq!(machine_outputs(m), vec![SEQ_SLOT_OUTPUT]);
        assert_eq!(quick_move_targets(m, BROTH_ID), vec![SEQ_SLOT_BROTH]);
        assert_eq!(quick_move_targets(m, FRAGMENT_ID), vec![0, 1, 2]);
        assert!(is_output_slot(SEQUENCER, SEQ_SLOT_OUTPUT) && !is_output_slot(SEQUENCER, 0));
        assert_eq!(machine_data_slots(SEQUENCER), DATA_SEQ_END);
    }

    #[test]
    fn a_vector_holds_two_or_three_genes_within_stability() {
        // Хитин 2 (цена 2×2) + пружинные ноги 1 (1) = 5 — стабилен.
        let plan = vector_plan(&[Some("chitin:2"), None, Some("spring_legs:1")]);
        assert_eq!(plan.status, SeqStatus::Working);
        assert_eq!(plan.genes, vec![("chitin", 2), ("spring_legs", 1)]);
        assert_eq!(genome_cost(&plan.genes), 5);
        // Ровно шесть — ещё можно.
        assert_eq!(vector_plan(&[Some("chitin:2"), Some("spring_legs:1"), Some("albinism:1")]).status, SeqStatus::Working);
        // Семь — нет.
        let plan = vector_plan(&[Some("chitin:2"), Some("spring_legs:2"), Some("albinism:1")]);
        assert_eq!((plan.status, genome_cost(&plan.genes)), (SeqStatus::Unstable, 7));
        // Один фрагмент — мало, ни одного — пусто.
        assert_eq!(vector_plan(&[Some("chitin:1"), None, None]).status, SeqStatus::One);
        assert_eq!(vector_plan(&[None, None, None]).status, SeqStatus::Empty);
        // Тот же ген дважды, даже разной силы.
        assert_eq!(vector_plan(&[Some("chitin:1"), Some("chitin:2")]).status, SeqStatus::Twice);
        // Фрагмент без гена.
        assert_eq!(vector_plan(&[Some(""), Some("chitin:1")]).status, SeqStatus::Blank);
        assert_eq!(vector_plan(&[Some("nonsense:2"), Some("chitin:1")]).status, SeqStatus::Blank);
        // Четыре гена не влезают, даже дешёвые.
        let four = [Some("albinism:1"), Some("pallor:1"), Some("rust_hide:1"), Some("tumor:1")];
        assert_eq!(vector_plan(&four).status, SeqStatus::TooMany);
    }

    #[test]
    fn antisense_takes_the_first_fragment() {
        assert_eq!(antisense_plan(&[None, Some("horns:2"), Some("chitin:1")]).genes, vec![("horns", 2)]);
        assert_eq!(antisense_plan(&[None, None, None]).status, SeqStatus::Empty);
        assert_eq!(antisense_plan(&[Some("")]).status, SeqStatus::Blank);
        assert_eq!(antisense_plan(&[Some("chitin:3")]).status, SeqStatus::Working);
    }

    #[test]
    fn supplies_are_checked_in_order() {
        assert_eq!(seq_supplies(SeqMode::Vector, 3), (3, 1));
        assert_eq!(seq_supplies(SeqMode::Antisense, 1), (1, 1));
        assert_eq!(seq_supplies(SeqMode::Read, 5), (1, 0));
        let need = (2, 1);
        assert_eq!(seq_ready(SeqStatus::Working, need, 2, 1, true), SeqStatus::Working);
        assert_eq!(seq_ready(SeqStatus::Working, need, 1, 1, true), SeqStatus::NoPhosphorus);
        assert_eq!(seq_ready(SeqStatus::Working, need, 2, 0, true), SeqStatus::NoBroth);
        assert_eq!(seq_ready(SeqStatus::Working, need, 2, 1, false), SeqStatus::OutputFull);
        assert_eq!(seq_ready(SeqStatus::Unstable, need, 9, 9, true), SeqStatus::Unstable, "план важнее расходников");
    }

    #[test]
    fn one_prick_of_a_vector_splices_every_gene() {
        let have = vec![("chitin", 1), ("regeneration", 3)];
        let (genome, results) = splice_vector_mob(&have, &[("chitin", 2), ("horns", 1), ("regeneration", 2)]);
        assert_eq!(results, vec![Splice::Done, Splice::Done, Splice::Already]);
        assert_eq!(genome, vec![("chitin", 2), ("regeneration", 3), ("horns", 1)], "порядок таблицы, сила сильнейшая");
        let text = vector_message(&[("chitin", 2), ("horns", 1), ("regeneration", 2)], &results);
        assert_eq!(text, "Вектор вжился: Хитиновый панцирь, Рогатость; уже есть: Регенерация");

        let mut state = PlayerState::default();
        let results = splice_vector_player(&mut state, &[("chitin", 2), ("acid_glands", 1)]);
        assert_eq!(results, vec![Splice::Done, Splice::NotForHumans]);
        assert_eq!(player_gene(&state, "chitin"), 2);
        assert!(vector_message(&[("chitin", 2), ("acid_glands", 1)], &results).contains("не приживается: Кислотные железы"));
        assert_eq!(vector_message(&[("pallor", 1)], &[Splice::NotForHumans]), "Вектор не вжился; не приживается: Бледность");
    }

    #[test]
    fn antisense_removes_the_gene_from_mob_and_player() {
        let have = vec![("chitin", 2), ("horns", 1)];
        assert_eq!(remove_gene(&have, "chitin"), Some(vec![("horns", 1)]));
        assert_eq!(remove_gene(&have, "tail"), None);
        assert_eq!(remove_gene(&[("horns", 1)], "horns"), Some(vec![]), "последний ген — геном пуст");
        let mut state = PlayerState::default();
        assert_eq!(splice_player(&mut state, "chitin", 3), Splice::Done);
        assert!(gene_effects(&state).iter().any(|(e, _)| *e == "resistance"));
        assert!(unsplice_player(&mut state, "chitin"));
        assert_eq!(player_gene(&state, "chitin"), 0);
        assert!(gene_effects(&state).is_empty(), "эффекты гена больше не подновляются");
        assert!(!unsplice_player(&mut state, "chitin"), "второй раз — нечего");
        assert!(antisense_message("chitin", true).contains("удалён"));
        assert!(antisense_message("chitin", false).contains("нечего"));
    }

    #[test]
    fn the_world_player_keeps_vector_and_antisense() {
        let world = std::env::temp_dir().join(format!("biolab_seq_{}", std::process::id()));
        let uuid = 0x5E0_u128;
        let results = splice_vector_world_player(&world, uuid, &[("chitin", 1), ("aquatic", 2)]).unwrap();
        assert_eq!(results, vec![Splice::Done, Splice::Done]);
        assert_eq!(world_player_gene(&world, uuid, "aquatic"), 2);
        let saved = state_from_text(&std::fs::read_to_string(state_path(&world, uuid)).unwrap());
        assert_eq!(player_gene(&saved, "chitin"), 1, "записано на диск");
        assert_eq!(unsplice_world_player(&world, uuid, "aquatic"), Some(true));
        assert_eq!(unsplice_world_player(&world, uuid, "aquatic"), Some(false));
        let saved = state_from_text(&std::fs::read_to_string(state_path(&world, uuid)).unwrap());
        assert_eq!(player_gene(&saved, "aquatic"), 0);
        let _ = std::fs::remove_dir_all(&world);
    }

    #[test]
    fn the_patient_is_the_nearest_on_a_table() {
        let origin = [0, 64, 0];
        let mobs = [(7, [5.0, 64.0, 0.5]), (3, [2.5, 64.5, 2.5]), (9, [0.5, 70.0, 0.5])];
        assert_eq!(nearest_patient(origin, &mobs), Some(3));
        assert_eq!(nearest_patient(origin, &mobs[..1]), Some(7));
        assert_eq!(nearest_patient(origin, &mobs[2..]), None, "слишком высоко");
        assert_eq!(nearest_patient(origin, &[(1, [7.0, 64.0, 0.5])]), None, "дальше шести блоков");
        assert!(read_still_valid((3, "chitin:1"), Some((3, "chitin:1"))));
        assert!(!read_still_valid((3, "chitin:2"), Some((3, "chitin:1"))), "геном поменяли — читать заново");
        assert!(!read_still_valid((4, "chitin:1"), Some((3, "chitin:1"))));
        assert!(!read_still_valid((3, ""), None));
    }

    #[test]
    fn vector_items_are_labelled_from_their_data() {
        let text = vector_text(&[("horns", 1), ("chitin", 2)]);
        assert_eq!(text, "chitin:2,horns:1");
        assert_eq!(vector_label(&text), "Генный вектор: Хитиновый панцирь ++, Рогатость +");
        assert_eq!(item_label(GENE_VECTOR, |k| (k == VECTOR_KEY).then(|| text.clone())), Some(vector_label(&text)));
        assert_eq!(antisense_label("chitin"), "Антисмысловой вектор: Хитиновый панцирь");
        assert_eq!(item_label(ANTISENSE_VECTOR, |k| (k == ANTISENSE_KEY).then(|| "horns".to_string())).unwrap(), "Антисмысловой вектор: Рогатость");
        let tip = vector_tooltip(&text);
        assert!(tip.iter().any(|l| l == "Цена 6 из 6, генов 2 из 3"), "{tip:?}");
        assert!(tip.last().unwrap().contains("приживаются все"));
        assert!(antisense_tooltip("horns")[0].contains("Рогатость"));
        assert!(!item_hint(GENE_VECTOR).is_empty() && !item_hint(ANTISENSE_VECTOR).is_empty());
    }

    #[test]
    fn the_monitor_gets_the_genome_through_numbers() {
        let mut data = vec![0; DATA_SEQ_END];
        let genes: Vec<(&str, i32)> = vec![("gigantism", 3), ("chitin", 1), ("sedation_resistance", 2)];
        write_seq(&mut data, SeqMode::Read, SeqStatus::Read, &genes);
        let shown = seq_shown(&data);
        assert_eq!(shown.genes, genes);
        assert_eq!((shown.mode, shown.status, shown.cost, shown.count), (SeqMode::Read, SeqStatus::Read, 12, 3));
        assert_eq!(seq_status_text(&shown), "Цена генов: 12");
        assert!(seq_status_long(&shown).contains("лабораторный"));
        // Три строки: ген, «…и ещё 2», итог.
        let lines = seq_monitor(&shown, 3);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].0, "Гигантизм +++");
        assert_eq!(lines[1].0, "…и ещё 2");
        assert_eq!(lines[2].0, "Цена генов: 12");
        // Влезает целиком — без «и ещё».
        let lines = seq_monitor(&shown, 4);
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[2].0, "Устойчивость к снотворному ++");
        // Генов больше, чем чисел: на экран приходят первые восемь.
        let many: Vec<(&str, i32)> = GENES.iter().take(10).map(|g| (g.name, 1)).collect();
        write_seq(&mut data, SeqMode::Read, SeqStatus::Read, &many);
        let shown = seq_shown(&data);
        assert_eq!((shown.genes.len(), shown.count), (SEQ_SHOWN_GENES, 10));
        assert!(seq_tooltip(&shown).iter().any(|l| l == "§7…и ещё 2"));
        // До первого пакета.
        let empty = seq_shown(&[]);
        assert_eq!((empty.mode, empty.status), (SeqMode::Vector, SeqStatus::Idle));
        assert_eq!(seq_monitor(&empty, 4), vec![("Ждёт".to_string(), SCREEN_TEXT_DIM)]);
        // Антисмысл помечает ген минусом.
        write_seq(&mut data, SeqMode::Antisense, SeqStatus::NoBroth, &[("horns", 2)]);
        let lines = seq_monitor(&seq_shown(&data), 4);
        assert_eq!(lines[0].0, "- Рогатость ++");
        assert_eq!(lines[1].0, "Нужен раствор");
        for s in SeqStatus::ALL {
            assert_eq!(SeqStatus::from_id(s.id()), s);
        }
    }

    #[test]
    fn mode_buttons_sit_left_of_the_slots() {
        let s = machine_screen(SEQUENCER).unwrap();
        let rects = screen_rects(&s);
        for (i, b) in seq_buttons().iter().enumerate() {
            assert!(b[0] >= 4.0 && b[1] >= SCREEN_HEADER && b[1] + b[3] <= s.inventory[1] - 12.0, "кнопка {i}");
            for r in &rects {
                assert!(!rects_overlap(*b, *r), "кнопка {i} налезает на {r:?}");
            }
            let centre = [b[0] + b[2] / 2.0, b[1] + b[3] / 2.0];
            assert_eq!(seq_button_at(centre), Some(SeqMode::ALL[i]));
        }
        assert_eq!(seq_button_at([100.0, 30.0]), None);
        assert_eq!(seq_press(0, 2), Some(SeqMode::Read));
        assert_eq!(seq_press(2, 2), None, "тот же режим");
        assert_eq!(seq_press(0, 7), None, "чужая кнопка");
        assert!(seq_button_tooltip(SeqMode::Vector, SeqMode::Vector).iter().any(|l| l.contains("Выбрано")));
        // Подсказки экрана: над кнопкой — режим, над монитором — геном.
        let tip = screen_tooltip(SEQUENCER, &[], [13.0, 43.0]);
        assert_eq!(tip.first().map(String::as_str), Some("Антисмысловой вектор"));
    }

    /// Блоки секвенатора в `content/`: стойка и монитор склеиваются, у консоли
    /// шаблон печи; меню — со своими числами.
    #[test]
    fn sequencer_blocks_are_declared() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/horizon");
        for id in [SEQUENCER, SEQUENCER_RACK, SEQUENCER_MONITOR] {
            let name = id.trim_start_matches("biolab:");
            let text = std::fs::read_to_string(root.join(format!("block/{name}.json"))).unwrap_or_else(|_| panic!("нет блока {id}"));
            let json = Json::parse(&text).unwrap();
            let formed = json.get("state_properties").and_then(|p| p.get(FORMED)).is_some();
            assert_eq!(formed, is_formable(id), "{id}");
            if id == SEQUENCER {
                assert_eq!(json.get("template").and_then(Json::str), Some("minecraft:furnace"));
            }
        }
        let menu = Json::parse(&std::fs::read_to_string(root.join("menu/sequencer.json")).unwrap()).unwrap();
        assert_eq!(menu.get("data_slots").and_then(Json::num), Some(DATA_SEQ_END as f64));
        for item in [GENE_VECTOR, ANTISENSE_VECTOR] {
            let name = item.trim_start_matches("biolab:");
            assert!(root.join(format!("item/{name}.json")).exists(), "нет предмета {item}");
        }
    }
}

// ---- M8: биореактор (GDD 4.18, п. 2) ----
//
// Куб 3×3×3: стенки, пол и потолок из корпуса, стекла установки и портов,
// внутри пусто — там среда. Контроллер (`biolab:bioreactor`) стоит в середине
// передней стенки, под серединой пола — источник тепла (те же блоки, что у
// реактора и колонны). Порт бульона отдельно не нужен: входной порт кладёт
// бульон в слот бульона контроллера (`SlotRole::Fuel`), сырьё — во входы.
//
// Варит партией: до `BIOREACTOR_BATCH` предметов за раз по рецептам
// ферментера и инкубатора (`BIOREACTOR_RECIPES`, своих рецептов нет). Рецепт
// каждого предмета ищется так же, как у этих машин, — сперва ферментера,
// потом инкубатора: культура в биореакторе идёт в экстракт, а не в отбор
// (отбор штамма — тонкая работа инкубатора). Партия — по одному предмету за
// рецепт, время партии — самый долгий рецепт в ней, сокращённый на четверть
// (`bioreactor_time`). Одна партия съедает одну порцию питательного раствора.
//
// Температура среды — модель реактора (`next_temperature`), но среда — бак
// воды за стальным днищем: источник греет её не до своего потолка, а до
// `ROOM_TEMPERATURE + потолок / MEDIUM_HEAT_DIVISOR` (`medium_target`), и шаг
// в `BIOREACTOR_HEAT_EVERY` тиков, а не каждый тик. Костёр — 35 °C, костёр
// душ — 37 °C: в диапазоне 30–40. Магма (45), огонь (55), лава (67) —
// перегрев. Холодная среда (ниже 30) — партия ждёт, прогресс стоит; горячая
// (выше 40) — начатая партия испорчена: сырьё и порция раствора пропали.
//
// Скисание: с шансом `BIOREACTOR_SOUR_PERCENT` готовая партия — слизь
// (`SOUR_ITEM`, по шарику за предмет), если контроллер не в карантинной
// камере (`in_quarantine`, контракт волны 2 ниже).
//
// Исполнение — модуль `crate::game::biolab_bioreactor` (`bioreactor.rs`).

/// Биореактор: контроллер в середине передней стенки куба.
pub const BIOREACTOR: &str = "biolab:bioreactor";
/// Чьи рецепты варит биореактор — по порядку: первый совпавший и варится.
pub const BIOREACTOR_RECIPES: [&str; 2] = ["biolab:fermenter", INCUBATOR];
/// Сколько предметов в партии.
pub const BIOREACTOR_BATCH: u32 = 8;
/// Слоты контроллера: восемь входов, бульон, восемь выходов (у каждого
/// предмета партии — свой: культуры не складываются).
pub const BIOREACTOR_INPUTS: usize = 8;
pub const BIOREACTOR_BROTH: usize = BIOREACTOR_INPUTS;
pub const BIOREACTOR_FIRST_OUTPUT: usize = BIOREACTOR_BROTH + 1;
pub const BIOREACTOR_OUTPUTS: usize = 8;
pub const BIOREACTOR_SLOTS: usize = BIOREACTOR_FIRST_OUTPUT + BIOREACTOR_OUTPUTS;

pub const BIOREACTOR_SLOT_ROLES: [SlotRole; BIOREACTOR_SLOTS] = {
    let mut roles = [SlotRole::Input; BIOREACTOR_SLOTS];
    roles[BIOREACTOR_BROTH] = SlotRole::Fuel;
    let mut i = BIOREACTOR_FIRST_OUTPUT;
    while i < BIOREACTOR_SLOTS {
        roles[i] = SlotRole::Output;
        i += 1;
    }
    roles
};

/// Нужды схемы: стенка (корпус, стекло или порт), пустота внутри, тепло.
pub const BIOREACTOR_NEEDS: [Need; 3] = [
    Need { parts: &[Part::Block(CASING), Part::Block(CASING_GLASS), Part::Port], one: "блока корпуса", many: "блоков корпуса" },
    Need { parts: &[Part::Air], one: "места для среды", many: "мест для среды" },
    Need { parts: &[Part::Heat], one: "источника тепла", many: "источников тепла" },
];

/// Пол и потолок: девять клеток стенки.
const BIOREACTOR_LID: [Cell; 9] = [
    cell(-1, 0, 0), cell(0, 0, 0), cell(1, 0, 0),
    cell(-1, 1, 0), cell(0, 1, 0), cell(1, 1, 0),
    cell(-1, 2, 0), cell(0, 2, 0), cell(1, 2, 0),
];
/// Ярус контроллера: стенки вокруг, в середине пусто.
const BIOREACTOR_RING: [Cell; 8] = [
    cell(-1, 0, 0), cell(1, 0, 0),
    cell(-1, 1, 0), cell(0, 1, 1), cell(1, 1, 0),
    cell(-1, 2, 0), cell(0, 2, 0), cell(1, 2, 0),
];

pub const BIOREACTOR_SHAPE: Shape = Shape {
    layers: &[
        Layer { cells: &[cell(0, 1, 2)], min: 1, max: 1, name: "под днищем" },
        Layer { cells: &BIOREACTOR_LID, min: 1, max: 1, name: "" },
        Layer { cells: &BIOREACTOR_RING, min: 1, max: 1, name: "" },
        Layer { cells: &BIOREACTOR_LID, min: 1, max: 1, name: "" },
    ],
    controller: 2,
};

/// Где источник тепла от контроллера: `[вправо, вверх, назад]` — под
/// серединой пола.
pub const BIOREACTOR_HEAT_AT: [i32; 3] = [0, -2, 1];

/// Диапазон среды, °C: ниже — ждёт, выше — партия портится.
pub const BIOREACTOR_RANGE: (i32, i32) = (30, 40);
/// Во сколько раз среда за днищем холоднее потолка источника (над
/// комнатной).
pub const MEDIUM_HEAT_DIVISOR: i32 = 20;
/// Раз в сколько тиков меняется температура среды: бак большой.
pub const BIOREACTOR_HEAT_EVERY: i64 = 10;
/// Партия идёт столько от самого долгого рецепта в ней: три четверти.
pub const BIOREACTOR_SPEED: (i32, i32) = (3, 4);
/// Шанс скиснуть вне карантина, процентов на партию.
pub const BIOREACTOR_SOUR_PERCENT: u64 = 5;
/// Во что скисает партия: по шарику на предмет.
pub const SOUR_ITEM: &str = "minecraft:slime_ball";
/// Звук скисшей партии: шлепок слизи (перегрев звучит как у реактора).
pub const BIOREACTOR_SOUR_SOUND: Sound = block("minecraft:entity.slime.squish", 0.8, 0.7);

/// Сейв контроллера: температура среды и чем кончилась прошлая партия.
pub const BIO_TEMPERATURE_KEY: &str = "biolab_bio_temperature";
pub const BIO_LAST_KEY: &str = "biolab_bio_last";

/// Чем кончилась партия (`BIO_LAST_KEY`, `DATA_BIO_LAST`).
pub const BATCH_NONE: i32 = 0;
pub const BATCH_DONE: i32 = 1;
pub const BATCH_SOUR: i32 = 2;
pub const BATCH_SPOILED: i32 = 3;

// Свои числа экрана биореактора — после общих чисел установки.
/// Температура среды.
pub const DATA_BIO_TEMPERATURE: usize = DATA_MB_END;
/// До скольки греет источник среду; −1 — не греет (нет источника или
/// установка разобрана).
pub const DATA_BIO_TARGET: usize = DATA_MB_END + 1;
/// Сколько предметов в партии сейчас.
pub const DATA_BIO_BATCH: usize = DATA_MB_END + 2;
/// Чем кончилась прошлая партия (`BATCH_*`).
pub const DATA_BIO_LAST: usize = DATA_MB_END + 3;
/// 1 — контроллер в карантинной камере: партия не скисает.
pub const DATA_BIO_QUARANTINE: usize = DATA_MB_END + 4;
pub const DATA_BIO_END: usize = DATA_MB_END + 5;

/// До скольки источник под днищем греет среду; None — блок не греет.
pub fn medium_target(block: &str) -> Option<i32> {
    heat_target(block).map(|ceiling| ROOM_TEMPERATURE + ceiling / MEDIUM_HEAT_DIVISOR)
}

/// Среда по температуре.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Medium {
    /// Ниже диапазона: партия ждёт.
    Cold,
    /// В диапазоне: варит.
    Fit,
    /// Выше: начатая партия портится.
    Hot,
}

pub fn medium(temperature: i32) -> Medium {
    let (min, max) = BIOREACTOR_RANGE;
    if temperature < min {
        Medium::Cold
    } else if temperature > max {
        Medium::Hot
    } else {
        Medium::Fit
    }
}

/// Пора ли менять температуру среды.
pub fn medium_due(ticks: i64) -> bool {
    ticks.rem_euclid(BIOREACTOR_HEAT_EVERY) == 0
}

/// Что взять в партию: `inputs` — входные слоты по порядку, у каждого
/// сколько в нём лежит, если для него есть рецепт (None — пусто или
/// рецепта нет). Берём по порядку слотов, сколько есть, пока не наберётся
/// `BIOREACTOR_BATCH`. Вернёт (слот, сколько).
pub fn bioreactor_take(inputs: &[Option<u32>]) -> Vec<(usize, u32)> {
    let mut left = BIOREACTOR_BATCH;
    let mut take = Vec::new();
    for (slot, count) in inputs.iter().enumerate() {
        if left == 0 {
            break;
        }
        let Some(count) = count else { continue };
        let n = (*count).min(left);
        if n > 0 {
            take.push((slot, n));
            left -= n;
        }
    }
    take
}

/// Время партии: самый долгий рецепт в ней, сокращённый (`BIOREACTOR_SPEED`).
/// Пустая партия — 0: работы нет.
pub fn bioreactor_time(times: &[i32]) -> i32 {
    let Some(longest) = times.iter().copied().max() else {
        return 0;
    };
    let (num, den) = BIOREACTOR_SPEED;
    (longest.max(1) * num / den).max(1)
}

/// Скисла ли готовая партия. В карантине — никогда; иначе с шансом
/// `BIOREACTOR_SOUR_PERCENT` от зерна (место и тик, как выдача гена).
pub fn bioreactor_sours(seed: u64, quarantine: bool) -> bool {
    if quarantine {
        return false;
    }
    let mixed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(29);
    mixed % 100 < BIOREACTOR_SOUR_PERCENT
}

/// Что экран биореактора знает о среде и партии.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BioReading {
    pub temperature: i32,
    pub target: Option<i32>,
    pub batch: u32,
    pub last: i32,
    pub quarantine: bool,
}

pub fn bio_reading(data: &[i32]) -> BioReading {
    let target = screen_datum(data, DATA_BIO_TARGET, -1);
    BioReading {
        temperature: screen_datum(data, DATA_BIO_TEMPERATURE, ROOM_TEMPERATURE),
        target: (target >= 0).then_some(target),
        batch: screen_datum(data, DATA_BIO_BATCH, 0).max(0) as u32,
        last: screen_datum(data, DATA_BIO_LAST, BATCH_NONE),
        quarantine: screen_datum(data, DATA_BIO_QUARANTINE, 0) == 1,
    }
}

/// Короткая строка состояния на табло собранного биореактора.
pub fn bioreactor_status(r: &BioReading) -> String {
    match medium(r.temperature) {
        Medium::Cold => "Среда холодная".to_string(),
        Medium::Hot => "Перегрев!".to_string(),
        Medium::Fit if r.batch > 0 => format!("Партия: {}", r.batch),
        Medium::Fit => "Среда готова".to_string(),
    }
}

/// Строки подсказки над табло собранного биореактора: среда, нагрев,
/// карантин, прошлая партия.
pub fn bioreactor_lines(data: &[i32]) -> Vec<String> {
    let r = bio_reading(data);
    let (min, max) = BIOREACTOR_RANGE;
    let mut lines = vec![format!("Среда: {} °C, нужно {min}–{max}", r.temperature)];
    lines.push(match r.target {
        None => "§cНет нагрева: под днищем нужен огонь".to_string(),
        Some(t) if t > max => format!("§cНагрев до {t} °C — перегреет партию"),
        Some(t) if t < min => format!("§eНагрев до {t} °C — мало"),
        Some(t) => format!("Нагрев до {t} °C"),
    });
    if r.batch > 0 {
        lines.push(format!("В партии {} из {BIOREACTOR_BATCH}", r.batch));
    }
    lines.push(if r.quarantine {
        "В карантине: партия не скиснет".to_string()
    } else {
        format!("§7Вне карантина партия может скиснуть ({BIOREACTOR_SOUR_PERCENT}%)")
    });
    match r.last {
        BATCH_DONE => lines.push("§7Прошлая партия готова".to_string()),
        BATCH_SOUR => lines.push("§eПрошлая партия скисла в слизь".to_string()),
        BATCH_SPOILED => lines.push("§cПрошлая партия испорчена перегревом".to_string()),
        _ => {}
    }
    lines
}

/// Подсказка над табло биореактора: сборка (общая) и, если собран, — среда.
pub fn bioreactor_tooltip(data: &[i32]) -> Vec<String> {
    let mut lines = multiblock_tooltip(BIOREACTOR, data);
    if matches!(multiblock_shown(data), MultiblockShown::Formed { .. }) {
        lines.extend(bioreactor_lines(data));
    }
    lines
}

/// Шкала градусника на табло: от комнатной до 50 °C.
pub const BIO_SCALE: (i32, i32) = (ROOM_TEMPERATURE, 50);

/// Где на трубке градусника `tube` (x, y, ширина, высота) эта температура.
pub fn bio_thermo_x(tube: [f32; 4], temperature: i32) -> f32 {
    let (lo, hi) = BIO_SCALE;
    let f = (temperature.clamp(lo, hi) - lo) as f32 / (hi - lo) as f32;
    tube[0] + (tube[2] * f).floor()
}

#[cfg(test)]
mod bioreactor_tests {
    use super::*;

    fn world(blocks: &[([i32; 3], &str)]) -> impl Fn([i32; 3]) -> String + use<> {
        let map: std::collections::HashMap<[i32; 3], String> =
            blocks.iter().map(|(p, b)| (*p, b.to_string())).collect();
        move |p| map.get(&p).cloned().unwrap_or_else(|| "minecraft:air".to_string())
    }

    fn reactor() -> &'static Multiblock {
        multiblock(BIOREACTOR).expect("биореактор — установка")
    }

    /// Куб у контроллера `site`: всё корпус, середина пусто, под днищем
    /// `heat`, справа в среднем ярусе — стекло, слева — входной порт, на
    /// потолке посередине — выходной.
    fn cube(site: Site, heat: &'static str) -> Vec<([i32; 3], &'static str)> {
        let mut out = vec![(place(site.origin, site.facing, BIOREACTOR_HEAT_AT), heat)];
        for y in -1..=1 {
            for r in -1..=1 {
                for b in 0..=2 {
                    let block = match [r, y, b] {
                        [0, 0, 0] => BIOREACTOR,
                        [0, 0, 1] => continue,
                        [-1, 0, 1] => INPUT_PORT,
                        [0, 1, 1] => OUTPUT_PORT,
                        [1, 0, _] => CASING_GLASS,
                        _ => CASING,
                    };
                    out.push((place(site.origin, site.facing, [r, y, b]), block));
                }
            }
        }
        out
    }

    #[test]
    fn a_full_cube_over_a_fire_is_formed_whatever_the_facing() {
        for facing in [Facing::North, Facing::South, Facing::West, Facing::East] {
            let site = Site { origin: [10, 70, -5], facing };
            let Verdict::Formed(a) = check_multiblock(reactor(), site, &world(&cube(site, "minecraft:campfire"))) else {
                panic!("{facing:?}: не собрано");
            };
            assert_eq!(a.inputs().collect::<Vec<_>>(), vec![place(site.origin, facing, [-1, 0, 1])]);
            assert_eq!(a.outputs().collect::<Vec<_>>(), vec![place(site.origin, facing, [0, 1, 1])]);
            // Стенки (26 без контроллера, пустоты и двух портов) и огонь.
            assert_eq!(a.parts.len(), 27 - 4 + 1);
            assert!(a.parts.contains(&place(site.origin, facing, BIOREACTOR_HEAT_AT)));
        }
    }

    #[test]
    fn a_broken_cube_says_what_is_missing() {
        let mb = reactor();
        let site = Site { origin: [0, 64, 0], facing: Facing::South };
        // Без огня.
        let mut blocks = cube(site, "minecraft:stone");
        let Verdict::Broken(short) = check_multiblock(mb, site, &world(&blocks)) else { panic!() };
        let texts: Vec<String> = short.iter().map(|s| shortage_text(mb, s)).collect();
        assert_eq!(texts, vec!["Нет источника тепла: под днищем"]);
        // Нет задней стенки среднего яруса и угла потолка; внутри земля.
        blocks = cube(site, "minecraft:lava");
        for at in [[-1, 0, 2], [0, 0, 2], [1, 0, 2], [1, 0, 1], [1, 1, 2]] {
            let p = place(site.origin, site.facing, at);
            blocks.retain(|(q, _)| *q != p);
        }
        blocks.push((place(site.origin, site.facing, [0, 0, 1]), "minecraft:dirt"));
        let Verdict::Broken(short) = check_multiblock(mb, site, &world(&blocks)) else { panic!() };
        let texts: Vec<String> = short.iter().map(|s| shortage_text(mb, s)).collect();
        assert_eq!(
            texts,
            vec![
                "Нет места для среды: 2-й ярус, в середине",
                "Нет 2 блоков корпуса: 2-й ярус, справа",
                "Нет блока корпуса: 2-й ярус, слева",
                "Нет блока корпуса: 2-й ярус, сзади",
                "Нет блока корпуса: 3-й ярус, справа",
            ]
        );
    }

    #[test]
    fn the_medium_warms_to_range_only_over_a_gentle_fire() {
        assert_eq!(medium_target("minecraft:campfire"), Some(35));
        assert_eq!(medium_target("minecraft:soul_campfire"), Some(37));
        assert_eq!(medium_target("minecraft:stone"), None);
        for (heat, fits) in [("minecraft:campfire", true), ("minecraft:soul_campfire", true), ("minecraft:magma_block", false), ("minecraft:fire", false), ("minecraft:lava", false)] {
            let target = medium_target(heat).unwrap();
            assert_eq!(medium(target) == Medium::Fit, fits, "{heat}: {target}");
            // Среда доходит до потолка источника и там стоит.
            let mut t = ROOM_TEMPERATURE;
            for _ in 0..100 {
                t = next_temperature(t, Some(target));
            }
            assert_eq!(t, target, "{heat}");
        }
        assert_eq!(medium(29), Medium::Cold);
        assert_eq!(medium(30), Medium::Fit);
        assert_eq!(medium(40), Medium::Fit);
        assert_eq!(medium(41), Medium::Hot);
        assert!(medium_due(20) && !medium_due(21));
    }

    #[test]
    fn a_batch_takes_up_to_eight_in_slot_order() {
        assert_eq!(bioreactor_take(&[Some(64)]), vec![(0, 8)]);
        assert_eq!(bioreactor_take(&[Some(3), None, Some(1), Some(10)]), vec![(0, 3), (2, 1), (3, 4)]);
        assert_eq!(bioreactor_take(&[None, None]), vec![]);
        // Культуры по одной в слоте: восемь слотов — восемь культур.
        let one = [Some(1); BIOREACTOR_INPUTS];
        assert_eq!(bioreactor_take(&one).len(), 8);
        assert_eq!(bioreactor_time(&[160, 200, 160]), 150, "самый долгий, на четверть быстрее");
        assert_eq!(bioreactor_time(&[]), 0);
        assert_eq!(bioreactor_time(&[1]), 1);
    }

    #[test]
    fn a_batch_sours_rarely_and_never_in_quarantine() {
        let sour = (0..10_000u64).filter(|s| bioreactor_sours(*s, false)).count();
        assert!((300..=700).contains(&sour), "скисло {sour} из 10000 — шанс 5%");
        assert!((0..10_000u64).all(|s| !bioreactor_sours(s, true)));
    }

    #[test]
    fn the_controller_has_inputs_broth_and_outputs() {
        let machine = machine(BIOREACTOR).expect("контроллер — машина");
        assert_eq!(machine.slots, BIOREACTOR_SLOTS);
        assert_eq!(machine_inputs(machine), (0..=BIOREACTOR_BROTH).collect::<Vec<_>>());
        assert_eq!(machine_outputs(machine), (BIOREACTOR_FIRST_OUTPUT..BIOREACTOR_SLOTS).collect::<Vec<_>>());
        // Бульон — только в свой слот, сырьё ферментера и инкубатора — во входы.
        assert_eq!(quick_move_targets(machine, BROTH_ID), vec![BIOREACTOR_BROTH]);
        for item in ["minecraft:wheat", "minecraft:sugar", "biolab:swab", "minecraft:bread", "biolab:culture"] {
            assert_eq!(quick_move_targets(machine, item), (0..BIOREACTOR_INPUTS).collect::<Vec<_>>(), "{item}");
        }
        assert!(!hopper_accepts(machine, "minecraft:dirt"));
        assert!(!hopper_accepts(machine, "minecraft:bone"), "сырьё перегонного куба — не сюда");
        // Воронки: сверху — сырьё, сбоку — бульон, снизу — готовое.
        let faces = machine_faces(machine);
        assert_eq!(faces[1], (0..BIOREACTOR_INPUTS).collect::<Vec<_>>());
        assert_eq!(faces[2], vec![BIOREACTOR_BROTH]);
        assert!(machine_data_slots(BIOREACTOR) == DATA_BIO_END && DATA_BIO_END > DATA_MB_END);
        // Свои числа — после общих чисел установки.
        assert!(DATA_BIO_TEMPERATURE >= DATA_MB_END);
    }

    /// Рецепты, которые варит биореактор, есть в `content/`: и ферментера, и
    /// инкубатора. Раствор, который он ест, ферментер и делает — из пшеницы.
    #[test]
    fn it_brews_what_the_fermenter_and_incubator_brew() {
        for kind in BIOREACTOR_RECIPES {
            assert!(is_machine(kind), "{kind}");
            assert!(!machine_ingredients(kind).is_empty(), "{kind}: нет рецептов");
            for item in machine_ingredients(kind) {
                assert!(slot_accepts(machine(BIOREACTOR).unwrap(), 0, item), "{kind}: {item}");
            }
        }
        assert!(machine_ingredients("biolab:fermenter").contains(&"minecraft:wheat"));
        assert!(!machine_ingredients("biolab:fermenter").contains(&BROTH_ID), "раствор — не сырьё");
    }

    #[test]
    fn the_screen_reads_the_medium() {
        let mut data = vec![0; DATA_BIO_END];
        data[DATA_MB_STATE] = MB_FORMED;
        data[DATA_BIO_TEMPERATURE] = 35;
        data[DATA_BIO_TARGET] = 35;
        data[DATA_BIO_BATCH] = 5;
        data[DATA_BIO_LAST] = BATCH_SOUR;
        let r = bio_reading(&data);
        assert_eq!(r, BioReading { temperature: 35, target: Some(35), batch: 5, last: BATCH_SOUR, quarantine: false });
        assert_eq!(bioreactor_status(&r), "Партия: 5");
        let lines = bioreactor_tooltip(&data);
        assert_eq!(lines[0], "Биореактор: собрана");
        assert!(lines.contains(&"Среда: 35 °C, нужно 30–40".to_string()));
        assert!(lines.iter().any(|l| l.contains("скисла в слизь")));
        assert!(lines.iter().any(|l| l.contains("может скиснуть")));
        data[DATA_BIO_QUARANTINE] = 1;
        data[DATA_BIO_TARGET] = 45;
        let lines = bioreactor_tooltip(&data);
        assert!(lines.iter().any(|l| l.contains("не скиснет")));
        assert!(lines.iter().any(|l| l.contains("перегреет")));
        assert_eq!(bio_reading(&[]).target, None, "до первого пакета");
        assert_eq!(bioreactor_status(&bio_reading(&[])), "Среда холодная");
        // Разобран — только нехватка, без среды.
        data[DATA_MB_STATE] = MB_BROKEN;
        assert!(!bioreactor_tooltip(&data).iter().any(|l| l.starts_with("Среда")));
        let tube = [0.0, 0.0, 30.0, 4.0];
        assert_eq!(bio_thermo_x(tube, 0), 0.0);
        assert_eq!(bio_thermo_x(tube, 35), 15.0);
        assert_eq!(bio_thermo_x(tube, 900), 30.0);
    }

    /// Блок контроллера: шаблон печи (меню), своё свойство `formed` — общий
    /// вид на всю переднюю стенку; меню, лут, рецепт, кирка.
    #[test]
    fn the_controller_is_declared_in_content() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap_or_else(|_| panic!("нет {p}"));
        let json = Json::parse(&read("data/biolab/horizon/block/bioreactor.json")).unwrap();
        assert_eq!(json.get("template").and_then(Json::str), Some("minecraft:furnace"));
        let formed = json.get("state_properties").and_then(|p| p.get(FORMED)).expect("formed");
        assert_eq!(formed.get("type").and_then(Json::str), Some("boolean"));
        assert!(is_formable(BIOREACTOR));
        assert!(read("data/biolab/loot_table/blocks/bioreactor.json").contains(BIOREACTOR));
        assert!(read("data/minecraft/tags/block/mineable/pickaxe.json").contains(BIOREACTOR));
        let recipe = Json::parse(&read("data/biolab/recipe/bioreactor.json")).unwrap();
        assert_eq!(recipe.get("result").and_then(|r| r.get("id")).and_then(Json::str), Some(BIOREACTOR));
        let research = read(RESEARCH_PATH);
        assert!(research.contains("= bioreactor"), "запись в журнале");
        assert!(research_index("bioreactor").is_some());
    }
}

// ---- M8: карантинная камера (GDD 4.18, п. 4) ----
//
// Чистая комната любой формы до 9×9×5 внутри (по X, по Z, в высоту): стены,
// пол и потолок — из чистых панелей (`biolab:clean_panel`) и стекла
// установки, в стене — фильтр (контроллер `biolab:quarantine`) и хотя бы
// один шлюз. Проверка — заливка изнутри (`check_room`, через
// `Check::Custom`), ответ — тот же `Verdict`, что у схем.
//
// *Что такое комната.* Заливка идёт от клетки перед лицом фильтра (если там
// оболочка — от клетки за ним: фильтр можно поставить лицом наружу) по
// шести сторонам через всё, кроме оболочки (панели, стекло установки,
// фильтры) и дверей. Машины, сундуки, факелы внутри комнаты заливке не
// мешают, а земля вместо пола — дыра: комната только из панелей. Заливка
// ограничена коробкой «старт ± 9 по X и Z, ± 5 в высоту» — в неё влезает
// любая допустимая комната и ещё клетка, — заливка останавливается на первом
// же выходе за коробку, поэтому у фильтра посреди поля читается несколько
// сотен блоков, у закрытой комнаты 9×9×5 — около 750; каждый блок
// читается один раз (`Survey`), проверка — раз в секунду (`check_due`).
//
// *Где дыра.* Заливка дошла до края коробки — комната не закрыта. Путь
// заливки от старта до края идёт через дыру; каждая клетка пути пробуется:
// «заткни её — закроется ли комната?». Закрывают и сама дыра, и клетка
// сразу за ней, и узкий проход внутри комнаты; дыра из них — та, у которой
// больше всего соседей-стен (она сидит в оболочке), при равенстве — дальше
// от старта. Дыра шире одной клетки (нет пола целиком)
// одной пробкой не закрывается — тогда это первая клетка пути на краю или
// за краем коробки всей увиденной оболочки.
//
// *Шлюз.* Двери ванильные (любой `…_door`), для заливки — стена. У каждой
// двери в стене комнаты смотрим, что за ней (по ходу из комнаты):
// - снова дверь — «две двери подряд», шлюз, если за второй не оболочка;
// - пустое место — тамбур: заливка от него не больше 3×3×3; тамбур со своей
//   второй дверью наружу — шлюз; тамбур без второй двери — чулан, не беда;
//   «тамбур» больше 3×3×3 — это улица: дверь прямо наружу, камера не
//   закрыта («Дверь без шлюза: x y z»);
// - оболочка или сама комната — не выход.
// Нет ни одного шлюза — «Нет шлюза».
//
// *Реестр* собранных камер (`in_quarantine`, контракт волны 2): фильтр с
// живым фильтрующим патроном раз в секунду кладёт сюда внутренние клетки
// своей комнаты, между проверками только отмечается. Разобрали, вынули
// патрон, сломали фильтр — запись снимается; фильтр перестал тикать (чанк
// выгружен, взрыв) — через `ROOM_STALE_MS` запись не в счёт. После
// перезахода реестр пуст и восстанавливается первой же проверкой (она идёт
// сразу после загрузки). Реестр — в копии `runtime.rs` игрового крейта
// (`crate::game::biolab_core`): звать из хуков игры, не из lib-копии.
//
// *С M9* (GDD 4.23 п. 11, раздел «M9: лаборатория») камера не «держит/нет»:
// проверка — `seal_room` (пробоины до `MAX_HOLES`, дверь без шлюза и без
// шлюза вовсе — не отказ, а множитель), в реестре — доля утечки камеры,
// собранной даже без патрона; `in_quarantine` с этой долей отвечает «нет».
//
// *Утечка.* Машина, которая работает (лампа горит) с культурой тяжёлого
// штамма (тяжесть 4–5) во входе вне камеры, раз в `LEAK_EVERY` тиков
// заражает скот рядом и отмечает утечку; игрок в радиусе получает штамм на
// своём шаге болезни (`leak_near`).

/// Фильтр карантинной камеры — её контроллер.
pub const QUARANTINE: &str = "biolab:quarantine";
/// Чистая панель: стены, пол и потолок камеры.
pub const CLEAN_PANEL: &str = "biolab:clean_panel";
/// Фильтрующий патрон — сменный расходник фильтра.
pub const FILTER_CARTRIDGE: &str = "biolab:filter_cartridge";

/// Наибольшая комната внутри: по X, в высоту, по Z.
pub const ROOM_MAX: [i32; 3] = [9, 5, 9];
/// Наибольший тамбур шлюза.
pub const VESTIBULE_MAX: [i32; 3] = [3, 3, 3];

/// Чем блок служит комнате.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomBlock {
    /// Оболочка: панель, стекло установки, фильтр.
    Shell,
    /// Дверь: для заливки — стена, для шлюза — проход.
    Door,
    /// Всё остальное: воздух, машины, земля.
    Open,
}

/// Ванильная дверь любого дерева или металла (люк — не дверь).
pub fn is_door(block: &str) -> bool {
    block.ends_with("_door")
}

pub fn room_block(block: &str) -> RoomBlock {
    if block == CLEAN_PANEL || block == CASING_GLASS || block == QUARANTINE {
        RoomBlock::Shell
    } else if is_door(block) {
        RoomBlock::Door
    } else {
        RoomBlock::Open
    }
}

/// Закрытая камера: внутренние клетки (по порядку), их коробка и сколько
/// дверей ведут в шлюзы.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Room {
    pub cells: Vec<[i32; 3]>,
    pub min: [i32; 3],
    pub max: [i32; 3],
    pub airlocks: u32,
}

impl Room {
    /// Размер по X, в высоту, по Z.
    pub fn size(&self) -> [i32; 3] {
        span(self.min, self.max)
    }
}

/// Почему камера не закрыта.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomFault {
    /// Перед фильтром и за ним — оболочка: комнаты нет.
    NoRoom,
    /// Дыра наружу: в этой точке.
    Hole([i32; 3]),
    /// Комната закрыта, но больше 9×9×5: её размер.
    TooBig([i32; 3]),
    /// Дверь ведёт прямо наружу, без второй.
    OpenDoor([i32; 3]),
    /// Ни одного шлюза.
    NoAirlock,
}

impl RoomFault {
    /// Номер нужды в `QUARANTINE_NEEDS`.
    pub fn need(self) -> usize {
        match self {
            RoomFault::Hole(_) => 0,
            RoomFault::NoAirlock => 1,
            RoomFault::TooBig(_) => 2,
            RoomFault::OpenDoor(_) => 3,
            RoomFault::NoRoom => 4,
        }
    }

    /// Точка, о которой речь (дыра, дверь), или размер комнаты.
    pub fn spot(self) -> [i32; 3] {
        match self {
            RoomFault::Hole(p) | RoomFault::TooBig(p) | RoomFault::OpenDoor(p) => p,
            RoomFault::NoRoom | RoomFault::NoAirlock => [0; 3],
        }
    }
}

/// Нужды камеры для каркаса: номер — `RoomFault::need`. Слова — для общего
/// текста нехватки («Нет шлюза»); экран камеры пишет свои
/// (`quarantine_fault_text`), с точкой дыры.
pub const QUARANTINE_NEEDS: [Need; 5] = [
    Need { parts: &[], one: "стены без дыр", many: "стен без дыр" },
    Need { parts: &[], one: "шлюза", many: "шлюзов" },
    Need { parts: &[], one: "комнаты до 9×9×5", many: "комнат до 9×9×5" },
    Need { parts: &[], one: "второй двери шлюза", many: "вторых дверей шлюза" },
    Need { parts: &[], one: "комнаты перед фильтром", many: "комнат перед фильтром" },
];

const ROOM_DIRS: [[i32; 3]; 6] = [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1], [0, 1, 0], [0, -1, 0]];
const ROOM_SIDES: [[i32; 3]; 4] = [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]];

fn add3(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn span(min: [i32; 3], max: [i32; 3]) -> [i32; 3] {
    [max[0] - min[0] + 1, max[1] - min[1] + 1, max[2] - min[2] + 1]
}

/// Блоки, прочитанные проверкой: каждая точка читается один раз.
struct Survey<'a> {
    at: BlockAt<'a>,
    seen: std::collections::HashMap<[i32; 3], RoomBlock>,
    /// Где оболочка — стекло установки (герметичность, «M9: лаборатория»).
    glass: std::collections::HashSet<[i32; 3]>,
}

impl Survey<'_> {
    fn get(&mut self, p: [i32; 3]) -> RoomBlock {
        if let Some(b) = self.seen.get(&p) {
            return *b;
        }
        let id = (self.at)(p);
        let b = room_block(&id);
        if id == CASING_GLASS {
            self.glass.insert(p);
        }
        self.seen.insert(p, b);
        b
    }
}

/// Предел заливки: коробка (комната) или размах (тамбур).
#[derive(Clone, Copy)]
enum Bound {
    Box([i32; 3], [i32; 3]),
    Span([i32; 3]),
}

/// Итог заливки: клетки по порядку обхода, откуда в каждую пришли, их
/// коробка и коробка увиденной оболочки; `breach` — клетка, из которой
/// заливка упёрлась в предел (тогда она не закрыта).
struct Flood {
    cells: Vec<[i32; 3]>,
    set: std::collections::HashSet<[i32; 3]>,
    parent: std::collections::HashMap<[i32; 3], [i32; 3]>,
    min: [i32; 3],
    max: [i32; 3],
    shell: Option<([i32; 3], [i32; 3])>,
    breach: Option<[i32; 3]>,
}

/// Заливка от `start` через всё, кроме оболочки, дверей и `walls`.
fn flood(survey: &mut Survey, start: [i32; 3], bound: Bound, walls: &std::collections::HashSet<[i32; 3]>) -> Flood {
    let mut f = Flood {
        cells: vec![start],
        set: std::collections::HashSet::from([start]),
        parent: std::collections::HashMap::new(),
        min: start,
        max: start,
        shell: None,
        breach: None,
    };
    let mut next = 0;
    while next < f.cells.len() {
        let c = f.cells[next];
        next += 1;
        for d in ROOM_DIRS {
            let n = add3(c, d);
            if f.set.contains(&n) || walls.contains(&n) {
                continue;
            }
            let fits = match bound {
                Bound::Box(lo, hi) => (0..3).all(|i| lo[i] <= n[i] && n[i] <= hi[i]),
                Bound::Span(limit) => (0..3).all(|i| n[i].max(f.max[i]) - n[i].min(f.min[i]) < limit[i]),
            };
            if survey.get(n) != RoomBlock::Open {
                let (lo, hi) = f.shell.unwrap_or((n, n));
                f.shell = Some((
                    [lo[0].min(n[0]), lo[1].min(n[1]), lo[2].min(n[2])],
                    [hi[0].max(n[0]), hi[1].max(n[1]), hi[2].max(n[2])],
                ));
                continue;
            }
            if !fits {
                f.breach = Some(c);
                return f;
            }
            for i in 0..3 {
                f.min[i] = f.min[i].min(n[i]);
                f.max[i] = f.max[i].max(n[i]);
            }
            f.set.insert(n);
            f.parent.insert(n, c);
            f.cells.push(n);
        }
    }
    f
}

/// Коробка заливки комнаты: старт ± предел комнаты. В неё влезает любая
/// допустимая комната со стартом внутри и ещё клетка — чтобы комнату на
/// клетку больше предела назвать большой, а не дырявой.
fn room_box(start: [i32; 3]) -> Bound {
    Bound::Box([start[0] - ROOM_MAX[0], start[1] - ROOM_MAX[1], start[2] - ROOM_MAX[2]], add3(start, ROOM_MAX))
}

/// Дыра в незакрытой заливке: (нашлась ли пробкой точно, где).
fn locate_hole(survey: &mut Survey, start: [i32; 3], f: &Flood) -> (bool, [i32; 3]) {
    let Some(end) = f.breach else {
        return (false, start);
    };
    let mut path = vec![end];
    while let Some(p) = f.parent.get(path.last().unwrap_or(&start)) {
        path.push(*p);
    }
    // `path` — от края к старту. Сам старт не пробуем: заткнуть его — это
    // убрать комнату, а не дыру. Закрывают комнату и дыра, и клетка сразу за
    // ней (если за дырой одна свободная клетка), и узкий проход внутри
    // комнаты; дыра из них — та, что глубже всех сидит в оболочке (больше
    // соседей-стен), при равенстве — дальше от старта.
    let mut best: Option<(usize, [i32; 3])> = None;
    for p in path.iter().filter(|p| **p != start) {
        let plug = std::collections::HashSet::from([*p]);
        if flood(survey, start, room_box(start), &plug).breach.is_some() {
            continue;
        }
        let walls = ROOM_DIRS.iter().filter(|d| survey.get(add3(*p, **d)) != RoomBlock::Open).count();
        if best.is_none_or(|(w, _)| walls > w) {
            best = Some((walls, *p));
        }
    }
    if let Some((_, p)) = best {
        return (true, p);
    }
    path.reverse();
    let outside = |p: &[i32; 3]| match f.shell {
        Some((lo, hi)) => (0..3).any(|i| p[i] <= lo[i] || p[i] >= hi[i]),
        None => true,
    };
    let at = path.iter().skip(1).find(|p| outside(p)).copied().unwrap_or(end);
    (false, at)
}

/// Нижняя клетка двери (дверь в две клетки высотой).
fn door_base(survey: &mut Survey, mut p: [i32; 3]) -> [i32; 3] {
    for _ in 0..2 {
        let below = [p[0], p[1] - 1, p[2]];
        if survey.get(below) != RoomBlock::Door {
            break;
        }
        p = below;
    }
    p
}

/// Шлюзы закрытой комнаты: сколько дверей ведут в шлюз, или дверь прямо
/// наружу (первая по обходу).
fn room_airlocks(survey: &mut Survey, cells: &[[i32; 3]], room: &std::collections::HashSet<[i32; 3]>) -> Result<u32, [i32; 3]> {
    let (locks, open) = room_doors(survey, cells, room);
    match open.first() {
        Some(door) => Err(*door),
        None => Ok(locks),
    }
}

/// Двери закрытой комнаты: сколько ведут в шлюз и какие — прямо наружу
/// (нижние клетки, по порядку обхода).
fn room_doors(survey: &mut Survey, cells: &[[i32; 3]], room: &std::collections::HashSet<[i32; 3]>) -> (u32, Vec<[i32; 3]>) {
    let mut doors: Vec<([i32; 3], [i32; 3])> = Vec::new();
    for &c in cells {
        for d in ROOM_SIDES {
            let n = add3(c, d);
            if survey.get(n) == RoomBlock::Door && !doors.iter().any(|(p, _)| *p == n) {
                doors.push((n, d));
            }
        }
    }
    let mut locks: Vec<[i32; 3]> = Vec::new();
    let mut open: Vec<[i32; 3]> = Vec::new();
    for (door, d) in doors {
        let beyond = add3(door, d);
        if room.contains(&beyond) {
            continue;
        }
        let lock = match survey.get(beyond) {
            RoomBlock::Shell => false,
            // Две двери подряд: за второй — не стена и не та же комната.
            RoomBlock::Door => {
                let out = add3(beyond, d);
                !room.contains(&out) && survey.get(out) != RoomBlock::Shell
            }
            // Тамбур: закрытый и со своей второй дверью наружу.
            RoomBlock::Open => {
                let hall = flood(survey, beyond, Bound::Span(VESTIBULE_MAX), room);
                if hall.breach.is_some() {
                    let base = door_base(survey, door);
                    if !open.contains(&base) {
                        open.push(base);
                    }
                    continue;
                }
                let own = door_base(survey, door);
                let mut exit = false;
                for &v in &hall.cells {
                    for e in ROOM_SIDES {
                        let m = add3(v, e);
                        if survey.get(m) != RoomBlock::Door || door_base(survey, m) == own {
                            continue;
                        }
                        let out = add3(m, e);
                        if !room.contains(&out) && !hall.set.contains(&out) && survey.get(out) != RoomBlock::Shell {
                            exit = true;
                        }
                    }
                }
                exit
            }
        };
        let base = door_base(survey, door);
        if lock && !locks.contains(&base) {
            locks.push(base);
        }
    }
    (locks.len() as u32, open)
}

/// Заливка закрылась: размер и шлюзы.
fn finish_room(survey: &mut Survey, f: Flood) -> Result<Room, RoomFault> {
    let size = span(f.min, f.max);
    if (0..3).any(|i| size[i] > ROOM_MAX[i]) {
        return Err(RoomFault::TooBig(size));
    }
    let mut cells = f.cells;
    cells.sort();
    let airlocks = room_airlocks(survey, &cells, &f.set).map_err(RoomFault::OpenDoor)?;
    if airlocks == 0 {
        return Err(RoomFault::NoAirlock);
    }
    Ok(Room { cells, min: f.min, max: f.max, airlocks })
}

/// Проверка камеры с фильтром в `site`: заливка изнутри. Пробуется клетка
/// перед лицом фильтра, потом за ним; закрылась одна — это комната. Не
/// закрылась ни одна — дыра той, где её нашла пробка (иначе — перед лицом).
pub fn check_room(site: Site, at: BlockAt) -> Result<Room, RoomFault> {
    let mut survey = Survey { at, seen: std::collections::HashMap::new(), glass: std::collections::HashSet::new() };
    let front = place(site.origin, site.facing, [0, 0, -1]);
    let back = place(site.origin, site.facing, [0, 0, 1]);
    let mut holes: Vec<(bool, [i32; 3])> = Vec::new();
    for start in [front, back] {
        if survey.get(start) != RoomBlock::Open {
            continue;
        }
        let f = flood(&mut survey, start, room_box(start), &std::collections::HashSet::new());
        if f.breach.is_none() {
            return finish_room(&mut survey, f);
        }
        holes.push(locate_hole(&mut survey, start, &f));
    }
    let hole = holes.iter().find(|(exact, _)| *exact).or(holes.first());
    match hole {
        Some((_, p)) => Err(RoomFault::Hole(*p)),
        None => Err(RoomFault::NoRoom),
    }
}

/// Итог проверки камеры для каркаса: собрана (секции — число клеток
/// комнаты; частей для склейки нет: панели не склеиваются) или одна строка
/// нехватки.
pub fn room_verdict(result: &Result<Room, RoomFault>) -> Verdict {
    match result {
        Ok(room) => Verdict::Formed(Assembly { sections: room.cells.len() as u32, ..Default::default() }),
        Err(fault) => {
            Verdict::Broken(vec![Shortage { need: fault.need(), count: 1, tier: Tier::None, side: Side::None }])
        }
    }
}

/// Последние проверки камер по месту фильтра: их подробности (клетки,
/// точка дыры) в `Verdict` не помещаются. Кладёт `quarantine_check`, сразу
/// забирает работа фильтра в том же тике (`take_room_check`).
type RoomChecks = std::collections::HashMap<[i32; 3], Result<Sealed, RoomFault>>;

fn room_checks() -> &'static std::sync::Mutex<RoomChecks> {
    static CHECKS: std::sync::OnceLock<std::sync::Mutex<RoomChecks>> = std::sync::OnceLock::new();
    CHECKS.get_or_init(Default::default)
}

/// Проверка камеры для `Check::Custom`. С M9 пробоины, дверь без шлюза и
/// отсутствие шлюза камеру не рушат, а делают её негерметичной
/// (`seal_room`, раздел «M9: лаборатория»).
pub fn quarantine_check(_mb: &Multiblock, site: Site, at: BlockAt) -> Verdict {
    let result = seal_room(site, at);
    let verdict = seal_verdict(&result);
    if let Ok(mut checks) = room_checks().lock() {
        checks.insert(site.origin, result);
    }
    verdict
}

/// Подробности проверки камеры, сделанной в этом тике; None — проверки не
/// было.
pub fn take_room_check(origin: [i32; 3]) -> Option<Result<Sealed, RoomFault>> {
    room_checks().lock().ok()?.remove(&origin)
}

/// Слоты фильтра: один — патрон.
pub const QUARANTINE_SLOTS: usize = 1;
pub const QUARANTINE_SLOT_ROLES: [SlotRole; QUARANTINE_SLOTS] = [SlotRole::Input];

/// Строка установки для `MULTIBLOCKS`.
pub const QUARANTINE_MULTIBLOCK: Multiblock = Multiblock {
    controller: QUARANTINE,
    title: "Карантинная камера",
    needs: &QUARANTINE_NEEDS,
    check: Check::Custom(quarantine_check),
    slots: &QUARANTINE_SLOT_ROLES,
};

// Фильтрующий патрон: прочность — ванильный износ предмета
// (`minecraft:max_damage` в его файле, тест сверяет). Держит камера — патрон
// изнашивается на единицу раз в `FILTER_WEAR_EVERY` тиков: 120 × 10 с — 20
// минут, игровые сутки. Износился — пропадает; без патрона камера собрана,
// но не держит.

/// Прочность патрона.
pub const FILTER_DURABILITY: i64 = 120;
/// Раз в сколько тиков держащая камера изнашивает патрон.
pub const FILTER_WEAR_EVERY: i64 = 200;

/// Сколько прочности осталось у патрона в слоте; None — патрона нет (или
/// в слоте не патрон, или он износился).
pub fn filter_left(item: Option<(&str, i64)>) -> Option<i64> {
    let (id, damage) = item?;
    (id == FILTER_CARTRIDGE && damage < FILTER_DURABILITY).then(|| FILTER_DURABILITY - damage.max(0))
}

/// Пора ли изнашивать патрон: раз в `FILTER_WEAR_EVERY`, разнесено по месту.
pub fn filter_wear_due(ticks: i64, pos: [i32; 3]) -> bool {
    let spread = (pos[0] as i64 * 11 + pos[2] as i64 * 5 + pos[1] as i64).rem_euclid(FILTER_WEAR_EVERY);
    (ticks + spread).rem_euclid(FILTER_WEAR_EVERY) == 0
}

/// Остаток патрона в процентах, вверх: живой патрон — хотя бы 1 %.
pub fn filter_percent(left: i64) -> i32 {
    ((left.clamp(0, FILTER_DURABILITY) * 100 + FILTER_DURABILITY - 1) / FILTER_DURABILITY) as i32
}

// Свои числа экрана фильтра (после общих `DATA_MB_*`). Точка дыры или двери
// — мировые координаты, а слот данных 16-битный: каждая — двумя числами.

/// Точка нехватки: x, y, z — по два числа (старшие и младшие 16 бит).
pub const DATA_QR_SPOT: usize = DATA_MB_END;
/// Размер комнаты по X, в высоту, по Z (собранной или слишком большой).
pub const DATA_QR_SIZE: usize = DATA_QR_SPOT + 6;
/// Остаток патрона, %; −1 — патрона нет.
pub const DATA_QR_FILTER: usize = DATA_QR_SIZE + 3;
/// Держит ли камера сейчас: 1 — да.
pub const DATA_QR_HOLDS: usize = DATA_QR_FILTER + 1;
/// Герметичность, % (GDD 4.23 п. 11, «M9: лаборатория»).
pub const DATA_QR_SEAL: usize = DATA_QR_HOLDS + 1;
/// Пробоин, дверей без шлюза, доля стекла в оболочке (%), шлюзов.
pub const DATA_QR_HOLES: usize = DATA_QR_SEAL + 1;
pub const DATA_QR_DOORS: usize = DATA_QR_HOLES + 1;
pub const DATA_QR_GLASS: usize = DATA_QR_DOORS + 1;
pub const DATA_QR_LOCKS: usize = DATA_QR_GLASS + 1;
pub const DATA_QR_END: usize = DATA_QR_LOCKS + 1;

/// Координата двумя 16-битными числами — какими их увидит экран.
pub fn split_coord(v: i32) -> [i32; 2] {
    [(v >> 16) as i16 as i32, v as i16 as i32]
}

pub fn join_coord(hi: i32, lo: i32) -> i32 {
    (hi << 16) | (lo as u16 as i32)
}

/// Записать подробности проверки в числа экрана (`data` длиной не меньше
/// `DATA_QR_END`).
pub fn write_room(data: &mut [i32], result: &Result<Sealed, RoomFault>) {
    if data.len() < DATA_QR_END {
        return;
    }
    // Собранная негерметичная камера показывает первую пробоину, а нет —
    // первую дверь без шлюза.
    let (spot, size) = match result {
        Ok(s) => (s.holes.first().or(s.open_doors.first()).copied().unwrap_or([0; 3]), s.room.size()),
        Err(RoomFault::TooBig(size)) => ([0; 3], *size),
        Err(fault) => (fault.spot(), [0; 3]),
    };
    let (holes, doors, glass, locks) = match result {
        Ok(s) => (
            s.holes.len() as i32,
            s.open_doors.len() as i32,
            if s.shell == 0 { 0 } else { (s.glass * 100 / s.shell) as i32 },
            s.room.airlocks as i32,
        ),
        Err(_) => (0, 0, 0, 0),
    };
    data[DATA_QR_HOLES] = holes;
    data[DATA_QR_DOORS] = doors;
    data[DATA_QR_GLASS] = glass;
    data[DATA_QR_LOCKS] = locks;
    for i in 0..3 {
        let [hi, lo] = split_coord(spot[i]);
        data[DATA_QR_SPOT + 2 * i] = hi;
        data[DATA_QR_SPOT + 2 * i + 1] = lo;
        data[DATA_QR_SIZE + i] = size[i];
    }
}

/// Записать герметичность в числа экрана.
pub fn write_seal(data: &mut [i32], f: &SealFactors) {
    if data.len() < DATA_QR_END {
        return;
    }
    data[DATA_QR_SEAL] = seal_percent(f);
}

/// Записать патрон и «держит ли» в числа экрана.
pub fn write_filter(data: &mut [i32], left: Option<i64>, holds: bool) {
    if data.len() < DATA_QR_END {
        return;
    }
    data[DATA_QR_FILTER] = left.map_or(-1, filter_percent);
    data[DATA_QR_HOLDS] = i32::from(holds);
}

/// Точка нехватки с экрана.
pub fn room_spot(data: &[i32]) -> [i32; 3] {
    std::array::from_fn(|i| {
        join_coord(screen_datum(data, DATA_QR_SPOT + 2 * i, 0), screen_datum(data, DATA_QR_SPOT + 2 * i + 1, 0))
    })
}

/// Размер комнаты с экрана.
pub fn room_size(data: &[i32]) -> [i32; 3] {
    std::array::from_fn(|i| screen_datum(data, DATA_QR_SIZE + i, 0))
}

/// «9×5×9» — по X, в высоту, по Z, как ширина × высота × глубина.
pub fn size_text(size: [i32; 3]) -> String {
    format!("{}×{}×{}", size[0], size[1], size[2])
}

/// Строка нехватки камеры: с точкой, где она.
pub fn quarantine_fault_text(need: usize, spot: [i32; 3]) -> String {
    let [x, y, z] = spot;
    match need {
        0 => format!("Дыра в стене: {x} {y} {z}"),
        1 => "Нет шлюза".to_string(),
        2 => "Комната больше 9×9×5".to_string(),
        3 => format!("Дверь без шлюза: {x} {y} {z}"),
        _ => "Фильтр не выходит в комнату".to_string(),
    }
}

/// Подсказка к нехватке: что с ней делать.
pub fn quarantine_fault_hint(need: usize) -> &'static str {
    match need {
        0 => "Закройте её чистой панелью или стеклом установки",
        1 => "Нужны две двери подряд или дверь, тамбур и дверь",
        2 => "Внутри не больше 9 в ширину и глубину и 5 в высоту",
        3 => "За дверью — вторая дверь или тамбур со второй",
        _ => "Поставьте фильтр в стену комнаты",
    }
}

/// Строка о патроне.
pub fn filter_line(data: &[i32]) -> String {
    match screen_datum(data, DATA_QR_FILTER, -1) {
        p if p < 0 => "Нет патрона: камера течёт".to_string(),
        p => format!("Патрон: {p}%"),
    }
}

/// Табло фильтра: заголовок, его цвет и строки.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuarantineBoard {
    pub head: &'static str,
    pub ink: u32,
    pub lines: Vec<String>,
}

pub const BOARD_GREEN: u32 = 0x5A_E8B4;
pub const BOARD_RED: u32 = 0xE8_5A4A;
pub const BOARD_AMBER: u32 = 0xE0_A040;

pub fn quarantine_board(data: &[i32]) -> QuarantineBoard {
    match multiblock_shown(data) {
        MultiblockShown::Unknown => {
            QuarantineBoard { head: "Проверка комнаты…", ink: SCREEN_TEXT_DIM, lines: Vec::new() }
        }
        MultiblockShown::Formed { sections, .. } => {
            let room = format!("Комната {}, {sections} {}", size_text(room_size(data)), plural(sections, "клетка", "клетки", "клеток"));
            let seal = format!("Герметичность {}%", screen_datum(data, DATA_QR_SEAL, 100));
            if screen_datum(data, DATA_QR_HOLDS, 0) > 0 {
                QuarantineBoard { head: "Камера держит", ink: BOARD_GREEN, lines: vec![room, seal, filter_line(data)] }
            } else {
                // Подтекает (GDD 4.23 п. 11): работает, но не герметична.
                let mut lines = vec![seal];
                lines.extend(leak_reason(data));
                lines.push(filter_line(data));
                QuarantineBoard { head: "Подтекает", ink: BOARD_AMBER, lines }
            }
        }
        MultiblockShown::Broken { rows, .. } => QuarantineBoard {
            head: "Не закрыта",
            ink: BOARD_RED,
            lines: rows.first().map(|s| quarantine_fault_text(s.need, room_spot(data))).into_iter().collect(),
        },
    }
}

/// Подсказка над табло фильтра.
pub fn quarantine_tooltip(data: &[i32]) -> Vec<String> {
    let board = quarantine_board(data);
    let mut lines = vec![format!("Карантинная камера: {}", board.head.to_lowercase())];
    match multiblock_shown(data) {
        MultiblockShown::Broken { rows, .. } => {
            if let Some(s) = rows.first() {
                lines.push(format!("§c{}", quarantine_fault_text(s.need, room_spot(data))));
                if s.need == 2 {
                    lines.push(format!("§cСейчас: {}", size_text(room_size(data))));
                }
                lines.push(format!("§7{}", quarantine_fault_hint(s.need)));
            }
            lines.push("§7Схема — в журнале лаборанта".to_string());
        }
        MultiblockShown::Formed { .. } => {
            lines.extend(board.lines);
            lines.extend(seal_lines(data));
            if screen_datum(data, DATA_QR_HOLDS, 0) > 0 {
                lines.push("§7Внутри почти нельзя заразиться".to_string());
            } else if screen_datum(data, DATA_QR_HOLES, 0) > 0 {
                lines.push(format!("§7{}", quarantine_fault_hint(0)));
            } else if screen_datum(data, DATA_QR_DOORS, 0) > 0 {
                lines.push(format!("§7{}", quarantine_fault_hint(3)));
            } else if screen_datum(data, DATA_QR_FILTER, -1) < 0 {
                lines.push("§7Вставьте фильтрующий патрон: уголь и шерсть".to_string());
            } else {
                lines.push("§7Смените патрон или замените стекло панелями".to_string());
            }
        }
        MultiblockShown::Unknown => {}
    }
    lines
}

/// Держит ли камера: собрана и патрон живой.
pub fn room_holds(formed: bool, left: Option<i64>) -> bool {
    formed && left.is_some_and(|l| l > 0)
}

/// Через сколько миллисекунд без отметки фильтра запись реестра не в счёт:
/// фильтр отмечается каждый тик, проверяет комнату раз в секунду.
pub const ROOM_STALE_MS: u64 = 5_000;

/// Камера в реестре.
struct HeldRoom {
    dimension: String,
    filter: [i32; 3],
    min: [i32; 3],
    max: [i32; 3],
    cells: std::collections::HashSet<[i32; 3]>,
    seen: u64,
    /// Доля утечки камеры 0–1 (GDD 4.23 п. 11).
    leak: f32,
}

/// Реестр держащих камер. Время — миллисекунды монотонных часов (`now`),
/// чтобы правило проверялось тестами.
#[derive(Default)]
pub struct QuarantineRooms {
    rooms: Vec<HeldRoom>,
}

impl QuarantineRooms {
    /// Камера фильтра `filter` держит комнату `room` (после проверки).
    pub fn hold(&mut self, dimension: &str, filter: [i32; 3], room: &Room, leak: f32, now: u64) {
        self.rooms.retain(|r| !(r.dimension == dimension && r.filter == filter) && now.saturating_sub(r.seen) < 12 * ROOM_STALE_MS);
        self.rooms.push(HeldRoom {
            dimension: dimension.to_string(),
            filter,
            min: room.min,
            max: room.max,
            cells: room.cells.iter().copied().collect(),
            seen: now,
            leak: leak.clamp(0.0, 1.0),
        });
    }

    /// Фильтр держит дальше (между проверками). false — записи нет: её надо
    /// положить проверкой.
    pub fn touch(&mut self, dimension: &str, filter: [i32; 3], now: u64) -> bool {
        match self.rooms.iter_mut().find(|r| r.dimension == dimension && r.filter == filter) {
            Some(r) => {
                r.seen = now;
                true
            }
            None => false,
        }
    }

    /// Камера больше не держит.
    pub fn forget(&mut self, dimension: &str, filter: [i32; 3]) {
        self.rooms.retain(|r| !(r.dimension == dimension && r.filter == filter));
    }

    /// Точка внутри собранной камеры (любой герметичности).
    pub fn contains(&self, dimension: &str, pos: [i32; 3], now: u64) -> bool {
        self.rooms.iter().any(|r| r.has(dimension, pos, now))
    }

    /// Доля утечки в точке: самая герметичная камера, где она лежит; вне
    /// камер — 1.
    pub fn leak_at(&self, dimension: &str, pos: [i32; 3], now: u64) -> f32 {
        self.rooms.iter().filter(|r| r.has(dimension, pos, now)).map(|r| r.leak).fold(1.0, f32::min)
    }
}

impl HeldRoom {
    fn has(&self, dimension: &str, pos: [i32; 3], now: u64) -> bool {
        self.dimension == dimension
            && now.saturating_sub(self.seen) < ROOM_STALE_MS
            && (0..3).all(|i| self.min[i] <= pos[i] && pos[i] <= self.max[i])
            && self.cells.contains(&pos)
    }
}

fn quarantine_rooms() -> &'static std::sync::Mutex<QuarantineRooms> {
    static ROOMS: std::sync::OnceLock<std::sync::Mutex<QuarantineRooms>> = std::sync::OnceLock::new();
    ROOMS.get_or_init(Default::default)
}

/// Монотонные часы реестров, мс.
fn quarantine_clock() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64
}

/// Фильтр в `filter` держит комнату `room` (после проверки) с долей утечки
/// `leak`.
pub fn hold_room(dimension: &str, filter: [i32; 3], room: &Room, leak: f32) {
    if let Ok(mut rooms) = quarantine_rooms().lock() {
        rooms.hold(dimension, filter, room, leak, quarantine_clock());
    }
}

/// Фильтр держит дальше; false — записи нет.
pub fn touch_room(dimension: &str, filter: [i32; 3]) -> bool {
    quarantine_rooms().lock().is_ok_and(|mut rooms| rooms.touch(dimension, filter, quarantine_clock()))
}

/// Камера фильтра больше не держит: разобрана, без патрона, сломан фильтр.
pub fn forget_room(dimension: &str, filter: [i32; 3]) {
    if let Ok(mut rooms) = quarantine_rooms().lock() {
        rooms.forget(dimension, filter);
    }
}

/// Точка `at` (позиция сущности) внутри держащей камеры: по клетке ног.
pub fn in_quarantine_at(dimension: &str, at: [f64; 3]) -> bool {
    in_quarantine(dimension, [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32])
}

// Утечка тяжёлого штамма вне карантина (GDD 4.18 п. 4, 4.10 «Утечка»).

/// С какой тяжести штамм утекает.
pub const LEAK_SEVERITY: i32 = 4;
/// Раз в сколько тиков работающая машина с тяжёлым штаммом даёт утечку.
pub const LEAK_EVERY: i64 = 200;
/// Радиус утечки, блоков.
pub const LEAK_RADIUS: f64 = 6.0;
/// Насколько утечка поднимает заражение игрока за шаг болезни: больше, чем
/// близость к зомби (`CONTACT_STEP`).
pub const LEAK_STEP: u32 = 4;
/// Сколько держится отметка утечки: до следующей и ещё секунда.
pub const LEAK_HOLD_MS: u64 = (LEAK_EVERY as u64) * 50 + 1_000;
/// Что слышит игрок, заразившийся от утечки.
pub const LEAK_MESSAGE: &str = "Утечка: тяжёлый штамм вне карантина — вы заражены";

/// Штамм, который утекает из машины: самый тяжёлый с тяжестью от 4 среди
/// культур во входах, если машина работает и стоит не в камере.
pub fn leak_strain(cultures: &[Strain], working: bool, quarantined: bool) -> Option<Strain> {
    if !working || quarantined {
        return None;
    }
    cultures
        .iter()
        .filter(|s| s.severity >= LEAK_SEVERITY)
        .max_by_key(|s| (s.severity, s.infectivity))
        .copied()
}

/// Пора ли машине давать утечку: раз в `LEAK_EVERY`, разнесено по месту.
pub fn leak_due(ticks: i64, pos: [i32; 3]) -> bool {
    let spread = (pos[0] as i64 * 3 + pos[2] as i64 * 17 + pos[1] as i64 * 7).rem_euclid(LEAK_EVERY);
    (ticks + spread).rem_euclid(LEAK_EVERY) == 0
}

/// Заразится ли моб от утечки: шанс 10 % за единицу заразности.
pub fn leak_catches(strain: &Strain, seed: u64) -> bool {
    seed % 100 < 10 * strain.infectivity.clamp(0, 5) as u64
}

/// В радиусе ли точка.
pub fn within(a: [f64; 3], b: [f64; 3], radius: f64) -> bool {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= radius * radius
}

/// Отметки утечек: где, каким штаммом и до какого часа.
#[derive(Default)]
pub struct Leaks {
    list: Vec<(String, [f64; 3], Strain, u64)>,
}

impl Leaks {
    pub fn note(&mut self, dimension: &str, at: [f64; 3], strain: Strain, now: u64) {
        self.list.retain(|(d, p, _, until)| *until > now && !(d == dimension && *p == at));
        self.list.push((dimension.to_string(), at, strain, now + LEAK_HOLD_MS));
    }

    /// Самый тяжёлый штамм живых утечек в радиусе от точки.
    pub fn near(&self, dimension: &str, at: [f64; 3], now: u64) -> Option<Strain> {
        self.list
            .iter()
            .filter(|(d, p, _, until)| d == dimension && *until > now && within(*p, at, LEAK_RADIUS))
            .map(|(_, _, s, _)| *s)
            .max_by_key(|s| (s.severity, s.infectivity))
    }
}

fn leaks() -> &'static std::sync::Mutex<Leaks> {
    static LEAKS: std::sync::OnceLock<std::sync::Mutex<Leaks>> = std::sync::OnceLock::new();
    LEAKS.get_or_init(Default::default)
}

/// Отметить утечку из машины в точке `at`.
pub fn note_leak(dimension: &str, at: [f64; 3], strain: Strain) {
    if let Ok(mut list) = leaks().lock() {
        list.note(dimension, at, strain, quarantine_clock());
    }
}

/// Штамм утечки рядом с точкой `at`, если она не в камере.
pub fn leak_near(dimension: &str, at: [f64; 3]) -> Option<Strain> {
    if in_quarantine_at(dimension, at) {
        return None;
    }
    leaks().lock().ok()?.near(dimension, at, quarantine_clock())
}

#[cfg(test)]
mod quarantine_tests {
    use super::*;

    type World = std::collections::HashMap<[i32; 3], &'static str>;

    /// Комната: внутри `lo..=hi`, оболочка из панелей на клетку вокруг
    /// (рёбра и углы тоже).
    fn shell(lo: [i32; 3], hi: [i32; 3]) -> World {
        let mut w = World::new();
        for x in lo[0] - 1..=hi[0] + 1 {
            for y in lo[1] - 1..=hi[1] + 1 {
                for z in lo[2] - 1..=hi[2] + 1 {
                    let inside = (0..3).all(|i| lo[i] <= [x, y, z][i] && [x, y, z][i] <= hi[i]);
                    if !inside {
                        w.insert([x, y, z], CLEAN_PANEL);
                    }
                }
            }
        }
        w
    }

    /// Дверь в две клетки.
    fn door(w: &mut World, at: [i32; 3]) {
        w.insert(at, "minecraft:oak_door");
        w.insert([at[0], at[1] + 1, at[2]], "minecraft:oak_door");
    }

    /// Комната 5×3×5 (x 0..4, y 64..66, z 0..4), фильтр в южной стене
    /// (z = 5) лицом внутрь (на север), шлюз — две двери подряд в западной
    /// стене (x = −1 и −2) у z = 2.
    fn lab() -> World {
        let mut w = shell([0, 64, 0], [4, 66, 4]);
        w.insert([2, 64, 5], QUARANTINE);
        door(&mut w, [-1, 64, 2]);
        door(&mut w, [-2, 64, 2]);
        w
    }

    const FILTER: Site = Site { origin: [2, 64, 5], facing: Facing::North };

    fn reader(w: &World) -> impl Fn([i32; 3]) -> String + '_ {
        move |p| w.get(&p).copied().unwrap_or("minecraft:air").to_string()
    }

    fn check(w: &World, site: Site) -> Result<Room, RoomFault> {
        check_room(site, &reader(w))
    }

    #[test]
    fn a_closed_room_with_an_airlock_is_formed() {
        let room = check(&lab(), FILTER).expect("камера собрана");
        assert_eq!(room.cells.len(), 75);
        assert_eq!(room.size(), [5, 3, 5]);
        assert_eq!(room.airlocks, 1, "одна дверь в две клетки — один шлюз");
        assert!(room.cells.contains(&[0, 64, 0]) && !room.cells.contains(&[-1, 64, 2]));
        // Машины, сундук и земля внутри заливке не мешают.
        let mut w = lab();
        w.insert([1, 64, 1], "biolab:incubator");
        w.insert([2, 64, 2], "minecraft:dirt");
        w.insert([3, 65, 3], "minecraft:chest");
        assert_eq!(check(&w, FILTER).map(|r| r.cells.len()), Ok(75));
    }

    #[test]
    fn the_filter_may_face_outwards() {
        // Фильтр лицом на юг — перед лицом улица, комната за ним.
        let outward = Site { facing: Facing::South, ..FILTER };
        assert_eq!(check(&lab(), outward).map(|r| r.cells.len()), Ok(75));
        // В стене меж двух панелей (лицом в стену) комнаты нет.
        let sideways = Site { facing: Facing::East, ..FILTER };
        assert_eq!(check(&lab(), sideways), Err(RoomFault::NoRoom));
    }

    #[test]
    fn a_hole_is_found_where_it_is() {
        for hole in [[4, 65, -1], [5, 64, 3], [0, 63, 0], [2, 67, 2], [-1, 66, 4]] {
            let mut w = lab();
            w.remove(&hole);
            assert_eq!(check(&w, FILTER), Err(RoomFault::Hole(hole)), "дыра {hole:?}");
            // И у фильтра лицом наружу — та же дыра.
            let outward = Site { facing: Facing::South, ..FILTER };
            assert_eq!(check(&w, outward), Err(RoomFault::Hole(hole)), "наружу, дыра {hole:?}");
        }
        // Пол из земли — дыра в полу, где-то под комнатой.
        let mut w = lab();
        for x in 0..=4 {
            for z in 0..=4 {
                w.insert([x, 63, z], "minecraft:dirt");
            }
        }
        let Err(RoomFault::Hole(p)) = check(&w, FILTER) else { panic!("пол из земли — не комната") };
        assert_eq!(p[1], 63, "дыра — в полу: {p:?}");
    }

    #[test]
    fn a_narrow_room_is_not_mistaken_for_a_hole() {
        // Г-образная комната: узкий проход в клетку между двумя частями;
        // дыра — в дальней части. Пробка в проходе тоже «закрыла» бы
        // комнату, но дыра ищется снаружи внутрь.
        let mut w = shell([0, 64, 0], [2, 65, 2]);
        for (p, b) in shell([6, 64, 0], [8, 65, 2]) {
            w.insert(p, b);
        }
        // Проход x 3..5 на z = 1, y 64..65.
        for x in 3..=5 {
            for y in 64..=65 {
                w.insert([x, y, 1], "minecraft:air");
                w.insert([x, y, 0], CLEAN_PANEL);
                w.insert([x, y, 2], CLEAN_PANEL);
            }
            w.insert([x, 63, 1], CLEAN_PANEL);
            w.insert([x, 66, 1], CLEAN_PANEL);
        }
        w.insert([1, 64, 3], QUARANTINE);
        door(&mut w, [-1, 64, 1]);
        door(&mut w, [-2, 64, 1]);
        let site = Site { origin: [1, 64, 3], facing: Facing::North };
        assert_eq!(check(&w, site).map(|r| r.cells.len()), Ok(9 * 2 + 3 * 2 + 9 * 2));
        w.remove(&[9, 64, 2]);
        assert_eq!(check(&w, site), Err(RoomFault::Hole([9, 64, 2])));
    }

    #[test]
    fn a_room_bigger_than_nine_by_nine_by_five_is_refused() {
        // 10 в ширину.
        let mut w = shell([0, 64, 0], [9, 66, 4]);
        w.insert([2, 64, 5], QUARANTINE);
        door(&mut w, [-1, 64, 2]);
        door(&mut w, [-2, 64, 2]);
        assert_eq!(check(&w, FILTER), Err(RoomFault::TooBig([10, 3, 5])));
        // Шесть в высоту.
        let mut w = shell([0, 64, 0], [4, 69, 4]);
        w.insert([2, 64, 5], QUARANTINE);
        door(&mut w, [-1, 64, 2]);
        door(&mut w, [-2, 64, 2]);
        assert_eq!(check(&w, FILTER), Err(RoomFault::TooBig([5, 6, 5])));
        // 9×5×9 — ровно предел.
        let mut w = shell([0, 64, 0], [8, 68, 8]);
        w.insert([2, 64, 9], QUARANTINE);
        door(&mut w, [-1, 64, 2]);
        door(&mut w, [-2, 64, 2]);
        let site = Site { origin: [2, 64, 9], facing: Facing::North };
        assert_eq!(check(&w, site).map(|r| r.cells.len()), Ok(405));
    }

    #[test]
    fn an_airlock_is_two_doors_in_a_row_or_a_hall_between() {
        // Без дверей — нет шлюза.
        let mut w = shell([0, 64, 0], [4, 66, 4]);
        w.insert([2, 64, 5], QUARANTINE);
        assert_eq!(check(&w, FILTER), Err(RoomFault::NoAirlock));
        // Одна дверь прямо на улицу.
        let mut one = w.clone();
        door(&mut one, [-1, 64, 2]);
        assert_eq!(check(&one, FILTER), Err(RoomFault::OpenDoor([-1, 64, 2])));
        // Дверь, тамбур 1×2×2 из панелей, вторая дверь.
        let mut hall = one.clone();
        for p in shell([-3, 64, 2], [-2, 65, 2]) {
            hall.entry(p.0).or_insert(p.1);
        }
        assert_eq!(check(&hall, FILTER), Err(RoomFault::NoAirlock), "чулан без второй двери — не шлюз");
        door(&mut hall, [-4, 64, 2]);
        assert_eq!(check(&hall, FILTER).map(|r| r.airlocks), Ok(1));
        // Вторая дверь из тамбура — в бок, тоже выход.
        hall.insert([-4, 64, 2], CLEAN_PANEL);
        hall.insert([-4, 65, 2], CLEAN_PANEL);
        door(&mut hall, [-2, 64, 3]);
        assert_eq!(check(&hall, FILTER).map(|r| r.airlocks), Ok(1));
        // Дверь за дверью, но за второй — панель: не выход.
        let mut dead = w.clone();
        door(&mut dead, [-1, 64, 2]);
        door(&mut dead, [-2, 64, 2]);
        dead.insert([-3, 64, 2], CLEAN_PANEL);
        dead.insert([-3, 65, 2], CLEAN_PANEL);
        assert_eq!(check(&dead, FILTER), Err(RoomFault::NoAirlock));
        // Шлюз есть, но вторая дверь — прямо наружу: камера не закрыта.
        let mut both = lab();
        door(&mut both, [5, 64, 2]);
        assert_eq!(check(&both, FILTER), Err(RoomFault::OpenDoor([5, 64, 2])));
        // Железная и медная двери — тоже двери, люк — нет.
        assert!(is_door("minecraft:iron_door") && is_door("minecraft:waxed_copper_door"));
        assert!(!is_door("minecraft:oak_trapdoor"));
    }

    #[test]
    fn the_check_reads_each_block_once_and_not_too_many() {
        let count = std::cell::Cell::new(0usize);
        let w = lab();
        let at = |p: [i32; 3]| {
            count.set(count.get() + 1);
            w.get(&p).copied().unwrap_or("minecraft:air").to_string()
        };
        assert!(check_room(FILTER, &at).is_ok());
        // Комната 75 клеток, оболочка вокруг (без рёбер) 110, шлюз.
        assert!(count.get() < 200, "закрытая 5×3×5: прочитано {}", count.get());
        // Фильтр посреди поля и в дырявой большой комнате — потолок.
        count.set(0);
        let field = World::from([([2, 64, 5], QUARANTINE)]);
        let at = |p: [i32; 3]| {
            count.set(count.get() + 1);
            field.get(&p).copied().unwrap_or("minecraft:air").to_string()
        };
        assert!(matches!(check_room(FILTER, &at), Err(RoomFault::Hole(_))));
        assert!(count.get() < 4_000, "в поле прочитано {}", count.get());
        count.set(0);
        let mut big = shell([0, 64, 0], [8, 68, 8]);
        big.insert([2, 64, 9], QUARANTINE);
        big.remove(&[9, 66, 4]);
        let at = |p: [i32; 3]| {
            count.set(count.get() + 1);
            big.get(&p).copied().unwrap_or("minecraft:air").to_string()
        };
        let site = Site { origin: [2, 64, 9], facing: Facing::North };
        assert_eq!(check_room(site, &at), Err(RoomFault::Hole([9, 66, 4])));
        assert!(count.get() < 4_000, "в дырявой 9×5×9 прочитано {}", count.get());
    }

    #[test]
    fn the_framework_gets_the_verdict_and_the_screen_the_details() {
        let mb = multiblock(QUARANTINE).expect("камера — установка");
        assert!(matches!(mb.check, Check::Custom(_)));
        let w = lab();
        let verdict = check_multiblock(mb, FILTER, &reader(&w));
        let Verdict::Formed(a) = &verdict else { panic!("не собрана") };
        assert_eq!(a.sections, 75);
        assert!(a.parts.is_empty() && a.ports.is_empty(), "панели не склеиваются, портов нет");
        let detail = take_room_check(FILTER.origin).expect("подробности");
        assert!(take_room_check(FILTER.origin).is_none(), "забираются один раз");
        let mut data = vec![0; machine_data_slots(QUARANTINE)];
        assert_eq!(data.len(), DATA_QR_END);
        write_verdict(&mut data, &verdict);
        write_room(&mut data, &detail);
        let sealed = detail.as_ref().expect("камера");
        write_seal(&mut data, &seal_factors(sealed, Some(96)));
        write_filter(&mut data, Some(96), true);
        let board = quarantine_board(&data);
        assert_eq!(board.head, "Камера держит");
        assert_eq!(board.lines, vec!["Комната 5×3×5, 75 клеток", "Герметичность 98%", "Патрон: 80%"]);
        // Без патрона камера собрана, но течёт (GDD 4.23 п. 11).
        write_seal(&mut data, &seal_factors(sealed, None));
        write_filter(&mut data, None, false);
        let board = quarantine_board(&data);
        assert_eq!(board.head, "Подтекает");
        assert_eq!(board.lines, vec!["Герметичность 20%", "Нет патрона: камера течёт"]);

        // Дыра далеко от нуля — координаты доходят до экрана целыми.
        let far = [-1_234_567, 70, 2_000_001];
        let mut w = shell(add3(far, [-2, 0, -2]), add3(far, [2, 2, 2]));
        let site = Site { origin: add3(far, [0, 0, 3]), facing: Facing::North };
        w.insert(site.origin, QUARANTINE);
        let hole = add3(far, [3, 1, 0]);
        w.remove(&hole);
        let verdict = check_multiblock(mb, site, &reader(&w));
        write_verdict(&mut data, &verdict);
        let detail = take_room_check(site.origin).unwrap();
        write_room(&mut data, &detail);
        write_seal(&mut data, &seal_factors(detail.as_ref().expect("с пробоиной — собрана"), Some(FILTER_DURABILITY)));
        write_filter(&mut data, Some(FILTER_DURABILITY), false);
        // Экран получает числа 16-битными.
        let wire: Vec<i32> = data.iter().map(|v| *v as i16 as i32).collect();
        let board = quarantine_board(&wire);
        assert_eq!(board.head, "Подтекает");
        assert_eq!(board.lines[1], format!("Пробоина: {} {} {}", hole[0], hole[1], hole[2]));
        let tip = quarantine_tooltip(&wire);
        assert!(tip.iter().any(|l| l.contains("чистой панелью")), "{tip:?}");
        // Дыр больше, чем камера терпит, — «Не закрыта», как раньше.
        for d in [[-3, 1, 0], [0, 1, -3], [0, -1, 0], [0, 3, 0], [-3, 0, 1]] {
            w.remove(&add3(far, d));
        }
        let verdict = check_multiblock(mb, site, &reader(&w));
        write_verdict(&mut data, &verdict);
        write_room(&mut data, &take_room_check(site.origin).unwrap());
        let wire: Vec<i32> = data.iter().map(|v| *v as i16 as i32).collect();
        let board = quarantine_board(&wire);
        assert_eq!(board.head, "Не закрыта");
        assert!(board.lines[0].starts_with("Дыра в стене: "), "{:?}", board.lines);
        for v in [0, 1, -1, 65_535, 65_536, -65_536, 29_999_999, -29_999_999] {
            let [hi, lo] = split_coord(v);
            assert_eq!(join_coord(hi as i16 as i32, lo as i16 as i32), v, "{v}");
        }
        // Общий текст нехватки тоже читается.
        let no_lock = Shortage { need: RoomFault::NoAirlock.need(), count: 1, tier: Tier::None, side: Side::None };
        assert_eq!(shortage_text(mb, &no_lock), "Нет шлюза");
        assert_eq!(quarantine_fault_text(2, [0; 3]), "Комната больше 9×9×5");
    }

    #[test]
    fn the_filter_wears_out() {
        assert_eq!(filter_left(Some((FILTER_CARTRIDGE, 0))), Some(FILTER_DURABILITY));
        assert_eq!(filter_left(Some((FILTER_CARTRIDGE, FILTER_DURABILITY - 1))), Some(1));
        assert_eq!(filter_left(Some((FILTER_CARTRIDGE, FILTER_DURABILITY))), None, "износился");
        assert_eq!(filter_left(Some(("minecraft:coal", 0))), None);
        assert_eq!(filter_left(None), None);
        assert!(room_holds(true, Some(1)) && !room_holds(false, Some(50)) && !room_holds(true, None));
        assert_eq!((filter_percent(FILTER_DURABILITY), filter_percent(1), filter_percent(0)), (100, 1, 0));
        let pos = [10, 64, -3];
        let due: Vec<i64> = (0..2 * FILTER_WEAR_EVERY).filter(|t| filter_wear_due(*t, pos)).collect();
        assert_eq!(due.len(), 2);
        assert_eq!(due[1] - due[0], FILTER_WEAR_EVERY);
        // Патрон живёт игровые сутки.
        assert_eq!(FILTER_DURABILITY * FILTER_WEAR_EVERY, 24_000);
    }

    /// Патрон в `content/`: прочность — та же, что считает фильтр, стопка 1.
    #[test]
    fn the_cartridge_in_content_wears_like_the_filter_counts() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/horizon");
        let text = std::fs::read_to_string(root.join("item/filter_cartridge.json")).expect("нет патрона");
        let json = Json::parse(&text).unwrap();
        let max = json.get("components").and_then(|c| c.get("minecraft:max_damage")).and_then(Json::num);
        assert_eq!(max, Some(FILTER_DURABILITY as f64));
        assert_eq!(json.get("max_stack_size").and_then(Json::num), Some(1.0));
        // Фильтр — на шаблоне печи (меню), панель — без своего свойства.
        let filter = Json::parse(&std::fs::read_to_string(root.join("block/quarantine.json")).unwrap()).unwrap();
        assert_eq!(filter.get("template").and_then(Json::str), Some("minecraft:furnace"));
        let panel = Json::parse(&std::fs::read_to_string(root.join("block/clean_panel.json")).unwrap()).unwrap();
        assert!(panel.get("state_properties").is_none() && !is_formable(CLEAN_PANEL));
    }

    #[test]
    fn the_registry_answers_where_it_is_clean() {
        let room = check(&lab(), FILTER).unwrap();
        let mut rooms = QuarantineRooms::default();
        let ow = "minecraft:overworld";
        assert!(!rooms.contains(ow, [2, 64, 2], 0));
        rooms.hold(ow, FILTER.origin, &room, 0.0, 1_000);
        assert!(rooms.contains(ow, [2, 64, 2], 1_500));
        assert!(!rooms.contains("minecraft:the_nether", [2, 64, 2], 1_500), "другое измерение");
        assert!(!rooms.contains(ow, [-1, 64, 2], 1_500), "дверь — не комната");
        assert!(!rooms.contains(ow, [2, 63, 2], 1_500), "пол — не комната");
        // Фильтр перестал отмечаться — запись не в счёт.
        assert!(!rooms.contains(ow, [2, 64, 2], 1_000 + ROOM_STALE_MS));
        assert!(rooms.touch(ow, FILTER.origin, 1_000 + ROOM_STALE_MS));
        assert!(rooms.contains(ow, [2, 64, 2], 1_000 + ROOM_STALE_MS));
        // Разобрали — снято.
        rooms.forget(ow, FILTER.origin);
        assert!(!rooms.contains(ow, [2, 64, 2], 1_000 + ROOM_STALE_MS));
        assert!(!rooms.touch(ow, FILTER.origin, 2_000), "нечего отмечать — нужна проверка");
        assert!(!in_quarantine(ow, [123_456, 64, 2]), "живой реестр пуст");
    }

    #[test]
    fn heavy_strains_leak_outside_the_room() {
        let heavy = Strain::of([3, 2, 4, 1]);
        let worse = Strain::of([1, 2, 5, 1]);
        assert_eq!(leak_strain(&[WILD, heavy], true, false), Some(heavy));
        assert_eq!(leak_strain(&[heavy, worse], true, false), Some(worse), "самый тяжёлый");
        assert_eq!(leak_strain(&[heavy], true, true), None, "в карантине");
        assert_eq!(leak_strain(&[heavy], false, false), None, "машина стоит");
        assert_eq!(leak_strain(&[WILD, Strain::of([5, 5, 3, 5])], true, false), None, "тяжесть 3 не утекает");
        let pos = [4, 70, 9];
        let due = (0..2 * LEAK_EVERY).filter(|t| leak_due(*t, pos)).count();
        assert_eq!(due, 2);
        let mut leaks = Leaks::default();
        let ow = "minecraft:overworld";
        leaks.note(ow, [0.5, 64.5, 0.5], heavy, 0);
        assert_eq!(leaks.near(ow, [3.0, 64.0, 3.0], 100), Some(heavy));
        assert_eq!(leaks.near(ow, [9.0, 64.0, 0.0], 100), None, "далеко");
        assert_eq!(leaks.near("minecraft:the_end", [0.5, 64.0, 0.5], 100), None);
        assert_eq!(leaks.near(ow, [0.5, 64.0, 0.5], LEAK_HOLD_MS), None, "отметка истекла");
        assert!(leak_catches(&heavy, 29) && !leak_catches(&heavy, 30));
    }
}

// ---- M9: доза и болезнь (GDD 4.23 п. 1, 2, 8, 12) ----
//
// Доза вместо мгновенного заражения (п. 1). Источник — лужа под ногами, зомби
// вплотную, удар заражённого, утечка тяжёлого штамма, съеденное — больше не
// поднимает заражение сам, а кладёт сырую дозу в очередь (`queue_dose`). Тик
// дозы (`dose_tick.rs`) разбирает очередь, режет каждую порцию множителем
// спецодежды по каналу (`suit_multiplier`) и копит. Раз в `DOSE_EVERY` —
// проверка: доза дошла до порога — порог снимается, и игрок заражается
// штаммом самой крупной порции (как раньше заражал источник); не дошла —
// тает (×0,97). Короткая ошибка прощается, долгая работа рядом с заразой — нет.
//
// Приступы (п. 2): симптомы не держатся всю стадию, а накатывают волной на
// 5–10 с (тяжесть штамма); промежуток сокращается к поздней стадии, за 2 с до
// приступа — стук сердца. Управление не отбираем: симптомы — прежние ванильные
// эффекты. Часы — `game.ticks`.
//
// Гонка (п. 8): сила болезни — на сколько заражение растёт за шаг болезни,
// иммунитет — сколько за тот же шаг снимает лекарство в окне. Две полосы —
// доли этой пары, и короткий прогноз.
//
// Звук опасности (п. 12): анализатор в руке работает дозиметром — щёлкает тем
// чаще, чем выше доза, свежий поток и источники рядом; в маске слышно своё
// дыхание (`suit_multiplier(…, Air) < 1`); утечка звенит тревогой.

/// Порог дозы: дошла — заражение.
pub const DOSE_THRESHOLD: f32 = 100.0;
/// Во сколько раз доза тает за проверку, пока не дошла до порога.
pub const DOSE_DECAY: f32 = 0.97;
/// Как часто проверка дозы, в тиках (раз в секунду).
pub const DOSE_EVERY: u64 = 20;
/// Ниже этого доза считается нулём и забывает свой источник.
pub const DOSE_FLOOR: f32 = 0.5;
/// На сколько поднимает заражение доза, пробившая порог: прежний шаг удара
/// заражённого (`INFECTED_HIT_STEP`) — заражение как раньше, но только когда
/// доза набрана.
pub const DOSE_INFECTION: u32 = 10;

/// Сырая доза от зомби вплотную за шаг болезни (`INFECTION_EVERY`, 3 с):
/// без защиты порог набирается за ~15 с, при множителе контакта 0,25 — никогда.
pub const CONTACT_DOSE: f32 = 24.0;
/// Сырая доза утечки тяжёлого штамма рядом за шаг болезни: ~12 с без маски.
pub const LEAK_DOSE: f32 = 30.0;
/// Удар заражённого: один прощается, два за ~13 с — заражение.
pub const INFECTED_HIT_DOSE: f32 = 60.0;
/// Доза лужи за единицу её заразности (`Fluid::infects`) за срабатывание
/// (`FLUID_EVERY`): заразная слизь (4) заражает за ~6 с стояния в ней.
pub const PUDDLE_DOSE_PER_POINT: f32 = 5.0;
/// Доза съеденного за единицу прежнего «заражения едой» (`TAINTED_FOOD`):
/// одна гнилая плоть прощается, две подряд — нет.
pub const FOOD_DOSE_PER_POINT: f32 = 4.0;

/// Сырая доза лужи с заразностью `infects` за одно срабатывание.
pub fn puddle_dose(infects: u32) -> f32 {
    infects as f32 * PUDDLE_DOSE_PER_POINT
}

/// Сырая доза заражённой еды; `None` — еда чистая.
pub fn tainted_food_dose(item: &str) -> Option<f32> {
    tainted_food(item).map(|amount| amount as f32 * FOOD_DOSE_PER_POINT)
}

/// Чем пробила порог доза: канал и штамм источника (`None` — дикий).
pub type DoseSource = (DoseChannel, Option<Strain>);

/// Накопленная доза игрока. Живёт в памяти сервера: без источника она за
/// пару минут сходит на нет, писать её на диск незачем.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Exposure {
    /// Сколько накоплено (уже после спецодежды).
    pub amount: f32,
    /// Откуда самая крупная порция текущего накопления — её штамм заразит.
    pub source: Option<DoseSource>,
    /// Величина этой порции (тает вместе с дозой).
    pub biggest: f32,
    /// Сырой поток (до спецодежды) с прошлой проверки — для дозиметра.
    pub incoming: f32,
    /// Сырой поток прошлых проверок, сглаженный (`FLOW_FADE`).
    pub recent: f32,
}

impl Exposure {
    /// Порция сырой дозы `raw` по каналу; до игрока доходит `raw × multiplier`.
    pub fn add(&mut self, channel: DoseChannel, raw: f32, multiplier: f32, strain: Option<Strain>) {
        // `!(x > 0)` отсекает и ноль, и NaN.
        if !(raw > 0.0) {
            return;
        }
        self.incoming += raw;
        let taken = raw * multiplier.max(0.0);
        if !(taken > 0.0) {
            return;
        }
        self.amount += taken;
        if taken >= self.biggest {
            self.biggest = taken;
            self.source = Some((channel, strain));
        }
    }

    /// Проверка раз в `DOSE_EVERY`. Доза у порога и заражаться можно
    /// (`may_infect`: не в карантине) — порог снимается и возвращается
    /// источник; иначе доза тает.
    pub fn check(&mut self, may_infect: bool) -> Option<DoseSource> {
        self.recent = self.recent * FLOW_FADE + self.incoming;
        self.incoming = 0.0;
        if self.recent < 0.01 {
            self.recent = 0.0;
        }
        if may_infect && self.amount >= DOSE_THRESHOLD {
            self.amount -= DOSE_THRESHOLD;
            let source = self.source.unwrap_or((DoseChannel::Contact, None));
            // Следующее накопление выбирает источник заново.
            self.biggest = 0.0;
            if self.amount < DOSE_FLOOR {
                self.amount = 0.0;
                self.source = None;
            }
            return Some(source);
        }
        self.amount *= DOSE_DECAY;
        self.biggest *= DOSE_DECAY;
        if self.amount < DOSE_FLOOR {
            self.amount = 0.0;
            self.biggest = 0.0;
            self.source = None;
        }
        None
    }

    /// Доза в процентах порога (для анализатора).
    pub fn percent(&self) -> u32 {
        (self.amount / DOSE_THRESHOLD * 100.0).round().clamp(0.0, 999.0) as u32
    }
}

/// Пора ли проверять дозу.
pub fn dose_check_due(ticks: u64) -> bool {
    ticks % DOSE_EVERY == 0
}

fn exposures() -> &'static std::sync::Mutex<std::collections::HashMap<u128, Exposure>> {
    static AT: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u128, Exposure>>> =
        std::sync::OnceLock::new();
    AT.get_or_init(Default::default)
}

/// Доза игрока на запись. Только сервер.
pub fn with_exposure<R>(player: u128, f: impl FnOnce(&mut Exposure) -> R) -> Option<R> {
    let mut table = exposures().lock().ok()?;
    Some(f(table.entry(player).or_default()))
}

/// Состояние игрока без шага тика (`player_tick` прибавил бы тик): `None` —
/// игрок в этом мире ещё не тикал.
pub fn world_player_state(world: &std::path::Path, uuid: u128) -> Option<PlayerState> {
    states().lock().ok()?.get(&(world.to_path_buf(), uuid)).copied()
}

/// Что сказать игроку, которого доза только что заразила.
pub fn dose_infected_message(channel: DoseChannel, strain: Option<&Strain>) -> String {
    let heavy = strain.is_some_and(|s| s.severity >= LEAK_SEVERITY);
    match channel {
        DoseChannel::Air if heavy => LEAK_MESSAGE.to_string(),
        DoseChannel::Air => "Доза из воздуха перешла порог: вы заражены".to_string(),
        DoseChannel::Contact => "Доза от контакта перешла порог: вы заражены".to_string(),
        DoseChannel::Splash => "Брызги: доза перешла порог — вы заражены".to_string(),
        DoseChannel::Ingest => "Съеденное: доза перешла порог — вы заражены".to_string(),
    }
}

/// Канал, по которому сервер шлёт клиенту дозу (процент порога, байт).
pub const NET_DOSE: &str = "biolab_core:dose";

/// Пакет дозы: процент порога, обрезанный до байта.
pub fn dose_packet(exposure: &Exposure) -> Vec<u8> {
    vec![exposure.percent().min(255) as u8]
}

/// Процент дозы из пакета; пустой пакет — не наш.
pub fn dose_from_packet(data: &[u8]) -> Option<u32> {
    data.first().map(|v| u32::from(*v))
}

fn known_dose() -> &'static std::sync::atomic::AtomicU64 {
    static VALUE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
    &VALUE
}

/// Клиент получил дозу от сервера.
pub fn remember_dose(percent: u32) {
    known_dose().store(u64::from(percent), std::sync::atomic::Ordering::Relaxed);
}

/// Доза для анализатора; `None` — сервер ещё не присылал.
pub fn shown_dose() -> Option<u32> {
    match known_dose().load(std::sync::atomic::Ordering::Relaxed) {
        u64::MAX => None,
        percent => Some(percent as u32),
    }
}

// Приступы (п. 2).

/// За сколько тиков до приступа стучит сердце.
pub const SEIZURE_WARN: u64 = 40;
/// Промежуток между приступами в начале стадии симптомов (60 с)…
pub const SEIZURE_GAP_EARLY: u64 = 1_200;
/// …и к концу кризиса (15 с).
pub const SEIZURE_GAP_LATE: u64 = 300;
/// Самый длинный приступ (тяжесть 5), тиков.
pub const SEIZURE_MAX_LENGTH: u64 = 200;

/// Длина приступа: 5 с у лёгкого штамма, до 10 с у тяжёлого.
pub fn seizure_length(strain: &Strain) -> u64 {
    100 + 25 * (strain.severity.clamp(1, 5) as u64 - 1)
}

/// Как далеко зашла болезнь: 0 — симптомы только начались, 1 — заражение 100.
pub fn disease_progress(infection: u32, strain: &Strain) -> f32 {
    let onset = strain_onset(strain) as f32;
    ((infection as f32 - onset) / (100.0 - onset).max(1.0)).clamp(0.0, 1.0)
}

/// Промежуток до следующего приступа: от `SEIZURE_GAP_EARLY` к
/// `SEIZURE_GAP_LATE` по ходу болезни, ±15 % от зерна — не метроном.
pub fn seizure_gap(progress: f32, seed: u64) -> u64 {
    let (early, late) = (SEIZURE_GAP_EARLY as f32, SEIZURE_GAP_LATE as f32);
    let base = early + (late - early) * progress.clamp(0.0, 1.0);
    let spread = (seed % 31) as f32 / 30.0 * 0.3 - 0.15;
    (base * (1.0 + spread)).round() as u64
}

/// Расписание приступов одного игрока.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seizure {
    /// Когда начнётся следующий (0 — не назначен).
    pub next: u64,
    /// Когда кончится идущий (0 или прошлое — не идёт).
    pub until: u64,
    /// Предупреждение перед `next` уже прозвучало.
    pub warned: bool,
    /// Сколько приступов было: подмешивается в зерно промежутка.
    pub count: u64,
}

/// Что случилось с приступом на этом тике.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeizureEvent {
    /// Затишье.
    Calm,
    /// Через `SEIZURE_WARN` тиков приступ: пора стучать сердцу.
    Warn,
    /// Приступ начался.
    Start,
    /// Приступ идёт.
    Attack,
    /// Приступ кончился.
    End,
}

impl SeizureEvent {
    /// Держать ли симптомы на игроке.
    pub fn symptoms(self) -> bool {
        matches!(self, SeizureEvent::Start | SeizureEvent::Attack)
    }
}

/// Шаг расписания на тике `now`. Приступы бывают только в стадиях симптомов
/// и кризиса; `seed` — своё у игрока, чтобы двое не болели в такт.
pub fn seizure_step(s: &mut Seizure, now: u64, infection: u32, strain: &Strain, seed: u64) -> SeizureEvent {
    let ill = matches!(stage_with(infection, strain), Stage::Symptoms | Stage::Crisis);
    if !ill {
        let was_on = s.until > now;
        *s = Seizure::default();
        return if was_on { SeizureEvent::End } else { SeizureEvent::Calm };
    }
    // Часы ушли назад (мир загружен заново): прошлое расписание не годится.
    if s.until > now + 2 * SEIZURE_MAX_LENGTH || s.next > now + 2 * SEIZURE_GAP_EARLY {
        *s = Seizure::default();
    }
    let progress = disease_progress(infection, strain);
    if s.until > now {
        return SeizureEvent::Attack;
    }
    if s.until != 0 {
        s.until = 0;
        s.next = now + seizure_gap(progress, seed ^ s.count.wrapping_mul(0x9E37_79B9));
        s.warned = false;
        return SeizureEvent::End;
    }
    if s.next == 0 {
        // Первый приступ — через полпромежутка после начала симптомов.
        s.next = now + seizure_gap(progress, seed) / 2;
        s.warned = false;
        return SeizureEvent::Calm;
    }
    if now >= s.next {
        s.until = now + seizure_length(strain);
        s.next = 0;
        s.warned = false;
        s.count += 1;
        return SeizureEvent::Start;
    }
    if !s.warned && now + SEIZURE_WARN >= s.next {
        s.warned = true;
        return SeizureEvent::Warn;
    }
    SeizureEvent::Calm
}

fn seizures() -> &'static std::sync::Mutex<std::collections::HashMap<u128, Seizure>> {
    static AT: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u128, Seizure>>> =
        std::sync::OnceLock::new();
    AT.get_or_init(Default::default)
}

/// Зерно приступов игрока.
pub fn seizure_seed(player: u128) -> u64 {
    (player as u64) ^ ((player >> 64) as u64)
}

/// Шаг приступов игрока (тик дозы, сервер).
pub fn seizure_tick(player: u128, now: u64, infection: u32, strain: &Strain) -> SeizureEvent {
    let Ok(mut table) = seizures().lock() else {
        return SeizureEvent::Calm;
    };
    let s = table.entry(player).or_default();
    seizure_step(s, now, infection, strain, seizure_seed(player))
}

/// Идёт ли у игрока приступ: тогда тик болезни держит симптомы.
pub fn seizure_now(player: u128, now: u64) -> bool {
    seizures().lock().is_ok_and(|table| table.get(&player).is_some_and(|s| s.until > now))
}

// Две полосы: болезнь против иммунитета (п. 8).

/// Прогноз для анализатора.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forecast {
    /// Не болен.
    Healthy,
    /// Лекарство сильнее: заражение уйдёт примерно за столько секунд.
    Recovering { seconds: u32 },
    /// Силы равны: заражение стоит.
    Even,
    /// Болезнь сильнее; `underdosed` — лекарство есть, но ниже окна.
    Losing { underdosed: bool },
}

/// Гонка: доли силы болезни и иммунитета (в сумме 1, у здорового — нули).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Race {
    pub disease: f32,
    pub defense: f32,
    pub forecast: Forecast,
}

/// Гонка по тому, что знает анализатор: заражение, штамм и кровь. Болезнь
/// за шаг (`INFECTION_EVERY`) прибавляет `strain_step`, лекарство в окне за
/// тот же шаг снимает `cures` на каждом шаге кинетики (`PHARMA_EVERY`).
pub fn disease_race(infection: u32, strain: &Strain, drugs: &[DrugState; DRUG_COUNT]) -> Race {
    if infection == 0 {
        return Race { disease: 0.0, defense: 0.0, forecast: Forecast::Healthy };
    }
    let pharma_steps = (INFECTION_EVERY / PHARMA_EVERY).max(1) as u32;
    let growth = strain_step(strain);
    let mut cure = 0;
    let mut underdosed = false;
    for (drug, state) in DRUGS.iter().zip(drugs) {
        if drug.cures == 0 {
            continue;
        }
        match level_of(drug, state) {
            Level::Therapeutic => cure += drug.cures * pharma_steps,
            Level::Subtherapeutic => underdosed = true,
            _ => {}
        }
    }
    let forecast = if cure > growth {
        let steps = infection.div_ceil(cure - growth);
        Forecast::Recovering { seconds: steps * (INFECTION_EVERY / 20) as u32 }
    } else if cure == growth {
        Forecast::Even
    } else {
        Forecast::Losing { underdosed }
    };
    let total = (growth + cure).max(1) as f32;
    Race { disease: growth as f32 / total, defense: cure as f32 / total, forecast }
}

/// Прогноз строкой.
pub fn forecast_text(forecast: Forecast) -> String {
    match forecast {
        Forecast::Healthy => "Прогноз: здоровы".to_string(),
        Forecast::Recovering { seconds } if seconds >= 60 => {
            format!("Прогноз: выздоравливаете (~{} мин)", seconds.div_ceil(60))
        }
        Forecast::Recovering { seconds } => format!("Прогноз: выздоравливаете (~{seconds} с)"),
        Forecast::Even => "Прогноз: равновесие".to_string(),
        Forecast::Losing { underdosed: true } => "Прогноз: болезнь сильнее, лекарства мало".to_string(),
        Forecast::Losing { underdosed: false } => "Прогноз: болезнь сильнее".to_string(),
    }
}

/// Цвета полос (ARGB): доза, болезнь, иммунитет и подложка.
pub const BAR_DOSE: u32 = 0xFFFF_A040;
pub const BAR_DISEASE: u32 = 0xFFE0_4040;
pub const BAR_DEFENSE: u32 = 0xFF50_D050;
pub const BAR_TRACK: u32 = 0x60FF_FFFF;
/// Длина полосы в пикселях интерфейса.
pub const BAR_WIDTH: f32 = 60.0;

/// Строка панели дозы: текст и, может быть, полоса (доля 0–1 и цвет).
#[derive(Clone, Debug, PartialEq)]
pub struct HudRow {
    pub text: String,
    pub bar: Option<(f32, u32)>,
}

/// Панель под анализатором: доза, две полосы (если болен) и прогноз.
pub fn dose_rows(dose_percent: u32, infection: u32, strain: &Strain, drugs: &[DrugState; DRUG_COUNT]) -> Vec<HudRow> {
    let mut rows = vec![HudRow {
        text: format!("Доза: {dose_percent}% порога"),
        bar: Some(((dose_percent as f32 / 100.0).clamp(0.0, 1.0), BAR_DOSE)),
    }];
    let race = disease_race(infection, strain, drugs);
    if infection > 0 {
        rows.push(HudRow { text: "Болезнь".to_string(), bar: Some((race.disease, BAR_DISEASE)) });
        rows.push(HudRow { text: "Иммунитет".to_string(), bar: Some((race.defense, BAR_DEFENSE)) });
    }
    rows.push(HudRow { text: forecast_text(race.forecast), bar: None });
    rows
}

// Звук опасности (п. 12).

/// Радиус, в котором дозиметр чует источники (заражённых мобов, утечку).
pub const DOSIMETER_RANGE: f64 = 8.0;
/// Как тает сглаженный сырой поток за проверку.
pub const FLOW_FADE: f32 = 0.5;
/// Сырой поток за проверку, при котором дозиметр трещит без остановки.
pub const FLOW_FULL: f32 = 30.0;
/// Как часто слышно дыхание в маске, тиков.
pub const BREATH_EVERY: u64 = 70;

/// Вклад источника на расстоянии `distance`: 1 вплотную, 0 на краю радиуса.
pub fn source_weight(distance: f64) -> f32 {
    (1.0 - distance / DOSIMETER_RANGE).clamp(0.0, 1.0) as f32
}

/// Опасность 0–1 для дозиметра: накопленная доза, свежий сырой поток и
/// источники рядом (сумма `source_weight`; два вплотную — максимум).
pub fn danger_level(exposure: &Exposure, sources_near: f32) -> f32 {
    let dose = exposure.amount / DOSE_THRESHOLD;
    let flow = exposure.recent / FLOW_FULL;
    let near = sources_near / 2.0;
    let level = dose.max(flow).max(near);
    if level.is_nan() { 0.0 } else { level.clamp(0.0, 1.0) }
}

/// Промежуток между щелчками дозиметра в тиках; `None` — молчит.
pub fn click_interval(level: f32) -> Option<u64> {
    if !(level >= 0.02) {
        return None;
    }
    Some((40.0 - 37.0 * level.clamp(0.0, 1.0).sqrt()).round().max(3.0) as u64)
}

/// Пора ли щёлкнуть: прошёл промежуток с прошлого щелчка (или часы ушли назад).
pub fn click_due(now: u64, last: u64, interval: Option<u64>) -> bool {
    interval.is_some_and(|every| now < last || now >= last + every)
}

/// Слышно ли дыхание: маска или костюм режут воздух.
pub fn breathes(air_multiplier: f32) -> bool {
    air_multiplier < 1.0
}

/// Пора ли вдох: раз в `BREATH_EVERY`, у каждого игрока свой такт.
pub fn breath_due(now: u64, seed: u64) -> bool {
    (now + seed % BREATH_EVERY) % BREATH_EVERY == 0
}

/// Дозиметр игрока: тик прошлого щелчка и опасность с последней проверки.
fn dosimeters() -> &'static std::sync::Mutex<std::collections::HashMap<u128, (u64, f32)>> {
    static AT: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u128, (u64, f32)>>> =
        std::sync::OnceLock::new();
    AT.get_or_init(Default::default)
}

/// Запомнить опасность вокруг игрока (раз в проверку дозы).
pub fn set_danger(player: u128, level: f32) {
    if let Ok(mut table) = dosimeters().lock() {
        table.entry(player).or_insert((0, 0.0)).1 = level;
    }
}

/// Щёлкнуть ли дозиметру игрока сейчас (по запомненной опасности); если
/// да — щелчок запоминается.
pub fn dosimeter_tick(player: u128, now: u64) -> bool {
    let Ok(mut table) = dosimeters().lock() else {
        return false;
    };
    let (last, level) = table.entry(player).or_insert((0, 0.0));
    if click_due(now, *last, click_interval(*level)) {
        *last = now;
        return true;
    }
    false
}

#[cfg(test)]
mod dose_tests {
    use super::*;

    /// Тики до первого пробоя: каждые `every` тиков порция `raw × mult`,
    /// проверка раз в `DOSE_EVERY`. `None` — за `limit` тиков не пробило.
    fn breakthrough_after(every: u64, raw: f32, mult: f32, limit: u64) -> Option<u64> {
        let mut e = Exposure::default();
        for t in 1..=limit {
            if t % every == 0 {
                e.add(DoseChannel::Contact, raw, mult, Some(WILD));
            }
            if dose_check_due(t) && e.check(true).is_some() {
                return Some(t);
            }
        }
        None
    }

    #[test]
    fn dose_below_threshold_fades_by_three_percent() {
        let mut e = Exposure::default();
        e.add(DoseChannel::Contact, 50.0, 1.0, None);
        assert_eq!(e.check(true), None);
        assert!((e.amount - 48.5).abs() < 1e-3, "{}", e.amount);
        for _ in 0..400 {
            e.check(true);
        }
        assert_eq!(e.amount, 0.0, "сошла на нет");
        assert_eq!(e.source, None, "источник забыт");
    }

    #[test]
    fn dose_at_threshold_infects_with_the_biggest_source() {
        let heavy = Strain::of([4, 2, 5, 3]);
        let mut e = Exposure::default();
        e.add(DoseChannel::Contact, 20.0, 1.0, None);
        e.add(DoseChannel::Air, 70.0, 1.0, Some(heavy));
        e.add(DoseChannel::Ingest, 30.0, 1.0, None);
        assert_eq!(e.check(true), Some((DoseChannel::Air, Some(heavy))));
        assert!((e.amount - 20.0).abs() < 1e-3, "порог снят, остаток держится");
        assert_eq!(e.percent(), 20);
        // В карантине доза не пробивает, а тает.
        let mut quarantined = Exposure::default();
        quarantined.add(DoseChannel::Contact, 150.0, 1.0, None);
        assert_eq!(quarantined.check(false), None);
        assert!(quarantined.amount < 150.0);
    }

    #[test]
    fn suit_divides_the_flow_and_buys_time() {
        let mut e = Exposure::default();
        e.add(DoseChannel::Contact, 120.0, 0.1, None);
        assert_eq!(e.check(true), None, "защита 90 %: двенадцать из ста двадцати");
        assert!((e.incoming - 0.0).abs() < 1e-6 && (e.recent - 120.0).abs() < 1e-3, "дозиметр видит сырой поток");
        // Зомби вплотную: без защиты ~15 с, при множителе 0,5 — втрое дольше,
        // при 0,25 (костюм на контакт) — никогда.
        let bare = breakthrough_after(INFECTION_EVERY, CONTACT_DOSE, 1.0, 20 * 600).unwrap();
        assert!((200..=400).contains(&bare), "без защиты за {bare} тиков");
        let half = breakthrough_after(INFECTION_EVERY, CONTACT_DOSE, 0.5, 20 * 600).unwrap();
        assert!(half >= 2 * bare, "{half} против {bare}");
        assert_eq!(breakthrough_after(INFECTION_EVERY, CONTACT_DOSE, 0.25, 20 * 600), None);
        // Утечка: без маски ~12 с, респиратор без фильтра (0,6) — дольше, с
        // фильтром (0,15) — держит.
        let leak = breakthrough_after(INFECTION_EVERY, LEAK_DOSE, 1.0, 20 * 600).unwrap();
        assert!(leak <= bare);
        assert!(breakthrough_after(INFECTION_EVERY, LEAK_DOSE, 0.6, 20 * 600).unwrap() > leak);
        assert_eq!(breakthrough_after(INFECTION_EVERY, LEAK_DOSE, 0.15, 20 * 600), None);
        // Заразная слизь под ногами — несколько секунд.
        let puddle = breakthrough_after(FLUID_EVERY, puddle_dose(4), 1.0, 20 * 60).unwrap();
        assert!((80..=200).contains(&puddle), "лужа за {puddle} тиков");
        assert!(LEAK_DOSE > CONTACT_DOSE && INFECTED_HIT_DOSE > CONTACT_DOSE);
    }

    #[test]
    fn one_hit_or_one_meal_is_forgiven_two_are_not() {
        for raw in [INFECTED_HIT_DOSE, tainted_food_dose("minecraft:rotten_flesh").unwrap()] {
            let mut e = Exposure::default();
            e.add(DoseChannel::Contact, raw, 1.0, None);
            let mut infected = false;
            for _ in 0..30 {
                infected |= e.check(true).is_some();
            }
            assert!(!infected, "одна порция {raw} прощается");
            let mut e2 = Exposure::default();
            e2.add(DoseChannel::Contact, raw, 1.0, None);
            for _ in 0..5 {
                e2.check(true);
            }
            e2.add(DoseChannel::Contact, raw, 1.0, None);
            assert!(e2.check(true).is_some(), "две за пять секунд — заражение");
        }
        assert_eq!(tainted_food_dose("minecraft:bread"), None);
        assert!(tainted_food_dose("minecraft:spider_eye").unwrap() < DOSE_THRESHOLD);
    }

    #[test]
    fn nonsense_doses_change_nothing() {
        let mut e = Exposure::default();
        e.add(DoseChannel::Air, 0.0, 1.0, None);
        e.add(DoseChannel::Air, -5.0, 1.0, None);
        e.add(DoseChannel::Air, f32::NAN, 1.0, None);
        e.add(DoseChannel::Air, 10.0, f32::NAN, None);
        assert_eq!(e.amount, 0.0);
        assert_eq!(e.source, None);
        assert_eq!(danger_level(&e, f32::NAN), 0.0);
    }

    #[test]
    fn infection_messages_name_the_channel() {
        let heavy = Strain::of([3, 2, 4, 2]);
        assert_eq!(dose_infected_message(DoseChannel::Air, Some(&heavy)), LEAK_MESSAGE);
        assert!(dose_infected_message(DoseChannel::Air, Some(&WILD)).contains("воздуха"));
        assert!(dose_infected_message(DoseChannel::Ingest, None).contains("Съеденное"));
        let mut e = Exposure::default();
        e.add(DoseChannel::Contact, 37.4, 1.0, None);
        assert_eq!(dose_from_packet(&dose_packet(&e)), Some(37));
        assert_eq!(dose_from_packet(&[]), None);
    }

    #[test]
    fn seizures_come_in_waves_after_a_heartbeat() {
        let strain = WILD;
        let onset = strain_onset(&strain);
        let mut s = Seizure::default();
        // Инкубация — без приступов.
        assert_eq!(seizure_step(&mut s, 100, onset - 1, &strain, 7), SeizureEvent::Calm);
        assert_eq!(s, Seizure::default());
        // Симптомы: волны с предупреждением.
        let mut events = Vec::new();
        for t in 1_000..1_000 + 20 * 300 {
            events.push((t, seizure_step(&mut s, t, onset + 5, &strain, 7)));
        }
        let starts: Vec<u64> = events.iter().filter(|(_, e)| *e == SeizureEvent::Start).map(|(t, _)| *t).collect();
        let warns: Vec<u64> = events.iter().filter(|(_, e)| *e == SeizureEvent::Warn).map(|(t, _)| *t).collect();
        let ends: Vec<u64> = events.iter().filter(|(_, e)| *e == SeizureEvent::End).map(|(t, _)| *t).collect();
        assert!(starts.len() >= 3, "приступы повторяются: {starts:?}");
        for start in &starts {
            assert!(warns.contains(&(start - SEIZURE_WARN)), "стук сердца за 2 с до {start}");
        }
        for (start, end) in starts.iter().zip(&ends) {
            let length = end - start;
            assert!((100..=200).contains(&length), "приступ {length} тиков");
        }
        let on = events.iter().filter(|(_, e)| e.symptoms()).count();
        assert!(on * 2 < events.len(), "между приступами — затишье");
        // Выздоровел посреди приступа — приступ кончился.
        s.until = 1_000_000;
        assert_eq!(seizure_step(&mut s, 999_000, 0, &strain, 7), SeizureEvent::End);
        assert_eq!(s, Seizure::default());
    }

    #[test]
    fn seizures_grow_closer_and_longer() {
        let mild = Strain::of([2, 3, 1, 1]);
        let severe = Strain::of([2, 3, 5, 1]);
        assert_eq!(seizure_length(&mild), 100);
        assert_eq!(seizure_length(&severe), 200);
        let onset = strain_onset(&mild);
        assert_eq!(disease_progress(onset, &mild), 0.0);
        assert_eq!(disease_progress(100, &mild), 1.0);
        for seed in 0..64 {
            let early = seizure_gap(0.0, seed);
            let late = seizure_gap(1.0, seed);
            assert!(late < early, "к поздней стадии чаще");
            assert!((1_020..=1_380).contains(&early) && (255..=345).contains(&late));
        }
        // Часы ушли назад (мир перезагружен): расписание сбрасывается, а не
        // держит приступ на час.
        let mut s = Seizure { next: 0, until: 50_000, warned: false, count: 3 };
        assert_ne!(seizure_step(&mut s, 10, onset + 1, &mild, 1), SeizureEvent::Attack);
        assert!(s.until <= 10 + SEIZURE_MAX_LENGTH);
    }

    #[test]
    fn race_shows_who_wins() {
        let none = [DrugState::default(); DRUG_COUNT];
        assert_eq!(disease_race(0, &WILD, &none).forecast, Forecast::Healthy);
        let sick = disease_race(40, &WILD, &none);
        assert_eq!(sick.forecast, Forecast::Losing { underdosed: false });
        assert_eq!((sick.disease, sick.defense), (1.0, 0.0));

        let penicillin = &DRUGS[COURSE_DRUG];
        let (min, max) = penicillin.window;
        let mut window = none;
        window[COURSE_DRUG].blood = (min + max) / 2;
        let healing = disease_race(40, &WILD, &window);
        assert!(healing.defense > healing.disease);
        assert!((healing.disease + healing.defense - 1.0).abs() < 1e-6);
        let Forecast::Recovering { seconds } = healing.forecast else { panic!("{:?}", healing.forecast) };
        assert!(seconds > 0 && seconds < 60, "{seconds}");
        assert!(forecast_text(healing.forecast).starts_with("Прогноз: выздоравливаете"));

        let mut low = none;
        low[COURSE_DRUG].blood = 1.max(min / 2);
        assert_eq!(disease_race(40, &WILD, &low).forecast, Forecast::Losing { underdosed: true });
        assert!(forecast_text(Forecast::Losing { underdosed: true }).contains("мало"));
        assert_eq!(forecast_text(Forecast::Recovering { seconds: 125 }), "Прогноз: выздоравливаете (~3 мин)");

        let rows = dose_rows(37, 40, &WILD, &none);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].text, "Доза: 37% порога");
        assert_eq!(rows[0].bar.map(|b| b.1), Some(BAR_DOSE));
        assert_eq!(rows[3].text, "Прогноз: болезнь сильнее");
        let healthy = dose_rows(0, 0, &WILD, &none);
        assert_eq!(healthy.len(), 2, "здоровому — только доза и прогноз");
        assert_eq!(dose_rows(250, 0, &WILD, &none)[0].bar.map(|b| b.0), Some(1.0));
    }

    #[test]
    fn dosimeter_clicks_faster_near_danger() {
        assert_eq!(click_interval(0.0), None, "чисто — молчит");
        let mut last = u64::MAX;
        for step in 1..=10 {
            let interval = click_interval(step as f32 / 10.0).unwrap();
            assert!(interval <= last && interval >= 3, "{interval}");
            last = interval;
        }
        assert_eq!(click_interval(1.0), Some(3));
        assert!(click_due(100, 60, Some(40)) && !click_due(99, 60, Some(40)));
        assert!(click_due(5, 60, Some(40)), "часы ушли назад");
        assert!(!click_due(1_000, 0, None));

        let mut e = Exposure::default();
        assert_eq!(danger_level(&e, 0.0), 0.0);
        assert_eq!(danger_level(&e, 2.0), 1.0, "двое вплотную");
        assert_eq!(source_weight(0.0), 1.0);
        assert_eq!(source_weight(DOSIMETER_RANGE + 1.0), 0.0);
        e.add(DoseChannel::Contact, CONTACT_DOSE, 0.05, None);
        e.check(true);
        assert!(danger_level(&e, 0.0) > 0.5, "в костюме доза мала, но поток виден");

        let player = 0xD05E_u128;
        assert!(!dosimeter_tick(player, 1_000), "опасность ещё не измерена");
        set_danger(player, 1.0);
        assert!(dosimeter_tick(player, 1_001));
        assert!(!dosimeter_tick(player, 1_002) && dosimeter_tick(player, 1_004), "треск раз в 3 тика");
        set_danger(player, 0.0);
        assert!(!dosimeter_tick(player, 2_000), "ушёл — замолчал");
    }

    #[test]
    fn breath_only_behind_a_mask() {
        assert!(!breathes(suit_multiplier(&Worn::default(), DoseChannel::Air)), "без маски не слышно");
        assert!(breathes(0.15) && !breathes(1.0));
        let beats = (0..BREATH_EVERY * 4).filter(|t| breath_due(*t, 12345)).count();
        assert_eq!(beats, 4);
    }

    #[test]
    fn new_sounds_are_wired() {
        for cue in [Cue::DosimeterClick, Cue::Breath, Cue::SeizureWarning, Cue::LeakAlarm] {
            assert!(sound(cue).is_some(), "{cue:?}");
        }
        assert_eq!(sound(Cue::DosimeterClick).map(|s| s.event), Some("biolab:item.dosimeter.click"));
        assert!(!RESERVED_SOUNDS.contains(&"biolab:item.dosimeter.click"));
    }
}

// ---- M9: пропорции (GDD 4.24) ----
//
// Девять генов формы частей тела (в конце `GENES`). Вид (масштаб частей
// модели) и числа (прибавки к атрибутам) — в `biolab-deep`
// (`biolab_proportions`). Здесь — то, что видно только всем мобам разом, в
// тике мира (`proportions.rs`): дальний удар длинных рук и глаз большой
// головы, плюс подсказка «у этого вида нет рук» при уколе. Хитбокс моба от
// генов не меняется — дальность удара считается числом.

/// Гены пропорций в порядке таблицы — тот же список, что в `biolab-deep`
/// (`PROPORTION_GENES`, `biolab_proportions::GENES`).
pub const PROPORTION_GENES: [&str; 9] = [
    "long_arms",
    "heavy_arms",
    "macrocephaly",
    "microcephaly",
    "short_legs",
    "stilt_legs",
    "bloated_torso",
    "sunken_chest",
    "asymmetry",
];

/// Виды с руками (без `minecraft:`) — те же, что `biolab_proportions::ARMED_KINDS`.
pub const ARMED_KINDS: [&str; 30] = [
    "player", "zombie", "husk", "drowned", "zombie_villager", "skeleton", "stray",
    "bogged", "parched", "wither_skeleton", "piglin", "piglin_brute", "zombified_piglin",
    "enderman", "iron_golem", "snow_golem", "copper_golem", "witch", "villager",
    "wandering_trader", "pillager", "vindicator", "evoker", "illusioner", "vex", "allay",
    "creaking", "warden", "giant", "biolab:infected",
];

/// Виды без ног — те же, что `biolab_proportions::LEGLESS_KINDS`.
pub const LEGLESS_KINDS: [&str; 26] = [
    "slime", "magma_cube", "sulfur_cube", "ghast", "happy_ghast", "squid", "glow_squid",
    "cod", "salmon", "tropical_fish", "pufferfish", "tadpole", "dolphin", "guardian",
    "elder_guardian", "shulker", "phantom", "bat", "silverfish", "endermite", "blaze",
    "breeze", "nautilus", "zombie_nautilus", "vex", "allay",
];

/// Есть ли у вида руки (вид — как его пишет игра, с `minecraft:` или без).
pub fn kind_has_arms(kind: &str) -> bool {
    ARMED_KINDS.contains(&kind.strip_prefix("minecraft:").unwrap_or(kind))
}

/// Есть ли у вида ноги.
pub fn kind_has_legs(kind: &str) -> bool {
    !LEGLESS_KINDS.contains(&kind.strip_prefix("minecraft:").unwrap_or(kind))
}

/// Что сказать после укола гена пропорций виду, у которого такой части нет:
/// ген прижился, но ничего не делает (GDD 4.24). None — часть есть или ген
/// не про руки и ноги.
pub fn proportion_note(kind: &str, gene: &str) -> Option<&'static str> {
    match gene {
        "long_arms" | "heavy_arms" if !kind_has_arms(kind) => {
            Some("У этого вида нет рук: ген прижился, но спит")
        }
        "short_legs" | "stilt_legs" if !kind_has_legs(kind) => {
            Some("У этого вида нет ног: ген прижился, но спит")
        }
        _ => None,
    }
}

/// Долгорукость: ближе этого бьёт сам ванильный удар — длинной руке
/// работа начинается дальше (блоки по горизонтали, центр к центру).
pub const LONG_ARM_FROM: f64 = 1.6;
/// Долгорукость: докуда дотягивается рука по силе гена (3 / 3,5 / 4 блока).
pub fn long_arm_reach(strength: i32) -> f64 {
    MELEE_REACH + 0.5 * f64::from(strength.clamp(1, 3))
}
/// Долгорукость: замах длинной руки — раз в 2 с, реже обычного удара (побочка
/// GDD 4.24: «медленнее замах»). Кратно `BEHAVIOUR_EVERY`.
pub const LONG_ARM_EVERY: u64 = 40;

/// Большеголовость: в каком радиусе враждебный моб замечает игрока сам
/// (12 / 16 / 20 блоков) — и сам светится сквозь стены: большая голова
/// видна издалека, прятаться ей некуда.
pub fn big_head_radius(strength: i32) -> f64 {
    8.0 + 4.0 * f64::from(strength.clamp(1, 3))
}
/// Сколько тиков держится свечение большой головы (дольше прохода тика мира).
pub const BIG_HEAD_GLOW_TICKS: i32 = 40;

#[cfg(test)]
mod proportion_tests {
    use super::*;

    #[test]
    fn proportion_genes_close_the_table() {
        // Дописаны блоком: номера старых генов (сейвы, данные машин) на месте.
        assert_eq!(gene("sedation_resistance").map(|_| ()), Some(()));
        for name in PROPORTION_GENES {
            let kind = gene(name).expect(name);
            assert!(takes_in_humans(kind), "{name}: на игроке тоже работает (GDD 4.24)");
            assert!(marker_for(name, 3).is_some());
            assert!(kind.cost <= 2);
        }
        let first = GENES.iter().position(|g| g.name == "long_arms").unwrap();
        for (i, name) in PROPORTION_GENES.iter().enumerate() {
            assert_eq!(GENES[first + i].name, *name);
        }
        // Асимметрия — только вид, без ванильного эффекта.
        assert!(gene("asymmetry").unwrap().player.is_none());
        assert!(player_mechanic("asymmetry").is_some());
        assert_eq!(behaviour_strength(&[("long_arms", 2)], "long_reach"), 2);
        assert_eq!(behaviour_strength(&[("macrocephaly", 3)], "big_head"), 3);
    }

    #[test]
    fn long_arms_reach_past_the_vanilla_blow() {
        assert!(LONG_ARM_FROM < MELEE_REACH);
        assert!(long_arm_reach(1) > MELEE_REACH);
        assert!(long_arm_reach(3) > long_arm_reach(1));
        assert_eq!(long_arm_reach(3), 4.0);
        assert_eq!(LONG_ARM_EVERY % BEHAVIOUR_EVERY, 0, "замах ловится тиком мира");
        assert!(LONG_ARM_EVERY > 20, "замах реже ванильного удара");
        assert!(big_head_radius(1) < big_head_radius(3));
        assert!(big_head_radius(3) < far_sight_radius(1), "многоглазие видит дальше головы");
        assert!(BIG_HEAD_GLOW_TICKS as u64 > BEHAVIOUR_EVERY);
    }

    #[test]
    fn missing_parts_are_named() {
        assert_eq!(proportion_note("minecraft:cow", "long_arms"), Some("У этого вида нет рук: ген прижился, но спит"));
        assert_eq!(proportion_note("minecraft:zombie", "long_arms"), None);
        assert_eq!(proportion_note("slime", "stilt_legs"), Some("У этого вида нет ног: ген прижился, но спит"));
        assert_eq!(proportion_note("minecraft:cow", "stilt_legs"), None);
        assert_eq!(proportion_note("minecraft:cow", "macrocephaly"), None, "голова есть у всех");
        assert!(kind_has_arms("biolab:infected") && kind_has_arms("player"));
    }
}

// ---- M9: лаборатория (GDD 4.23 п. 7, 9, 11) ----
//
// Три куска, у каждого чистые правила здесь и исполнение в `lab.rs`
// (`crate::game::biolab_lab`):
// - *Очки знания* (п. 7): узел карты открывается не только своим условием
//   («главный опыт» — как раньше, сразу), но и очками от побочных опытов и
//   провалов: перегрев реактора, скисшая партия, сорванный курс. Повтор того
//   же опыта платит меньше: 1 → 0,5 → 0,25 → 0,1 от базы. Очки и счёт
//   повторов живут в состоянии игрока рядом с маской изученного (`LabProgress`
//   в `PlayerState`), клиенту уходят процентами в хвосте пакета
//   `biolab_core:research`.
// - *Криобанк* (п. 9): образцы (культура, фрагмент ДНК, ткань, векторы) вне
//   холода портятся по игровым часам — у стака срок `SPOIL_KEY` (игровое
//   время следующей ступени порчи). Криобанк сдвигает срок вперёд, пока
//   образец лежит в нём, термобокс в инвентаре — на три четверти.
// - *Герметичность числом* (п. 11): карантинная камера — не «держит/нет», а
//   произведение множителей (стены, двери, фильтр, пробоины). Комната с
//   пробоинами до `MAX_HOLES` собирается и подтекает: `quarantine_leak`
//   отдаёт долю утечки в точке, `in_quarantine` — «защищает ли сейчас»,
//   с этой долей вероятности не защищает.

// == Очки знания (GDD 4.23 п. 7) ==

/// Сколько очков нужно узлу, чтобы открыться. Главный опыт узла (его
/// условие в `RESEARCH`) даёт сразу всё — узел открывается как раньше.
pub const RESEARCH_THRESHOLD: u32 = 100;
/// Доля базы за повтор того же опыта, процентов: первый раз, второй,
/// третий, дальше (13-BALANCE §8 у друга).
pub const REPEAT_SHARE: [u32; 4] = [100, 50, 25, 10];

/// Очков за опыт, который делают в `times`-й раз (0 — впервые).
pub fn repeat_points(base: u32, times: u32) -> u32 {
    base * REPEAT_SHARE[(times as usize).min(REPEAT_SHARE.len() - 1)] / 100
}

/// Опыт, который приносит узлу очки.
pub struct Experiment {
    pub id: &'static str,
    /// Узел, которому идут очки.
    pub node: &'static str,
    /// Очков за первый раз.
    pub base: u32,
    /// Неудача: «это тоже знание».
    pub failure: bool,
    pub title: &'static str,
}

/// Побочные опыты и провалы. Узел — тот, чья запись объясняет, как сделать
/// правильно: перегрев учит окну температур ослабления, скисание —
/// карантину, сорванный курс — сыворотке. «Тот же опыт» — тот же id и та же
/// подробность (что сгорело, какой штамм), поэтому разные культуры платят
/// полностью, а одна и та же — всё меньше.
pub const EXPERIMENTS: [Experiment; 6] = [
    Experiment { id: "reactor_overheat", node: "weakened", base: 40, failure: true, title: "Перегрев реактора" },
    Experiment { id: "bioreactor_overheat", node: "weakened", base: 30, failure: true, title: "Перегрев биореактора" },
    Experiment { id: "batch_sour", node: "quarantine", base: 50, failure: true, title: "Скисшая партия" },
    Experiment { id: "course_broken", node: "remedy", base: 50, failure: true, title: "Сорванный курс лечения" },
    // Нежизнеспособный мутант (конфликт генов, 4.23 п. 5) — источник ставит
    // агент «логика генов» (`note_lab_event` / `research_credit`).
    Experiment { id: "mutant_died", node: "sequencer", base: 40, failure: true, title: "Нежизнеспособный мутант" },
    // Успех тоже повторяется: каждая новая линия культуры — знание о
    // штаммах, а прогон той же самой — почти ничего.
    Experiment { id: "culture_bred", node: "strain", base: 25, failure: false, title: "Селекция культуры" },
];

pub fn experiment_index(id: &str) -> Option<usize> {
    EXPERIMENTS.iter().position(|e| e.id == id)
}

/// Сколько разных опытов помнит игрок для затухания; дальше вытесняется
/// самый редкий (он и платит меньше всего).
pub const REPEAT_SLOTS: usize = 64;
/// Узлов, для которых хранятся очки: по биту маски.
pub const POINT_SLOTS: usize = 128;

/// Очки узлов и счёт повторов опытов. Лежит в `PlayerState` (он `Copy`,
/// поэтому массивы): очки — по номеру узла, повтор — `ключ << 8 | раз`,
/// ключ — 24 бита от «опыт + подробность», 0 — пусто. В файле — по id
/// узла и шестнадцатеричным ключом, так что перестановка таблицы узлов
/// сейвы не путает.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabProgress {
    pub points: [u16; POINT_SLOTS],
    pub repeats: [u32; REPEAT_SLOTS],
}

impl Default for LabProgress {
    fn default() -> Self {
        Self { points: [0; POINT_SLOTS], repeats: [0; REPEAT_SLOTS] }
    }
}

/// Ключ опыта: FNV-1a от «id/подробность», 24 бита, не ноль.
pub fn experiment_key(id: &str, detail: &str) -> u32 {
    let mut h: u32 = 0x811C_9DC5;
    for b in id.bytes().chain([b'/']).chain(detail.bytes()) {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    (h ^ (h >> 24)) & 0xFF_FFFF | 1
}

impl LabProgress {
    /// Сколько раз опыт уже делали.
    pub fn times(&self, key: u32) -> u32 {
        self.repeats.iter().find(|r| **r >> 8 == key && **r != 0).map_or(0, |r| *r & 0xFF)
    }

    /// Отметить ещё один раз.
    pub fn bump(&mut self, key: u32) {
        if let Some(r) = self.repeats.iter_mut().find(|r| **r != 0 && **r >> 8 == key) {
            *r = (key << 8) | ((*r & 0xFF) + 1).min(0xFF);
            return;
        }
        let slot = match self.repeats.iter().position(|r| *r == 0) {
            Some(i) => i,
            None => (0..REPEAT_SLOTS).min_by_key(|i| self.repeats[*i] & 0xFF).unwrap_or(0),
        };
        self.repeats[slot] = (key << 8) | 1;
    }

    /// Очки узла.
    pub fn of(&self, node: usize) -> u32 {
        self.points.get(node).map_or(0, |p| u32::from(*p))
    }
}

/// Что принёс опыт.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Credit {
    pub experiment: usize,
    pub node: usize,
    pub gained: u32,
    pub points: u32,
}

/// Засчитать опыт: очки узлу с затуханием за повтор. None — узла нет или
/// он уже изучен (повтор всё равно считается: изученное не копит).
pub fn credit_experiment(lab: &mut LabProgress, research: u128, experiment: usize, detail: &str) -> Option<Credit> {
    let e = EXPERIMENTS.get(experiment)?;
    let node = research_index(e.node)?;
    let key = experiment_key(e.id, detail);
    let times = lab.times(key);
    lab.bump(key);
    if research & research_bit(node) != 0 || node >= POINT_SLOTS {
        return None;
    }
    let gained = repeat_points(e.base, times);
    let points = (lab.of(node) + gained).min(u32::from(u16::MAX));
    lab.points[node] = points as u16;
    Some(Credit { experiment, node, gained, points })
}

/// Узлы, набравшие порог очков.
pub fn points_mask(lab: &LabProgress) -> u128 {
    (0..RESEARCH.len().min(POINT_SLOTS))
        .filter(|i| lab.of(*i) >= RESEARCH_THRESHOLD)
        .fold(0, |mask, i| mask | research_bit(i))
}

/// Сообщение в чат об опыте.
pub fn credit_message(c: &Credit) -> String {
    let e = &EXPERIMENTS[c.experiment];
    let head = if e.failure { "Неудача — тоже знание" } else { "Опыт" };
    let shown = c.points.min(RESEARCH_THRESHOLD);
    if c.gained == 0 {
        return format!("{head}: «{}» уже ничему не учит", e.title);
    }
    format!(
        "{head}: «{}» +{} к «{}» ({shown} из {RESEARCH_THRESHOLD})",
        e.title, c.gained, RESEARCH[c.node].title
    )
}

/// Прогресс узлов для клиента: процент порога по узлу, изученные — 100.
pub fn research_progress_bytes(lab: &LabProgress, mask: u128) -> Vec<u8> {
    (0..RESEARCH.len())
        .map(|i| {
            if mask & research_bit(i) != 0 {
                100
            } else {
                (lab.of(i) * 100 / RESEARCH_THRESHOLD).min(100) as u8
            }
        })
        .collect()
}

/// Пакет карты с прогрессом: 16 байт маски, затем байт процента на узел.
/// Старый клиент читает первые 16 байт и хвост не видит.
pub fn research_packet_with(mask: u128, progress: &[u8]) -> Vec<u8> {
    let mut out = research_packet(mask);
    out.extend_from_slice(progress);
    out
}

/// Прогресс узлов из пакета (пусто — пакет старый).
pub fn research_progress_from_packet(data: &[u8]) -> Vec<u8> {
    data.get(16..).map(<[u8]>::to_vec).unwrap_or_default()
}

fn known_progress() -> &'static std::sync::Mutex<Vec<u8>> {
    static KNOWN: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());
    &KNOWN
}

/// Клиент запомнил прогресс узлов.
pub fn remember_research_progress(progress: Vec<u8>) {
    *known_progress().lock().unwrap_or_else(|e| e.into_inner()) = progress;
}

/// Процент порога узла, как его знает клиент.
pub fn shown_node_progress(index: usize) -> u8 {
    known_progress().lock().unwrap_or_else(|e| e.into_inner()).get(index).copied().unwrap_or(0)
}

/// Подсказка узла с прогрессом очков: у неизученного, начатого узла —
/// строка «Знание: 40 из 100».
pub fn research_tooltip_with(mask: u128, progress: u8, index: usize) -> Vec<String> {
    let mut lines = research_tooltip(mask, index);
    if mask & research_bit(index) == 0 && progress > 0 {
        let points = u32::from(progress) * RESEARCH_THRESHOLD / 100;
        lines.push(format!("Знание: {points} из {RESEARCH_THRESHOLD} — опыты и неудачи"));
    }
    lines
}

/// Очки в файле: `research_points=weakened:40,remedy:25`,
/// `research_repeats=1a2b3c:2,…` (ключ — шестнадцатеричный).
pub fn lab_to_text(lab: &LabProgress) -> String {
    let mut text = String::new();
    let points: Vec<String> = (0..RESEARCH.len().min(POINT_SLOTS))
        .filter(|i| lab.of(*i) > 0)
        .map(|i| format!("{}:{}", RESEARCH[i].id, lab.of(i)))
        .collect();
    if !points.is_empty() {
        text.push_str(&format!("research_points={}\n", points.join(",")));
    }
    let repeats: Vec<String> = lab
        .repeats
        .iter()
        .filter(|r| **r != 0)
        .map(|r| format!("{:x}:{}", r >> 8, r & 0xFF))
        .collect();
    if !repeats.is_empty() {
        text.push_str(&format!("research_repeats={}\n", repeats.join(",")));
    }
    text
}

/// Строка файла про очки; false — строка не про них.
pub fn lab_from_line(lab: &mut LabProgress, key: &str, value: &str) -> bool {
    match key {
        "research_points" => {
            for part in value.split(',') {
                if let Some((id, n)) = part.trim().split_once(':')
                    && let Some(i) = research_index(id)
                    && i < POINT_SLOTS
                    && let Ok(n) = n.parse::<u16>()
                {
                    lab.points[i] = n;
                }
            }
            true
        }
        "research_repeats" => {
            let mut slot = 0;
            for part in value.split(',') {
                if slot >= REPEAT_SLOTS {
                    break;
                }
                if let Some((k, n)) = part.trim().split_once(':')
                    && let Ok(k) = u32::from_str_radix(k, 16)
                    && let Ok(n) = n.parse::<u32>()
                    && k != 0
                    && k <= 0xFF_FFFF
                {
                    lab.repeats[slot] = (k << 8) | n.min(0xFF);
                    slot += 1;
                }
            }
            true
        }
        _ => false,
    }
}

/// Засчитать опыт игроку мира и сохранить. Вернёт строку для чата.
/// Только сервер (состояния игроков — его).
pub fn research_credit(world: &std::path::Path, uuid: u128, experiment: &str, detail: &str) -> Option<String> {
    let experiment = experiment_index(experiment)?;
    let mut table = states().lock().ok()?;
    let state = table.entry((world.to_path_buf(), uuid)).or_insert_with(|| {
        std::fs::read_to_string(state_path(world, uuid)).map(|t| state_from_text(&t)).unwrap_or_default()
    });
    let credit = credit_experiment(&mut state.lab, state.research, experiment, detail);
    let path = state_path(world, uuid);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, state_to_text(state));
    credit.map(|c| credit_message(&c))
}

/// Прогресс узлов игрока для пакета клиенту.
pub fn research_progress_of(world: &std::path::Path, uuid: u128) -> Vec<u8> {
    let Ok(table) = states().lock() else {
        return Vec::new();
    };
    match table.get(&(world.to_path_buf(), uuid)) {
        Some(state) => research_progress_bytes(&state.lab, state.research),
        None => Vec::new(),
    }
}

// Опыты машин: у машины нет игрока, поэтому она отмечает событие в точке, а
// засчитывает его каждый игрок, который был рядом (на своём шаге карты, раз
// в секунду). Отметка живёт `LAB_EVENT_HOLD_MS`.

/// Радиус, в котором игрок «видел» опыт машины, блоков.
pub const LAB_EVENT_RADIUS: f64 = 8.0;
/// Сколько живёт отметка опыта.
pub const LAB_EVENT_HOLD_MS: u64 = 3_000;

struct LabEvent {
    dimension: String,
    at: [f64; 3],
    experiment: usize,
    detail: String,
    seq: u64,
    until: u64,
}

/// Отметки опытов машин и что из них каждый игрок уже засчитал.
#[derive(Default)]
pub struct LabEvents {
    list: Vec<LabEvent>,
    next: u64,
    seen: std::collections::HashMap<u128, u64>,
}

impl LabEvents {
    pub fn note(&mut self, dimension: &str, at: [f64; 3], experiment: usize, detail: &str, now: u64) {
        self.list.retain(|e| e.until > now);
        if self.list.len() >= 64 {
            return;
        }
        self.next += 1;
        self.list.push(LabEvent {
            dimension: dimension.to_string(),
            at,
            experiment,
            detail: detail.to_string(),
            seq: self.next,
            until: now + LAB_EVENT_HOLD_MS,
        });
    }

    /// Опыты рядом с игроком, которых он ещё не засчитывал.
    pub fn take(&mut self, player: u128, dimension: &str, at: [f64; 3], now: u64) -> Vec<(usize, String)> {
        let seen = self.seen.get(&player).copied().unwrap_or(0);
        let mine = self
            .list
            .iter()
            .filter(|e| e.seq > seen && e.until > now && e.dimension == dimension && within(e.at, at, LAB_EVENT_RADIUS))
            .map(|e| (e.experiment, e.detail.clone()))
            .collect();
        self.seen.insert(player, self.next);
        mine
    }
}

fn lab_events() -> &'static std::sync::Mutex<LabEvents> {
    static EVENTS: std::sync::OnceLock<std::sync::Mutex<LabEvents>> = std::sync::OnceLock::new();
    EVENTS.get_or_init(Default::default)
}

/// Машина в точке `at` поставила опыт `experiment` (id из `EXPERIMENTS`).
pub fn note_lab_event(dimension: &str, at: [f64; 3], experiment: &str, detail: &str) {
    let Some(index) = experiment_index(experiment) else {
        return;
    };
    if let Ok(mut events) = lab_events().lock() {
        events.note(dimension, at, index, detail, quarantine_clock());
    }
}

/// Засчитать игроку опыты машин рядом; строки — в чат.
pub fn credit_lab_events(world: &std::path::Path, uuid: u128, dimension: &str, at: [f64; 3]) -> Vec<String> {
    let taken = match lab_events().lock() {
        Ok(mut events) => events.take(uuid, dimension, at, quarantine_clock()),
        Err(_) => return Vec::new(),
    };
    taken
        .into_iter()
        .filter_map(|(i, detail)| research_credit(world, uuid, EXPERIMENTS[i].id, &detail))
        .collect()
}

// == Криобанк и порча образцов (GDD 4.23 п. 9) ==
//
// Образец без холода портится ступенями: раз в `SAMPLE_SHELF` тиков игрового
// времени (`world.game_time`, идёт в сейве мира) — культура дичает на шаг к
// дикому штамму, фрагмент и вектор теряют единицу силы гена (до пустого),
// антисмысл выдыхается, ткань сгнивает. Срок следующей ступени — `SPOIL_KEY`
// в `custom_data` (игровое время, Long): отсчёт идёт и в сундуке, и на
// земле, где стак никто не тикает, — считается по часам при следующем
// взгляде. Криобанк каждую секунду сдвигает срок на прошедшее время (внутри
// не портится), термобокс в инвентаре — на три четверти (вчетверо
// медленнее). Без срока образец свежий: срок ставится, когда он впервые
// попал в инвентарь или в криобанк.

/// Криобанк: холодильник со слотами, без рецептов.
pub const CRYOBANK: &str = "biolab:cryobank";
/// Термобокс: в инвентаре замедляет порчу образцов.
pub const THERMOBOX: &str = "biolab:thermobox";
/// Слотов криобанка: ряд, как у сундука.
pub const CRYOBANK_SLOTS: usize = 9;
/// Срок следующей ступени порчи в `custom_data`, игровое время.
pub const SPOIL_KEY: &str = "biolab_spoil_at";
/// Тиков в тепле до ступени порчи: двое игровых суток.
pub const SAMPLE_SHELF: i64 = 48_000;
/// Доля времени, которую термобокс снимает, из четырёх: 3 — вчетверо
/// медленнее.
pub const THERMOBOX_SAVES: i64 = 3;
/// Раз в сколько тиков инвентарь и криобанк учитывают время.
pub const SPOIL_EVERY: i64 = 20;
/// Во что сгнивает ткань.
pub const ROTTEN_TISSUE: &str = "minecraft:rotten_flesh";

/// Образец: то, что портится вне холода.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sample {
    Culture,
    Fragment,
    Tissue,
    Vector,
    Antisense,
}

pub fn sample_kind(item: &str) -> Option<Sample> {
    match item {
        CULTURE_ID => Some(Sample::Culture),
        FRAGMENT_ID => Some(Sample::Fragment),
        TISSUE_ID => Some(Sample::Tissue),
        GENE_VECTOR => Some(Sample::Vector),
        ANTISENSE_VECTOR => Some(Sample::Antisense),
        _ => None,
    }
}

pub fn is_sample(item: &str) -> bool {
    sample_kind(item).is_some()
}

/// Ключ данных, в котором образец держит то, что портится.
pub fn sample_key(kind: Sample) -> Option<&'static str> {
    match kind {
        Sample::Culture => Some(STRAIN_KEY),
        Sample::Fragment => Some(GENE_KEY),
        Sample::Vector => Some(VECTOR_KEY),
        Sample::Antisense => Some(ANTISENSE_KEY),
        Sample::Tissue => None,
    }
}

/// Сколько ступеней порчи прошло к часу `now` при сроке `deadline`.
pub fn spoil_steps(deadline: i64, now: i64) -> u32 {
    if now < deadline {
        return 0;
    }
    (1 + (now - deadline) / SAMPLE_SHELF).min(16) as u32
}

/// Срок после того, как прошло `steps` ступеней: следующая — через полный
/// срок от просроченного.
pub fn next_deadline(deadline: i64, steps: u32) -> i64 {
    deadline + SAMPLE_SHELF * i64::from(steps)
}

/// Свежесть, процентов: сколько осталось до следующей ступени.
pub fn freshness(deadline: i64, now: i64) -> i32 {
    (((deadline - now).max(0) * 100 + SAMPLE_SHELF - 1) / SAMPLE_SHELF).clamp(0, 100) as i32
}

/// Насколько холод сдвигает срок за `elapsed` тиков: криобанк — целиком,
/// термобокс — на три четверти, тепло — нисколько.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cold {
    Warm,
    Thermobox,
    Cryobank,
}

pub fn cold_push(cold: Cold, elapsed: i64) -> i64 {
    match cold {
        Cold::Warm => 0,
        Cold::Thermobox => elapsed * THERMOBOX_SAVES / 4,
        Cold::Cryobank => elapsed,
    }
}

/// Культура дичает: за ступень одна характеристика, дальше всех ушедшая от
/// дикой, делает шаг к ней (при равенстве — первая по порядку).
pub fn strain_goes_wild(strain: &Strain, steps: u32) -> Strain {
    let mut v = strain.values();
    let w = WILD.values();
    for _ in 0..steps {
        let far = (0..4).filter(|i| v[*i] != w[*i]).max_by_key(|i| ((v[*i] - w[*i]).abs(), std::cmp::Reverse(*i)));
        let Some(i) = far else { break };
        v[i] += (w[i] - v[i]).signum();
    }
    Strain::of(v)
}

/// Что стало с образцом за `steps` ступеней порчи.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Spoiled {
    /// Данные образца теперь такие (пусто — пустой фрагмент/вектор).
    Data(String),
    /// Образец сгнил: теперь это другой предмет без данных.
    Item(&'static str),
}

pub fn spoil_sample(kind: Sample, data: Option<&str>, steps: u32) -> Spoiled {
    let s = steps as i32;
    match kind {
        Sample::Tissue => Spoiled::Item(ROTTEN_TISSUE),
        Sample::Culture => {
            let strain = data.and_then(strain_from_text).unwrap_or(WILD);
            Spoiled::Data(strain_to_text(&strain_goes_wild(&strain, steps)))
        }
        Sample::Fragment => match data.and_then(gene_from_text) {
            Some((name, strength)) if strength > s => Spoiled::Data(gene_text(name, strength - s)),
            _ => Spoiled::Data(String::new()),
        },
        Sample::Vector => {
            let genes: Vec<(&'static str, i32)> = data
                .map(vector_genes)
                .unwrap_or_default()
                .into_iter()
                .filter(|(_, strength)| *strength > s)
                .map(|(name, strength)| (name, strength - s))
                .collect();
            Spoiled::Data(genome_to_text(&genes))
        }
        Sample::Antisense => Spoiled::Data(String::new()),
    }
}

/// Строка свежести для подсказки образца.
pub fn freshness_line(deadline: Option<i64>, now: i64) -> String {
    match deadline {
        None => "Свежий образец · вне холода портится".to_string(),
        Some(d) => {
            let p = freshness(d, now);
            let hours = ((d - now).max(0) + 999) / 1000;
            format!("Свежесть: {p}% · до порчи ~{hours} ч игрового времени")
        }
    }
}

/// Цвет строк свежести в подсказке предмета: холодный голубой.
pub const FRESH_INK: u32 = 0xFF_7FC8E0;

/// Экран криобанка: табло сверху, ряд слотов, инвентарь как у печи.
pub fn cryobank_screen() -> MachineScreen {
    MachineScreen {
        width: SCREEN_WIDTH,
        height: SCREEN_HEIGHT,
        slots: (0..CRYOBANK_SLOTS).map(|i| Some([8.0 + i as f32 * 18.0, 52.0])).collect(),
        inventory: [8.0, SCREEN_INVENTORY_Y],
        gauge: [8.0, 19.0, 160.0, 30.0],
        progress: [0.0, 0.0, 0.0, 0.0],
    }
}

/// Что делать, чтобы образец не портился.
pub const FRESHNESS_HINT: &str = "Храните в криобанке; в поход — термобокс";

/// Что примет слот криобанка: только образцы.
pub fn cryobank_accepts(item: &str) -> bool {
    is_sample(item)
}

/// Подсказка над прибором криобанка.
pub fn cryobank_tooltip() -> Vec<String> {
    vec![
        "Криобанк: −80 °C".to_string(),
        "§7Культуры, фрагменты, ткань и векторы здесь не портятся".to_string(),
        "§7Вне холода образец портится за двое игровых суток".to_string(),
    ]
}

// == Герметичность камеры числом (GDD 4.23 п. 11) ==
//
// Заливка та же, что в M8 (`check_room`), но пробоины не рушат камеру:
// заливка дошла до края — ищем дыру (клетку пути с ≥ 4 соседями-стенами),
// затыкаем её мысленно и льём снова, до `MAX_HOLES` раз. Закрылась —
// камера собрана с пробоинами и подтекает. Дверь без шлюза и отсутствие
// шлюза — тоже не отказ, а множитель. Отказ остаётся у «нет комнаты»,
// «комната больше 9×9×5» и дыры шире клетки или больше `MAX_HOLES` дыр —
// с тем же текстом, что раньше.

/// Сколько пробоин камера терпит (дальше — «Не закрыта»).
pub const MAX_HOLES: usize = 4;
/// Множитель стен, сплошь из стекла установки (панели — 1).
pub const GLASS_SEAL: f32 = 0.9;
/// Множитель за каждую дверь прямо наружу.
pub const OPEN_DOOR_SEAL: f32 = 0.7;
/// Множитель за каждую пробоину.
pub const HOLE_SEAL: f32 = 0.6;
/// Фильтр: без патрона и у изношенного до нуля.
pub const NO_FILTER_SEAL: f32 = 0.2;
pub const WORN_FILTER_SEAL: f32 = 0.9;
/// С какой герметичности камера «держит» (зелёная лампа).
pub const SEAL_HOLDS: f32 = 0.9;

/// Комната с подробностями герметичности.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sealed {
    pub room: Room,
    /// Пробоины, по порядку нахождения.
    pub holes: Vec<[i32; 3]>,
    /// Двери прямо наружу (нижняя клетка).
    pub open_doors: Vec<[i32; 3]>,
    /// Блоков оболочки вокруг комнаты и сколько из них стекло.
    pub shell: u32,
    pub glass: u32,
}

/// Множители герметичности.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SealFactors {
    pub glass: u32,
    pub shell: u32,
    pub airlocks: u32,
    pub open_doors: u32,
    pub holes: u32,
    /// Остаток патрона, %; None — патрона нет.
    pub filter: Option<i32>,
}

/// Стены, двери, фильтр, пробоины — по множителю.
pub fn seal_parts(f: &SealFactors) -> [f32; 4] {
    let glass = if f.shell == 0 { 0.0 } else { f.glass as f32 / f.shell as f32 };
    let walls = 1.0 - (1.0 - GLASS_SEAL) * glass.clamp(0.0, 1.0);
    let doors = OPEN_DOOR_SEAL.powi(f.open_doors as i32);
    let filter = match f.filter {
        None => NO_FILTER_SEAL,
        Some(p) => WORN_FILTER_SEAL + (1.0 - WORN_FILTER_SEAL) * (p.clamp(0, 100) as f32 / 100.0),
    };
    let holes = HOLE_SEAL.powi(f.holes as i32);
    [walls, doors, filter, holes]
}

/// Герметичность 0–1: произведение множителей.
pub fn tightness(f: &SealFactors) -> f32 {
    seal_parts(f).iter().product::<f32>().clamp(0.0, 1.0)
}

/// Доля утечки 0–1: 0 — герметична.
pub fn room_leak(f: &SealFactors) -> f32 {
    // Округление до тысячных: идеальная камера — ровно ноль.
    ((1.0 - tightness(f)) * 1000.0).round() / 1000.0
}

/// Герметичность, процентов вниз: 99,9 — ещё не 100.
pub fn seal_percent(f: &SealFactors) -> i32 {
    let milli = (room_leak(f) * 1000.0).round() as i32;
    (1000 - milli) / 10
}

pub fn seal_factors(sealed: &Sealed, filter: Option<i64>) -> SealFactors {
    SealFactors {
        glass: sealed.glass,
        shell: sealed.shell,
        airlocks: sealed.room.airlocks,
        open_doors: sealed.open_doors.len() as u32,
        holes: sealed.holes.len() as u32,
        filter: filter.map(filter_percent),
    }
}

/// Пробка для пробоины на пути заливки: клетка пути (не старт и не пробка)
/// с наибольшим числом соседей-стен, не меньше четырёх (сидит в оболочке);
/// при равенстве — ближе к краю. None — дыра шире клетки.
fn pick_plug(survey: &mut Survey, start: [i32; 3], f: &Flood, plugs: &std::collections::HashSet<[i32; 3]>) -> Option<[i32; 3]> {
    let end = f.breach?;
    let mut path = vec![end];
    while let Some(p) = f.parent.get(path.last().unwrap_or(&start)) {
        path.push(*p);
    }
    let mut best: Option<(usize, [i32; 3])> = None;
    for p in path.iter().filter(|p| **p != start && !plugs.contains(*p)) {
        let walls = ROOM_DIRS
            .iter()
            .filter(|d| {
                let n = add3(*p, **d);
                plugs.contains(&n) || survey.get(n) != RoomBlock::Open
            })
            .count();
        if walls >= 4 && best.is_none_or(|(w, _)| walls > w) {
            best = Some((walls, *p));
        }
    }
    best.map(|(_, p)| p)
}

/// Заливка закрылась (с пробками `holes`): размер, двери, оболочка.
fn finish_seal(survey: &mut Survey, f: Flood, holes: Vec<[i32; 3]>) -> Result<Sealed, RoomFault> {
    let size = span(f.min, f.max);
    if (0..3).any(|i| size[i] > ROOM_MAX[i]) {
        return Err(RoomFault::TooBig(size));
    }
    let mut cells = f.cells;
    cells.sort();
    let (airlocks, open_doors) = room_doors(survey, &cells, &f.set);
    let plugged: std::collections::HashSet<[i32; 3]> = holes.iter().copied().collect();
    let mut shell = std::collections::HashSet::new();
    for &c in &cells {
        for d in ROOM_DIRS {
            let n = add3(c, d);
            if !plugged.contains(&n) && survey.get(n) == RoomBlock::Shell {
                shell.insert(n);
            }
        }
    }
    let glass = shell.iter().filter(|p| survey.glass.contains(*p)).count() as u32;
    Ok(Sealed {
        room: Room { cells, min: f.min, max: f.max, airlocks },
        holes,
        open_doors,
        shell: shell.len() as u32,
        glass,
    })
}

/// Камера с пробоинами: та же заливка, что `check_room`, но до `MAX_HOLES`
/// дыр затыкаются мысленно. Ошибка — как у `check_room` (та же точка дыры).
pub fn seal_room(site: Site, at: BlockAt) -> Result<Sealed, RoomFault> {
    let mut survey = Survey { at, seen: std::collections::HashMap::new(), glass: std::collections::HashSet::new() };
    let front = place(site.origin, site.facing, [0, 0, -1]);
    let back = place(site.origin, site.facing, [0, 0, 1]);
    let mut faults: Vec<(bool, [i32; 3])> = Vec::new();
    let mut too_big: Option<RoomFault> = None;
    let mut best: Option<Sealed> = None;
    for start in [front, back] {
        if survey.get(start) != RoomBlock::Open {
            continue;
        }
        let mut plugs = std::collections::HashSet::new();
        let mut order: Vec<[i32; 3]> = Vec::new();
        loop {
            let f = flood(&mut survey, start, room_box(start), &plugs);
            if f.breach.is_none() {
                let clean = order.is_empty();
                match finish_seal(&mut survey, f, order.clone()) {
                    // Закрылась без пробок — это комната, как у `check_room`.
                    result if clean => return result,
                    Ok(s) => {
                        if best.as_ref().is_none_or(|b| s.holes.len() < b.holes.len()) {
                            best = Some(s);
                        }
                    }
                    Err(fault) => too_big = too_big.or(Some(fault)),
                }
                break;
            }
            if order.is_empty() {
                faults.push(locate_hole(&mut survey, start, &f));
            }
            if order.len() >= MAX_HOLES {
                break;
            }
            let Some(p) = pick_plug(&mut survey, start, &f, &plugs) else {
                break;
            };
            plugs.insert(p);
            order.push(p);
        }
    }
    if let Some(s) = best {
        return Ok(s);
    }
    if let Some(fault) = too_big {
        return Err(fault);
    }
    match faults.iter().find(|(exact, _)| *exact).or(faults.first()) {
        Some((_, p)) => Err(RoomFault::Hole(*p)),
        None => Err(RoomFault::NoRoom),
    }
}

/// Итог проверки камеры с пробоинами для каркаса.
pub fn seal_verdict(result: &Result<Sealed, RoomFault>) -> Verdict {
    room_verdict(&result.as_ref().map(|s| s.room.clone()).map_err(|f| *f))
}

/// Почему камера подтекает: первая причина строкой (с точкой, где она).
pub fn leak_reason(data: &[i32]) -> Option<String> {
    let [x, y, z] = room_spot(data);
    let holes = screen_datum(data, DATA_QR_HOLES, 0);
    if holes > 0 {
        let more = if holes > 1 { format!(" (+{})", holes - 1) } else { String::new() };
        return Some(format!("Пробоина: {x} {y} {z}{more}"));
    }
    if screen_datum(data, DATA_QR_DOORS, 0) > 0 {
        return Some(format!("Дверь без шлюза: {x} {y} {z}"));
    }
    if screen_datum(data, DATA_QR_FILTER, -1) < 0 {
        return None;
    }
    let glass = screen_datum(data, DATA_QR_GLASS, 0);
    if glass > 0 {
        return Some(format!("Стекло: {glass}% стен"));
    }
    None
}

/// Подсказка: множители герметичности по строке.
pub fn seal_lines(data: &[i32]) -> Vec<String> {
    let f = SealFactors {
        glass: screen_datum(data, DATA_QR_GLASS, 0).max(0) as u32,
        shell: 100,
        airlocks: screen_datum(data, DATA_QR_LOCKS, 0).max(0) as u32,
        open_doors: screen_datum(data, DATA_QR_DOORS, 0).max(0) as u32,
        holes: screen_datum(data, DATA_QR_HOLES, 0).max(0) as u32,
        filter: Some(screen_datum(data, DATA_QR_FILTER, -1)).filter(|p| *p >= 0),
    };
    let [walls, doors, filter, holes] = seal_parts(&f);
    let pct = |v: f32| (v * 100.0).round() as i32;
    vec![
        format!("§7Стены ×{}%, двери ×{}%, фильтр ×{}%, пробоины ×{}%", pct(walls), pct(doors), pct(filter), pct(holes)),
        format!("§7Утечка {}%: заражение проходит в такой доле проверок", 100 - screen_datum(data, DATA_QR_SEAL, 100)),
    ]
}

/// Детерминированный бросок «защитила ли камера» в точке: один на секунду,
/// чтобы все проверки одного мгновения видели одно и то же.
pub fn sealed_here(leak: f32, pos: [i32; 3], now_ms: u64) -> bool {
    if leak <= 0.0 {
        return true;
    }
    if leak >= 1.0 {
        return false;
    }
    let mut h = (now_ms / 1000).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for v in pos {
        h ^= v as u32 as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    ((h >> 32) % 1000) as f32 >= leak * 1000.0
}

#[cfg(test)]
mod lab_tests {
    use super::*;

    #[test]
    fn repeating_an_experiment_pays_less_and_less() {
        let shares: Vec<u32> = (0..6).map(|t| repeat_points(40, t)).collect();
        assert_eq!(shares, vec![40, 20, 10, 4, 4, 4], "1 → 0,5 → 0,25 → 0,1 от базы");
        let mut lab = LabProgress::default();
        let overheat = experiment_index("reactor_overheat").unwrap();
        let node = research_index("weakened").unwrap();
        let mask = research_bit(0);
        let a = credit_experiment(&mut lab, mask, overheat, "biolab:culture").unwrap();
        assert_eq!((a.node, a.gained, a.points), (node, 40, 40));
        let b = credit_experiment(&mut lab, mask, overheat, "biolab:culture").unwrap();
        assert_eq!((b.gained, b.points), (20, 60), "тот же опыт — половина");
        // Другая подробность — другой опыт: снова полная база.
        let c = credit_experiment(&mut lab, mask, overheat, "biolab:mold").unwrap();
        assert_eq!((c.gained, c.points), (40, 100));
        assert_ne!(points_mask(&lab) & research_bit(node), 0, "порог набран — узел открыт");
        assert!(credit_message(&a).contains("Неудача — тоже знание"), "{}", credit_message(&a));
        assert!(credit_message(&a).contains("«Ослабленная культура» (40 из 100)"), "{}", credit_message(&a));
        // Изученный узел не копит, но повтор считается.
        let mask = mask | research_bit(node);
        assert_eq!(credit_experiment(&mut lab, mask, overheat, "biolab:mold"), None);
        assert_eq!(lab.times(experiment_key("reactor_overheat", "biolab:mold")), 2);
    }

    #[test]
    fn every_experiment_points_at_a_node_and_failures_pay() {
        for e in &EXPERIMENTS {
            assert!(research_index(e.node).is_some(), "{}: нет узла {}", e.id, e.node);
            assert!(e.base > 0 && e.base < RESEARCH_THRESHOLD, "{}: опыт не должен открывать узел сразу", e.id);
        }
        for id in ["reactor_overheat", "bioreactor_overheat", "batch_sour", "course_broken", "mutant_died"] {
            assert!(EXPERIMENTS[experiment_index(id).unwrap()].failure, "{id} — провал");
        }
        assert!(RESEARCH.len() <= POINT_SLOTS);
    }

    #[test]
    fn opened_nodes_stay_opened_and_points_open_new_ones() {
        // Совместимость: изученное по-старому остаётся изученным, а очки
        // добавляют узлы к условиям.
        let mut s = PlayerState::default();
        let bench = research_index("bench").unwrap();
        s.research = research_bit(0) | research_bit(bench);
        let text = state_to_text(&s);
        assert!(!text.contains("research_points"), "без очков файл прежний");
        let back = state_from_text(&text);
        assert_eq!(back.research, s.research);
        let remedy = research_index("remedy").unwrap();
        s.lab.points[remedy] = 75;
        s.lab.bump(experiment_key("course_broken", "2,3,2,1"));
        let back = state_from_text(&state_to_text(&s));
        assert_eq!(back.lab, s.lab, "очки и повторы — в файле");
        assert!(state_to_text(&s).contains("research_points=remedy:75"));
        assert_eq!(points_mask(&back.lab) & research_bit(remedy), 0, "75 из 100 — ещё нет");
    }

    #[test]
    fn the_repeat_memory_is_bounded() {
        let mut lab = LabProgress::default();
        for i in 0..(REPEAT_SLOTS as u32 + 10) {
            lab.bump(experiment_key("culture_bred", &i.to_string()));
        }
        assert_eq!(lab.repeats.iter().filter(|r| **r != 0).count(), REPEAT_SLOTS);
        let key = experiment_key("culture_bred", "fresh");
        lab.bump(key);
        lab.bump(key);
        assert_eq!(lab.times(key), 2);
        assert_ne!(experiment_key("a", "b"), experiment_key("a", "c"));
        assert_ne!(experiment_key("a", ""), 0);
    }

    #[test]
    fn progress_rides_in_the_research_packet() {
        let mut lab = LabProgress::default();
        let weakened = research_index("weakened").unwrap();
        lab.points[weakened] = 40;
        let mask = research_bit(0);
        let bytes = research_progress_bytes(&lab, mask);
        assert_eq!(bytes.len(), RESEARCH.len());
        assert_eq!((bytes[0], bytes[weakened]), (100, 40));
        let packet = research_packet_with(mask, &bytes);
        assert_eq!(research_from_packet(&packet), Some(mask), "старое чтение маски цело");
        assert_eq!(research_progress_from_packet(&packet), bytes);
        assert!(research_progress_from_packet(&research_packet(mask)).is_empty());
        let tip = research_tooltip_with(mask, 40, weakened);
        assert!(tip.last().unwrap().starts_with("Знание: 40 из 100"), "{tip:?}");
        assert_eq!(research_tooltip_with(mask, 0, weakened), research_tooltip(mask, weakened));
    }

    #[test]
    fn machine_experiments_reach_players_nearby_once() {
        let mut events = LabEvents::default();
        let ow = "minecraft:overworld";
        events.note(ow, [0.5, 64.5, 0.5], 0, "biolab:culture", 0);
        assert_eq!(events.take(1, ow, [3.0, 64.0, 3.0], 100), vec![(0, "biolab:culture".to_string())]);
        assert!(events.take(1, ow, [3.0, 64.0, 3.0], 200).is_empty(), "засчитано один раз");
        assert_eq!(events.take(2, ow, [1.0, 64.0, 1.0], 200).len(), 1, "второй игрок рядом — тоже");
        assert!(events.take(3, ow, [30.0, 64.0, 1.0], 200).is_empty(), "далеко");
        assert!(events.take(4, "minecraft:the_nether", [0.5, 64.0, 0.5], 200).is_empty());
        assert!(events.take(5, ow, [0.5, 64.0, 0.5], LAB_EVENT_HOLD_MS).is_empty(), "отметка истекла");
    }

    #[test]
    fn samples_spoil_in_steps_by_the_game_clock() {
        assert_eq!(spoil_steps(1_000, 999), 0);
        assert_eq!(spoil_steps(1_000, 1_000), 1);
        assert_eq!(spoil_steps(1_000, 1_000 + SAMPLE_SHELF), 2);
        assert_eq!(next_deadline(1_000, 2), 1_000 + 2 * SAMPLE_SHELF);
        assert_eq!(freshness(SAMPLE_SHELF, 0), 100);
        assert_eq!(freshness(SAMPLE_SHELF, SAMPLE_SHELF / 2), 50);
        assert_eq!(freshness(10, 20), 0);
        // Криобанк сдвигает срок целиком, термобокс — на три четверти.
        assert_eq!(cold_push(Cold::Cryobank, 20), 20);
        assert_eq!(cold_push(Cold::Thermobox, 20), 15);
        assert_eq!(cold_push(Cold::Warm, 20), 0);
        // Двое суток в тепле — ступень; в термобоксе — восемь.
        let warm = SAMPLE_SHELF;
        let boxed = SAMPLE_SHELF * 4 / (4 - THERMOBOX_SAVES);
        assert_eq!(boxed, 4 * warm);
    }

    #[test]
    fn the_cryobank_screen_fits_and_takes_only_samples() {
        let s = machine_screen(CRYOBANK).expect("свой экран");
        assert_eq!(s.slots.len(), CRYOBANK_SLOTS);
        let label_y = s.inventory[1] - 11.0;
        for at in s.slots.iter().flatten() {
            let f = slot_frame(*at);
            assert!(f[0] >= 3.0 && f[0] + f[2] <= s.width - 3.0, "{f:?} вне панели");
            assert!(f[1] >= s.gauge[1] + s.gauge[3] && f[1] + f[3] < label_y, "{f:?} налезает на табло или подпись");
        }
        assert!(s.gauge[1] >= SCREEN_HEADER);
        assert_eq!(machine_title(CRYOBANK), "Криобанк");
        assert!(has_block_entity(CRYOBANK) && !is_machine(CRYOBANK));
        assert_eq!(menu_slot_rule(CRYOBANK, 0, CULTURE_ID), Some(true));
        assert_eq!(menu_slot_rule(CRYOBANK, 0, "minecraft:dirt"), Some(false));
        assert_eq!(container_slot_accepts(CRYOBANK, 8, FRAGMENT_ID), Some(true));
        assert_eq!(container_slot_accepts(CRYOBANK, 9, FRAGMENT_ID), Some(false));
        assert!(!screen_tooltip(CRYOBANK, &[], [20.0, 25.0]).is_empty(), "подсказка над табло");
    }

    #[test]
    fn spoiling_weakens_each_kind_of_sample() {
        // Культура дичает к дикому штамму, дальше всего ушедшее — первым.
        let bred = Strain::of([5, 3, 4, 1]);
        assert_eq!(strain_goes_wild(&bred, 1), Strain::of([4, 3, 4, 1]));
        assert_eq!(strain_goes_wild(&bred, 2), Strain::of([3, 3, 4, 1]), "поровну — первая по порядку");
        assert_eq!(strain_goes_wild(&bred, 10), WILD, "совсем одичала");
        assert_eq!(strain_goes_wild(&WILD, 3), WILD);
        assert_eq!(spoil_sample(Sample::Culture, Some("5,3,4,1"), 1), Spoiled::Data("4,3,4,1".into()));
        // Фрагмент теряет силу гена, потом пустеет.
        assert_eq!(spoil_sample(Sample::Fragment, Some("chitin:3"), 1), Spoiled::Data("chitin:2".into()));
        assert_eq!(spoil_sample(Sample::Fragment, Some("chitin:1"), 1), Spoiled::Data(String::new()));
        assert_eq!(fragment_label(""), "Фрагмент ДНК: пустой");
        // Вектор — каждый ген на единицу, выдохшиеся уходят.
        let Spoiled::Data(v) = spoil_sample(Sample::Vector, Some("chitin:2,horns:1"), 1) else { panic!() };
        assert_eq!(vector_genes(&v), vec![("chitin", 1)]);
        assert_eq!(spoil_sample(Sample::Antisense, Some("horns"), 1), Spoiled::Data(String::new()));
        assert_eq!(antisense_label(""), "Антисмысловой вектор: пустой");
        assert_eq!(spoil_sample(Sample::Tissue, None, 1), Spoiled::Item(ROTTEN_TISSUE));
        // Что портится и что криобанк принимает.
        for item in [CULTURE_ID, FRAGMENT_ID, TISSUE_ID, GENE_VECTOR, ANTISENSE_VECTOR] {
            assert!(is_sample(item) && cryobank_accepts(item), "{item}");
        }
        assert!(!is_sample(FLASK_ID) && !cryobank_accepts("minecraft:dirt"));
        assert!(freshness_line(Some(SAMPLE_SHELF), 0).starts_with("Свежесть: 100%"));
    }

    type World = std::collections::HashMap<[i32; 3], &'static str>;

    /// Камера 5×3×5, как в `quarantine_tests`: фильтр в южной стене лицом
    /// на север, шлюз — две двери в западной стене.
    fn lab_room() -> World {
        let mut w = World::new();
        let (lo, hi) = ([0, 64, 0], [4, 66, 4]);
        for x in lo[0] - 1..=hi[0] + 1 {
            for y in lo[1] - 1..=hi[1] + 1 {
                for z in lo[2] - 1..=hi[2] + 1 {
                    if !(0..3).all(|i| lo[i] <= [x, y, z][i] && [x, y, z][i] <= hi[i]) {
                        w.insert([x, y, z], CLEAN_PANEL);
                    }
                }
            }
        }
        w.insert([2, 64, 5], QUARANTINE);
        for (x, y) in [(-1, 64), (-1, 65), (-2, 64), (-2, 65)] {
            w.insert([x, y, 2], "minecraft:oak_door");
        }
        w
    }

    const SITE: Site = Site { origin: [2, 64, 5], facing: Facing::North };

    fn seal(w: &World) -> Result<Sealed, RoomFault> {
        seal_room(SITE, &|p| w.get(&p).copied().unwrap_or("minecraft:air").to_string())
    }

    fn strict(w: &World) -> Result<Room, RoomFault> {
        check_room(SITE, &|p| w.get(&p).copied().unwrap_or("minecraft:air").to_string())
    }

    #[test]
    fn a_whole_room_is_tight_and_matches_the_strict_check() {
        let s = seal(&lab_room()).expect("камера");
        assert_eq!(Ok(s.room.clone()), strict(&lab_room()));
        assert!(s.holes.is_empty() && s.open_doors.is_empty());
        assert_eq!(s.glass, 0);
        assert!(s.shell >= 100, "оболочка вокруг 5×3×5: {}", s.shell);
        let f = seal_factors(&s, Some(FILTER_DURABILITY));
        assert_eq!(room_leak(&f), 0.0, "идеальная камера не течёт");
        assert_eq!(seal_percent(&f), 100);
        assert!(tightness(&f) >= SEAL_HOLDS);
    }

    #[test]
    fn holes_make_the_room_leak_instead_of_breaking_it() {
        let mut w = lab_room();
        w.remove(&[4, 65, -1]);
        assert_eq!(strict(&w), Err(RoomFault::Hole([4, 65, -1])), "строгая проверка — прежняя");
        let s = seal(&w).expect("с пробоиной камера собрана");
        assert_eq!(s.holes, vec![[4, 65, -1]]);
        assert_eq!(s.room.cells.len(), 75, "дыра — не комната");
        let one = room_leak(&seal_factors(&s, Some(FILTER_DURABILITY)));
        assert!((one - (1.0 - HOLE_SEAL)).abs() < 1e-3, "{one}");
        w.remove(&[0, 63, 0]);
        w.remove(&[2, 67, 2]);
        let s = seal(&w).expect("три дыры");
        assert_eq!(s.holes.len(), 3);
        let three = room_leak(&seal_factors(&s, Some(FILTER_DURABILITY)));
        assert!(three > one, "дыр больше — течёт сильнее");
        // Больше `MAX_HOLES` — уже не камера, с прежним текстом дыры.
        for p in [[5, 64, 3], [-1, 66, 4], [1, 63, 3]] {
            w.remove(&p);
        }
        assert!(matches!(seal(&w), Err(RoomFault::Hole(_))));
        // Пол из земли — дыра шире клетки: не камера.
        let mut dirt = lab_room();
        for x in 0..=4 {
            for z in 0..=4 {
                dirt.insert([x, 63, z], "minecraft:dirt");
            }
        }
        assert!(matches!(seal(&dirt), Err(RoomFault::Hole(_))));
        // Фильтр посреди поля — тоже.
        let field = World::from([([2, 64, 5], QUARANTINE)]);
        assert!(matches!(seal(&field), Err(RoomFault::Hole(_))));
    }

    #[test]
    fn doors_glass_and_the_filter_are_factors() {
        // Дверь прямо наружу: строгая — отказ, здесь — множитель.
        let mut w = lab_room();
        w.insert([5, 64, 2], "minecraft:oak_door");
        w.insert([5, 65, 2], "minecraft:oak_door");
        assert_eq!(strict(&w), Err(RoomFault::OpenDoor([5, 64, 2])));
        let s = seal(&w).expect("собрана");
        assert_eq!(s.open_doors, vec![[5, 64, 2]]);
        let f = seal_factors(&s, Some(FILTER_DURABILITY));
        assert!((tightness(&f) - OPEN_DOOR_SEAL).abs() < 1e-3);
        // Стеклянная стена — чуть хуже панелей.
        let mut glass = lab_room();
        for x in 0..=4 {
            for y in 64..=66 {
                glass.insert([x, y, -1], CASING_GLASS);
            }
        }
        let s = seal(&glass).unwrap();
        assert_eq!(s.glass, 15);
        let t = tightness(&seal_factors(&s, Some(FILTER_DURABILITY)));
        assert!(t < 1.0 && t > GLASS_SEAL, "{t}");
        // Фильтр: без патрона камера почти не защищает, износ — понемногу.
        let s = seal(&lab_room()).unwrap();
        assert!((tightness(&seal_factors(&s, None)) - NO_FILTER_SEAL).abs() < 1e-3);
        let half = tightness(&seal_factors(&s, Some(FILTER_DURABILITY / 2)));
        assert!(half < 1.0 && half >= SEAL_HOLDS, "{half}");
        // Без дверей вовсе — запечатана: строгая «Нет шлюза», здесь — цела.
        let mut sealed = lab_room();
        for (x, y) in [(-1, 64), (-1, 65)] {
            sealed.insert([x, y, 2], CLEAN_PANEL);
        }
        assert_eq!(strict(&sealed), Err(RoomFault::NoAirlock));
        assert_eq!(room_leak(&seal_factors(&seal(&sealed).unwrap(), Some(FILTER_DURABILITY))), 0.0);
    }

    #[test]
    fn the_leak_share_is_the_share_of_unprotected_checks() {
        assert!(sealed_here(0.0, [1, 2, 3], 5_000));
        assert!(!sealed_here(1.0, [1, 2, 3], 5_000));
        let open = (0..2_000u64).filter(|s| !sealed_here(0.3, [*s as i32, 64, 7], s * 1000)).count();
        assert!((450..750).contains(&open), "≈30 % из 2000: {open}");
        // В одну секунду в одной точке ответ один.
        assert_eq!(sealed_here(0.5, [4, 64, 4], 10_100), sealed_here(0.5, [4, 64, 4], 10_900));
    }

    #[test]
    fn the_registry_keeps_the_leak_of_each_room() {
        let s = seal(&lab_room()).unwrap();
        let mut rooms = QuarantineRooms::default();
        let ow = "minecraft:overworld";
        assert_eq!(rooms.leak_at(ow, [2, 64, 2], 0), 1.0, "нет камеры — течёт всё");
        rooms.hold(ow, SITE.origin, &s.room, 0.25, 1_000);
        assert_eq!(rooms.leak_at(ow, [2, 64, 2], 1_500), 0.25);
        assert_eq!(rooms.leak_at(ow, [-1, 64, 2], 1_500), 1.0, "дверь — не комната");
        assert_eq!(rooms.leak_at(ow, [2, 64, 2], 1_000 + ROOM_STALE_MS), 1.0, "фильтр замолчал");
        assert_eq!(quarantine_leak(ow, [987_654, 64, 2]), 1.0, "живой реестр пуст");
    }

    #[test]
    fn the_filter_screen_shows_tightness() {
        let mut w = lab_room();
        w.remove(&[4, 65, -1]);
        let mb = multiblock(QUARANTINE).unwrap();
        let verdict = check_multiblock(mb, SITE, &|p| w.get(&p).copied().unwrap_or("minecraft:air").to_string());
        assert!(matches!(verdict, Verdict::Formed(_)), "с пробоиной камера собрана");
        let detail = take_room_check(SITE.origin).expect("подробности");
        let mut data = vec![0; machine_data_slots(QUARANTINE)];
        write_verdict(&mut data, &verdict);
        write_room(&mut data, &detail);
        let s = detail.as_ref().unwrap();
        let f = seal_factors(s, Some(FILTER_DURABILITY));
        write_seal(&mut data, &f);
        write_filter(&mut data, Some(FILTER_DURABILITY), tightness(&f) >= SEAL_HOLDS);
        let board = quarantine_board(&data);
        assert_eq!(board.head, "Подтекает");
        assert_eq!(board.lines[0], "Герметичность 60%");
        assert_eq!(board.lines[1], "Пробоина: 4 65 -1");
        let tip = quarantine_tooltip(&data);
        assert!(tip.iter().any(|l| l.contains("пробоины ×60%")), "{tip:?}");
    }
}

// ---- M9: контракт параллельной работы (GDD 4.23, 4.25) ----
//
// Доза заразы (4.23 п. 1) идёт игроку по каналам; спецодежда (4.25) режет
// каждый канал своим множителем; загрязнённая еда и другие источники
// (4.23 п. 13) не заражают сами, а кладут дозу в очередь — её разбирает тик
// дозы. Сигнатуры не менять: тела заполняют агенты «доза» (очередь — уже
// рабочая) и «спецодежда» (`suit_multiplier`).

/// Канал, по которому зараза доходит до игрока.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DoseChannel {
    /// Воздух: газ, споры, утечка штамма.
    Air,
    /// Контакт: лужа, заражённый блок, удар заражённого, мутантная ткань.
    Contact,
    /// Брызги: работа машин, вскрытие на столе.
    Splash,
    /// Съеденное: загрязнённая еда (костюм не помогает).
    Ingest,
}

/// Одна порция дозы от источника.
#[derive(Clone, Debug, PartialEq)]
pub struct QueuedDose {
    pub player: u128,
    pub channel: DoseChannel,
    pub amount: f32,
    /// Штамм источника; `None` — дикий.
    pub strain: Option<Strain>,
}

/// Очередь доз от источников. Только сервер.
pub fn dose_queue() -> &'static std::sync::Mutex<Vec<QueuedDose>> {
    static AT: std::sync::OnceLock<std::sync::Mutex<Vec<QueuedDose>>> = std::sync::OnceLock::new();
    AT.get_or_init(Default::default)
}

/// Положить дозу игроку в очередь (её снимет тик дозы игрока).
pub fn queue_dose(player: u128, channel: DoseChannel, amount: f32, strain: Option<Strain>) {
    if let Ok(mut queue) = dose_queue().lock() {
        if queue.len() < 256 {
            queue.push(QueuedDose { player, channel, amount, strain });
        }
    }
}

/// Забрать все дозы этого игрока из очереди.
pub fn take_queued_doses(player: u128) -> Vec<QueuedDose> {
    let Ok(mut queue) = dose_queue().lock() else {
        return Vec::new();
    };
    let (mine, rest): (Vec<_>, Vec<_>) = queue.drain(..).partition(|d| d.player == player);
    *queue = rest;
    mine
}

/// Что надето на игрока: по слоту брони (голова, грудь, ноги, ступни) — id
/// предмета и строка `custom_data` (пусто — нет). Собирает тот, кто зовёт.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Worn {
    pub slots: [(String, String); 4],
}

/// Множитель потока дозы по каналу от надетого (1.0 — защиты нет, 0.05 —
/// полный костюм с фильтром). Тело — агента «спецодежда»; доза только зовёт.
pub fn suit_multiplier(worn: &Worn, channel: DoseChannel) -> f32 {
    let _ = (worn, channel);
    1.0
}

// ---- M8: связь установок между собой (контракт волны 2) ----
//
// Карантинная камера (GDD 4.18 п. 4) отвечает на вопрос «эта точка внутри
// собранной чистой комнаты?». Ею пользуются биореактор (партия не скисает в
// карантине), болезнь (внутри нельзя заразиться) и утечка тяжёлых штаммов.
// Сигнатуру не менять: тело заполняет агент карантина, остальные только
// вызывают. Пока ни одна камера не держит — нигде не карантин.

/// Точка `pos` уровня `dimension` — внутри собранной карантинной камеры,
/// и камера защищает её сейчас. С M9 (GDD 4.23 п. 11) камера бывает
/// негерметичной: с долей `quarantine_leak` проверок ответ «нет» (бросок —
/// один на точку в секунду, `sealed_here`). Герметичная — всегда «да».
pub fn in_quarantine(dimension: &str, pos: [i32; 3]) -> bool {
    // Реестр камер (раздел «M8: карантинная камера»): фильтр кладёт туда
    // клетки своей закрытой комнаты и её долю утечки.
    sealed_here(quarantine_leak(dimension, pos), pos, quarantine_clock())
}

/// Доля утечки камеры в точке: 0 — герметична, 1 — камеры нет. Для тех, кто
/// режет поток (доза), а не бросает «да/нет».
pub fn quarantine_leak(dimension: &str, pos: [i32; 3]) -> f32 {
    quarantine_rooms().lock().map_or(1.0, |rooms| rooms.leak_at(dimension, pos, quarantine_clock()))
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
pub const SCREEN_NAMES: [(&str, &str, &str); 18] = [
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
    (COLUMN, "Ректификационная колонна", "Перегонка"),
    (QUARANTINE, "Фильтр карантинной камеры", "Фильтрация"),
    // Порты установок (GDD 4.18): не машины, но экран у них свой.
    (INPUT_PORT, "Входной порт", "Подача"),
    (OUTPUT_PORT, "Выходной порт", "Выдача"),
    (SEQUENCER, "Генный секвенатор", "Секвенирование"),
    (BIOREACTOR, "Биореактор", "Партия"),
    // Криобанк (GDD 4.23 п. 9): не машина, но экран у него свой.
    (CRYOBANK, "Криобанк", "Хранение"),
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
    // Криобанк (GDD 4.23 п. 9): табло холода во всю ширину и ряд из девяти
    // слотов над инвентарём, как у сундука.
    if id == CRYOBANK {
        return Some(cryobank_screen());
    }
    // Порт установки: пять слотов в ряд, как у воронки; слева — стрелка
    // подачи, справа — табличка «вход»/«выход».
    if port(id).is_some() {
        return Some(MachineScreen {
            width: SCREEN_WIDTH,
            height: HOPPER_HEIGHT,
            slots: (0..PORT_SLOTS).map(|i| Some([44.0 + i as f32 * 18.0, 20.0])).collect(),
            inventory: [8.0, HOPPER_INVENTORY_Y],
            gauge: [14.0, 18.0, 20.0, 20.0],
            progress: [138.0, 18.0, 30.0, 20.0],
        });
    }
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
        // Ректификационная колонна: слева башня (шкала — пар поднимается по
        // тарелкам), у её подножия вход, сверху в ряд пять фракций — слева
        // направо с верхней тарелки вниз, под ними табло сборки.
        COLUMN => MachineScreen {
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            slots: std::iter::once(Some([32.0, 54.0]))
                .chain((0..COLUMN_TRAYS.1).map(|k| Some([54.0 + k as f32 * 18.0, 18.0])))
                .collect(),
            inventory: [8.0, SCREEN_INVENTORY_Y],
            gauge: [52.0, 38.0, 118.0, 34.0],
            progress: [6.0, 17.0, 22.0, 55.0],
        },
        // Генный секвенатор: слева кнопки режимов (`seq_buttons`), сверху три
        // фрагмента, под ними фосфор, раствор и выход, между рядами — шкала;
        // справа монитор с генами.
        SEQUENCER => MachineScreen {
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            slots: vec![
                Some([26.0, 18.0]),
                Some([44.0, 18.0]),
                Some([62.0, 18.0]),
                Some([26.0, 54.0]),
                Some([44.0, 54.0]),
                Some([62.0, 54.0]),
            ],
            inventory: [8.0, SCREEN_INVENTORY_Y],
            gauge: [84.0, 17.0, 87.0, 54.0],
            progress: [25.0, 38.0, 54.0, 12.0],
        },
        // Биореактор: слева восемь входов (4×2), справа восемь выходов,
        // между ними стрелка партии и слот раствора; внизу во всю ширину
        // табло — сборка, состояние среды и градусник.
        BIOREACTOR => MachineScreen {
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            slots: (0..BIOREACTOR_INPUTS)
                .map(|i| Some([8.0 + (i % 4) as f32 * 18.0, 18.0 + (i / 4) as f32 * 18.0]))
                .chain(std::iter::once(Some([81.0, 36.0])))
                .chain((0..BIOREACTOR_OUTPUTS).map(|i| Some([100.0 + (i % 4) as f32 * 18.0, 18.0 + (i / 4) as f32 * 18.0])))
                .collect(),
            inventory: [8.0, SCREEN_INVENTORY_Y],
            gauge: [5.0, 55.0, 166.0, 16.0],
            progress: [81.0, 20.0, 16.0, 11.0],
        },
        // Фильтр карантинной камеры: слева патрон и под ним его остаток,
        // справа табло: держит ли камера, чего не хватает.
        QUARANTINE => MachineScreen {
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            slots: vec![Some([20.0, 26.0])],
            inventory: [8.0, SCREEN_INVENTORY_Y],
            gauge: [48.0, 18.0, 122.0, 54.0],
            progress: [8.0, 50.0, 40.0, 20.0],
        },
        _ => return None,
    };
    Some(screen)
}

/// Выход ли этот слот — по роли слота (у центрифуги выходы 1–4, у машин на
/// меню печки и сетке — последний слот, у колонны — фракции), у выходного
/// порта — все. Выходы на экране в бирюзовой рамке.
pub fn is_output_slot(id: &str, slot: usize) -> bool {
    if let Some(p) = port(id) {
        return p.kind == PortKind::Output && slot < PORT_SLOTS;
    }
    match machine(id) {
        Some(m) => slot_role(m, slot) == SlotRole::Output,
        None => false,
    }
}

/// Строка о портах установки.
pub fn ports_line(inputs: u32, outputs: u32) -> String {
    if inputs == 0 && outputs == 0 {
        return "Портов нет: сырьё и выход — в самом контроллере".to_string();
    }
    format!("Порты: входов {inputs}, выходов {outputs}")
}

/// Подсказка над табло установки: собрана ли, а если нет — вся нехватка.
pub fn multiblock_tooltip(id: &str, data: &[i32]) -> Vec<String> {
    let Some(mb) = multiblock(id) else {
        return Vec::new();
    };
    match multiblock_shown(data) {
        MultiblockShown::Unknown => vec![format!("{}: проверяю схему…", mb.title)],
        MultiblockShown::Formed { sections, inputs, outputs } => {
            let mut lines = vec![format!("{}: собрана", mb.title)];
            if id == COLUMN {
                lines.push(column_status(sections));
            }
            lines.push(ports_line(inputs, outputs));
            lines
        }
        MultiblockShown::Broken { rows, total } => {
            let mut lines = vec![format!("{}: не собрана", mb.title)];
            lines.extend(shortage_lines(mb, &rows, total).into_iter().map(|l| format!("§c{l}")));
            lines.push("§7Схема — в журнале лаборанта".to_string());
            lines
        }
    }
}

/// Подсказка над прибором порта.
pub fn port_tooltip(id: &str) -> Vec<String> {
    match port(id).map(|p| p.kind) {
        Some(PortKind::Input) => vec![
            "Входной порт".to_string(),
            "§7Контроллер установки рядом забирает отсюда сырьё".to_string(),
        ],
        Some(PortKind::Output) => vec![
            "Выходной порт".to_string(),
            "§7Сюда контроллер кладёт готовое; воронки забирают".to_string(),
        ],
        None => Vec::new(),
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
    if port(id).is_some() {
        let over = inside(screen.gauge, at) || inside(screen.progress, at);
        return if over { port_tooltip(id) } else { Vec::new() };
    }
    if id == CRYOBANK {
        return if inside(screen.gauge, at) { cryobank_tooltip() } else { Vec::new() };
    }
    // Кнопки режимов секвенатора — вне прибора и шкалы.
    if id == SEQUENCER
        && let Some(mode) = seq_button_at(at)
    {
        return seq_button_tooltip(mode, seq_shown(data).mode);
    }
    if inside(screen.progress, at) {
        // Шкала фильтра — остаток патрона, а не ход работы.
        if id == QUARANTINE {
            return vec![filter_line(data), "§7Патрон изнашивается, пока камера держит".to_string()];
        }
        let mut lines = Vec::new();
        if id == SEQUENCER {
            let shown = seq_shown(data);
            lines.push(shown.mode.title().to_string());
            lines.push(seq_status_text(&shown));
        }
        if let MultiblockShown::Formed { sections, .. } = multiblock_shown(data)
            && id == COLUMN
        {
            lines.push(column_status(sections));
        }
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
        // Монитор собранного секвенатора — гены; несобранного — нехватка.
        SEQUENCER if matches!(multiblock_shown(data), MultiblockShown::Formed { .. }) => {
            let mut lines = seq_tooltip(&seq_shown(data));
            lines.extend(multiblock_tooltip(id, data).into_iter().skip(1));
            lines
        }
        BIOREACTOR => bioreactor_tooltip(data),
        QUARANTINE => quarantine_tooltip(data),
        id if multiblock(id).is_some() => multiblock_tooltip(id, data),
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
        assert_eq!(blocks.len(), 36, "блоков стало больше — решите, выпадают ли новые");
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
        // Части многоблочных установок (GDD 4.18) — тоже.
        for id in [CASING, CASING_GLASS, INPUT_PORT, OUTPUT_PORT, COLUMN_TRAY, COLUMN_CONDENSER, SEQUENCER_RACK, SEQUENCER_MONITOR, CLEAN_PANEL, CRYOBANK] {
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
        // Части установок — металл и стекло в раме: киркой.
        for id in [CASING, CASING_GLASS, INPUT_PORT, OUTPUT_PORT, COLUMN, COLUMN_TRAY, COLUMN_CONDENSER, SEQUENCER, SEQUENCER_RACK, SEQUENCER_MONITOR, QUARANTINE, CLEAN_PANEL, CRYOBANK] {
            assert!(pickaxe.contains(&id.to_string()), "{id}");
        }
    }
}
