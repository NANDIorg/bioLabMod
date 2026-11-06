// Поведение своих эффектов (GDD 4.7) по контракту платформы, API 56
// (`docs/native-mods.md` пина, «what the mob effects of a content mod do»).
//
// Раньше урон и поведение считали два миксина `biolab-deep` — тик эффектов
// игрока и тик эффектов моба, — и таблица урона жила в двух копиях со своими
// часами (`game.ticks` у игрока, `m.age` у моба). Теперь вопрос «действует ли
// эффект в этот тик» и само действие задаёт платформа, одинаково для игрока
// и для моба, а таблица и ритм — одни, здесь.
//
// Контракт и почему действие отложено:
// * `native_should_apply_effect_tick_this_tick(effect, tick_count, amplifier)`
//   — срабатывает ли эффект. Кто носит эффект, хук не говорит, поэтому ритм
//   не может зависеть от носителя: он считается от часов игры (`game.ticks`).
//   Свой `tick_count` (остаток длительности) не годится в часы — GDD 3.7:
//   фармакология подновляет эффекты каждую секунду, и остаток через кратное
//   периоду число просто не проходит.
// * `native_apply_effect_tick(effect, dimension, entity, amplifier, keep)` —
//   действие. Игра в нём только для чтения (`&self`): моб в этот момент вынут
//   из мира, эффекты игрока — тоже. Поэтому здесь действие только
//   записывается в очередь (`Effects::apply`), а выполняет его тик игрока
//   (`effect_actions.rs`, `native_player_tick`) — там игра изменяема, моб на
//   месте, а игрок — тот, кого очередь назвала. Снять эффект можно сразу:
//   `keep = false` (мутаген, истративший себя на мутацию).
//
// Модуль чистый — без типов игры: в игре он `crate::game::biolab_effects`,
// а тесты идут обычным `cargo test` (`tests/effects.rs`).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Эффекты контентного мода (`data/biolab/horizon/mob_effect/*.json`). Те же
/// имена, что в `runtime.rs`; совпадение сверяет `tests/effects.rs`.
pub const ACID_BURN: &str = "biolab:acid_burn";
pub const TOXICOSIS: &str = "biolab:toxicosis";
pub const BLEEDING: &str = "biolab:bleeding";
pub const SEDATION: &str = "biolab:sedation";
pub const MUTAGENESIS: &str = "biolab:mutagenesis";
/// Паралич кураре (GDD 4.20, M5): держит без цели, как седация, и душит —
/// «без антидота может задохнуться».
pub const PARALYSIS: &str = "biolab:paralysis";
/// Ключ очереди для удушья паралича: одно срабатывание эффекта даёт два
/// действия (цель и урон), а очередь не пускает второе с тем же ключом.
const PARALYSIS_CHOKE: &str = "biolab:paralysis/choke";

/// Удушье паралича: раз в 5 секунд, удар в полсердца. Не в `HURTING`: тот
/// бьёт только по ритму, а паралич ещё и каждый тик держит моба без цели.
pub const CHOKE_PERIOD: u64 = 100;
pub const CHOKE_DAMAGE: f32 = 1.0;

// ---- Типы урона (GDD 4.26, Б; M11-17) ----
//
// Раньше все наши эффекты били ванильной магией: в чате «убит магией», и ни
// броня, ни лут, ни будущая спецодежда не отличали кислоту от яда. Теперь у
// мода свои типы урона — файлы `content/data/biolab/damage_type/*.json`, теги
// в `content/data/minecraft/tags/damage_type/` и сообщения о смерти
// `death.attack.<message_id>` в языках. Игрок берёт тип из реестра мира
// (`EffectAction::Hurt(id, сила)` → `health_resources.damage.id(id)`,
// `game/potions.rs` пина; нет такого id — удара нет вовсе), моб — из флагов
// `NamedDamage` (`docs/native-mods.md` пина, «Named mob damage»): флаги
// обязаны совпадать с тегами, это сверяет `tests/effects.rs`.

