// BioLab, «глубокая» часть: то, для чего стабильных хук-поинтов не хватает, —
// мутанты (прибавки к атрибутам, поведение по генам, внешний вид). Поведение
// своих эффектов отсюда ушло в `biolab-core` на стабильные хуки API 56, а
// геном и операционная с M11 — свои данные на сущностях (attachments, ниже).
//
// Бандл цепляется к внутренним функциям игры и привязан к коммиту
// d91cab6. После обновления Horizon цели надо перепроверять.
//
// Этот файл — одновременно Cargo-библиотека (тесты) и модуль
// `crate::game::biolab_deep` внутри игры.

/// Идентификатор мода в Options > Mods и в horizon-mods.txt.
pub const ID: &str = "biolab_deep";

// ---- Мутанты: что делает бинарник ----
//
// Геном бросается новому мобу в бинарнике (хук `native_mobs_joined`) и
// хранится данными на самом мобе (`biolab:genome`, ниже). Прибавки к
// атрибутам и поведение по генам тоже считает бинарник — проход перед тиком
// мобов (`mutant_tick.rs`), где видны и данные, и мобы. Вид рисует lib-крейт
// по тому, что пришло на клиент (`view_sync.rs` → `biolab_bridge`). Общий
// контракт частей — имена генов, их цена и форма данных; тест
// `tests/modules.rs` следит, чтобы копии не разошлись.

/// Гены по номерам: тот же порядок, что `mutants::GENE_NAMES`.
pub const GENE_NAMES: [&str; 40] = [
    "gigantism",
    "dwarfism",
    "hypermetabolism",
    "chitin",
    "dense_bone",
    "brute_force",
    "sinew",
    "spring_legs",
    "thin_skin",
    "bioluminescence",
    "albinism",
    "melanism",
    "iridescence",
    "rust_hide",
    "pallor",
    "violet_blood",
    "regeneration",
    "thermal",
    "cryophilia",
    "photophobia",
    "aquatic",
    "toxin_tolerance",
    "acid_glands",
    "volatile",
    "necrosis",
    "toxic_blood",
    "thorns",
    "healing_aura",
    "predator",
    "pack_instinct",
    "loyalty",
    "rending_claws",
    "polymelia",
    "dicephaly",
    "horns",
    "tail",
    "tentacles",
    "polyopia",
    "tumor",
    "membranes",
];

/// Цена гена за единицу силы (та же, что в lib).
pub const GENE_COSTS: [i32; 40] = [2, 1, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1, 2, 1, 1, 2, 3, 2, 2, 1, 2, 2, 3, 3, 2, 2, 3, 3, 2, 2, 2, 3, 3, 3, 2, 2, 3, 2, 1, 2];

/// Сколько генов влезает в природный геном и какую сумму цен он выдерживает.
/// Это правило только природного броска (`roll`): лаборатория — укол
/// фрагмента, чан, мутаген, носилки — вживляет сколько угодно (GDD 4.19).
pub const MAX_GENES: usize = 3;
pub const STABILITY: i32 = 6;

/// Модификаторы с прибавками генома: по одному на атрибут. Ставит их мост
/// `biolab_bridge::apply_genome` по `mutants::scaled_stats`; прибавки
/// остаются модификаторами — у них стабильного API нет. Тот же список, что
/// `mutants::STAT_MODIFIERS`.
pub const STAT_MODIFIERS: [&str; 5] = [
    "biolab:genome_health",
    "biolab:genome_speed",
    "biolab:genome_damage",
    "biolab:genome_armor",
    "biolab:genome_knockback",
];

/// Метки генома старого формата (до M11): модификатор дальности
/// преследования с нулевой прибавкой, чьё имя и есть ген. Новых таких меток
/// мод не ставит; список нужен только миграции старых миров.
pub const GENE_MARKERS: [&str; 120] = [
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
];

// ---- Кто бывает мутантом ----
//
// Естественный мутант — любой живой моб, которому при появлении в мире выпал
// геном. Шанс зависит от нрава вида: среди враждебных мутантов больше, чтобы
// ткань добывалась обычной охотой, среди мирных и нейтральных — редкость,
// иначе стадо у дома превращалось бы в зоопарк. Вид узнаём по id, как его
// называют хук-поинты (`minecraft:zombie`): так таблица — чистая функция и
// проверяется тестами без игры.

/// Нрав вида: от него зависят шанс мутации и гены, которые ему годятся.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Temperament {
    /// Нападает на игрока сам.
    Hostile,
    /// Нападает, только если его задели (волк, пчела, эндермен).
    Neutral,
    /// Не нападает никогда: скот, рыбы, летучая мышь.
    Passive,
}

