// Свои частицы мода (GDD 3.9, «Частицы»): что и где пускает дым, споры,
// пузыри и кровь. Модуль чистый — без типов игры, — поэтому его тесты идут
// обычным `cargo test` (`tests/particles.rs`), а в игре он живёт рядом с
// `biolab_core` как `crate::game::biolab_particles`. Шлют частицы хелперы:
// фон — `ambient.rs`, разовые всплески — `inject.rs` и `thorns.rs`.
//
// Что умеет пин (31f8349, API 103, `docs/native-mods.md`): тип частицы мода —
// это спрайт-сет `assets/biolab/particles/<имя>.json` и больше ничего.
// Частица ведёт себя как ванильный `SimpleParticleType`: плоский билборд без
// оттенка, живёт 20–40 тиков, один случайный спрайт из набора на всю жизнь,
// гравитации нет, трение 0,98. Скорость из вызова задаёт только направление:
// клиент (`Particle::moving`) добавляет к ней разброс ±0,4 по осям,
// нормирует, умножает на 0,06–0,18 и прибавляет 0,1 вверх. Отсюда два
// способа двигаться, других в пине нет — см. `Drift`. Декларативное
// поведение в том же JSON (`horizon:behaviour`: гравитация, кривые размера и
// прозрачности, смена кадров) есть только в `main` новее пина.

/// Споры заражения: заразная слизь и мутанты с некрозом.
pub const SPORE: &str = "biolab:spore";
/// Споры мутагена: фиолетовые искры над разлитым мутагеном.
pub const MUTAGEN_SPORE: &str = "biolab:mutagen_spore";
/// Кислотный дым: лужа кислоты, кислотный ожог, кислотная аура.
pub const ACID_FUME: &str = "biolab:acid_fume";
/// Пена брожения над работающим ферментером.
pub const FOAM: &str = "biolab:foam";
/// Пузыри питательного раствора над работающим клонирующим чаном.
pub const BUBBLE: &str = "biolab:bubble";
/// Пар над работающим инкубатором: тёплая влажная камера.
pub const STEAM: &str = "biolab:steam";
/// «Сон» над усыплённым: буква Z. Заменяет ванильные завитки седации.
pub const SLEEP: &str = "biolab:sleep";
/// Капли крови: кровотечение и срез скальпелем.
pub const BLOOD: &str = "biolab:blood";
/// Газ снотворного: облако аэрозоля и газовой гранаты (GDD 4.20).
pub const GAS: &str = "biolab:gas";

/// Все частицы мода. На каждую нужен `particles/<имя>.json` и его текстуры —
/// это проверяет `tests/particles.rs`. Частица без файла у клиента просто не
/// рисуется, ошибки нигде не будет, поэтому проверка важна.
pub const ALL: [&str; 9] = [SPORE, MUTAGEN_SPORE, ACID_FUME, FOAM, BUBBLE, STEAM, SLEEP, BLOOD, GAS];

/// Как частица движется. Своего поведения у типа мода в пине нет, есть
/// только направление стартовой скорости.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drift {
    /// Нулевая скорость: направление случайно, частица расходится облачком и
    /// в среднем всплывает (~0,1 блока за тик в начале, около двух блоков за
    /// жизнь). Место — гауссов разброс самого пакета.
    Float,
    /// Скорость строго вниз: нормированная она почти гасит прибавку 0,1
    /// вверх, и частица висит и медленно оседает (около полублока за жизнь).
    /// Так пакет шлёт одну частицу (`count` 0) ровно в точку, поэтому место
    /// разбрасываем сами — `sends`.
    Sink,
}

/// Одна «затяжка» частиц: какая, как движется, откуда и как часто.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Puff {
    pub name: &'static str,
    pub drift: Drift,
    /// Высота точки выпуска. У блока — над его поверхностью (верх машины,
    /// зеркало лужи), у существа — доля его роста (1,0 — макушка).
    pub y: f64,
    /// Разброс места по осям, в блоках.
    pub spread: [f64; 3],
    /// Сколько частиц за раз.
    pub count: i32,
    /// Раз в сколько циклов фона (`EVERY` тиков); 1 — каждый цикл. Фаза у
    /// каждого источника своя (`due`), чтобы соседние лужи не пыхали разом.
    pub one_in: u64,
}

/// Цикл фона: как часто `ambient.rs` смотрит вокруг игрока, в тиках. Частица
/// живёт 20–40 тиков и быстро уходит вверх; первая версия (раз в секунду по
/// одной) в игре почти не читалась, поэтому почти всё пыхает каждый цикл,
/// машины — по две.
pub const EVERY: u64 = 10;
/// Насколько далеко вокруг игрока ищем лужи и машины: по горизонтали и по
/// вертикали, в блоках. Это 21×21×7 ≈ 3 тыс. блоков раз в полсекунды.
pub const SCAN: i32 = 10;
pub const SCAN_Y: i32 = 3;
/// Мобы дальше этого от игрока частиц не получают: пакет частиц и так
/// доходит только до игроков в 32 блоках.
pub const REACH: f64 = 24.0;
/// Рост игрока, для частиц на нём самом.
pub const PLAYER_HEIGHT: f64 = 1.8;