/// Тип урона: id в реестре и то, что моб узнаёт по тегам. Модуль чистый,
/// поэтому флаги — булевы, а `Damage` моба из них собирает
/// `crate::game::biolab_damage` (`damage.rs`).
#[derive(Debug, PartialEq, Eq)]
pub struct DamageType {
    pub id: &'static str,
    /// `#minecraft:bypasses_armor`: броня не режет урон.
    pub bypasses_armor: bool,
    /// `#minecraft:no_knockback`: удар не отбрасывает.
    pub no_knockback: bool,
    /// `#minecraft:panic_causes`: мирный моб от удара пускается бежать.
    pub panic_causes: bool,
}

/// Ванильная магия — там, где урон не наш, и у кровотечения (своего типа у
/// него нет). Флаги — ванильные: мимо брони, без отброса, пугает. Моб
/// получает её как `DamageKind::Magic`, а не как именованный урон.
pub const DAMAGE_MAGIC: DamageType =
    DamageType { id: "minecraft:magic", bypasses_armor: true, no_knockback: true, panic_causes: true };
/// Кислотный ожог: броня держит (кислота её и так портит), без отброса.
pub const DAMAGE_ACID: DamageType =
    DamageType { id: "biolab:acid", bypasses_armor: false, no_knockback: true, panic_causes: true };
/// Яды: токсикоз, удушье кураре, смертельная доза газа. Мимо брони.
pub const DAMAGE_TOXIN: DamageType =
    DamageType { id: "biolab:toxin", bypasses_armor: true, no_knockback: true, panic_causes: true };
/// Болезнь и её симптомы. Мимо брони.
pub const DAMAGE_INFECTION: DamageType =
    DamageType { id: "biolab:infection", bypasses_armor: true, no_knockback: true, panic_causes: true };
/// Нестабильный геном, мутаген: тело рвёт само себя — мимо брони.
pub const DAMAGE_MUTATION: DamageType =
    DamageType { id: "biolab:mutation", bypasses_armor: true, no_knockback: true, panic_causes: true };

/// Типы урона мода: у каждого свой JSON, теги и сообщения о смерти.
pub const OUR_DAMAGE: [&DamageType; 4] = [&DAMAGE_ACID, &DAMAGE_TOXIN, &DAMAGE_INFECTION, &DAMAGE_MUTATION];

/// Эффект болезни (`runtime.rs`, `INFECTION_EFFECT`).
pub const INFECTION: &str = "biolab:infection";

/// Какой эффект каким уроном бьёт — одна таблица на игрока и моба. Болезнь и
/// мутаген сами сейчас не бьют (симптомы болезни — ванильные эффекты, мутаген
/// копит нагрузку), но их тип записан здесь же: ударят — бить будут своим.
/// Смертельная доза газа (`sleep.rs`) — это токсикоз и бьёт его типом.
pub const EFFECT_DAMAGE: [(&str, &DamageType); 6] = [
    (ACID_BURN, &DAMAGE_ACID),
    (TOXICOSIS, &DAMAGE_TOXIN),
    (PARALYSIS, &DAMAGE_TOXIN),
    (BLEEDING, &DAMAGE_MAGIC),
    (INFECTION, &DAMAGE_INFECTION),
    (MUTAGENESIS, &DAMAGE_MUTATION),
];

/// Тип урона эффекта; незнакомый эффект — ванильная магия.
pub fn damage_of(effect: &str) -> &'static DamageType {
    EFFECT_DAMAGE
        .iter()
        .find(|(id, _)| *id == effect)
        .map_or(&DAMAGE_MAGIC, |(_, damage)| *damage)
}