/// Шанс мутации враждебного вида, в процентах.
pub const HOSTILE_PERCENT: u32 = 15;
/// Шанс мутации нейтрального и мирного вида, в процентах.
pub const CALM_PERCENT: u32 = 5;

/// Кто не мутирует никогда, и почему. Таблица явная: незнакомый вид из
/// обновления игры и так не мутирует (`temperament` даёт None), а здесь —
/// осознанные исключения среди известных.
pub const NEVER_MUTANT: [(&str, &str); 15] = [
    ("ender_dragon", "босс"),
    ("wither", "босс"),
    ("elder_guardian", "страж крепости, мини-босс"),
    ("warden", "страж Глубин: и без генома на пределе честного"),
    ("villager", "на нём держатся торговля и деревня"),
    ("wandering_trader", "торговец"),
    ("trader_llama", "ходит с торговцем, его часть"),
    ("iron_golem", "защитник деревни, его строит игрок"),
    ("snow_golem", "его строит игрок"),
    ("copper_golem", "его строит игрок, он помощник"),
    ("allay", "помощник игрока"),
    ("armadillo", "источник щитков: его держат ради ресурса"),
    ("creaking", "умирает только с сердцем — ткань с него не срезать"),
    ("giant", "сам в мире не появляется, только командой"),
    ("mannequin", "манекен, а не существо"),
];

/// Враждебные виды.
pub const HOSTILE_KINDS: [&str; 34] = [
    "zombie", "husk", "drowned", "zombie_villager", "skeleton", "stray", "bogged",
    "parched", "wither_skeleton", "creeper", "spider", "cave_spider", "endermite",
    "silverfish", "slime", "magma_cube", "witch", "phantom", "pillager", "vindicator",
    "evoker", "illusioner", "vex", "ravager", "piglin_brute", "hoglin", "zoglin",
    "ghast", "blaze", "guardian", "shulker", "breeze", "camel_husk", "zombie_nautilus",
];

/// Нейтральные виды: нападают в ответ.
pub const NEUTRAL_KINDS: [&str; 12] = [
    "enderman", "piglin", "zombified_piglin", "wolf", "bee", "polar_bear", "llama",
    "panda", "dolphin", "goat", "pufferfish", "nautilus",
];

/// Мирные виды: не нападают никогда.
pub const PASSIVE_KINDS: [&str; 30] = [
    "pig", "cow", "mooshroom", "sheep", "chicken", "rabbit", "horse", "donkey", "mule",
    "skeleton_horse", "zombie_horse", "camel", "cat", "ocelot", "parrot", "turtle",
    "frog", "tadpole", "axolotl", "fox", "squid", "glow_squid", "cod", "salmon",
    "tropical_fish", "bat", "sniffer", "happy_ghast", "strider", "sulfur_cube",
];

/// Вид без пространства имён: хук-поинты пишут `minecraft:zombie`, сама
/// игра — `zombie`.
fn bare_kind(kind: &str) -> &str {
    kind.strip_prefix("minecraft:").unwrap_or(kind)
}

/// Нрав вида; None — вид не мутирует: исключение или незнакомый вид.
pub fn temperament(kind: &str) -> Option<Temperament> {
    let kind = bare_kind(kind);
    if NEVER_MUTANT.iter().any(|(never, _)| *never == kind) {
        return None;
    }
    if HOSTILE_KINDS.contains(&kind) {
        Some(Temperament::Hostile)
    } else if NEUTRAL_KINDS.contains(&kind) {
        Some(Temperament::Neutral)
    } else if PASSIVE_KINDS.contains(&kind) {
        Some(Temperament::Passive)
    } else {
        None
    }
}

/// Шанс естественной мутации вида, в процентах.
pub fn mutation_percent(kind: &str) -> u32 {
    match temperament(kind) {
        Some(Temperament::Hostile) => HOSTILE_PERCENT,
        Some(Temperament::Neutral | Temperament::Passive) => CALM_PERCENT,
        None => 0,
    }
}

