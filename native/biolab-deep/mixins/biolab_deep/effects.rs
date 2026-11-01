// Поведение своих эффектов на мобах.
//
// Эффекты мобов в Horizon живут отдельно от эффектов игрока
// (`m.extra.effects`, а не `PlayerEffects`), и тикает их другая функция
// (`mobs::ai::tick_effects`), поэтому нужен второй хук. Логика урона та же,
// что у игрока в `runtime.rs`: тест `tests/modules.rs` следит, чтобы они не
// разошлись.
//
// Этот файл подключается как модуль `crate::biolab_effects` крейта
// horizon-client (там живёт `mobs`).

/// Кислотный ожог: эффект контентного мода.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