/// Эффект, который бьёт по здоровью: как часто и насколько сильно.
#[derive(Debug)]
pub struct Hurting {
    pub id: &'static str,
    /// Период в тиках на нулевом уровне.
    pub period: i32,
    /// Урон за удар на нулевом уровне и прибавка за каждый уровень.
    pub base: f32,
    pub per_level: f32,
    /// На сколько уровней эффект вообще может ускориться. У токсикоза он
    /// нулевой нарочно: передозировка должна оставлять время на антидот,
    /// а не убивать за три секунды (проверено в игре — убивала).
    pub max_shift: i32,
}

/// Все наши эффекты с уроном — и для игрока, и для моба. Период каждого
/// считается от своего базового, но никогда не опускается ниже кадров
/// неуязвимости (см. `effect_period`).
pub const HURTING: [Hurting; 3] = [
    // Кислота жжёт часто и больно.
    Hurting { id: ACID_BURN, period: 20, base: 1.0, per_level: 1.0, max_shift: 4 },
    // Токсикоз — это передозировка: бьёт редко и не разгоняется совсем.
    // Проверено в игре: при ускорении вдвое тройная доза убивала за пять
    // секунд, и антидот не успевал подействовать. Теперь на самый тяжёлый
    // перебор уходит около двадцати секунд — время добежать до аптечки.
    Hurting { id: TOXICOSIS, period: 40, base: 1.0, per_level: 0.5, max_shift: 0 },
    // Кровотечение: слабые частые удары, пока не перевязали.
    Hurting { id: BLEEDING, period: 30, base: 1.0, per_level: 0.5, max_shift: 2 },
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

/// Ответ на `should_apply_effect_tick_this_tick`: действует ли наш эффект в
/// тик `clock` (`game.ticks`). Бьющие — раз в свой период; седация и
/// мутаген — каждый тик (одна держит моба без цели, другой копит нагрузку).
/// Чужие и ванильные эффекты — нет: их решает не наш мод.
pub fn acts(effect: &str, clock: u64, amplifier: i32) -> bool {
    if let Some(h) = hurting(effect) {
        return clock % effect_period(h, amplifier) as u64 == 0;
    }
    matches!(effect, SEDATION | MUTAGENESIS | PARALYSIS)
}

/// Снимает ли эффект с моба цель (седация, паралич): `native_mob_effect_added`
/// сбрасывает её сразу, не дожидаясь первого тика эффекта.
pub fn calms(effect: &str) -> bool {
    effect == SEDATION || effect == PARALYSIS
}

/// Кто носит эффект: игрок по uuid (None — игрок без своего uuid в хуке,
/// то есть хозяин окна, а не гость), или моб уровня `dimension` по
/// `Entity.getId`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Who {
    Player(Option<u128>),
    Mob { dimension: String, id: u32 },
}

/// Что сделать с носителем, когда дойдёт ход.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Act {
    /// Урон: сила и тип (`damage_of` эффекта).
    Hurt(f32, &'static DamageType),
    /// Моб забывает, кого преследовал (седация). Ванильного эффекта с таким
    /// поведением нет, а дротик для отлова мутантов без него не работает.
    Calm,
    /// Мутаген набрал нагрузку: мобу добавляется ген.
    Mutate,
}

/// Сколько тиков под мутагеном нужно мобу, чтобы случилась мутация. Мутаген
/// не мутирует мгновенно: укол дротиком виден как процесс, а не вспышка.
pub const MOB_MUTAGEN_LOAD: i32 = 160;

/// Сколько тиков действие ждёт в очереди. Её разбирает тик игрока того же
/// уровня; если на уровне никого нет, мобы там и не тикают, и старое
/// действие уже ничего не значит.
pub const STALE: u64 = 100;

/// Предел очереди: если её никто не разбирает, она не растёт без конца.
pub const MAX_PENDING: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
struct Pending {
    /// Тик, в который эффект сработал.
    at: u64,
    who: Who,
    effect: &'static str,
    act: Act,
}