/// Лужи: что поднимается над ними. Отстойник (`sludge`) не дымит — это
/// просто грязь.
pub const PUDDLES: [(&str, Puff); 3] = [
    ("biolab:acid_puddle", Puff { name: ACID_FUME, drift: Drift::Float, y: 0.1, spread: [0.3, 0.02, 0.3], count: 1, one_in: 1 }),
    ("biolab:slime", Puff { name: SPORE, drift: Drift::Float, y: 0.1, spread: [0.3, 0.02, 0.3], count: 1, one_in: 1 }),
    ("biolab:mutagen_puddle", Puff { name: MUTAGEN_SPORE, drift: Drift::Float, y: 0.1, spread: [0.3, 0.02, 0.3], count: 1, one_in: 1 }),
];

/// Машины: что над ними, пока они заняты (отметка `BUSY_KEY` в сейве блока,
/// её ставит тик машины). По одному виду на машину: брожение пенится,
/// клонирующий раствор пузырится, инкубатор парит.
pub const MACHINES: [(&str, Puff); 3] = [
    ("biolab:fermenter", Puff { name: FOAM, drift: Drift::Float, y: 0.05, spread: [0.2, 0.02, 0.2], count: 2, one_in: 1 }),
    ("biolab:cloning_vat", Puff { name: BUBBLE, drift: Drift::Float, y: 0.05, spread: [0.2, 0.02, 0.2], count: 2, one_in: 1 }),
    ("biolab:incubator", Puff { name: STEAM, drift: Drift::Float, y: 0.05, spread: [0.25, 0.02, 0.25], count: 2, one_in: 1 }),
];

/// Эффекты на существе (мобе или игроке): что над ним, пока эффект идёт.
/// Ванильные завитки этих эффектов у мобов прячем (`hides_swirls`): иначе
/// поверх наших частиц крутились бы цветные спирали того же смысла.
pub const EFFECTS: [(&str, Puff); 3] = [
    ("biolab:sedation", Puff { name: SLEEP, drift: Drift::Float, y: 1.05, spread: [0.15, 0.05, 0.15], count: 1, one_in: 1 }),
    ("biolab:acid_burn", Puff { name: ACID_FUME, drift: Drift::Float, y: 0.5, spread: [0.25, 0.3, 0.25], count: 2, one_in: 1 }),
    ("biolab:bleeding", Puff { name: BLOOD, drift: Drift::Sink, y: 0.6, spread: [0.25, 0.2, 0.25], count: 2, one_in: 1 }),
];

/// Поведение генов мутанта (флаги `GENE_BEHAVIOUR` в `biolab_core`): некроз
/// заразен — от него споры, кислотные железы окутывают дымом.
pub const BEHAVIOURS: [(&str, Puff); 2] = [
    ("necrosis", Puff { name: SPORE, drift: Drift::Float, y: 0.6, spread: [0.35, 0.4, 0.35], count: 1, one_in: 1 }),
    ("acid_aura", Puff { name: ACID_FUME, drift: Drift::Float, y: 0.3, spread: [0.6, 0.3, 0.6], count: 2, one_in: 1 }),
];

/// Всплеск крови в момент среза скальпелем — разово, сверх кровотечения.
pub const CUT: Puff = Puff { name: BLOOD, drift: Drift::Sink, y: 0.6, spread: [0.3, 0.25, 0.3], count: 8, one_in: 1 };

/// Газовое облако (GDD 4.20): клубы по всему шару, пока облако висит. Висят
/// и оседают (`Sink`) — газ тяжелее воздуха и не должен улетать вверх, как
/// дым. `spread` — доля радиуса облака: точки разбрасывает `sends` в кубе
/// `spread × радиус` (`cloud_puff`).
pub const CLOUD: Puff = Puff { name: GAS, drift: Drift::Sink, y: 0.0, spread: [0.8, 0.5, 0.8], count: 6, one_in: 1 };

/// Хлопок разбитой гранаты — разом, гуще облака.
pub const BURST: Puff = Puff { name: GAS, drift: Drift::Float, y: 0.0, spread: [0.6, 0.4, 0.6], count: 16, one_in: 1 };

