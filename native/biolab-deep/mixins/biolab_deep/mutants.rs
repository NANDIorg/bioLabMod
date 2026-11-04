// Мутанты: ванильный моб с геномом, который меняет его характеристики,
// поведение и вид.
//
// Своих видов мобов платформа не поддерживает (виды — закрытый enum Species),
// поэтому мутант — это обычный моб любого вида, которому при рождении выпал
// геном (шансы по видам — `roll_for` в бинарнике). Всё ниже не зависит от
// вида: оттенок и размер ложатся поверх ванильных, а здоровье считается от
// базового здоровья вида (`scaled_stats`).
// Геном — сила 0–3 у каждого гена таблицы. Природный мутант рождается с
// одним-двумя генами в пределах стабильности (бросок в бинарнике), а
// лаборатория вживляет сколько угодно (GDD 4.19).
//
// Обязанности поделены между двумя частями бандла, потому что модуль lib
// бинарнику не виден:
// * бинарник бросает геном и хранит его данными на мобе (`biolab:genome`,
//   `data.rs`), а перед тиком мобов выставляет прибавки и поведение через
//   мост `biolab_bridge` (`mutant_tick.rs`);
// * эта библиотека знает, что гены значат (прибавки, оттенок, размер,
//   поведение), и держит геномы, пришедшие в окно, для отрисовки.
// Общий контракт — имена генов и форма данных; его сверяет `tests/modules.rs`.
//
// Этот файл подключается как модуль `crate::biolab_mutants` крейта
// horizon-client (там живёт `mobs`).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Имена генов в том же порядке, что и в бинарнике. Имя уходит в сейв, так
/// что менять его можно только вместе с чтением старых миров.
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

/// Сколько генов в таблице: геном хранит силу каждого.
pub const GENE_COUNT: usize = GENE_NAMES.len();

/// Один ген: номер в таблице и сила 1–3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gene {
    pub index: usize,
    pub strength: i32,
}

/// Геном мутанта: сила 0–3 у каждого гена таблицы (0 — гена нет).
///
/// Раньше геном был тремя слотами: природный мутант больше трёх генов не
/// несёт. Но лаборатория (укол фрагмента, чан, мутаген, носилки) с GDD 4.19
/// вживляет сколько угодно генов, и четвёртый ген из меток при чтении
/// терялся бы. Поэтому место есть под все гены таблицы сразу, а предел
/// числа генов и стабильности остался только у природного броска
/// (`roll` в бинарнике). Номер гена — его место в `GENE_NAMES`: по нему же
/// читают метки и сейвы, поэтому новые гены дописываются только в конец.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Genome {
    pub strengths: [u8; GENE_COUNT],
}

// `Default` для массивов длиннее 32 сам не выводится.
impl Default for Genome {
    fn default() -> Self {
        Self { strengths: [0; GENE_COUNT] }
    }
}

impl Genome {
    /// Гены по порядку таблицы, без пустот.
    pub fn list(&self) -> impl Iterator<Item = Gene> + '_ {
        self.strengths
            .iter()
            .enumerate()
            .filter(|(_, s)| **s > 0)
            .map(|(index, s)| Gene { index, strength: i32::from(*s) })
    }

    /// Сила гена по его номеру; 0 — гена нет.
    pub fn strength(&self, index: usize) -> u8 {
        self.strengths.get(index).copied().unwrap_or(0)
    }

    /// Сила гена по имени (`"horns"`); 0 — гена нет или имя незнакомое.
    /// Этим читает геном отрисовка наростов: `genome(id)?.strength_of("tail")`.
    pub fn strength_of(&self, name: &str) -> u8 {
        gene_index(name).map_or(0, |index| self.strength(index))
    }

    /// Генов в геноме.
    pub fn count(&self) -> usize {
        self.strengths.iter().filter(|s| **s > 0).count()
    }

    /// Пустой ли геном: обычный моб.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Добавить ген: новый занимает своё место, имеющийся только усиливается.
    /// Предела по числу генов и стабильности нет — это правило природного
    /// броска, а не генома. `false` — ген незнакомый или сила вне 1–3.
    pub fn add(&mut self, gene: Gene) -> bool {
        if gene.index >= GENE_COUNT || !(1..=3).contains(&gene.strength) {
            return false;
        }
        let slot = &mut self.strengths[gene.index];
        *slot = (*slot).max(gene.strength as u8);
        true
    }
}

