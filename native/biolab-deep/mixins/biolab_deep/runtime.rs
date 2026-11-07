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
pub const GENE_NAMES: [&str; 54] = [
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
    // M5 (GDD 4.20): масса для снотворного +1 за силу; считает `biolab-core`.
    "sedation_resistance",
    // M9: пропорции (GDD 4.24) - форма частей тела; рисует `biolab_proportions`.
    "long_arms",
    "heavy_arms",
    "macrocephaly",
    "microcephaly",
    "short_legs",
    "stilt_legs",
    "bloated_torso",
    "sunken_chest",
    "asymmetry",
    // M9: предохранители (GDD 4.23 п. 10) - ослабляют творение;
    // срок и вид ведёт `biolab-core`, аттенуацию - `expressed` здесь.
    "self_limiting",
    "host_restriction",
    "kill_switch",
    "attenuated",
];

/// Цена гена за единицу силы (та же, что в lib).
pub const GENE_COSTS: [i32; 54] = [2, 1, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1, 2, 1, 1, 2, 3, 2, 2, 1, 2, 2, 3, 3, 2, 2, 3, 3, 2, 2, 2, 3, 3, 3, 2, 2, 3, 2, 1, 2, 2, 2, 2, 1, 1, 1, 2, 2, 1, 1, 1, 1, 1, 1];

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
pub const GENE_MARKERS: [&str; 162] = [
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
    // M9: предохранители (GDD 4.23, п. 10)
    "biolab:gene/self_limiting/1",
    "biolab:gene/self_limiting/2",
    "biolab:gene/self_limiting/3",
    "biolab:gene/host_restriction/1",
    "biolab:gene/host_restriction/2",
    "biolab:gene/host_restriction/3",
    "biolab:gene/kill_switch/1",
    "biolab:gene/kill_switch/2",
    "biolab:gene/kill_switch/3",
    "biolab:gene/attenuated/1",
    "biolab:gene/attenuated/2",
    "biolab:gene/attenuated/3",
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
        // Свои виды мода (раздел «M11-А: Заражённый»).
        mod_temperament(kind)
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
    // M9 (GDD 4.23 п. 10): предохранители природе не выпадают — это
    // лабораторные кассеты, а не признак живого организма.
    let pool: Vec<usize> = (0..GENE_NAMES.len())
        .filter(|gene| gene_fits(*gene, temperament) && !is_safeguard(*gene))
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
        // M9 (GDD 4.24): природный мутант несёт не больше одной черты
        // пропорций; (GDD 4.23 п. 5) несовместимой пары в природе не бывает —
        // иначе мутант рождался бы мёртвым.
        if spent + GENE_COSTS[second] * strength <= STABILITY
            && !(is_proportion(first) && is_proportion(second))
            && pair_kind(first, second) != Some(PAIR_LETHAL)
        {
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

/// Где в сыром NBT моба искать старые метки. Сейв 1.21+ пишет
/// `attributes`/`modifiers`/`id`, а `/summon` хранится как его набрали — в
/// том числе старой формой `Attributes`/`Modifiers`/`Name` (так призывал
/// мутанта тест в игре 04.11.2026, и метки не находились).
pub const ATTRIBUTES_KEYS: [&str; 2] = ["attributes", "Attributes"];
pub const MODIFIERS_KEYS: [&str; 2] = ["modifiers", "Modifiers"];
pub const MODIFIER_ID_KEYS: [&str; 2] = ["id", "Name"];

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

// ---- M11-Б/В: урон и наследование (GDD 4.26, В) ----
//
// Урон (Б) целиком в `biolab-core` (`effects.rs`, `damage.rs`). Здесь —
// гены по наследству: детёныш мутантов и моб, превратившийся в другой вид.
//
// *Потомство.* Игра разводит животных в `Game::breed` (`Animal.
// spawnChildFromBreeding`): сначала спрашивает `native_baby_entity_spawn`
// (API 395), потом тут же, в том же тике, создаёт детёныша в точке
// родителя (`spawn_mob`), и он проходит `native_mobs_joined` — там, где
// природный бросок (`joined.rs`). Поэтому рождение запоминается заранее
// (`Births::expect`: геномы обоих родителей, точка, тик), а `joined.rs`
// узнаёт детёныша по точке и тику (`Births::claim`) и вместо броска пишет
// геном по наследству (`inherit`). Бросок такой геном не перезаписывает:
// детёныш получает и геном, и отметку «бросок был».
//
// Важно: игра зовёт `native_baby_entity_spawn` только при загруженном
// трейт-моде (`creatures.rs` пина: `!mod_api::mods().is_empty()`), а наши
// бандлы — миксины. Поэтому рождение запоминает и HEAD на саму `breed`
// (внутренняя функция, при обновлении Horizon проверить); два вызова одного
// рождения сливаются в одну запись.
//
// *Превращение* (`native_living_conversion_post`, API 390): житель в
// зомби-жителя, свинья от молнии, зомби в утопленника, грибная корова и т. п.
// Новый моб к этому моменту уже прошёл `native_mobs_joined` (свой бросок),
// а старый снят с уровня — его данные `Host` уже не отдаёт, их читает
// `heredity.rs` из памяти игры по uuid. Правило — `converted_genome` и
// `converted_surgery`.

/// Один из стольких унаследованных генов ослабевает на ступень.
pub const WEAKEN_IN: u64 = 4;
/// Сколько тиков рождение ждёт своего детёныша. Игра создаёт его в том же
/// вызове, что и спрашивает хук, так что хватает и нуля; запас — на случай,
/// если между ними когда-нибудь появится отложенное событие.
pub const BIRTH_TICKS: u64 = 2;
/// Как далеко от родителя может появиться детёныш, блоков: игра ставит его
/// ровно в точку родителя.
pub const BIRTH_REACH: f64 = 0.75;
/// Предел ожидающих рождений: если их никто не забирает, список не растёт.
pub const MAX_BIRTHS: usize = 64;

/// Перемешать биты зерна (splitmix64): соседние id дают несвязанные броски.
fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Зерно наследства: от детёныша и пары родителей (порядок пары неважен).
/// Детерминировано — одна и та же пара с тем же детёнышем даёт тот же геном.
pub fn heir_seed(baby: u32, parent: u32, partner: u32) -> u64 {
    let (a, b) = (parent.min(partner), parent.max(partner));
    mix(u64::from(baby) ^ mix((u64::from(a) << 32) | u64::from(b)))
}

/// Геном детёныша по геномам родителей. По каждому гену, который есть хотя
/// бы у одного из них, — бросок: от кого взять (у второго гена может не
/// быть — тогда он не передаётся) и не ослабнуть ли на ступень (один из
/// `WEAKEN_IN`). Ген, который есть у обоих, передаётся всегда, хотя может
/// ослабнуть; ген одного родителя — через раз. Так разведение двух мутантов
/// с одним геном закрепляет его, а случайная пара его теряет. Предела
/// стабильности нет: это не природный бросок, а разведение (как и укол
/// фрагмента, GDD 4.19).
pub fn inherit(mother: &[(usize, i32)], father: &[(usize, i32)], seed: u64) -> Vec<(usize, i32)> {
    let mut strengths = [[0i32; 2]; GENE_NAMES.len()];
    for (side, genome) in [mother, father].into_iter().enumerate() {
        for (gene, strength) in genome {
            if let Some(slot) = strengths.get_mut(*gene) {
                slot[side] = slot[side].max((*strength).clamp(0, 3));
            }
        }
    }
    let mut out = Vec::new();
    for (gene, [a, b]) in strengths.iter().enumerate() {
        if *a == 0 && *b == 0 {
            continue;
        }
        let draw = mix(seed ^ (gene as u64 + 1).wrapping_mul(0xD6E8_FEB8_6659_FD93));
        let mut strength = if draw & 1 == 0 { *a } else { *b };
        if strength > 0 && (draw >> 1) % WEAKEN_IN == 0 {
            strength -= 1;
        }
        if strength > 0 {
            out.push((gene, strength));
        }
    }
    // M9 (GDD 4.23 п. 5): у детёныша пары работают — мелкий разлад снимает
    // ступень, крупный дорастит уродство. Несовместимую пару не отменяем:
    // такой детёныш не жилец, и его добьёт тик мира `biolab-core`.
    apply_pairs(out, seed)
}

/// Вылупляется из яйца, а не рождается: черепаха, лягушка (головастик из
/// икры), нюхач. Их `breed` детёныша не создаёт — и ждать его незачем.
pub fn hatches_from_egg(kind: &str) -> bool {
    matches!(kind.trim_start_matches("minecraft:"), "turtle" | "frog" | "sniffer")
}

/// Ожидаемое рождение: где и когда появится детёныш и что у родителей.
#[derive(Clone, Debug, PartialEq)]
pub struct Birth {
    pub dimension: String,
    /// Точка родителя: детёныш появляется в ней.
    pub at: [f64; 3],
    /// `game.ticks` в момент рождения.
    pub tick: u64,
    pub parents: (u32, u32),
    /// Геномы родителей строками данных (`genome_text`), в порядке пары.
    pub genomes: (String, String),
}

impl Birth {
    /// Геном детёныша `baby` строкой данных; пусто — гены не передались.
    pub fn heir(&self, baby: u32) -> String {
        let seed = heir_seed(baby, self.parents.0, self.parents.1);
        genome_text(&inherit(&genome_from_text(&self.genomes.0), &genome_from_text(&self.genomes.1), seed))
    }
}

/// Рождения, ждущие своего детёныша.
#[derive(Debug, Default)]
pub struct Births {
    list: Vec<Birth>,
}

fn same_pair(a: (u32, u32), b: (u32, u32)) -> bool {
    a == b || a == (b.1, b.0)
}

fn fresh(born: u64, now: u64) -> bool {
    now >= born && now - born <= BIRTH_TICKS
}

impl Births {
    /// Запомнить рождение. Устаревшие и прежняя запись той же пары
    /// уходят (два хука одного рождения дают одну запись). Пара обычных
    /// мобов не запоминается: наследовать нечего, детёныш получит обычный
    /// природный бросок.
    pub fn expect(&mut self, birth: Birth) {
        self.list
            .retain(|b| fresh(b.tick, birth.tick) && !same_pair(b.parents, birth.parents));
        if genome_from_text(&birth.genomes.0).is_empty() && genome_from_text(&birth.genomes.1).is_empty() {
            return;
        }
        if self.list.len() >= MAX_BIRTHS {
            self.list.remove(0);
        }
        self.list.push(birth);
    }

    /// Новый моб в точке `at` уровня `dimension` в тик `tick` — чей-то
    /// детёныш? Ближайшее подходящее рождение забирается целиком: второй
    /// моб в той же точке его уже не получит.
    pub fn claim(&mut self, dimension: &str, at: [f64; 3], tick: u64) -> Option<Birth> {
        let near = |b: &Birth| {
            let d = [b.at[0] - at[0], b.at[1] - at[1], b.at[2] - at[2]];
            d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
        };
        let index = self
            .list
            .iter()
            .enumerate()
            .filter(|(_, b)| b.dimension == dimension && fresh(b.tick, tick) && near(b) <= BIRTH_REACH * BIRTH_REACH)
            .min_by(|(_, a), (_, b)| near(a).total_cmp(&near(b)))?
            .0;
        Some(self.list.remove(index))
    }

    /// Сколько рождений ждёт.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

/// Общий список рождений: запоминает хук разведения, забирает `joined.rs`.
/// Только сервер.
pub fn births() -> &'static std::sync::Mutex<Births> {
    static AT: std::sync::OnceLock<std::sync::Mutex<Births>> = std::sync::OnceLock::new();
    AT.get_or_init(Default::default)
}

/// Геном нового моба после превращения: `Some` — записать (пустая строка —
/// снять геном, который новому успел выпасть броском), `None` — оставить как
/// есть. Превращение сохраняет моба, а не бросает заново: мутант остаётся
/// мутантом, обычный (бросок у него уже был) — обычным. Только моб, у
/// которого броска не было вовсе (появился до мода), получает бросок
/// нового вида.
pub fn converted_genome(old: Option<&str>, old_checked: bool) -> Option<String> {
    let genes = genome_from_text(old.unwrap_or(""));
    // M9 (GDD 4.23 п. 10): видоспецифичность — кассета собрана под один вид.
    // Сменил вид (утонул, попал под молнию, стал грибной коровой) — весь
    // геном гаснет: конструкт в новом хозяине не работает.
    if genes.iter().any(|(gene, _)| GENE_NAMES.get(*gene) == Some(&"host_restriction")) {
        return Some(String::new());
    }
    if !genes.is_empty() {
        return Some(genome_text(&genes));
    }
    old_checked.then(String::new)
}

/// Что из операционной переживает превращение: срезанные конечности. Стол
/// и сон — нет: стол помнит своего пациента, а сон проход мутантов
/// пересчитывает сам по эффекту.
pub fn converted_surgery(bits: u8) -> u8 {
    bits & FLAYED_BITS
}

/// Сколько тиков превращение ждёт нового моба. Молния (`lightning.rs` пина)
/// снимает старого и ставит нового отложенным событием уровня — в тот же
/// тик или следующий; запас — на очередь событий.
pub const CONVERSION_TICKS: u64 = 5;

/// Ожидаемое превращение (`native_living_conversion_pre`): кем станет моб,
/// где и что ему достанется. Нужно для молнии — свинья в зомбифицированного
/// пиглина, житель в ведьму: там пин зовёт только `pre`, а `post` — нет
/// (новый моб ставится событием, без `convert_to`). Для обычного
/// `convert_to` запись тоже появляется; `post` (`converted.rs`) потом пишет
/// то же самое ещё раз — это безвредно.
#[derive(Clone, Debug, PartialEq)]
pub struct Conversion {
    pub dimension: String,
    /// Точка старого моба: новый появляется в ней.
    pub at: [f64; 3],
    pub tick: u64,
    /// Вид, которым моб станет (`minecraft:zombified_piglin`).
    pub outcome: String,
    /// Что записать новому (`converted_genome`); `None` — оставить его бросок.
    pub genome: Option<String>,
    /// Срезы операционной (`converted_surgery`).
    pub surgery: u8,
}

/// Превращения, ждущие нового моба. Только сервер.
#[derive(Debug, Default)]
pub struct Conversions {
    list: Vec<Conversion>,
}

impl Conversions {
    /// Запомнить превращение; устаревшие уходят. Обычному мобу без броска и
    /// срезов передавать нечего — не запоминаем.
    pub fn expect(&mut self, conversion: Conversion) {
        let now = conversion.tick;
        self.list
            .retain(|c| now >= c.tick && now - c.tick <= CONVERSION_TICKS);
        if conversion.genome.is_none() && conversion.surgery == 0 {
            return;
        }
        if self.list.len() >= MAX_BIRTHS {
            self.list.remove(0);
        }
        self.list.push(conversion);
    }

    /// Новый моб вида `kind` в точке `at` — чьё-то превращение? Ближайшее
    /// подходящее забирается целиком.
    pub fn claim(&mut self, dimension: &str, kind: &str, at: [f64; 3], tick: u64) -> Option<Conversion> {
        let near = |c: &Conversion| {
            let d = [c.at[0] - at[0], c.at[1] - at[1], c.at[2] - at[2]];
            d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
        };
        let index = self
            .list
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.dimension == dimension
                    && c.outcome == kind
                    && tick >= c.tick
                    && tick - c.tick <= CONVERSION_TICKS
                    && near(c) <= BIRTH_REACH * BIRTH_REACH
            })
            .min_by(|(_, a), (_, b)| near(a).total_cmp(&near(b)))?
            .0;
        Some(self.list.remove(index))
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

/// Общий список превращений: запоминает `conversion_pre.rs`, забирает
/// `joined.rs`.
pub fn conversions() -> &'static std::sync::Mutex<Conversions> {
    static AT: std::sync::OnceLock<std::sync::Mutex<Conversions>> = std::sync::OnceLock::new();
    AT.get_or_init(Default::default)
}

#[cfg(test)]
mod conversion_tests {
    use super::*;

    fn pig(tick: u64, genome: Option<&str>) -> Conversion {
        Conversion {
            dimension: "minecraft:overworld".into(),
            at: [10.5, 64.0, 10.5],
            tick,
            outcome: "minecraft:zombified_piglin".into(),
            genome: genome.map(str::to_owned),
            surgery: 0,
        }
    }

    #[test]
    fn lightning_conversion_is_claimed_by_its_outcome_in_place() {
        let mut list = Conversions::default();
        list.expect(pig(100, Some("horns:2")));
        // Другой вид в той же точке — не наш.
        assert!(list.claim("minecraft:overworld", "minecraft:zombie", [10.5, 64.0, 10.5], 101).is_none());
        // Далеко — тоже.
        assert!(list.claim("minecraft:overworld", "minecraft:zombified_piglin", [14.5, 64.0, 10.5], 101).is_none());
        let got = list
            .claim("minecraft:overworld", "minecraft:zombified_piglin", [10.6, 64.0, 10.4], 101)
            .expect("пиглин на месте свиньи");
        assert_eq!(got.genome.as_deref(), Some("horns:2"));
        assert!(list.is_empty(), "забирается один раз");
    }

    #[test]
    fn stale_or_empty_conversions_are_not_kept() {
        let mut list = Conversions::default();
        list.expect(pig(100, None));
        assert!(list.is_empty(), "обычной свинье передавать нечего");
        list.expect(pig(100, Some("")));
        assert_eq!(list.len(), 1, "бросок был — пустой геном тоже передаётся");
        assert!(list
            .claim("minecraft:overworld", "minecraft:zombified_piglin", [10.5, 64.0, 10.5], 100 + CONVERSION_TICKS + 1)
            .is_none());
    }
}

#[cfg(test)]
mod heredity_tests {
    use super::*;

    fn gene(name: &str) -> usize {
        GENE_NAMES.iter().position(|n| *n == name).unwrap()
    }

    fn birth(tick: u64, parents: (u32, u32), at: [f64; 3], genomes: (&str, &str)) -> Birth {
        Birth {
            dimension: "minecraft:overworld".into(),
            at,
            tick,
            parents,
            genomes: (genomes.0.into(), genomes.1.into()),
        }
    }

    #[test]
    fn a_shared_gene_always_passes_and_never_grows() {
        let horns = gene("horns");
        let mut weakened = 0;
        for seed in 0..2000u64 {
            let heir = inherit(&[(horns, 3)], &[(horns, 3)], mix(seed));
            assert_eq!(heir.len(), 1, "общий ген передаётся всегда");
            let (g, s) = heir[0];
            assert_eq!(g, horns);
            assert!(s == 3 || s == 2, "ослабнуть можно только на ступень: {s}");
            weakened += usize::from(s == 2);
        }
        // Один из четырёх — с запасом на разброс.
        assert!((350..650).contains(&weakened), "ослабло {weakened} из 2000");
    }

    #[test]
    fn a_gene_of_one_parent_passes_about_half_the_time() {
        let tail = gene("tail");
        let passed = (0..2000u64)
            .filter(|seed| !inherit(&[(tail, 2)], &[], mix(*seed)).is_empty())
            .count();
        // Половина, из неё четверть ослабевает до 1 (не до нуля).
        assert!((850..1150).contains(&passed), "передалось {passed} из 2000");
        // Ген силы 1 при ослаблении пропадает совсем — даже общий.
        let lost = (0..2000u64)
            .filter(|seed| inherit(&[(tail, 1)], &[(tail, 1)], mix(*seed)).is_empty())
            .count();
        assert!((350..650).contains(&lost), "пропал {lost} из 2000");
    }

    #[test]
    fn inheritance_takes_only_parents_genes() {
        let (horns, chitin, tail) = (gene("horns"), gene("chitin"), gene("tail"));
        for seed in 0..500u64 {
            for (g, s) in inherit(&[(horns, 2), (chitin, 1)], &[(tail, 3), (99, 3)], seed) {
                assert!([horns, chitin, tail].contains(&g), "чужой ген {g}");
                assert!((1..=3).contains(&s));
            }
        }
        assert!(inherit(&[], &[], 7).is_empty());
        // Детерминировано.
        assert_eq!(inherit(&[(horns, 2)], &[(tail, 3)], 42), inherit(&[(horns, 2)], &[(tail, 3)], 42));
        assert_eq!(heir_seed(5, 1, 2), heir_seed(5, 2, 1), "порядок пары неважен");
        assert_ne!(heir_seed(5, 1, 2), heir_seed(6, 1, 2));
    }

    #[test]
    fn a_birth_is_claimed_by_the_baby_in_place_and_in_time() {
        let mut births = Births::default();
        births.expect(birth(100, (1, 2), [10.0, 64.0, 10.0], ("horns:2", "")));
        assert_eq!(births.len(), 1);
        // Другой уровень, далеко, раньше рождения, слишком поздно — не он.
        assert!(births.claim("minecraft:the_nether", [10.0, 64.0, 10.0], 100).is_none());
        assert!(births.claim("minecraft:overworld", [12.0, 64.0, 10.0], 100).is_none());
        assert!(births.claim("minecraft:overworld", [10.0, 64.0, 10.0], 99).is_none());
        assert!(births.claim("minecraft:overworld", [10.0, 64.0, 10.0], 100 + BIRTH_TICKS + 1).is_none());
        let got = births.claim("minecraft:overworld", [10.2, 64.0, 10.0], 100).expect("детёныш");
        assert_eq!(got.parents, (1, 2));
        assert!(births.is_empty(), "забран целиком");
        assert!(births.claim("minecraft:overworld", [10.0, 64.0, 10.0], 100).is_none());
    }

    #[test]
    fn two_hooks_of_one_birth_make_one_record() {
        let mut births = Births::default();
        births.expect(birth(7, (1, 2), [0.0; 3], ("horns:1", "")));
        births.expect(birth(7, (2, 1), [0.0; 3], ("", "horns:1")));
        assert_eq!(births.len(), 1);
        // Новое рождение выметает устаревшие.
        births.expect(birth(7 + BIRTH_TICKS + 1, (3, 4), [5.0; 3], ("tail:1", "")));
        assert_eq!(births.len(), 1);
        // Обычные родители — наследовать нечего, природный бросок.
        births.expect(birth(10, (5, 6), [9.0; 3], ("", "")));
        assert_eq!(births.len(), 1);
        for i in 0..(MAX_BIRTHS as u32 + 5) {
            births.expect(birth(10, (100 + i, 1000 + i), [0.0; 3], ("tail:1", "")));
        }
        assert_eq!(births.len(), MAX_BIRTHS);
    }

    #[test]
    fn the_nearest_birth_wins() {
        let mut births = Births::default();
        births.expect(birth(1, (1, 2), [0.0, 0.0, 0.0], ("horns:1", "")));
        births.expect(birth(1, (3, 4), [0.5, 0.0, 0.0], ("tail:1", "")));
        assert_eq!(births.claim("minecraft:overworld", [0.45, 0.0, 0.0], 1).unwrap().parents, (3, 4));
        assert_eq!(births.claim("minecraft:overworld", [0.45, 0.0, 0.0], 1).unwrap().parents, (1, 2));
    }

    #[test]
    fn the_heir_genome_is_canonical_text() {
        let b = birth(0, (1, 2), [0.0; 3], ("horns:3,chitin:2", "horns:3"));
        for baby in 0..200 {
            let text = b.heir(baby);
            assert_eq!(genome_text(&genome_from_text(&text)), text);
            assert!(text.contains("horns:"), "общий ген: {text}");
        }
    }

    #[test]
    fn eggs_are_not_babies() {
        assert!(hatches_from_egg("minecraft:turtle") && hatches_from_egg("frog") && hatches_from_egg("sniffer"));
        assert!(!hatches_from_egg("minecraft:cow") && !hatches_from_egg("minecraft:mule"));
    }

    #[test]
    fn conversion_keeps_the_mob_as_it_was() {
        // Мутант остаётся мутантом, строка — каноническая.
        assert_eq!(converted_genome(Some("tail:1,horns:2"), true), Some("horns:2,tail:1".into()));
        assert_eq!(converted_genome(Some("horns:2"), false), Some("horns:2".into()));
        // Обычный остаётся обычным: бросок нового вида снимается.
        assert_eq!(converted_genome(None, true), Some(String::new()));
        assert_eq!(converted_genome(Some(""), true), Some(String::new()));
        // Броска не было вовсе — пусть остаётся бросок нового вида.
        assert_eq!(converted_genome(None, false), None);
        // Срезы — да, стол и сон — нет.
        assert_eq!(converted_surgery(0b0101 | TABLE_BIT | ASLEEP_BIT), 0b0101);
        assert_eq!(converted_surgery(TABLE_BIT), 0);
    }
}

// ---- M11-А: Заражённый (GDD 4.26 А) ----
//
// Свой вид мода `biolab:infected` (шаблон — зомби) мутирует при появлении
// как враждебный. Таблицы видов выше — ванильные (их сверяет тест со
// списком видов Horizon), свой вид — здесь. Вид в хук-поинтах всегда с
// пространством имён `biolab:`, `bare_kind` его не трогает.

/// Свои виды мода и их нрав.
pub const MOD_KINDS: [(&str, Temperament); 1] = [("biolab:infected", Temperament::Hostile)];

/// Нрав своего вида мода; None — вид не наш.
pub fn mod_temperament(kind: &str) -> Option<Temperament> {
    MOD_KINDS.iter().find(|(k, _)| *k == kind).map(|(_, t)| *t)
}

/// Вид так, как его называют хук-поинты: ванильный — `minecraft:zombie`
/// (сама игра пишет и `zombie`), свой — как есть, `biolab:infected`. Раньше
/// `joined.rs` дописывал `minecraft:` ко всему, и вид мода превращался в
/// `minecraft:biolab:infected`.
pub fn hook_kind(id: &str) -> String {
    if id.contains(':') {
        id.to_string()
    } else {
        format!("minecraft:{id}")
    }
}

#[cfg(test)]
mod infected_tests {
    use super::*;

    #[test]
    fn infected_mutates_like_a_hostile() {
        assert_eq!(temperament("biolab:infected"), Some(Temperament::Hostile));
        assert_eq!(mutation_percent("biolab:infected"), HOSTILE_PERCENT);
        assert_eq!(mutation_percent("biolab:other"), 0, "незнакомый вид мода не мутирует");
        // Доля мутантов на зёрнах `joined.rs` — как у зомби.
        let share = |kind: &str| (0u32..20_000).filter(|id| !roll_for(kind, id.wrapping_mul(2_654_435_761)).is_empty()).count();
        assert_eq!(share("biolab:infected"), share("minecraft:zombie"));
    }

    #[test]
    fn hook_kind_keeps_mod_namespaces() {
        assert_eq!(hook_kind("zombie"), "minecraft:zombie");
        assert_eq!(hook_kind("minecraft:zombie"), "minecraft:zombie");
        assert_eq!(hook_kind("biolab:infected"), "biolab:infected");
    }
}

// ---- M9: пропорции (GDD 4.24) ----
//
// Девять генов меняют не набор частей тела, а их форму: руки, голову, ноги,
// корпус, одну сторону. Масштаб частей и числа (прибавки к атрибутам с
// учётом того, есть ли у вида руки и ноги) — lib-модуль `biolab_proportions`
// (`proportions.rs`): его зовут поза модели (HEAD на `anim::setup`) и мост
// (`biolab_bridge`), а бинарник lib не видит и наоборот. Здесь — то, что
// решает бинарник: природный бросок.

/// Гены пропорций — в том же порядке, что `biolab_proportions::GENES`
/// (сверяет `tests/modules.rs`).
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

/// Ген пропорций ли это (по номеру в `GENE_NAMES`). Природный мутант несёт
/// не больше одной такой черты — этого хватает, чтобы он читался издалека,
/// а полный набор (и пары-противоположности вроде большой и маленькой
/// головы) собирается только в лаборатории.
pub fn is_proportion(gene: usize) -> bool {
    GENE_NAMES.get(gene).is_some_and(|name| PROPORTION_GENES.contains(name))
}

// ---- M9: сочетания и ограничители (GDD 4.23, пп. 5, 10) ----
//
// Правила пар и предохранителей живут в `biolab-core` (раздел «M9: сочетания
// и ограничители»): там и журнал пар, и часы гена, и уколы. Бинарнику нужна
// своя копия таблицы пар — геном он складывает сам в двух местах, куда
// `biolab-core` не достаёт: природный бросок (`roll`) и наследование
// (`inherit`). Копию сверяет `tests/modules.rs`.
//
// Что делает бинарник:
// * природе предохранители не выпадают (`is_safeguard`), и несовместимой
//   пары в броске не бывает — иначе мутант рождался бы мёртвым;
// * у детёныша пары работают: мелкий разлад снимает ступень, крупный
//   дорастит уродство. Несовместимость в наследстве не отменяется — такой
//   геном добивает тик мира `biolab-core` (там же игрок узнаёт пару);
// * аттенуация ослабляет чужие гены — это делает lib (`mutants::expressed`),
//   потому что прибавки и поведение считает она.

/// Виды пар: номер — тот же, что `PairKind::id()` в `biolab-core`.
pub const PAIR_SYNERGY: u8 = 0;
pub const PAIR_MINOR: u8 = 1;
pub const PAIR_MAJOR: u8 = 2;
pub const PAIR_LETHAL: u8 = 3;

/// Скрытая таблица пар: имена генов и вид пары, в том же порядке, что
/// `GENE_PAIRS` в `biolab-core`.
pub const GENE_PAIRS: [(&str, &str, u8); 28] = [
    ("chitin", "thorns", PAIR_SYNERGY),
    ("regeneration", "hypermetabolism", PAIR_SYNERGY),
    ("aquatic", "membranes", PAIR_SYNERGY),
    ("polyopia", "dicephaly", PAIR_SYNERGY),
    ("predator", "pack_instinct", PAIR_SYNERGY),
    ("spring_legs", "membranes", PAIR_SYNERGY),
    ("stilt_legs", "spring_legs", PAIR_SYNERGY),
    ("heavy_arms", "brute_force", PAIR_SYNERGY),
    ("kill_switch", "self_limiting", PAIR_SYNERGY),
    ("attenuated", "volatile", PAIR_SYNERGY),
    ("gigantism", "dwarfism", PAIR_MINOR),
    ("thermal", "cryophilia", PAIR_MINOR),
    ("albinism", "melanism", PAIR_MINOR),
    ("thin_skin", "chitin", PAIR_MINOR),
    ("photophobia", "bioluminescence", PAIR_MINOR),
    ("loyalty", "predator", PAIR_MINOR),
    ("dense_bone", "sinew", PAIR_MINOR),
    ("macrocephaly", "microcephaly", PAIR_MINOR),
    ("bloated_torso", "sunken_chest", PAIR_MINOR),
    ("short_legs", "stilt_legs", PAIR_MINOR),
    ("gigantism", "hypermetabolism", PAIR_MAJOR),
    ("volatile", "regeneration", PAIR_MAJOR),
    ("violet_blood", "toxic_blood", PAIR_MAJOR),
    ("brute_force", "dwarfism", PAIR_MAJOR),
    ("healing_aura", "necrosis", PAIR_MAJOR),
    ("acid_glands", "thin_skin", PAIR_LETHAL),
    ("volatile", "necrosis", PAIR_LETHAL),
    ("hypermetabolism", "cryophilia", PAIR_LETHAL),
];

/// Уродства GDD 4.19: из них вырастает лишний признак крупного разлада. Тот
/// же список и в том же порядке, что `DEFORMITIES` в `biolab-core`.
pub const DEFORMITIES: [&str; 8] =
    ["polymelia", "dicephaly", "horns", "tail", "tentacles", "polyopia", "tumor", "membranes"];

/// Предохранители (GDD 4.23 п. 10): природе не выпадают.
pub const SAFEGUARD_GENES: [&str; 4] = ["self_limiting", "host_restriction", "kill_switch", "attenuated"];

/// Номер гена по имени; None — такого гена в таблице нет.
pub fn gene_number(name: &str) -> Option<usize> {
    GENE_NAMES.iter().position(|n| *n == name)
}

/// Предохранитель ли это (по номеру в `GENE_NAMES`).
pub fn is_safeguard(gene: usize) -> bool {
    GENE_NAMES.get(gene).is_some_and(|name| SAFEGUARD_GENES.contains(name))
}

/// Пара таблицы номерами генов и видом; None — имена разошлись с таблицей.
pub fn pair_of(index: usize) -> Option<(usize, usize, u8)> {
    let (a, b, kind) = *GENE_PAIRS.get(index)?;
    Some((gene_number(a)?, gene_number(b)?, kind))
}

/// Вид пары двух генов (порядок неважен); None — гены уживаются молча.
pub fn pair_kind(a: usize, b: usize) -> Option<u8> {
    (0..GENE_PAIRS.len())
        .filter_map(pair_of)
        .find(|(x, y, _)| (*x == a && *y == b) || (*x == b && *y == a))
        .map(|(_, _, kind)| kind)
}

/// Пары, оба гена которых есть в геноме, — по порядку таблицы.
pub fn pairs_in(genome: &[(usize, i32)]) -> Vec<usize> {
    let has = |gene: usize| genome.iter().any(|(g, s)| *g == gene && *s > 0);
    (0..GENE_PAIRS.len())
        .filter(|i| pair_of(*i).is_some_and(|(a, b, _)| has(a) && has(b)))
        .collect()
}

/// Несовместимая пара в геноме; None — такой нет.
pub fn lethal_in(genome: &[(usize, i32)]) -> Option<usize> {
    pairs_in(genome)
        .into_iter()
        .find(|i| pair_of(*i).is_some_and(|(_, _, kind)| kind == PAIR_LETHAL))
}

/// Применить пары к собранному геному: мелкий разлад снимает ступень у
/// слабейшего из пары (поровну — у второго; ступень 1 уходит вовсе), крупный
/// дорастит одно уродство, которого ещё нет (сила 1, выбор — от зерна).
/// Синергия тут ничего не меняет (она про цену), несовместимость не
/// отменяется: такой геном добивает тик мира `biolab-core`.
pub fn apply_pairs(genome: Vec<(usize, i32)>, seed: u64) -> Vec<(usize, i32)> {
    let mut out = genome;
    let mut grown = false;
    for index in pairs_in(&out) {
        let Some((a, b, kind)) = pair_of(index) else {
            continue;
        };
        let strength = |genes: &[(usize, i32)], gene: usize| {
            genes.iter().find(|(g, _)| *g == gene).map_or(0, |(_, s)| *s)
        };
        match kind {
            PAIR_MINOR => {
                let (sa, sb) = (strength(&out, a), strength(&out, b));
                // Пару мог разбить разлад раньше по таблице.
                if sa == 0 || sb == 0 {
                    continue;
                }
                let weaker = if sa < sb { a } else { b };
                if let Some(at) = out.iter().position(|(g, _)| *g == weaker) {
                    out[at].1 -= 1;
                    if out[at].1 <= 0 {
                        out.remove(at);
                    }
                }
            }
            PAIR_MAJOR if !grown => {
                let free: Vec<usize> = DEFORMITIES
                    .iter()
                    .filter_map(|name| gene_number(name))
                    .filter(|gene| strength(&out, *gene) == 0)
                    .collect();
                if let Some(pick) = free.get((mix(seed ^ (index as u64 + 1)) % free.len().max(1) as u64) as usize) {
                    out.push((*pick, 1));
                    grown = true;
                }
            }
            _ => {}
        }
    }
    out.sort_by_key(|(gene, _)| *gene);
    out
}

#[cfg(test)]
mod proportion_tests {
    use super::*;

    #[test]
    fn proportion_genes_are_in_the_table() {
        for name in PROPORTION_GENES {
            let i = GENE_NAMES.iter().position(|n| *n == name).expect(name);
            assert!(is_proportion(i));
            assert!(GENE_COSTS[i] <= 2, "{name}: черта дешёвая, природе по карману");
        }
        assert!(!is_proportion(0), "гигантизм тянет всё тело — не пропорция");
        assert!(!is_proportion(GENE_NAMES.len()));
    }

    #[test]
    fn a_natural_mutant_has_at_most_one_proportion() {
        let mut seen = [false; PROPORTION_GENES.len()];
        for draw in 0..200_000u32 {
            for temper in [Temperament::Hostile, Temperament::Passive] {
                let rolled = roll(draw, 100, temper);
                let shapes: Vec<usize> = rolled.iter().map(|(g, _)| *g).filter(|g| is_proportion(*g)).collect();
                assert!(shapes.len() <= 1, "бросок {draw}: {rolled:?}");
                for g in shapes {
                    let at = PROPORTION_GENES.iter().position(|n| *n == GENE_NAMES[g]).unwrap();
                    seen[at] = true;
                }
            }
        }
        assert!(seen.iter().all(|s| *s), "природе выпадают все черты: {seen:?}");
    }
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
        // Обе формы NBT: сейв 1.21+ и старый призыв `/summon {Attributes:…}`.
        for keys in [ATTRIBUTES_KEYS, MODIFIERS_KEYS, MODIFIER_ID_KEYS] {
            assert_eq!(keys[0].to_lowercase(), keys[1].to_lowercase().replace("name", "id"));
        }
        assert!(ATTRIBUTES_KEYS.contains(&"Attributes") && MODIFIERS_KEYS.contains(&"Modifiers"));
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
        // Предохранители (GDD 4.23 п. 10) природе не выпадают никому.
        let missing: Vec<&str> = (0..GENE_NAMES.len())
            .filter(|g| !seen[*g] && gene_fits(*g, Temperament::Passive) && !is_safeguard(*g))
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
