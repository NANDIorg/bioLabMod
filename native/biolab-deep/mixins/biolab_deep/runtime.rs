// BioLab, «глубокая» часть: то, для чего стабильных хук-поинтов не хватает.
// Сейчас это поведение своих эффектов; дальше сюда придут мутанты (геном в NBT,
// статы, цели AI, внешний вид) и отрисовка иконок эффектов.
//
// Бандл цепляется к внутренним функциям игры и привязан к коммиту
// d91cab6. После обновления Horizon цели надо перепроверять.
//
// Этот файл — одновременно Cargo-библиотека (тесты) и модуль
// `crate::game::biolab_deep` внутри игры.

/// Идентификатор мода в Options > Mods и в horizon-mods.txt.
pub const ID: &str = "biolab_deep";
/// Тип урона, которым бьют наши эффекты (как у ванильного яда).
pub const DAMAGE_TYPE: &str = "minecraft:magic";

/// Кислотный ожог: эффект контентного мода, объявленный в
/// `data/biolab/horizon/mob_effect/acid_burn.json`. Id эффекта мода всегда с
/// пространством имён, у ванильных его нет (`poison`).
pub const ACID_BURN: &str = "biolab:acid_burn";

/// Эффект, который бьёт по здоровью: как часто и насколько сильно.
pub struct Hurting {
    pub id: &'static str,
    /// Период в тиках на нулевом уровне.
    pub period: i32,
    /// Урон за удар на нулевом уровне и прибавка за каждый уровень.
    pub base: f32,
    pub per_level: f32,
    /// На сколько уровней эффект вообще может ускориться. У токсикоза он
    /// маленький нарочно: передозировка должна оставлять время на антидот,
    /// а не убивать за три секунды (проверено в игре — убивала).
    pub max_shift: i32,
}

/// Все наши эффекты с уроном. Период каждого считается от своего базового,
/// но никогда не опускается ниже кадров неуязвимости (см. `effect_period`).
pub const HURTING: [Hurting; 3] = [
    // Кислота жжёт часто и больно.
    Hurting { id: "biolab:acid_burn", period: 20, base: 1.0, per_level: 1.0, max_shift: 4 },
    // Токсикоз — это передозировка: бьёт редко и не разгоняется совсем.
    // Проверено в игре: при ускорении вдвое тройная доза убивала за пять
    // секунд, и антидот не успевал подействовать. Теперь на самый тяжёлый
    // перебор уходит около двадцати секунд — время добежать до аптечки.
    Hurting { id: "biolab:toxicosis", period: 40, base: 1.0, per_level: 0.5, max_shift: 0 },
    // Кровотечение: слабые частые удары, пока не перевязали.
    Hurting { id: "biolab:bleeding", period: 30, base: 1.0, per_level: 0.5, max_shift: 2 },
];

/// Эффект с уроном по имени.
pub fn hurting(id: &str) -> Option<&'static Hurting> {
    HURTING.iter().find(|h| h.id == id)
}

/// Как часто бьёт эффект: вдвое чаще на каждом уровне, но не чаще чем раз в
/// 10 тиков.
///
/// Нижняя граница — не вкусовщина: после удара цель неуязвима 10 тиков
/// (ванильные invulnerability frames), поэтому более частые срабатывания
/// просто гасятся и уровень эффекта перестаёт ощущаться. Проверено в игре:
/// при периоде 5 тиков урон шёл вдвое медленнее расчётного. Поэтому уровень
/// растит силу удара (`effect_amount`), а не частоту.
pub fn effect_period(effect: &Hurting, amplifier: i32) -> i32 {
    (effect.period >> amplifier.clamp(0, effect.max_shift)).max(10)
}

/// Урон за одно срабатывание.
pub fn effect_amount(effect: &Hurting, amplifier: i32) -> f32 {
    effect.base + effect.per_level * amplifier.clamp(0, 10) as f32
}

/// Сколько урона нанести в этом тике, если это вообще наш бьющий эффект.
/// `clock` — часы бьющего: счётчик тиков игрока или возраст моба.
///
/// Раньше здесь был остаток длительности самого эффекта, и это оказалось
/// ловушкой: фармакология подновляет свои эффекты каждую секунду, остаток
/// всё время прыгает обратно к 60 и через кратное периоду число просто не
/// проходит — токсикоз висел на игроке, но не бил ни разу. Часы, которые
/// идут сами по себе, от этого не зависят.
pub fn effect_damage(id: &str, clock: i64, amplifier: i32) -> Option<f32> {
    let effect = hurting(id)?;
    (clock >= 0 && clock % i64::from(effect_period(effect, amplifier)) == 0)
        .then(|| effect_amount(effect, amplifier))
}