/// Гены, которые мирному мобу при броске не выпадают: атаки у него нет, и ген
/// либо ничего не делает, либо делает чушь.
/// * грубая сила — это только урон;
/// * светобоязнь — «днём не охотится», а он не охотится никогда;
/// * хищник — цель-добычу мирный моб не атакует, его ИИ цель просто бросает;
/// * лояльность — «не трогает людей», он и так не трогает;
/// * рвущие когти — кровотечение от когтей того, кто не бьёт: игрок истекал
///   бы кровью рядом с курицей от чужого удара;
/// * уродства GDD 4.19, которые работают только в бою: полимелия (третья
///   рука бьёт), рогатость (таран), щупальца (подтягивают к себе цель) и
///   хвост (бьёт стоящих сзади — курица хлестала бы игрока, который просто
///   прошёл мимо). Дицефалия, многоглазие, опухоль и перепонки мирным
///   годятся: у них остаются вид, здоровье и планирование, а «замечает
///   игрока» у мирного моба ничего не значит — цель ему не ставится.
///
/// Нейтральные виды атакуют в ответ, им годится весь набор. Уколом, фрагментом
/// или мутагеном игрок может вписать такой ген и мирному — это его опыт, а не
/// бросок, и поведение мутантов (`behaviour.rs` в core) от этого не ломается.
pub const ATTACK_GENES: [&str; 9] = [
    "brute_force",
    "photophobia",
    "predator",
    "loyalty",
    "rending_claws",
    "polymelia",
    "horns",
    "tentacles",
    "tail",
];

/// Годится ли ген виду с таким нравом при естественном броске.
pub fn gene_fits(gene: usize, temperament: Temperament) -> bool {
    let Some(name) = GENE_NAMES.get(gene) else {
        return false;
    };
    temperament != Temperament::Passive || !ATTACK_GENES.contains(name)
}

/// Геном по «броску»: один-два гена с силой 1–3, в пределах стабильности.
/// `percent` — шанс мутации, `temperament` отбирает подходящие гены.
/// Детерминирован (зерно — от id моба), чтобы прогон повторялся.
pub fn roll(draw: u32, percent: u32, temperament: Temperament) -> Vec<(usize, i32)> {
    if draw % 100 >= percent {
        return Vec::new();
    }
    let pool: Vec<usize> = (0..GENE_NAMES.len())
        .filter(|gene| gene_fits(*gene, temperament))
        .collect();
    let count = pool.len() as u32;
    if count < 2 {
        return Vec::new();
    }
    let pick = ((draw / 100) % count) as usize;
    let first = pool[pick];
    // Дорогой ген не может быть сильным: тройные кислотные железы в шесть
    // очков стабильности не укладываются, и такой моб был бы нежизнеспособен.
    let strength = (1 + ((draw / 300) % 3) as i32).min(STABILITY / GENE_COSTS[first]);
    let mut genome = vec![(first, strength)];
    let spent = GENE_COSTS[first] * strength;
    // Второй ген достаётся не каждому: треть бросков обходится одним. Шаг
    // по пулу не бывает нулевым, поэтому второй ген всегда другой.
    if (draw / 900) % 3 != 0 {
        let step = 1 + ((draw / 2700) % (count - 1)) as usize;
        let second = pool[(pick + step) % pool.len()];
        let strength = 1 + ((draw / 8100) % 3) as i32;
        if spent + GENE_COSTS[second] * strength <= STABILITY {
            genome.push((second, strength));
        }
    }
    genome
}

/// Естественный геном моба этого вида; пусто — моб обычный.
pub fn roll_for(kind: &str, draw: u32) -> Vec<(usize, i32)> {
    match temperament(kind) {
        Some(t) => roll(draw, mutation_percent(kind), t),
        None => Vec::new(),
    }
}

/// Цена генома: по ней проверяется стабильность.
pub fn cost(genes: &[(usize, i32)]) -> i32 {
    genes
        .iter()
        .map(|(gene, strength)| GENE_COSTS.get(*gene).copied().unwrap_or(9) * strength)
        .sum()
}

// ---- Данные на сущностях (attachments, M11) ----
//
// Геном мутанта, состояние операционной, отметка «бросок мутации был» и гены
// игрока — свои данные мода на носителе (`docs/native-mods.md` пина, «Server
// attachments»). Типы объявлены в бинарнике (`data.rs`, модуль
// `crate::game::biolab_deep_data`): там виден `Host` игры. Здесь — чистая
// часть контракта: ключи, поля сохранённой формы и их разбор. `biolab-core`
// объявляет те же типы у себя (ключ — пара «пространство имён, имя», и тип
// значения у обоих один и тот же: `String`, `u8`, `bool`), поэтому формы
// должны совпадать до байта — это сверяет `tests/modules.rs`.

