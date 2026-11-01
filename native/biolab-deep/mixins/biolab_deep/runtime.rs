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
pub const GENE_NAMES: [&str; 32] = [
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
];

/// Цена гена за единицу силы (та же, что в lib).
pub const GENE_COSTS: [i32; 32] = [2, 1, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1, 2, 1, 1, 2, 3, 2, 2, 1, 2, 2, 3, 3, 2, 2, 3, 3, 2, 2, 2, 3];

/// Сколько генов влезает в геном и какую сумму цен он выдерживает.
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

/// Доля мутантов среди зомби, в процентах. Прототип: мутируют все.
pub const MUTATION_PERCENT: u32 = 100;

/// Геном по «броску»: один-два гена с силой 1–3, в пределах стабильности.
/// Детерминирован, чтобы прогон повторялся и lib выдавал то же самое.
pub fn roll(draw: u32) -> Vec<(usize, i32)> {
    if draw % 100 >= MUTATION_PERCENT {
        return Vec::new();
    }
    let count = GENE_NAMES.len() as u32;
    let first = ((draw / 100) % count) as usize;
    // Дорогой ген не может быть сильным: тройные кислотные железы в шесть
    // очков стабильности не укладываются, и такой моб был бы нежизнеспособен.
    let strength = (1 + ((draw / 300) % 3) as i32).min(STABILITY / GENE_COSTS[first]);
    let mut genome = vec![(first, strength)];
    let mut spent = GENE_COSTS[first] * strength;
    // Второй ген достаётся не каждому: треть бросков обходится одним.
    if (draw / 900) % 3 != 0 {
        let step = 1 + ((draw / 2700) % (count - 1)) as usize;
        let second = (first + step) % GENE_NAMES.len();
        let strength = 1 + ((draw / 8100) % 3) as i32;
        if spent + GENE_COSTS[second] * strength <= STABILITY {
            spent += GENE_COSTS[second] * strength;
            genome.push((second, strength));
        }
    }
    let _ = spent;
    genome
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
pub fn our_modifier(id: &str) -> Option<&'static str> {
    STAT_MODIFIERS
        .iter()
        .chain(GENE_MARKERS.iter())
        .find(|known| **known == id)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_roll_stays_inside_stability() {
        for draw in 0..5000 {
            let genome = roll(draw);
            assert!(!genome.is_empty(), "мутируют все: бросок {draw}");
            assert!(genome.len() <= MAX_GENES);
            assert!(cost(&genome) <= STABILITY, "перебор на броске {draw}: {genome:?}");
            for (gene, strength) in &genome {
                assert!(*gene < GENE_NAMES.len());
                assert!((1..=3).contains(strength));
            }
            // Один ген дважды в геном не попадает.
            for (i, (gene, _)) in genome.iter().enumerate() {
                assert!(!genome[i + 1..].iter().any(|(other, _)| other == gene), "{genome:?}");
            }
        }
        assert_eq!(roll(7), roll(7), "бросок детерминирован");
    }

    #[test]
    fn some_rolls_give_two_genes() {
        let pairs = (0..5000).filter(|d| roll(*d).len() == 2).count();
        assert!(pairs > 500, "двойных геномов всего {pairs}");
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