/// Затяжка облака радиуса `radius`: разброс — доля радиуса, частиц больше у
/// большого облака.
pub fn cloud_puff(radius: f64) -> Puff {
    let r = radius.clamp(0.5, 8.0);
    Puff {
        spread: [CLOUD.spread[0] * r, CLOUD.spread[1] * r, CLOUD.spread[2] * r],
        count: (CLOUD.count as f64 * r / 2.0).round().max(2.0) as i32,
        ..CLOUD
    }
}

pub fn of_puddle(block: &str) -> Option<Puff> {
    PUDDLES.iter().find(|(b, _)| *b == block).map(|(_, p)| *p)
}

pub fn of_machine(block: &str) -> Option<Puff> {
    MACHINES.iter().find(|(b, _)| *b == block).map(|(_, p)| *p)
}

pub fn of_effect(effect: &str) -> Option<Puff> {
    EFFECTS.iter().find(|(e, _)| *e == effect).map(|(_, p)| *p)
}

/// Ванильные завитки этого эффекта у моба прячем: его показывают наши частицы.
pub fn hides_swirls(effect: &str) -> bool {
    of_effect(effect).is_some()
}

/// Зеркало лужи над низом её блока: лужа — снег по слоям, слой — 1/8 блока.
pub fn puddle_surface(layers: i32) -> f64 {
    f64::from(layers.clamp(1, 8)) / 8.0
}

/// Источник с этим зерном пыхает в этом цикле? Зерно — позиция блока или id
/// моба; хеш раскидывает фазы, чтобы соседи не пыхали в такт.
pub fn due(seed: u64, cycle: u64, one_in: u64) -> bool {
    one_in <= 1 || (mix(seed).wrapping_add(cycle)) % one_in == 0
}

/// Зерно источника-блока.
pub fn block_seed(pos: [i32; 3]) -> u64 {
    let [x, y, z] = pos.map(|v| u64::from(v as u32));
    x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663) ^ z.wrapping_mul(83_492_791)
}

/// Одна посылка `native_send_particles`: где, разброс, скорость, сколько.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Send {
    pub at: [f64; 3],
    pub spread: [f64; 3],
    pub speed: f64,
    pub count: i32,
}

/// Чем отправить затяжку из точки `at`. `Float` — одним пакетом: разброс
/// места делает клиент, скорость 0 оставляет направление случайным. `Sink` —
/// пакетами по одной частице (`count` 0, тогда скорость = `speed × spread` —
/// строго вниз), а место разбрасываем сами, детерминированно от `seed`.
pub fn sends(puff: &Puff, at: [f64; 3], seed: u64) -> Vec<Send> {
    match puff.drift {
        Drift::Float => vec![Send { at, spread: puff.spread, speed: 0.0, count: puff.count.max(1) }],
        Drift::Sink => (0..puff.count.max(1) as u64)
            .map(|n| {
                let h = mix(seed ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15));
                let unit = |shift: u32| f64::from(((h >> shift) & 0xFFFF) as u32) / 65_535.0 * 2.0 - 1.0;
                Send {
                    at: [
                        at[0] + unit(0) * puff.spread[0],
                        at[1] + unit(16) * puff.spread[1],
                        at[2] + unit(32) * puff.spread[2],
                    ],
                    spread: [0.0, -1.0, 0.0],
                    speed: 1.0,
                    count: 0,
                }
            })
            .collect(),
    }
}

/// Точка выпуска над существом: ноги в `feet`, рост `height`.
pub fn over_body(puff: &Puff, feet: [f64; 3], height: f64) -> [f64; 3] {
    [feet[0], feet[1] + height * puff.y, feet[2]]
}

/// Точка выпуска над блоком с поверхностью на высоте `surface` от его низа.
pub fn over_block(puff: &Puff, pos: [i32; 3], surface: f64) -> [f64; 3] {
    [
        f64::from(pos[0]) + 0.5,
        f64::from(pos[1]) + surface + puff.y,
        f64::from(pos[2]) + 0.5,
    ]
}