/// Пространство имён наших данных.
pub const DATA_NAMESPACE: &str = "biolab";
/// Геном моба (`Holder::Entity`): строка `ген:сила` через запятую, гены — в
/// порядке таблицы. Едет на клиент: по нему окно рисует наросты, оттенок и
/// размер.
pub const GENOME_DATA: &str = "genome";
/// Операционная (`Holder::Entity`): маска срезанных конечностей, «на столе»,
/// «спит». Едет на клиент: поза «лёжа» и кости.
pub const SURGERY_DATA: &str = "surgery";
/// «Бросок мутации уже был» (`Holder::Entity`): только сервер.
pub const CHECKED_DATA: &str = "mutation_checked";
/// Гены игрока (`Holder::Player`): та же строка, что у генома моба. Едет на
/// клиент и переживает смерть игрока.
pub const PLAYER_GENES_DATA: &str = "player_genes";

/// Поле генома в сохранённой форме: `{genes: "horns:2,tail:1"}`.
pub const GENES_FIELD: &str = "genes";
/// Поле отметки броска: `{checked: 1b}`.
pub const CHECKED_FIELD: &str = "checked";
/// Поля операционной: `{flayed: 5b, table: 1b, asleep: 0b}`.
pub const FLAYED_FIELD: &str = "flayed";
pub const TABLE_FIELD: &str = "table";
pub const ASLEEP_FIELD: &str = "asleep";

/// Биты состояния операционной в значении `u8`: нижние четыре — срезанные
/// конечности по номерам, потом «на столе» и «спит под снотворным».
pub const FLAYED_BITS: u8 = 0b1111;
pub const TABLE_BIT: u8 = 1 << 4;
pub const ASLEEP_BIT: u8 = 1 << 5;

/// Геном строкой для данных: гены в порядке таблицы, у каждого — самая
/// сильная из названных сил, вне 1–3 и незнакомое отброшено. Одна и та же
/// запись генома всегда даёт одну и ту же строку: так «изменилось ли» —
/// сравнение строк, и лишней синхронизации нет.
pub fn genome_text(genes: &[(usize, i32)]) -> String {
    let mut best = [0i32; GENE_NAMES.len()];
    for (gene, strength) in genes {
        if let Some(slot) = best.get_mut(*gene)
            && (1..=3).contains(strength)
        {
            *slot = (*slot).max(*strength);
        }
    }
    best.iter()
        .enumerate()
        .filter(|(_, s)| **s > 0)
        .map(|(gene, s)| format!("{}:{s}", GENE_NAMES[gene]))
        .collect::<Vec<_>>()
        .join(",")
}

/// Геном из строки данных. Незнакомый ген и битую пару пропускаем: мир от
/// новой версии мода не должен ломать старую.
pub fn genome_from_text(text: &str) -> Vec<(usize, i32)> {
    let mut best = [0i32; GENE_NAMES.len()];
    for part in text.split(',') {
        let Some((name, strength)) = part.trim().split_once(':') else {
            continue;
        };
        let (Some(gene), Ok(strength)) =
            (GENE_NAMES.iter().position(|n| *n == name), strength.parse::<i32>())
        else {
            continue;
        };
        if (1..=3).contains(&strength) {
            best[gene] = best[gene].max(strength);
        }
    }
    best.iter()
        .enumerate()
        .filter(|(_, s)| **s > 0)
        .map(|(gene, s)| (gene, *s))
        .collect()
}

/// Геном для отрисовки и прибавок: номер гена и сила байтом.
pub fn strengths(genes: &[(usize, i32)]) -> Vec<(usize, u8)> {
    genes
        .iter()
        .map(|(gene, strength)| (*gene, (*strength).clamp(0, 3) as u8))
        .collect()
}

/// Гены по именам — так их ждут модели игроков (`biolab_players`).
pub fn named(genes: &[(usize, i32)]) -> Vec<(String, u8)> {
    genes
        .iter()
        .filter_map(|(gene, strength)| {
            Some(((*GENE_NAMES.get(*gene)?).to_owned(), (*strength).clamp(0, 3) as u8))
        })
        .collect()
}