/// Седация: моб теряет цель, пока эффект висит. Замедление ему даёт
/// ванильный `slowness`, а вот забыть игрока ванилла не умеет.
pub const SEDATION: &str = "biolab:sedation";

/// Спит ли моб под седативным.
pub fn sedated(effects: &[(&'static str, i32, i32)]) -> bool {
    effects.iter().any(|(id, ticks, _)| *id == SEDATION && *ticks != 0)
}


// ---- Мутанты: что делает бинарник ----
//
// Геном выдаётся и возвращается после загрузки в бинарнике (хук
// `native_mobs_joined`), потому что только здесь виден сырой NBT мобов
// (`persist.raw`). Вид и поведение живут в lib-крейте и узнают геном по
// меткам на самом мобе — модификаторам атрибута с именем гена. Общий контракт
// двух частей — имена генов, их цена и формат метки; тест `tests/modules.rs`
// следит, чтобы копии не разошлись.

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

/// Модификаторы с прибавками генома: по одному на атрибут. Ставит их lib
/// (`mutants::stats`), а здесь список нужен, чтобы узнать свои модификаторы
/// в сыром NBT загруженного моба.
pub const STAT_MODIFIERS: [&str; 5] = [
    "biolab:genome_health",
    "biolab:genome_speed",
    "biolab:genome_damage",
    "biolab:genome_armor",
    "biolab:genome_knockback",
];

/// Метки генома: модификатор дальности преследования с нулевой прибавкой,
/// чьё имя и есть ген. Ванилла пишет его в сейв вместе с остальными.
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

/// Метка гена для сейва.
pub fn marker(gene: usize, strength: i32) -> Option<&'static str> {
    if gene >= GENE_NAMES.len() || !(1..=3).contains(&strength) {
        return None;
    }
    GENE_MARKERS.get(gene * 3 + (strength - 1) as usize).copied()
}

/// Ген по метке: нужен, чтобы дополнить геном уже помеченного моба.
pub fn gene_of_marker(marker: &str) -> Option<(usize, i32)> {
    let rest = marker.strip_prefix("biolab:gene/")?;
    let (name, strength) = rest.rsplit_once('/')?;
    let gene = GENE_NAMES.iter().position(|n| *n == name)?;
    let strength = strength.parse::<i32>().ok()?;
    (1..=3).contains(&strength).then_some((gene, strength))
}

/// Наш модификатор по имени из сейва: имена модификаторов в игре —
/// `&'static str`, поэтому строку из NBT сопоставляем с известным списком.
/// Мобы, которым в этой сессии уже вернули модификаторы из сейва
/// (`restore.rs`): каждый — один раз.
fn restored_ids() -> &'static std::sync::Mutex<std::collections::HashSet<u32>> {
    static IDS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<u32>>> =
        std::sync::OnceLock::new();
    IDS.get_or_init(Default::default)
}

/// Первый ли раз встречаем этого моба на тике восстановления.
pub fn first_restore(id: u32) -> bool {
    restored_ids().lock().map(|mut ids| ids.insert(id)).unwrap_or(false)
}

/// Отметка «бросок мутации уже был»: модификатор с нулевой прибавкой, как
/// метки генов. Пишется в сейв вместе с мобом, поэтому загруженный моб не
/// бросает заново, а новый — бросает, как бы он ни появился (призыв с NBT
/// тоже оставляет сырые данные, и по ним «новый» от «загруженного» не отличить).
pub const CHECKED_MARKER: &str = "biolab:mutation_checked";

/// Метки операционной (GDD 4.17): ставит сервер, читают отрисовка и сейв.
/// Без них в этом списке пациент после перезахода вставал бы со стола, а
/// срезанные конечности отрастали бы сами.
pub const SURGERY_MARKERS: [&str; 6] = [
    "biolab:surgery/table",
    "biolab:surgery/flayed/0",
    "biolab:surgery/flayed/1",
    "biolab:surgery/flayed/2",
    "biolab:surgery/flayed/3",
    "biolab:surgery/slow",
];