/// Цена гена за единицу силы: чем сильнее признак, тем дороже стабильность.
pub fn gene_cost(index: usize) -> i32 {
    const COSTS: [i32; 40] = [2, 1, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1, 2, 1, 1, 2, 3, 2, 2, 1, 2, 2, 3, 3, 2, 2, 3, 3, 2, 2, 2, 3, 3, 3, 2, 2, 3, 2, 1, 2];
    COSTS.get(index).copied().unwrap_or(9)
}

/// Во что обходится геном: сумма цен с учётом силы.
pub fn cost(genome: &Genome) -> i32 {
    genome
        .list()
        .map(|g| gene_cost(g.index) * g.strength)
        .sum()
}

/// Номер гена по имени.
pub fn gene_index(name: &str) -> Option<usize> {
    GENE_NAMES.iter().position(|n| *n == name)
}

/// Оттенок тела в RGB: красит первый ген генома, у которого есть свой цвет.
/// В ABGR, который ждёт отрисовка, его переводит `mobs::render::abgr`.
pub fn tint_rgb(genome: &Genome) -> Option<u32> {
    const TINTS: [Option<u32>; 40] = [Some(0xC8A064), Some(0xD8C8A0), Some(0xFF7A5A), Some(0x6E7A52), Some(0xE0DCC8), Some(0xB04030), Some(0xC06848), Some(0x90C070), Some(0xF0D8D0), Some(0x7CFF4A), Some(0xF4F0EC), Some(0x202024), Some(0x5AA0E0), Some(0xA05A28), Some(0xD8D0C8), Some(0x8A4AC8), Some(0xE86AA0), Some(0xE0803A), Some(0x8AD8F0), Some(0xC8C0B0), Some(0x4AB8C8), Some(0x9CC84A), Some(0x9CE04A), Some(0xB44AE0), Some(0x6A7A4A), Some(0x7AC86A), Some(0xD8D0B0), Some(0xF0A0C8), Some(0x6A4A3A), Some(0x8A7A5A), Some(0x70C0F0), Some(0x9A3A3A), Some(0xC89A8A), Some(0xB8A090), Some(0xC8B898), Some(0xA08060), Some(0x9A6A9A), Some(0xD0C060), Some(0xD88A7A), Some(0x7A6A80)];
    genome
        .list()
        .find_map(|g| TINTS.get(g.index).copied().flatten())
}

/// Во сколько раз мутант крупнее обычного моба: размеры генов перемножаются.
pub fn scale(genome: &Genome) -> f32 {
    const STEPS: [f32; 40] = [0.22, -0.18, -0.04, 0.05, 0.04, 0.06, 0.0, 0.0, -0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.06, 0.0, 0.08, 0.0, 0.0, 0.04, 0.02, 0.0, 0.0, 0.0, 0.0, 0.0, 0.04, 0.0];
    genome.list().fold(1.0, |acc, g| {
        acc * (1.0 + STEPS.get(g.index).copied().unwrap_or(0.0) * g.strength as f32)
    })
}