/// Состояние операционной полями сохранённой формы.
pub fn surgery_fields(bits: u8) -> [(&'static str, i8); 3] {
    [
        (FLAYED_FIELD, (bits & FLAYED_BITS) as i8),
        (TABLE_FIELD, i8::from(bits & TABLE_BIT != 0)),
        (ASLEEP_FIELD, i8::from(bits & ASLEEP_BIT != 0)),
    ]
}

/// Состояние операционной из полей; `field` — байт поля, если он есть.
/// Недостающее поле — ноль: так читается и форма, записанная до того, как
/// поле появилось.
pub fn surgery_from_fields(field: impl Fn(&str) -> Option<i8>) -> u8 {
    let flag = |name: &str, bit: u8| if field(name).unwrap_or(0) != 0 { bit } else { 0 };
    (field(FLAYED_FIELD).unwrap_or(0) as u8 & FLAYED_BITS)
        | flag(TABLE_FIELD, TABLE_BIT)
        | flag(ASLEEP_FIELD, ASLEEP_BIT)
}

// ---- Старые метки: только для миграции ----
//
// До M11 всё это хранилось метками — модификаторами `FollowRange` с нулевой
// прибавкой. Horizon при загрузке модификаторы не читает, поэтому метки
// старого мира лежат только в сыром NBT моба (`persist.raw`). Миграция
// (`data.rs`) читает их оттуда один раз, кладёт в данные и вычёркивает из
// сырого NBT, а следующий сейв пишет моба уже без них.

/// Метка гена старого формата.
pub fn marker(gene: usize, strength: i32) -> Option<&'static str> {
    if gene >= GENE_NAMES.len() || !(1..=3).contains(&strength) {
        return None;
    }
    GENE_MARKERS.get(gene * 3 + (strength - 1) as usize).copied()
}

/// Ген по метке старого формата.
pub fn gene_of_marker(marker: &str) -> Option<(usize, i32)> {
    let rest = marker.strip_prefix("biolab:gene/")?;
    let (name, strength) = rest.rsplit_once('/')?;
    let gene = GENE_NAMES.iter().position(|n| *n == name)?;
    let strength = strength.parse::<i32>().ok()?;
    (1..=3).contains(&strength).then_some((gene, strength))
}

/// Старая отметка «бросок мутации уже был».
pub const CHECKED_MARKER: &str = "biolab:mutation_checked";

/// Старые метки операционной: стол и срезанные конечности по номерам.
/// `biolab:surgery/slow` сюда не входит: это настоящая прибавка к скорости,
/// её пересчитывает тик операционной в `biolab-core`.
pub const TABLE_MARKER: &str = "biolab:surgery/table";
pub const FLAYED_MARKERS: [&str; 4] = [
    "biolab:surgery/flayed/0",
    "biolab:surgery/flayed/1",
    "biolab:surgery/flayed/2",
    "biolab:surgery/flayed/3",
];

/// Метка ли это старого формата (а не настоящая прибавка).
pub fn is_legacy_marker(id: &str) -> bool {
    id == CHECKED_MARKER
        || id == TABLE_MARKER
        || FLAYED_MARKERS.contains(&id)
        || gene_of_marker(id).is_some()
}

/// Что нашлось в старых метках моба.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Legacy {
    /// Гены строкой данных (`genome_text`); пусто — генов не было.
    pub genome: String,
    /// Биты операционной (`FLAYED_BITS`, `TABLE_BIT`).
    pub surgery: u8,
    pub checked: bool,
}

/// Разбор старых меток по id модификаторов моба. None — меток нет совсем.
/// Сумма меток и операция не важны: призванный командой моб с `amount:0`
/// (целое) или без `operation` раньше молча терял метки — теперь их узнаём
/// по имени.
pub fn read_legacy<'a>(ids: impl IntoIterator<Item = &'a str>) -> Option<Legacy> {
    let mut found = false;
    let mut genes = Vec::new();
    let mut legacy = Legacy::default();
    for id in ids {
        if let Some(gene) = gene_of_marker(id) {
            genes.push(gene);
        } else if id == CHECKED_MARKER {
            legacy.checked = true;
        } else if id == TABLE_MARKER {
            legacy.surgery |= TABLE_BIT;
        } else if let Some(limb) = FLAYED_MARKERS.iter().position(|m| *m == id) {
            legacy.surgery |= 1 << limb;
        } else {
            continue;
        }
        found = true;
    }
    legacy.genome = genome_text(&genes);
    found.then_some(legacy)
}

// ---- Проход мутантов на сервере ----
//
// Прибавки к атрибутам и поведение по генам (огонь, яды, планирование,
// регенерация) считает бинарник перед тиком мобов (`mutant_tick.rs`, хук
// `native_entity_tick_listened`, API 146): там видны и данные, и мобы. Хук
// спрашивается раз на семейство сущностей (мобы, предметы, вагонетки, лодки)
// за тик уровня, поэтому проход отсекается по «уровень, тик».

/// Каждые столько тиков проход перечитывает данные всех мобов: прибавки,
/// «спит», миграция отставших. Между переборами поведение идёт по запомненным
/// геномам — читать данные каждого моба каждый тик дорого: игра ищет носитель
/// перебором списка мобов.
pub const SCAN_EVERY: u64 = 10;

fn passes() -> &'static std::sync::Mutex<std::collections::HashMap<String, u64>> {
    static AT: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, u64>>> =
        std::sync::OnceLock::new();
    AT.get_or_init(Default::default)
}