/// Действия, которые дождались своего тика игрока.
#[derive(Debug, Default, PartialEq)]
pub struct Due {
    /// Урон игроку, который тикает сейчас: сила и тип.
    pub hurts: Vec<(f32, &'static DamageType)>,
    /// Мобы уровня, на котором он стоит: id и что с ним сделать.
    pub mobs: Vec<(u32, Act)>,
}

/// Состояние эффектов между хуками: очередь действий и нагрузка мутагена.
#[derive(Debug, Default)]
pub struct Effects {
    pending: Vec<Pending>,
    /// Нагрузка мутагена по мобам и тик, когда её прибавили последний раз.
    loads: HashMap<u32, (i32, u64)>,
}

impl Effects {
    /// `apply_effect_tick`: эффект `effect` уровня `amplifier` сработал на
    /// `who` в тик `clock`. Действие встаёт в очередь; ответ — оставить ли
    /// эффект (`keep`).
    ///
    /// Одно и то же срабатывание (тот же носитель, эффект и тик) в очередь
    /// второй раз не встаёт: тик эффектов игрока зовётся чаще раза в тик
    /// (GDD 3.7), и лишний удар всё равно погасили бы кадры неуязвимости.
    pub fn apply(&mut self, clock: u64, who: Who, effect: &str, amplifier: i32) -> bool {
        if let Some(h) = hurting(effect) {
            self.push(clock, who, h.id, Act::Hurt(effect_amount(h, amplifier), damage_of(h.id)));
            return true;
        }
        match effect {
            // У игрока седация — ванильное замедление, цели у него нет.
            SEDATION if matches!(who, Who::Mob { .. }) => {
                self.push(clock, who, SEDATION, Act::Calm);
                true
            }
            // Паралич: моб без цели каждый тик, и все — моб и игрок —
            // задыхаются раз в `CHOKE_PERIOD` по часам игры.
            PARALYSIS => {
                if clock % CHOKE_PERIOD == 0 {
                    self.push(clock, who.clone(), PARALYSIS_CHOKE, Act::Hurt(CHOKE_DAMAGE, damage_of(PARALYSIS)));
                }
                if matches!(who, Who::Mob { .. }) {
                    self.push(clock, who, PARALYSIS, Act::Calm);
                }
                true
            }
            MUTAGENESIS => {
                // Мутаген игрока считает `runtime.rs` по луже (`mutagen_tick`):
                // эффект на игроке нагрузку здесь не копит.
                let Who::Mob { id, .. } = &who else {
                    return true;
                };
                if !self.load(*id, clock) {
                    return true;
                }
                self.push(clock, who, MUTAGENESIS, Act::Mutate);
                // Мутация потратила мутаген: эффект уходит сразу.
                false
            }
            _ => true,
        }
    }

    fn push(&mut self, at: u64, who: Who, effect: &'static str, act: Act) {
        if self
            .pending
            .iter()
            .any(|p| p.at == at && p.effect == effect && p.who == who)
        {
            return;
        }
        if self.pending.len() >= MAX_PENDING {
            self.pending.remove(0);
        }
        self.pending.push(Pending { at, who, effect, act });
    }

    /// Прибавить мобу тик нагрузки мутагена; true — порог взят, пора
    /// мутировать (счётчик при этом обнуляется). Раз в тик, сколько бы раз
    /// хук ни позвали.
    pub fn load(&mut self, id: u32, clock: u64) -> bool {
        let (load, last) = self.loads.entry(id).or_insert((0, u64::MAX));
        if *last == clock {
            return false;
        }
        *last = clock;
        *load += 1;
        if *load < MOB_MUTAGEN_LOAD {
            return false;
        }
        *load = 0;
        true
    }

    /// Сколько нагрузки у моба сейчас.
    pub fn load_of(&self, id: u32) -> i32 {
        self.loads.get(&id).map_or(0, |(load, _)| *load)
    }

