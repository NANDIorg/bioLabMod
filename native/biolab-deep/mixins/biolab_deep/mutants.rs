// Мутанты: ванильный моб с геномом, который меняет его характеристики,
// поведение и вид.
//
// Своих видов мобов платформа не поддерживает (виды — закрытый enum Species),
// поэтому мутант — это обычный зомби, которому при рождении выпал геном.
// Геном — до трёх генов, у каждого сила 1–3; суммарная «цена» генов не
// превышает предела стабильности, иначе организм нежизнеспособен.
//
// Обязанности поделены между двумя частями бандла, потому что модуль lib
// бинарнику не виден:
// * бинарник (`runtime.rs`) бросает геном, считает прибавки к атрибутам и
//   вешает метки;
// * эта библиотека читает метки и отвечает за вид и поведение моба.
// Общий контракт — имена генов и формат метки; его сверяет `tests/modules.rs`.
//
// Этот файл подключается как модуль `crate::biolab_mutants` крейта
// horizon-client (там живёт `mobs`).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Имена генов в том же порядке, что и в бинарнике. Имя уходит в сейв, так
/// что менять его можно только вместе с чтением старых миров.
pub const GENE_NAMES: [&str; 31] = [
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
];

/// Сколько генов влезает в один геном.
pub const MAX_GENES: usize = 3;
/// Предел стабильности: сумма цен генов выше него организм не выдержит.
pub const STABILITY: i32 = 6;

/// Один ген: номер в таблице и сила 1–3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gene {
    pub index: usize,
    pub strength: i32,
}

/// Геном мутанта: до трёх генов.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Genome {
    pub genes: [Option<Gene>; MAX_GENES],
}

impl Genome {
    /// Гены по порядку, без пустот.
    pub fn list(&self) -> impl Iterator<Item = Gene> + '_ {
        self.genes.iter().filter_map(|g| *g)
    }

    /// Есть ли в геноме этот ген, и какой он силы.
    pub fn strength_of(&self, name: &str) -> i32 {
        let Some(index) = gene_index(name) else {
            return 0;
        };
        self.list()
            .find(|g| g.index == index)
            .map_or(0, |g| g.strength)
    }

    /// Добавить ген: усиливает уже имеющийся, иначе занимает свободное место.
    /// `false` — места нет или перебор по стабильности.
    pub fn add(&mut self, gene: Gene) -> bool {
        if gene.index >= GENE_NAMES.len() || !(1..=3).contains(&gene.strength) {
            return false;
        }
        let mut next = *self;
        match next.genes.iter_mut().find(|slot| {
            slot.is_some_and(|g| g.index == gene.index)
        }) {
            Some(slot) => *slot = Some(Gene { index: gene.index, strength: gene.strength.max(slot.unwrap().strength) }),
            None => match next.genes.iter_mut().find(|slot| slot.is_none()) {
                Some(slot) => *slot = Some(gene),
                None => return false,
            },
        }
        if cost(&next) > STABILITY {
            return false;
        }
        *self = next;
        true
    }
}

/// Цена гена за единицу силы: чем сильнее признак, тем дороже стабильность.
pub fn gene_cost(index: usize) -> i32 {
    const COSTS: [i32; 31] = [2, 1, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1, 2, 1, 1, 2, 3, 2, 2, 1, 2, 2, 3, 3, 2, 2, 3, 3, 2, 2, 2];
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
    const TINTS: [Option<u32>; 31] = [Some(0xC8A064), Some(0xD8C8A0), Some(0xFF7A5A), Some(0x6E7A52), Some(0xE0DCC8), Some(0xB04030), Some(0xC06848), Some(0x90C070), Some(0xF0D8D0), Some(0x7CFF4A), Some(0xF4F0EC), Some(0x202024), Some(0x5AA0E0), Some(0xA05A28), Some(0xD8D0C8), Some(0x8A4AC8), Some(0xE86AA0), Some(0xE0803A), Some(0x8AD8F0), Some(0xC8C0B0), Some(0x4AB8C8), Some(0x9CC84A), Some(0x9CE04A), Some(0xB44AE0), Some(0x6A7A4A), Some(0x7AC86A), Some(0xD8D0B0), Some(0xF0A0C8), Some(0x6A4A3A), Some(0x8A7A5A), Some(0x70C0F0)];
    genome
        .list()
        .find_map(|g| TINTS.get(g.index).copied().flatten())
}

/// Во сколько раз мутант крупнее обычного моба: размеры генов перемножаются.
pub fn scale(genome: &Genome) -> f32 {
    const STEPS: [f32; 31] = [0.22, -0.18, -0.04, 0.05, 0.04, 0.06, 0.0, 0.0, -0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.06, 0.0, 0.08, 0.0, 0.0];
    genome.list().fold(1.0, |acc, g| {
        acc * (1.0 + STEPS.get(g.index).copied().unwrap_or(0.0) * g.strength as f32)
    })
}