/// Первый ли вызов прохода в тике `ticks` уровня `dimension`.
pub fn first_pass(dimension: &str, ticks: u64) -> bool {
    let Ok(mut at) = passes().lock() else {
        return false;
    };
    at.insert(dimension.to_owned(), ticks) != Some(ticks)
}

/// Пора ли перечитать данные всех мобов.
pub fn scan_due(ticks: u64) -> bool {
    ticks % SCAN_EVERY == 0
}

type MutantTable = std::collections::HashMap<String, Vec<(u32, Vec<(usize, u8)>)>>;

fn mutant_table() -> &'static std::sync::Mutex<MutantTable> {
    static AT: std::sync::OnceLock<std::sync::Mutex<MutantTable>> = std::sync::OnceLock::new();
    AT.get_or_init(Default::default)
}

/// Запомнить мутантов уровня до следующего перебора. Это не источник
/// правды, а выжимка из данных: она живёт `SCAN_EVERY` тиков и только на
/// сервере, окно её не читает.
pub fn remember_mutants(dimension: &str, list: Vec<(u32, Vec<(usize, u8)>)>) {
    if let Ok(mut table) = mutant_table().lock() {
        table.insert(dimension.to_owned(), list);
    }
}

/// Мутанты уровня, запомненные последним перебором.
pub fn mutants(dimension: &str) -> Vec<(u32, Vec<(usize, u8)>)> {
    mutant_table()
        .lock()
        .ok()
        .and_then(|t| t.get(dimension).cloned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genome_text_is_canonical() {
        let horns = GENE_NAMES.iter().position(|n| *n == "horns").unwrap();
        let chitin = GENE_NAMES.iter().position(|n| *n == "chitin").unwrap();
        // Порядок таблицы, сильнейшая сила, мусор отброшен.
        let text = genome_text(&[(horns, 1), (chitin, 2), (horns, 3), (99, 1), (chitin, 7)]);
        assert_eq!(text, "chitin:2,horns:3");
        assert_eq!(genome_from_text(&text), vec![(chitin, 2), (horns, 3)]);
        assert_eq!(genome_text(&genome_from_text(&text)), text);
        // Незнакомое и битое пропускается, а не роняет весь геном.
        assert_eq!(genome_from_text("ghost:1,horns:2,tail,chitin:x,tail:9"), vec![(horns, 2)]);
        assert!(genome_from_text("").is_empty());
        assert_eq!(genome_text(&[]), "");
        assert_eq!(named(&[(horns, 2)]), vec![("horns".to_owned(), 2)]);
        assert_eq!(strengths(&[(horns, 2)]), vec![(horns, 2u8)]);
    }

    #[test]
    fn surgery_fields_round_trip() {
        for bits in 0..=(FLAYED_BITS | TABLE_BIT | ASLEEP_BIT) {
            let fields = surgery_fields(bits);
            let back = surgery_from_fields(|name| {
                fields.iter().find(|(f, _)| *f == name).map(|(_, v)| *v)
            });
            assert_eq!(back, bits, "{bits:#b}");
        }
        assert_eq!(
            surgery_fields(ASLEEP_BIT | 0b0101),
            [(FLAYED_FIELD, 5), (TABLE_FIELD, 0), (ASLEEP_FIELD, 1)]
        );
        // Формы без поля — ноль, лишние биты маски срезов не проходят.
        assert_eq!(surgery_from_fields(|_| None), 0);
        assert_eq!(surgery_from_fields(|f| (f == FLAYED_FIELD).then_some(-1)), FLAYED_BITS);
    }

    #[test]
    fn old_markers_become_data() {
        let ids = [
            "minecraft:random_spawn_bonus",
            "biolab:gene/horns/1",
            "biolab:gene/chitin/2",
            "biolab:gene/horns/3",
            "biolab:genome_health",
            "biolab:surgery/table",
            "biolab:surgery/flayed/2",
            "biolab:surgery/slow",
            "biolab:mutation_checked",
        ];
        let legacy = read_legacy(ids).expect("метки есть");
        assert_eq!(legacy.genome, "chitin:2,horns:3");
        assert_eq!(legacy.surgery, TABLE_BIT | 0b100);
        assert!(legacy.checked);
        // Настоящие прибавки — не метки: их снимать нельзя.
        assert!(!is_legacy_marker("biolab:genome_health"));
        assert!(!is_legacy_marker("biolab:surgery/slow"));
        assert!(!is_legacy_marker("minecraft:random_spawn_bonus"));
        for marker in GENE_MARKERS
            .iter()
            .chain(&FLAYED_MARKERS)
            .chain([&TABLE_MARKER, &CHECKED_MARKER])
        {
            assert!(is_legacy_marker(marker), "{marker}");
        }
        // Только отметка броска: моб проверен, генома нет.
        let checked = read_legacy(["biolab:mutation_checked"]).unwrap();
        assert_eq!(checked, Legacy { genome: String::new(), surgery: 0, checked: true });
        assert_eq!(read_legacy(["minecraft:random_spawn_bonus", "biolab:genome_speed"]), None);
        assert_eq!(read_legacy([]), None);
    }

    #[test]
    fn the_pass_runs_once_a_tick_per_level() {
        assert!(first_pass("test:a", 7));
        assert!(!first_pass("test:a", 7), "второе семейство того же тика");
        assert!(first_pass("test:b", 7), "другой уровень — свой проход");
        assert!(first_pass("test:a", 8));
        assert!(scan_due(0) && scan_due(SCAN_EVERY) && !scan_due(SCAN_EVERY + 1));
        remember_mutants("test:a", vec![(5, vec![(1, 2)])]);
        assert_eq!(mutants("test:a"), vec![(5, vec![(1, 2)])]);
        assert!(mutants("test:c").is_empty());
    }

    const ALL_TEMPERS: [Temperament; 3] =
        [Temperament::Hostile, Temperament::Neutral, Temperament::Passive];

    #[test]
    fn a_roll_stays_inside_stability() {
        for temper in ALL_TEMPERS {
            for draw in 0..5000 {
                let genome = roll(draw, 100, temper);
                assert!(!genome.is_empty(), "при шансе 100% мутируют все: бросок {draw}");
                assert!(genome.len() <= MAX_GENES);
                assert!(cost(&genome) <= STABILITY, "перебор на броске {draw}: {genome:?}");
                for (gene, strength) in &genome {
                    assert!(*gene < GENE_NAMES.len());
                    assert!((1..=3).contains(strength));
                    assert!(gene_fits(*gene, temper), "{temper:?}: {}", GENE_NAMES[*gene]);
                }
                // Один ген дважды в геном не попадает.
                for (i, (gene, _)) in genome.iter().enumerate() {
                    assert!(!genome[i + 1..].iter().any(|(other, _)| other == gene), "{genome:?}");
                }
            }
        }
        assert_eq!(roll(7, 100, Temperament::Hostile), roll(7, 100, Temperament::Hostile), "бросок детерминирован");
        assert!(roll(7, 0, Temperament::Hostile).is_empty(), "при шансе 0% мутантов нет");
    }

    #[test]
    fn some_rolls_give_two_genes() {
        for temper in ALL_TEMPERS {
            let pairs = (0..5000).filter(|d| roll(*d, 100, temper).len() == 2).count();
            assert!(pairs > 500, "{temper:?}: двойных геномов всего {pairs}");
        }
    }

    /// Все виды Horizon (`mobs/species.rs`, 26.3) и дракон, который живёт
    /// отдельно от мобов. Новый вид из обновления игры надо разобрать сюда.
    const HORIZON_KINDS: [&str; 89] = [
        "pig", "cow", "mooshroom", "sheep", "chicken", "rabbit", "zombie_horse",
        "skeleton_horse", "giant", "horse", "donkey", "mule", "llama", "trader_llama",
        "goat", "panda", "polar_bear", "fox", "wolf", "cat", "ocelot", "parrot", "bee",
        "turtle", "armadillo", "frog", "axolotl", "squid", "glow_squid", "tadpole", "cod",
        "salmon", "tropical_fish", "pufferfish", "dolphin", "bat", "villager",
        "wandering_trader", "iron_golem", "snow_golem", "copper_golem", "zombie", "husk",
        "drowned", "zombie_villager", "skeleton", "stray", "parched", "bogged",
        "wither_skeleton", "creeper", "spider", "cave_spider", "enderman", "endermite",
        "silverfish", "slime", "magma_cube", "witch", "phantom", "evoker", "illusioner",
        "allay", "creaking", "camel", "happy_ghast", "sniffer", "breeze", "wither",
        "ravager", "vex", "shulker", "guardian", "elder_guardian", "pillager", "vindicator",
        "piglin", "piglin_brute", "zombified_piglin", "hoglin", "zoglin", "ghast", "blaze",
        "strider", "warden", "camel_husk", "nautilus", "zombie_nautilus", "sulfur_cube",
    ];

    #[test]
    fn every_kind_is_sorted_exactly_once() {
        let mut problems = Vec::new();
        for kind in HORIZON_KINDS.iter().chain(["mannequin", "ender_dragon"].iter()) {
            let places = [
                NEVER_MUTANT.iter().any(|(k, _)| k == kind),
                HOSTILE_KINDS.contains(kind),
                NEUTRAL_KINDS.contains(kind),
                PASSIVE_KINDS.contains(kind),
            ]
            .iter()
            .filter(|found| **found)
            .count();
            if places != 1 {
                problems.push(format!("{kind}: в таблицах {places} раз"));
            }
        }
        let tables = NEVER_MUTANT.len() + HOSTILE_KINDS.len() + NEUTRAL_KINDS.len() + PASSIVE_KINDS.len();
        assert_eq!(tables, HORIZON_KINDS.len() + 2, "в таблицах есть вид, которого нет в игре");
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    #[test]
    fn chance_depends_on_temperament() {
        assert_eq!(mutation_percent("minecraft:zombie"), HOSTILE_PERCENT);
        assert_eq!(mutation_percent("creeper"), HOSTILE_PERCENT);
        assert_eq!(mutation_percent("minecraft:wolf"), CALM_PERCENT);
        assert_eq!(mutation_percent("minecraft:cow"), CALM_PERCENT);
        assert_eq!(temperament("minecraft:cow"), Some(Temperament::Passive));
        assert_eq!(temperament("minecraft:enderman"), Some(Temperament::Neutral));
        for (never, why) in NEVER_MUTANT {
            assert_eq!(mutation_percent(never), 0, "{never}");
            assert_eq!(mutation_percent(&format!("minecraft:{never}")), 0, "{never}");
            assert!(roll_for(never, 0).is_empty(), "{never}");
            assert!(!why.is_empty());
        }
        assert_eq!(mutation_percent("minecraft:player"), 0, "незнакомый вид не мутирует");
        assert!(HOSTILE_PERCENT > CALM_PERCENT, "враждебных мутантов больше");
    }

    /// Доля мутантов на тех же зёрнах, что в игре (`joined.rs`): id подряд,
    /// умноженный на константу Кнута.
    #[test]
    fn natural_share_is_close_to_the_table() {
        let share = |kind: &str| {
            let mutants = (0u32..20_000)
                .filter(|id| !roll_for(kind, id.wrapping_mul(2_654_435_761)).is_empty())
                .count();
            mutants as f64 / 20_000.0 * 100.0
        };
        let zombie = share("minecraft:zombie");
        let cow = share("minecraft:cow");
        assert!((12.0..18.0).contains(&zombie), "зомби-мутантов {zombie:.1}%");
        assert!((3.0..7.0).contains(&cow), "коров-мутантов {cow:.1}%");
        assert_eq!(share("minecraft:villager"), 0.0);
    }

    #[test]
    fn passive_mobs_never_roll_attack_genes() {
        for name in ATTACK_GENES {
            let gene = GENE_NAMES.iter().position(|n| *n == name).expect(name);
            assert!(!gene_fits(gene, Temperament::Passive), "{name}");
            assert!(gene_fits(gene, Temperament::Neutral), "{name}");
            assert!(gene_fits(gene, Temperament::Hostile), "{name}");
        }
        let mut seen = [false; GENE_NAMES.len()];
        for draw in 0..200_000 {
            for (gene, _) in roll(draw, 100, Temperament::Passive) {
                assert!(!ATTACK_GENES.contains(&GENE_NAMES[gene]), "корове выпал {}", GENE_NAMES[gene]);
                seen[gene] = true;
            }
        }
        // Остальные гены мирным достаются: фильтр не выкосил лишнего.
        let missing: Vec<&str> = (0..GENE_NAMES.len())
            .filter(|g| !seen[*g] && gene_fits(*g, Temperament::Passive))
            .map(|g| GENE_NAMES[g])
            .collect();
        assert!(missing.is_empty(), "мирным не выпадают: {missing:?}");
        assert!(!gene_fits(GENE_NAMES.len(), Temperament::Hostile));
    }

    #[test]
    fn markers_cover_every_gene_and_strength() {
        for gene in 0..GENE_NAMES.len() {
            for strength in 1..=3 {
                let m = marker(gene, strength).expect("метка есть");
                assert_eq!(gene_of_marker(m), Some((gene, strength)));
                assert!(is_legacy_marker(m), "{m}");
            }
        }
        assert_eq!(marker(GENE_NAMES.len(), 1), None);
        assert_eq!(marker(0, 4), None);
        assert_eq!(gene_of_marker("biolab:genome_health"), None);
    }
}