/// Прибавки одного гена за единицу силы:
/// здоровье, скорость в долях базы, урон, броня, сопротивление отбрасыванию.
/// Статы ставит эта библиотека, а не бинарник: так ген, вживлённый уколом уже
/// живому мобу, получает свои прибавки там же, где читаются метки.
pub const GENE_STATS: [[f64; 5]; 40] = [
    [6.00, -0.08, 1.50, 0.00, 0.10],
    [-3.00, 0.14, -0.50, 0.00, -0.10],
    [-2.00, 0.22, 1.00, 0.00, 0.00],
    [1.00, -0.03, 0.00, 3.00, 0.20],
    [3.00, -0.05, 0.00, 1.00, 0.40],
    [0.00, -0.04, 3.00, 0.00, 0.00],
    [4.00, 0.00, 0.50, 0.00, 0.00],
    [0.00, 0.06, 0.00, 0.00, 0.00],
    [-4.00, 0.18, 0.00, -1.00, 0.00],
    [0.00, 0.00, 0.00, 0.00, 0.00],
    [0.00, 0.00, 0.00, 0.00, 0.00],
    [0.00, 0.00, 0.00, 0.00, 0.00],
    [0.00, 0.04, 0.00, 1.00, 0.00],
    [1.00, 0.00, 0.00, 1.00, 0.00],
    [0.00, 0.00, 0.00, 0.00, 0.00],
    [2.00, 0.00, 0.00, 0.00, 0.00],
    [0.00, 0.00, 0.00, 0.00, 0.00],
    [2.00, 0.00, 0.00, 0.00, 0.00],
    [1.00, -0.05, 0.00, 1.00, 0.00],
    [0.00, 0.05, 0.00, 0.00, 0.00],
    [1.00, 0.00, 0.00, 0.00, 0.00],
    [1.00, 0.00, 0.00, 1.00, 0.00],
    [0.00, 0.00, 1.00, 0.00, 0.00],
    [-1.00, 0.05, 0.00, 0.00, 0.00],
    [-1.00, 0.00, 0.50, 0.00, 0.00],
    [1.00, 0.00, 0.00, 0.00, 0.00],
    [1.00, -0.03, 0.00, 1.00, 0.10],
    [0.00, 0.00, 0.00, 0.00, 0.00],
    [1.00, 0.06, 1.00, 0.00, 0.00],
    [0.00, 0.00, 0.00, 0.00, 0.00],
    [2.00, 0.00, 0.00, 0.00, 0.00],
    [0.00, 0.00, 2.00, 0.00, 0.00],
    [0.00, 0.00, 0.50, 0.00, 0.00],
    [2.00, 0.00, 0.00, 0.00, 0.00],
    [0.00, 0.00, 1.00, 0.00, 0.10],
    [1.00, 0.02, 0.00, 0.00, 0.00],
    [1.00, 0.00, 0.00, 0.00, 0.00],
    [0.00, 0.00, 0.00, 0.00, 0.00],
    [4.00, -0.06, 0.00, 0.00, 0.05],
    [0.00, 0.00, 0.00, 0.00, 0.00],
];

/// Модификаторы, которыми геном меняет атрибуты: по одному на атрибут.
pub const STAT_MODIFIERS: [&str; 5] = [
    "biolab:genome_health",
    "biolab:genome_speed",
    "biolab:genome_damage",
    "biolab:genome_armor",
    "biolab:genome_knockback",
];

/// Суммарные прибавки генома.
pub fn stats(genome: &Genome) -> [f64; 5] {
    let mut total = [0.0; 5];
    for gene in genome.list() {
        let Some(row) = GENE_STATS.get(gene.index) else {
            continue;
        };
        for (sum, add) in total.iter_mut().zip(row) {
            *sum += add * f64::from(gene.strength);
        }
    }
    total
}

/// Здоровье зомби: под него подобраны прибавки `GENE_STATS`.
pub const REFERENCE_HEALTH: f64 = 20.0;

/// Прибавки генома для моба с базовым здоровьем `base_health`.
///
/// Прибавки здоровья в таблице — для зомби. Курице (4 сердца пополам) те же
/// +18 гигантизма сделали бы её крепче зомби, а −21 тонкой кожи с
/// карликовостью — сразу трупом с одним очком здоровья. Поэтому здоровье
/// масштабируется по виду (от четверти до двойной прибавки) и никогда не
/// срезает больше половины базового. Урон, броня, скорость и отбрасывание
/// не масштабируются: урон мерится здоровьем игрока, а скорость и так в
/// долях базы.
pub fn scaled_stats(genome: &Genome, base_health: f64) -> [f64; 5] {
    let mut total = stats(genome);
    let base = if base_health > 0.0 { base_health } else { REFERENCE_HEALTH };
    let factor = (base / REFERENCE_HEALTH).clamp(0.25, 2.0);
    total[0] = (total[0] * factor).max(-base / 2.0);
    total
}