    /// Тик игрока: забрать урон для него (`player` — его uuid в хуке) и
    /// действия над мобами его уровня. Устаревшее выбрасывается, остальное
    /// ждёт своего игрока.
    pub fn take(&mut self, clock: u64, player: Option<u128>, dimension: &str) -> Due {
        let mut due = Due::default();
        // Часы не идут назад внутри одного мира: запись «из будущего» —
        // остаток прошлого мира.
        self.pending
            .retain(|p| p.at <= clock && clock - p.at <= STALE);
        self.pending.retain(|p| match (&p.who, p.act) {
            (Who::Player(uuid), Act::Hurt(amount, damage)) if *uuid == player => {
                due.hurts.push((amount, damage));
                false
            }
            (Who::Mob { dimension: d, id }, act) if d == dimension => {
                due.mobs.push((*id, act));
                false
            }
            _ => true,
        });
        due
    }

    /// Сколько действий ждёт.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }
}

/// Общее состояние: хуки зовутся из разных мест тика, а помнить надо между
/// ними. Сервер в процессе один (окно одиночной игры — его клиент, а у
/// клиента хуки эффектов не работают: `remote`), так что делить его не с кем.
pub fn state() -> &'static Mutex<Effects> {
    static STATE: OnceLock<Mutex<Effects>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(Effects::default()))
}

/// Зерно мутации моба: от его id и возраста, как было в `biolab-deep`.
pub fn mutation_seed(id: u32, age: u64) -> u64 {
    u64::from(id).wrapping_mul(2_654_435_761) ^ age
}