/// SplitMix64: дешёвый хороший хеш, чтобы фазы и разброс не шли рядами.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_source_names_a_known_particle() {
        let all = PUDDLES.iter().chain(&MACHINES).chain(&EFFECTS).chain(&BEHAVIOURS).map(|(_, p)| p);
        let once = [CUT, CLOUD, BURST];
        for puff in all.chain(&once) {
            assert!(ALL.contains(&puff.name), "{} нет в ALL", puff.name);
            assert!(puff.count >= 1 && puff.one_in >= 1);
        }
        // И наоборот: в ALL нет частиц, которые никто не шлёт.
        for name in ALL {
            let used = PUDDLES.iter().chain(&MACHINES).chain(&EFFECTS).chain(&BEHAVIOURS)
                .any(|(_, p)| p.name == name)
                || once.iter().any(|p| p.name == name);
            assert!(used, "{name} никто не шлёт");
        }
    }

    #[test]
    fn names_are_namespaced_and_unique() {
        for name in ALL {
            let (ns, path) = name.split_once(':').expect("имя с пространством");
            assert_eq!(ns, "biolab");
            assert!(!path.is_empty() && path.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "{name}");
        }
        let mut sorted = ALL.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ALL.len());
    }

    #[test]
    fn lookups_find_their_sources() {
        assert_eq!(of_puddle("biolab:acid_puddle").map(|p| p.name), Some(ACID_FUME));
        assert_eq!(of_puddle("biolab:slime").map(|p| p.name), Some(SPORE));
        assert_eq!(of_puddle("biolab:mutagen_puddle").map(|p| p.name), Some(MUTAGEN_SPORE));
        assert!(of_puddle("biolab:sludge").is_none(), "отстойник не дымит");
        assert_eq!(of_machine("biolab:fermenter").map(|p| p.name), Some(FOAM));
        assert_eq!(of_machine("biolab:cloning_vat").map(|p| p.name), Some(BUBBLE));
        assert_eq!(of_machine("biolab:incubator").map(|p| p.name), Some(STEAM));
        assert!(of_machine("biolab:centrifuge").is_none());
        assert_eq!(of_effect("biolab:sedation").map(|p| p.name), Some(SLEEP));
        assert!(hides_swirls("biolab:bleeding") && !hides_swirls("minecraft:slowness"));
    }

    #[test]
    fn float_is_one_packet_with_no_speed() {
        let puff = of_puddle("biolab:slime").unwrap();
        let out = sends(&puff, [1.0, 2.0, 3.0], 7);
        assert_eq!(out, vec![Send { at: [1.0, 2.0, 3.0], spread: puff.spread, speed: 0.0, count: 1 }]);
    }

    #[test]
    fn sink_sends_single_particles_downwards_within_spread() {
        let out = sends(&CUT, [0.0, 10.0, 0.0], 42);
        assert_eq!(out.len(), CUT.count as usize);
        for s in &out {
            assert_eq!(s.count, 0, "одна частица ровно в точку");
            assert_eq!([s.spread[0] * s.speed, s.spread[1] * s.speed, s.spread[2] * s.speed], [0.0, -1.0, 0.0]);
            for k in 0..3 {
                let base = [0.0, 10.0, 0.0][k];
                assert!((s.at[k] - base).abs() <= CUT.spread[k] + 1e-9, "{s:?}");
            }
        }
        // Капли не в одной точке, и тот же зерно даёт те же капли.
        assert!(out.windows(2).any(|w| w[0].at != w[1].at));
        assert_eq!(out, sends(&CUT, [0.0, 10.0, 0.0], 42));
    }

    #[test]
    fn phases_spread_and_cadence_holds() {
        // Каждый источник пыхает ровно раз за `one_in` циклов.
        for seed in [0u64, 1, 99, block_seed([5, 64, -3])] {
            let hits = (0..20).filter(|c| due(seed, *c, 2)).count();
            assert_eq!(hits, 10);
            assert!((0..5).all(|c| due(seed, c, 1)));
        }
        // У соседних луж фазы разные хотя бы иногда.
        let phase = |pos| (0..2).find(|c| due(block_seed(pos), *c, 2));
        let phases: Vec<_> = (0..8).map(|x| phase([x, 64, 0])).collect();
        assert!(phases.iter().any(|p| *p != phases[0]), "{phases:?}");
    }

    #[test]
    fn a_cloud_fills_its_radius() {
        let small = cloud_puff(2.0);
        let big = cloud_puff(3.0);
        assert_eq!(small.name, GAS);
        assert!(big.count > small.count && big.spread[0] > small.spread[0]);
        assert!(big.spread[0] <= 3.0, "клубы не дальше радиуса");
        let out = sends(&big, [0.0, 64.0, 0.0], 5);
        assert_eq!(out.len(), big.count as usize);
        assert!(out.iter().all(|s| (s.at[0]).abs() <= big.spread[0] + 1e-9));
        assert!(cloud_puff(f64::NAN).count >= 2, "битый радиус не роняет облако");
    }

    #[test]
    fn points_sit_over_their_sources() {
        assert_eq!(puddle_surface(1), 0.125);
        assert_eq!(puddle_surface(5), 0.625);
        assert_eq!(puddle_surface(0), 0.125, "битое свойство — как один слой");
        let acid = of_puddle("biolab:acid_puddle").unwrap();
        let at = over_block(&acid, [2, 64, -1], puddle_surface(5));
        assert_eq!(at, [2.5, 64.0 + 0.625 + acid.y, -0.5]);
        let sleep = of_effect("biolab:sedation").unwrap();
        let head = over_body(&sleep, [0.0, 70.0, 0.0], 2.0);
        assert!(head[1] > 72.0, "сон — над макушкой");
    }
}