/// Оттенок мутанта поверх ванильного цвета тела (оба в ABGR): каналы
/// перемножаются. Так у мокрого волка остаётся тень, а у тропической рыбы —
/// узор её красителя, просто окрашенный геном; прозрачность — ванильная.
pub fn tint_over(vanilla: u32, tint: u32) -> u32 {
    let channel = |c: u32, shift: u32| (c >> shift) & 0xFF;
    let mul = |shift: u32| (channel(vanilla, shift) * channel(tint, shift) / 255) << shift;
    (vanilla & 0xFF00_0000) | mul(16) | mul(8) | mul(0)
}

/// Что ген делает, кроме чисел: по этим меткам работают тик моба и тик мира.
/// Пусто — ген только меняет характеристики или внешность.
pub const GENE_FLAGS: [&[&str]; 40] = [
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
];

/// Сила гена с таким поведением в этом геноме; 0 — такого гена нет.
pub fn flag_strength(genome: &Genome, flag: &str) -> i32 {
    genome
        .list()
        .filter(|g| {
            GENE_FLAGS
                .get(g.index)
                .is_some_and(|flags| flags.contains(&flag))
        })
        .map(|g| g.strength)
        .max()
        .unwrap_or(0)
}

/// Регенерация: сколько здоровья в секунду отрастает.
pub fn regeneration(genome: &Genome) -> f32 {
    0.5 * flag_strength(genome, "regen") as f32
}

/// Термостойкость: такой мутант не горит.
pub fn fireproof(genome: &Genome) -> bool {
    flag_strength(genome, "fireproof") > 0
}

/// Радиус ауры в блоках по её силе: 0 — ауры нет.
pub fn aura_radius(strength: i32) -> f64 {
    match strength {
        0 => 0.0,
        s => 2.0 + f64::from(s),
    }
}

/// Кислотные железы: радиус ауры в блоках.
pub fn acid_aura(genome: &Genome) -> f64 {
    aura_radius(flag_strength(genome, "acid_aura"))
}

/// Нестабильный геном: моб оставляет после себя лужу кислоты.
pub fn volatile(genome: &Genome) -> bool {
    flag_strength(genome, "volatile") > 0
}

/// Яды на такого мутанта не действуют.
pub fn toxin_proof(genome: &Genome) -> bool {
    flag_strength(genome, "toxin_proof") > 0
}

/// Перепонки (GDD 4.19): мутант планирует. Ванильное медленное падение
/// мобам Horizon не учитывает (его читает только физика игрока), поэтому
/// падение гасится прямо в скорости моба (`glide_fall`).
pub fn glides(genome: &Genome) -> bool {
    flag_strength(genome, "glide") > 0
}

/// Скорость падения с перепонками, блоков за тик: как у медленного падения
/// игрока, чуть быстрее на слабом гене. Положительная скорость (прыжок,
/// подброс) не меняется.
pub fn glide_fall(vertical: f64, strength: i32) -> f64 {
    let limit = match strength {
        s if s <= 0 => return vertical,
        1 => -0.16,
        2 => -0.12,
        _ => -0.08,
    };
    vertical.max(limit)
}

// ---- Седация ----
//
// Сам эффект (потеря цели) — в `biolab-core` на стабильных хуках API 56.
// Здесь он нужен тику моба (`mob_tick.rs`): усыплённому гасится скорость
// модификатором, а у прибавок к атрибутам стабильного API нет. Ещё по нему
// сервер ставит бит «спит» в данных операционной — окно кладёт уснувшего.
// Мутаген тоже переехал в `biolab-core` (нагрузка, выбор гена, данные).

/// Седативное: тот же id, что `SEDATION_EFFECT` в `biolab-core`
/// (сверяет `tests/modules.rs`).
pub const SEDATION: &str = "biolab:sedation";

/// Геномы мобов, которых рисует окно, по их id — копия того, что пришло с
/// сервера данными `biolab:genome` (attachment с `sync`).
///
/// Почему копия, а не чтение данных прямо в отрисовке: синхронизированное
/// значение окно держит в своём `Game` (`Host::attachments` → копия
/// `remote.attachments`), а моб рисуется в lib-крейте (`mobs::render`), куда
/// доходит только `&Mob` — ни `Game`, ни `Host` там нет. Поэтому раз в тик
/// окна бинарник переписывает сюда всё, что пришло (`view_sync.rs` →
/// `biolab_bridge::show`), а отрисовка читает отсюда. Источник — только
/// данные клиента: сервер сюда больше не пишет, и так это работает и в
/// сетевой игре, где сервер — другой процесс.
fn table() -> &'static Mutex<HashMap<u32, Genome>> {
    static TABLE: OnceLock<Mutex<HashMap<u32, Genome>>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Геном из пар «номер гена, сила»; незнакомое и вне 1–3 пропускается.