pub fn our_modifier(id: &str) -> Option<&'static str> {
    STAT_MODIFIERS
        .iter()
        .chain(GENE_MARKERS.iter())
        .chain(SURGERY_MARKERS.iter())
        .chain(std::iter::once(&CHECKED_MARKER))
        .find(|known| **known == id)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_mob_is_restored_once() {
        assert!(first_restore(4_000_000_001));
        assert!(!first_restore(4_000_000_001));
        assert!(first_restore(4_000_000_002));
    }

    #[test]
    fn the_checked_mark_survives_a_save() {
        assert_eq!(our_modifier(CHECKED_MARKER), Some(CHECKED_MARKER));
        for marker in SURGERY_MARKERS {
            assert_eq!(our_modifier(marker), Some(marker), "метка операционной теряется в сейве");
        }
        assert!(!GENE_MARKERS.contains(&CHECKED_MARKER), "отметка — не ген");
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
                assert_eq!(our_modifier(m), Some(m));
            }
        }
        assert_eq!(marker(GENE_NAMES.len(), 1), None);
        assert_eq!(marker(0, 4), None);
        assert_eq!(gene_of_marker("biolab:genome_health"), None);
    }


    #[test]
    fn each_hurting_effect_has_its_own_rhythm() {
        for (i, effect) in HURTING.iter().enumerate() {
            assert!(effect.id.starts_with("biolab:"), "{}", effect.id);
            assert!(effect.period >= 10, "{}", effect.id);
            assert!(effect.base > 0.0, "{}", effect.id);
            for other in &HURTING[i + 1..] {
                assert_ne!(effect.id, other.id);
            }
        }
        assert!(hurting("minecraft:poison").is_none());
    }

    #[test]
    fn only_our_effects_hit() {
        assert_eq!(effect_damage("minecraft:poison", 20, 0), None);
        assert_eq!(effect_damage("biolab:acid_burn", 20, 0), Some(1.0));
        assert_eq!(effect_damage("biolab:acid_burn", 19, 0), None);
        assert_eq!(effect_damage("biolab:toxicosis", 40, 0), Some(1.0));
        assert_eq!(effect_damage("biolab:toxicosis", 20, 0), None, "период вдвое длиннее");
        assert_eq!(effect_damage("biolab:bleeding", 15, 1), Some(1.5), "уровень ускоряет кровотечение");
        assert_eq!(effect_damage("biolab:bleeding", 14, 1), None);
        // Токсикоз не разгоняется: смерть от передозировки не мгновенная.
        assert_eq!(effect_damage("biolab:toxicosis", 40, 3), Some(2.5));
        assert_eq!(effect_damage("biolab:toxicosis", 20, 3), None, "перебор не разгоняет токсикоз");
        assert_eq!(effect_period(hurting("biolab:toxicosis").unwrap(), 9), 40);
        assert_eq!(effect_damage("biolab:acid_burn", -1, 0), None, "часы ещё не пошли");
    }

    #[test]
    fn the_clock_ticks_evenly() {
        // Сколько ударов за минуту: ровно по периоду, без провалов.
        let hits = (0..1200)
            .filter(|clock| effect_damage("biolab:toxicosis", *clock, 0).is_some())
            .count();
        assert_eq!(hits, 1200 / 40);
    }

    #[test]
    fn no_period_beats_invulnerability() {
        for effect in &HURTING {
            for amp in -1..12 {
                assert!(effect_period(effect, amp) >= 10, "{}", effect.id);
            }
        }
    }

    #[test]
    fn sedation_is_recognised() {
        assert!(sedated(&[(SEDATION, 100, 0)]));
        assert!(!sedated(&[("minecraft:slowness", 100, 0)]));
        assert!(!sedated(&[(SEDATION, 0, 0)]), "кончившийся эффект не считается");
        assert!(!sedated(&[]));
    }


    #[test]
    fn ignores_vanilla_effects() {
        assert_eq!(effect_damage("poison", 20, 0), None);
        assert_eq!(effect_damage("minecraft:wither", 20, 0), None);
    }

    #[test]
    fn hits_once_per_period() {
        assert_eq!(effect_damage(ACID_BURN, 40, 0), Some(1.0));
        assert_eq!(effect_damage(ACID_BURN, 39, 0), None);
        assert_eq!(effect_damage(ACID_BURN, 20, 0), Some(1.0));
    }

    #[test]
    fn amplifier_speeds_up_and_hurts_more() {
        let acid = hurting(ACID_BURN).unwrap();
        assert_eq!(effect_period(acid, 0), 20);
        assert_eq!(effect_period(acid, 1), 10);
        assert_eq!(
            effect_period(acid, 2),
            10,
            "период не опускается ниже кадров неуязвимости"
        );
        assert_eq!(effect_period(acid, 9), 10);
        assert_eq!(effect_damage(ACID_BURN, 10, 1), Some(2.0));
        assert_eq!(effect_damage(ACID_BURN, 10, 2), Some(3.0));
    }

    #[test]
    fn the_clock_starts_with_a_hit() {
        assert_eq!(effect_damage(ACID_BURN, 0, 0), Some(1.0));
        assert_eq!(effect_damage(ACID_BURN, -1, 0), None, "часы ещё не пошли");
    }
}