/// Прибавки одного гена за единицу силы:
/// здоровье, скорость в долях базы, урон, броня, сопротивление отбрасыванию.
/// Статы ставит эта библиотека, а не бинарник: так ген, вживлённый уколом уже
/// живому мобу, получает свои прибавки там же, где читаются метки.
pub const GENE_STATS: [[f64; 5]; 31] = [
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

/// Что ген делает, кроме чисел: по этим меткам работают тик моба и тик мира.
/// Пусто — ген только меняет характеристики или внешность.
pub const GENE_FLAGS: [&[&str]; 31] = [
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

// ---- Мутаген ----
//
// Мутаген не мутирует моба мгновенно: эффект `biolab:mutagenesis` копит
// «мутационную нагрузку», и на пороге организм срывается — добавляется ген.
// Так укол дротиком с мутагеном виден не как вспышка, а как процесс.

/// Эффект мутагена.
pub const MUTAGENESIS: &str = "biolab:mutagenesis";
/// Сколько тиков под мутагеном нужно, чтобы случилась мутация.
pub const MUTAGEN_LOAD: i32 = 160;

/// Накопленная нагрузка по мобам.
fn loads() -> &'static Mutex<HashMap<u32, i32>> {
    static LOADS: OnceLock<Mutex<HashMap<u32, i32>>> = OnceLock::new();
    LOADS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Добавить мобу нагрузку; true — порог взят, пора мутировать (счётчик при
/// этом обнуляется).
pub fn add_load(id: u32, amount: i32) -> bool {
    let Ok(mut table) = loads().lock() else {
        return false;
    };
    let load = table.entry(id).or_insert(0);
    *load += amount;
    if *load < MUTAGEN_LOAD {
        return false;
    }
    *load = 0;
    true
}

/// Сколько нагрузки у моба сейчас.
pub fn load_of(id: u32) -> i32 {
    loads().lock().map(|t| t.get(&id).copied().unwrap_or(0)).unwrap_or(0)
}

/// Какой ген добавит мутация: первый по кругу от зерна, который геном ещё
/// выдержит. None — организм уже полон, мутировать некуда.
pub fn mutation(genome: &Genome, seed: u64) -> Option<Gene> {
    let count = GENE_NAMES.len() as u64;
    for step in 0..count {
        let index = ((seed + step) % count) as usize;
        // Сила тоже от зерна, но слабый ген приживается чаще сильного.
        for strength in [1 + ((seed / 7) % 3) as i32, 2, 1] {
            let gene = Gene { index, strength };
            let mut next = *genome;
            if next.add(gene) && next != *genome {
                return Some(gene);
            }
        }
    }
    None
}

/// Геномы живых мобов по их id. Клиент и сервер одиночной игры — один
/// процесс, поэтому отрисовке виден геном, записанный при спавне.
fn table() -> &'static Mutex<HashMap<u32, Option<Genome>>> {
    static TABLE: OnceLock<Mutex<HashMap<u32, Option<Genome>>>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn remember(id: u32, genome: Genome) {
    if let Ok(mut t) = table().lock() {
        t.insert(id, Some(genome));
    }
}

/// Запомнить, что у моба генома нет: второй раз метки не ищем.
pub fn remember_plain(id: u32) {
    if let Ok(mut t) = table().lock() {
        t.entry(id).or_insert(None);
    }
}

/// Проверялся ли уже этот моб.
pub fn is_known(id: u32) -> bool {
    table().lock().is_ok_and(|t| t.contains_key(&id))
}

pub fn genome(id: u32) -> Option<Genome> {
    table().lock().ok()?.get(&id).copied().flatten()
}

/// Забыть моба: его геном перечитается по меткам (после сплайсинга).
pub fn forget(id: u32) {
    if let Ok(mut t) = table().lock() {
        t.remove(&id);
    }
}

/// Метки генома: имя модификатора, по которому ген узнаётся на мобе.
/// Тот же список, что в `runtime.rs` бинарника.
pub const GENE_MARKERS: [&str; 93] = [
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
    fn mutagen_builds_up_before_it_bites() {
        let id = 77_001;
        assert!(!add_load(id, MUTAGEN_LOAD - 1), "порог ещё не взят");
        assert_eq!(load_of(id), MUTAGEN_LOAD - 1);
        assert!(add_load(id, 1), "порог взят");
        assert_eq!(load_of(id), 0, "счётчик обнулился");
    }

    #[test]
    fn mutation_fits_what_is_left() {
        // Пустой геном принимает что угодно.
        let empty = Genome::default();
        assert!(mutation(&empty, 0).is_some());
        // Заполненный до предела мутировать некуда.
        let mut full = Genome::default();
        full.add(gene("regeneration", 2));
        assert_eq!(cost(&full), STABILITY);
        assert_eq!(mutation(&full, 3), None);
        // Остаток стабильности определяет, что приживётся.
        let mut almost = Genome::default();
        almost.add(gene("bioluminescence", 1));
        for seed in 0..50 {
            if let Some(g) = mutation(&almost, seed) {
                let mut next = almost;
                assert!(next.add(g));
                assert!(cost(&next) <= STABILITY, "{g:?}");
            }
        }
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
    fn a_genome_holds_up_to_three_genes() {
        let mut g = Genome::default();
        assert!(g.add(gene("bioluminescence", 1)));
        assert!(g.add(gene("chitin", 1)));
        assert!(g.add(gene("gigantism", 1)));
        assert_eq!(g.list().count(), 3);
        assert!(!g.add(gene("thermal", 1)), "мест больше нет");
        assert_eq!(g.strength_of("chitin"), 1);
        assert_eq!(g.strength_of("thermal"), 0);
    }

    #[test]
    fn stability_caps_the_genome() {
        let mut g = Genome::default();
        assert!(g.add(gene("regeneration", 2)), "цена 6 — ровно предел");
        assert!(!g.add(gene("acid_glands", 1)), "перебор по стабильности");
        assert_eq!(cost(&g), STABILITY);
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
    fn the_table_keeps_what_it_was_told() {
        let mut g = Genome::default();
        g.add(gene("bioluminescence", 3));
        remember(4242, g);
        assert_eq!(genome(4242), Some(g));
        assert!(is_known(4242));
        remember_plain(4243);
        assert!(is_known(4243));
        assert_eq!(genome(4243), None);
        forget(4242);
        assert!(!is_known(4242));
    }
}