pub fn genome_of(genes: &[(usize, u8)]) -> Genome {
    let mut genome = Genome::default();
    for (index, strength) in genes {
        genome.add(Gene { index: *index, strength: i32::from(*strength) });
    }
    genome
}

pub fn remember(id: u32, genome: Genome) {
    if let Ok(mut t) = table().lock() {
        t.insert(id, genome);
    }
}

/// Заменить всю таблицу окна тем, что пришло с сервера: моб, у которого
/// генома больше нет (или который ушёл), пропадает сам.
pub fn replace_all(list: Vec<(u32, Vec<(usize, u8)>)>) {
    let fresh: HashMap<u32, Genome> = list
        .into_iter()
        .map(|(id, genes)| (id, genome_of(&genes)))
        .filter(|(_, genome)| !genome.is_empty())
        .collect();
    if let Ok(mut t) = table().lock() {
        *t = fresh;
    }
}

pub fn genome(id: u32) -> Option<Genome> {
    table().lock().ok()?.get(&id).copied()
}

/// Метки генома старого формата (до M11): имя модификатора с геном. Мод их
/// больше не ставит; список держим, чтобы сверять его с миграцией бинарника.
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

/// Ген по метке: `biolab:gene/gigantism/2`.
pub fn decode_marker(marker: &str) -> Option<Gene> {
    let rest = marker.strip_prefix("biolab:gene/")?;
    let (name, strength) = rest.rsplit_once('/')?;
    let index = gene_index(name)?;
    let strength = strength.parse::<i32>().ok()?;
    (1..=3).contains(&strength).then_some(Gene { index, strength })
}

/// Метка для гена.
pub fn marker_of(gene: Gene) -> Option<&'static str> {
    let name = GENE_NAMES.get(gene.index)?;
    GENE_MARKERS
        .iter()
        .copied()
        .find(|m| *m == format!("biolab:gene/{name}/{}", gene.strength))
}

/// Геном строкой: `gigantism:2,chitin:1`.
pub fn to_text(genome: &Genome) -> String {
    genome
        .list()
        .map(|g| format!("{}:{}", GENE_NAMES[g.index], g.strength))
        .collect::<Vec<_>>()
        .join(",")
}