/// Какой ген добавит мутация: `strengths` — сила 0–3 каждого гена таблицы
/// (`GENES` в `runtime.rs`, по порядку). Берётся первый по кругу от зерна,
/// который что-то меняет — новый ген или усиление имеющегося; сила тоже от
/// зерна, но слабый ген приживается чаще сильного. Мутаген — лаборатория,
/// а не природа: предела стабильности у него нет (GDD 4.19). None — все гены
/// уже на тройке, мутировать нечему. Ответ — номер гена и сила 1–3.
pub fn mutation(strengths: &[u8], seed: u64) -> Option<(usize, i32)> {
    let count = strengths.len() as u64;
    for step in 0..count {
        let index = (seed.wrapping_add(step) % count) as usize;
        for strength in [1 + ((seed / 7) % 3) as i32, 2, 1] {
            if i32::from(strengths[index]) < strength {
                return Some((index, strength));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mob(id: u32) -> Who {
        Who::Mob { dimension: "minecraft:overworld".into(), id }
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

    /// Таблица урона: каждый бьющий эффект записан явно, каждый наш тип
    /// кому-то принадлежит, эффект назван один раз, чужое — магия.
    #[test]
    fn each_effect_names_its_damage_type() {
        for effect in HURTING.iter().map(|h| h.id).chain([PARALYSIS]) {
            assert!(EFFECT_DAMAGE.iter().any(|(id, _)| *id == effect), "{effect}: тип не записан");
        }
        for (i, (id, _)) in EFFECT_DAMAGE.iter().enumerate() {
            assert!(EFFECT_DAMAGE[i + 1..].iter().all(|(other, _)| other != id), "{id} дважды");
        }
        for ours in OUR_DAMAGE {
            assert!(ours.id.starts_with("biolab:"), "{}", ours.id);
            assert!(EFFECT_DAMAGE.iter().any(|(_, d)| *d == ours), "{}: никому не принадлежит", ours.id);
        }
        assert_eq!(damage_of(ACID_BURN), &DAMAGE_ACID);
        assert_eq!(damage_of(TOXICOSIS), &DAMAGE_TOXIN);
        assert_eq!(damage_of(PARALYSIS), &DAMAGE_TOXIN);
        assert_eq!(damage_of(BLEEDING), &DAMAGE_MAGIC);
        assert_eq!(damage_of("minecraft:poison"), &DAMAGE_MAGIC);
        // Спецификация (GDD 4.26, Б): яд и болезнь — мимо брони, кислота — нет.
        assert!(DAMAGE_TOXIN.bypasses_armor && DAMAGE_INFECTION.bypasses_armor);
        assert!(!DAMAGE_ACID.bypasses_armor);
        // Наши эффекты бьют без отброса, как магия: иначе кислота толкала бы.
        assert!(OUR_DAMAGE.iter().all(|d| d.no_knockback));
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
    fn vanilla_and_foreign_effects_are_not_ours() {
        for id in ["poison", "minecraft:poison", "minecraft:wither", "other:bleeding"] {
            for clock in 0..100 {
                assert!(!acts(id, clock, 0), "{id}");
            }
            let mut fx = Effects::default();
            assert!(fx.apply(0, mob(1), id, 0));
            assert_eq!(fx.pending(), 0, "{id}");
        }
    }

    #[test]
    fn sedation_and_mutagen_act_every_tick() {
        for clock in 0..50 {
            assert!(acts(SEDATION, clock, 0));
            assert!(acts(MUTAGENESIS, clock, 0));
        }
        assert!(calms(SEDATION) && !calms(MUTAGENESIS) && !calms(ACID_BURN));
    }

    #[test]
    fn paralysis_holds_every_tick_and_chokes_every_five_seconds() {
        assert!((0..30).all(|clock| acts(PARALYSIS, clock, 0)));
        assert!(calms(PARALYSIS));
        let mut fx = Effects::default();
        let mut hurts = 0;
        let mut calms_seen = 0;
        for clock in 1..=(CHOKE_PERIOD * 3) {
            assert!(fx.apply(clock, mob(4), PARALYSIS, 0), "паралич не снимается сам");
            // Двойной вызов в тот же тик ничего не добавляет.
            fx.apply(clock, mob(4), PARALYSIS, 0);
            for (_, act) in fx.take(clock, None, "minecraft:overworld").mobs {
                match act {
                    Act::Hurt(amount, damage) => {
                        assert_eq!(amount, CHOKE_DAMAGE);
                        assert_eq!(damage, &DAMAGE_TOXIN, "кураре душит ядом");
                        hurts += 1;
                    }
                    Act::Calm => calms_seen += 1,
                    Act::Mutate => panic!("паралич не мутирует"),
                }
            }
        }
        assert_eq!(hurts, 3, "раз в 5 секунд");
        assert_eq!(calms_seen, CHOKE_PERIOD * 3, "цель снимается каждый тик");
        // Игрок задыхается так же, но цели у него нет.
        let mut fx = Effects::default();
        fx.apply(CHOKE_PERIOD, Who::Player(Some(1)), PARALYSIS, 0);
        let due = fx.take(CHOKE_PERIOD, Some(1), "minecraft:overworld");
        assert_eq!(due.hurts, vec![(CHOKE_DAMAGE, &DAMAGE_TOXIN)]);
        assert!(due.mobs.is_empty());
    }

    #[test]
    fn sedation_calms_mobs_not_players() {
        let mut fx = Effects::default();
        assert!(fx.apply(5, mob(3), SEDATION, 0));
        assert!(fx.apply(5, Who::Player(Some(9)), SEDATION, 0));
        let due = fx.take(5, Some(9), "minecraft:overworld");
        assert_eq!(due.mobs, vec![(3, Act::Calm)]);
        assert!(due.hurts.is_empty());
    }

    #[test]
    fn mutagen_builds_up_before_it_bites() {
        let mut fx = Effects::default();
        for clock in 0..(MOB_MUTAGEN_LOAD as u64 - 1) {
            assert!(fx.apply(clock, mob(7), MUTAGENESIS, 0), "рано, тик {clock}");
            // Второй вызов в тот же тик нагрузку не прибавляет.
            assert!(fx.apply(clock, mob(7), MUTAGENESIS, 0));
        }
        assert_eq!(fx.load_of(7), MOB_MUTAGEN_LOAD - 1);
        assert_eq!(fx.pending(), 0);
        let last = MOB_MUTAGEN_LOAD as u64 - 1;
        assert!(!fx.apply(last, mob(7), MUTAGENESIS, 0), "порог взят — эффект снимается");
        assert_eq!(fx.load_of(7), 0, "счётчик обнулился");
        let due = fx.take(last, None, "minecraft:overworld");
        assert_eq!(due.mobs, vec![(7, Act::Mutate)]);
        // Нагрузка у каждого моба своя.
        assert!(fx.apply(last + 1, mob(8), MUTAGENESIS, 0));
        assert_eq!(fx.load_of(8), 1);
        assert_eq!(fx.load_of(7), 0);
        // Мутаген на игроке здесь не копится (его считает лужа).
        assert!(fx.apply(0, Who::Player(None), MUTAGENESIS, 0));
        assert_eq!(fx.pending(), 0);
    }

    #[test]
    fn the_queue_finds_its_player_and_level() {
        let mut fx = Effects::default();
        fx.apply(40, Who::Player(Some(1)), TOXICOSIS, 0);
        fx.apply(40, Who::Player(Some(2)), TOXICOSIS, 1);
        fx.apply(40, Who::Mob { dimension: "minecraft:the_nether".into(), id: 5 }, TOXICOSIS, 0);
        fx.apply(40, mob(6), ACID_BURN, 2);
        let due = fx.take(40, Some(2), "minecraft:overworld");
        assert_eq!(due.hurts, vec![(1.5, &DAMAGE_TOXIN)]);
        assert_eq!(due.mobs, vec![(6, Act::Hurt(3.0, &DAMAGE_ACID))]);
        assert_eq!(fx.pending(), 2, "чужой игрок и моб другого уровня ждут");
        let due = fx.take(41, Some(1), "minecraft:the_nether");
        assert_eq!(due.hurts, vec![(1.0, &DAMAGE_TOXIN)]);
        assert_eq!(due.mobs, vec![(5, Act::Hurt(1.0, &DAMAGE_TOXIN))]);
        assert_eq!(fx.pending(), 0);
    }

    #[test]
    fn stale_actions_are_dropped() {
        let mut fx = Effects::default();
        fx.apply(0, mob(1), BLEEDING, 0);
        fx.apply(1000, mob(2), BLEEDING, 0);
        // Через STALE тиков ещё ждёт, позже — нет; «будущее» — остаток
        // прошлого мира.
        assert_eq!(fx.take(STALE, None, "minecraft:the_end"), Due::default());
        assert_eq!(fx.pending(), 1, "запись из будущего выброшена");
        assert_eq!(fx.take(STALE + 1, None, "minecraft:overworld"), Due::default());
        assert_eq!(fx.pending(), 0);
    }

    #[test]
    fn the_queue_is_bounded() {
        let mut fx = Effects::default();
        for id in 0..(MAX_PENDING as u32 + 10) {
            fx.apply(0, mob(id), ACID_BURN, 0);
        }
        assert_eq!(fx.pending(), MAX_PENDING);
    }

    #[test]
    fn mutation_always_changes_something() {
        let empty = [0u8; 40];
        assert!(mutation(&empty, 0).is_some());
        let mut heavy = [0u8; 40];
        heavy[16] = 2;
        heavy[22] = 3;
        for seed in 0..500 {
            let (index, strength) = mutation(&heavy, seed).expect("мутировать есть чему");
            assert!((1..=3).contains(&strength));
            assert!(i32::from(heavy[index]) < strength, "мутация ничего не изменила");
        }
        assert_eq!(mutation(&[3u8; 40], 3), None, "все гены на тройке");
        assert_eq!(mutation(&[], 3), None);
        // Огромное зерно не переполняется.
        assert!(mutation(&empty, u64::MAX).is_some());
    }
}