/// Геном из строки. Неизвестные гены пропускаем: мир от новой версии мода не
/// должен ломать старую.
pub fn from_text(text: &str) -> Option<Genome> {
    let mut genome = Genome::default();
    for part in text.split(',').filter(|p| !p.is_empty()) {
        let (name, strength) = part.split_once(':')?;
        let index = gene_index(name)?;
        let strength = strength.parse::<i32>().ok()?;
        if !(1..=3).contains(&strength) {
            return None;
        }
        genome.add(Gene { index, strength });
    }
    genome.list().next().is_some().then_some(genome)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gene(name: &str, strength: i32) -> Gene {
        Gene { index: gene_index(name).unwrap(), strength }
    }

    #[test]
    fn genes_are_unique_and_priced() {
        for (i, name) in GENE_NAMES.iter().enumerate() {
            assert!(!name.is_empty());
            assert!(gene_cost(i) > 0, "{name} бесплатен");
            assert_eq!(gene_index(name), Some(i));
            for other in &GENE_NAMES[i + 1..] {
                assert_ne!(name, other);
            }
        }
        assert_eq!(gene_index("telepathy"), None);
    }

    #[test]
    fn a_genome_holds_every_gene_of_the_table() {
        // Лабораторный геном: все гены сразу, каждый на тройке.
        let mut g = Genome::default();
        for index in 0..GENE_COUNT {
            assert!(g.add(Gene { index, strength: 3 }), "{}", GENE_NAMES[index]);
        }
        assert_eq!(g.count(), GENE_COUNT);
        assert_eq!(g.list().count(), GENE_COUNT);
        assert!(cost(&g) > 6, "стабильность геном не ограничивает");
        assert_eq!(g.strength_of("tentacles"), 3);
        // Строкой он тоже переживает сейв целиком.
        assert_eq!(from_text(&to_text(&g)), Some(g));

        let mut g = Genome::default();
        assert!(g.is_empty());
        assert!(g.add(gene("bioluminescence", 1)));
        assert!(g.add(gene("chitin", 1)));
        assert!(g.add(gene("gigantism", 1)));
        assert!(g.add(gene("thermal", 1)), "четвёртый ген — уже лаборатория");
        assert_eq!(g.count(), 4);
        assert_eq!(g.strength_of("chitin"), 1);
        assert_eq!(g.strength_of("polymelia"), 0);
        assert_eq!(g.strength_of("telepathy"), 0, "незнакомое имя — ноль");
        assert_eq!(g.strength(GENE_COUNT + 5), 0);
    }

    #[test]
    fn deformity_genes_are_read_by_name() {
        // Новые гены GDD 4.19 дописаны в конец: старые номера на месте.
        assert_eq!(gene_index("rending_claws"), Some(31));
        for name in ["polymelia", "dicephaly", "horns", "tail", "tentacles", "polyopia", "tumor", "membranes"] {
            let index = gene_index(name).expect(name);
            assert!(index >= 32, "{name} сдвинул старые гены");
            let mut g = Genome::default();
            g.add(Gene { index, strength: 2 });
            assert_eq!(g.strength_of(name), 2, "{name}");
            let marker = marker_of(Gene { index, strength: 2 }).expect(name);
            assert_eq!(marker, format!("biolab:gene/{name}/2"));
        }
        let mut g = Genome::default();
        g.add(gene("membranes", 1));
        assert!(glides(&g));
        assert!(!glides(&Genome::default()));
        // Перепонки гасят только падение, и сильнее — на сильном гене.
        assert_eq!(glide_fall(-0.8, 0), -0.8, "без гена падает как падал");
        assert!(glide_fall(-0.8, 3) > glide_fall(-0.8, 1));
        assert_eq!(glide_fall(0.4, 3), 0.4, "прыжок не гасится");
        assert_eq!(glide_fall(-0.05, 1), -0.05, "медленное падение не трогаем");
    }

    #[test]
    fn the_same_gene_only_gets_stronger() {
        let mut g = Genome::default();
        assert!(g.add(gene("chitin", 1)));
        assert!(g.add(gene("chitin", 2)));
        assert_eq!(g.list().count(), 1);
        assert_eq!(g.strength_of("chitin"), 2);
        assert!(g.add(gene("chitin", 1)), "слабее — не ослабляет");
        assert_eq!(g.strength_of("chitin"), 2);
    }

    #[test]
    fn nonsense_genes_are_refused() {
        let mut g = Genome::default();
        assert!(!g.add(Gene { index: 99, strength: 1 }));
        assert!(!g.add(gene("chitin", 0)));
        assert!(!g.add(gene("chitin", 4)));
        assert_eq!(g.list().count(), 0);
    }

    #[test]
    fn stats_add_up_by_strength() {
        let mut one = Genome::default();
        one.add(gene("gigantism", 1));
        let mut two = Genome::default();
        two.add(gene("gigantism", 2));
        assert!((stats(&two)[0] - stats(&one)[0] * 2.0).abs() < 1e-9);
        let mut mixed = Genome::default();
        mixed.add(gene("gigantism", 1));
        mixed.add(gene("chitin", 1));
        assert!(stats(&mixed)[3] > 0.0, "хитин даёт броню");
        assert_eq!(stats(&Genome::default()), [0.0; 5]);
        assert_eq!(GENE_STATS.len(), GENE_NAMES.len());
    }

    #[test]
    fn health_bonus_follows_the_species() {
        let mut giant = Genome::default();
        giant.add(gene("gigantism", 3));
        // Зомби — эталон: прибавка как в таблице.
        assert_eq!(scaled_stats(&giant, 20.0), stats(&giant));
        // Курица крепнет соразмерно себе, а не на зомбиные +18.
        let chicken = scaled_stats(&giant, 4.0);
        assert!((chicken[0] - 18.0 * 0.25).abs() < 1e-9, "{chicken:?}");
        assert_eq!(chicken[2], stats(&giant)[2], "урон не масштабируется");
        // Равагер не получает впятеро больше: потолок — вдвое.
        assert!((scaled_stats(&giant, 100.0)[0] - 36.0).abs() < 1e-9);

        // Тонкая кожа с карликовостью не убивают моба самим геномом.
        let mut frail = Genome::default();
        frail.add(gene("thin_skin", 3));
        frail.add(gene("dwarfism", 3));
        assert!(stats(&frail)[0] < -20.0);
        for base in [4.0, 8.0, 10.0, 20.0, 40.0, 100.0] {
            let health = base + scaled_stats(&frail, base)[0];
            assert!(health >= base / 2.0 - 1e-9, "база {base}: осталось {health}");
        }
        // Неизвестная база (0) считается как у зомби.
        assert_eq!(scaled_stats(&giant, 0.0), stats(&giant));
    }

    #[test]
    fn tint_lies_over_the_vanilla_colour() {
        let tint = 0xFF40_80C0;
        assert_eq!(tint_over(0xFFFF_FFFF, tint), tint, "обычный моб — чистый оттенок");
        assert_eq!(tint_over(0x80FF_FFFF, tint) >> 24, 0x80, "прозрачность ванильная");
        let shaded = tint_over(0xFFBF_BFBF, tint);
        assert!(shaded & 0xFF < tint & 0xFF, "тень мокрого волка остаётся");
        assert_eq!(tint_over(0xFF00_0000, tint), 0xFF00_0000);
    }

    #[test]
    fn traits_follow_the_genes() {
        let mut g = Genome::default();
        g.add(gene("gigantism", 2));
        assert!(scale(&g) > 1.0);
        assert_eq!(tint_rgb(&g), Some(0xC8A064));
        assert_eq!(regeneration(&g), 0.0);
        assert!(!fireproof(&g));

        let mut g = Genome::default();
        g.add(gene("thermal", 1));
        g.add(gene("regeneration", 1));
        assert!(fireproof(&g));
        assert!(regeneration(&g) > 0.0);
        assert_eq!(acid_aura(&g), 0.0);

        let mut g = Genome::default();
        g.add(gene("acid_glands", 2));
        assert!(acid_aura(&g) > 0.0);
        assert!(!volatile(&g));
        let mut g = Genome::default();
        g.add(gene("volatile", 1));
        assert!(volatile(&g));
    }

    #[test]
    fn every_marker_decodes_and_encodes() {
        for marker in GENE_MARKERS {
            let g = decode_marker(marker).expect(marker);
            assert_eq!(marker_of(g), Some(marker));
        }
        assert_eq!(decode_marker("biolab:genome_health"), None);
        assert_eq!(decode_marker("biolab:gene/ghost/1"), None);
        assert_eq!(decode_marker("biolab:gene/chitin/9"), None);
    }

    #[test]
    fn genome_survives_a_round_trip() {
        let mut g = Genome::default();
        g.add(gene("gigantism", 2));
        g.add(gene("chitin", 1));
        let text = to_text(&g);
        assert_eq!(text, "gigantism:2,chitin:1");
        assert_eq!(from_text(&text), Some(g));
        assert_eq!(from_text(""), None);
        assert_eq!(from_text("ghost:1"), None);
    }

    #[test]
    fn the_table_is_what_arrived() {
        let mut g = Genome::default();
        g.add(gene("bioluminescence", 3));
        remember(4242, g);
        assert_eq!(genome(4242), Some(g));
        let horns = gene_index("horns").unwrap();
        // Пришёл новый снимок: прежнее уходит, пустой геном не хранится.
        replace_all(vec![(4243, vec![(horns, 2), (99, 1)]), (4244, vec![])]);
        assert_eq!(genome(4242), None);
        assert_eq!(genome(4243).map(|g| g.strength_of("horns")), Some(2));
        assert_eq!(genome(4243).map(|g| g.count()), Some(1));
        assert_eq!(genome(4244), None);
        assert!(genome_of(&[(horns, 0), (horns, 4)]).is_empty());
        replace_all(Vec::new());
        assert_eq!(genome(4243), None);
    }
}
