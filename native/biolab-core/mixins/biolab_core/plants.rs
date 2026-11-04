// Свои растения (GDD 4.20, «Свои растения»): валериана, белладонна,
// мандрагора. Таблица без типов игры: в игре это `crate::game::biolab_plants`,
// а тесты идут обычным `cargo test` (`tests/plants.rs` подключает файл как
// модуль и сверяет его с контентом).
//
// Что делает сам шаблон блока (`minecraft:beetroots`, классы `BeetrootBlock` →
// `CropBlock` в `block-behaviours.json`): рост на случайном тике, костная мука
// (`Kind::Crop`), выживание только на пашне и «выскакивание» с выпадением при
// вытаптывании — всё по классу, а не по имени (`horizon-world/src/level/tick.rs`,
// `bonemeal.rs`, `survive.rs`). Своего здесь три вещи:
//   * посадка семян (`plant_seed.rs`): семя — отдельный предмет с другим id,
//     игра сама его не сажает (см. `SEEDS_TEMPLATE`);
//   * где стоит дикое растение (`plant_ground.rs`): шаблон держится только на
//     пашне, а заросли растут на траве, подзоле и иле;
//   * крик мандрагоры (`mandrake.rs`) и семена с мобов (`plant_drops.rs`).

/// Шаблон семян. Ванильный предмет-блок свёклы, но **сажать сам не будет**:
/// какой блок ставит предмет, игра узнаёт по обратной таблице «блок → его
/// предмет» (`horizon-items/src/items.rs`, `block_for_item`), а у наших
/// культур предмета нет (`"item": false`). Копия же предмета-блока свёклы с
/// `"item": true` назвалась бы как блок (`biolab:valerian`) и сажала бы
/// ванильную свёклу: в привязке предмета `fields.block` переименовывается
/// только у шаблонов с одинаковым именем блока и предмета
/// (`mod_content.rs`, `items`). Шаблон даёт компост и вид семян.
pub const SEEDS_TEMPLATE: &str = "minecraft:beetroot_seeds";
/// Шаблон блока-культуры: четыре стадии, `age` 0–3.
pub const CROP_TEMPLATE: &str = "minecraft:beetroots";
/// Спелая стадия шаблона (`BeetrootBlock.MAX_AGE`).
pub const RIPE_AGE: &str = "3";
/// Пашня: на неё сажают семена.
pub const FARMLAND: &str = "minecraft:farmland";
/// Звук посадки — как у ванильных семян (тип звука `crop`).
pub const PLANT_SOUND: &str = "minecraft:item.crop.plant";
/// Категория громкости «блоки» (`SOUND_SOURCES` Horizon).
pub const SOURCE_BLOCK: usize = 4;
/// Ячейка шлема в нумерации `native_carried_stack` (`Inventory`: 100 ноги … 103 голова).
pub const HEAD_SLOT: i32 = 103;

/// Растение мода.
pub struct Plant {
    /// Блок-культура.
    pub crop: &'static str,
    pub seeds: &'static str,
    /// Урожай со спелого растения.
    pub harvest: &'static str,
    /// Имя в родительном падеже для сообщений: «семена валерианы».
    pub of: &'static str,
    /// На чём, кроме пашни, стоит дикое растение. Простой земли здесь нет:
    /// вытоптанная пашня становится землёй, и культура с неё должна слететь,
    /// как ванильная.
    pub wild_ground: &'static [&'static str],
    /// Ванильные биомы зарослей (модификатор `neoforge/biome_modifier/`).
    pub biomes: &'static [&'static str],
    /// Моб, с которого падают семена, и шанс «один из N» (запасной путь GDD).
    pub dropped_by: (&'static str, u64),
}

pub const PLANTS: [Plant; 3] = [
    Plant {
        crop: "biolab:valerian",
        seeds: "biolab:valerian_seeds",
        harvest: "biolab:valerian_root",
        of: "валерианы",
        wild_ground: &["minecraft:grass_block"],
        biomes: &["minecraft:meadow", "minecraft:plains", "minecraft:sunflower_plains", "minecraft:flower_forest"],
        dropped_by: ("minecraft:fox", 4),
    },
    Plant {
        crop: "biolab:belladonna",
        seeds: "biolab:belladonna_seeds",
        harvest: "biolab:belladonna_berries",
        of: "белладонны",
        wild_ground: &["minecraft:grass_block", "minecraft:podzol"],
        biomes: &[
            "minecraft:forest",
            "minecraft:birch_forest",
            "minecraft:old_growth_birch_forest",
            "minecraft:dark_forest",
        ],
        dropped_by: ("minecraft:zombie_villager", 5),
    },
    Plant {
        crop: "biolab:mandrake",
        seeds: "biolab:mandrake_seeds",
        harvest: "biolab:mandrake_root",
        of: "мандрагоры",
        wild_ground: &["minecraft:grass_block", "minecraft:mud"],
        biomes: &["minecraft:swamp", "minecraft:mangrove_swamp"],
        dropped_by: ("minecraft:witch", 3),
    },
];

pub const MANDRAKE: &str = "biolab:mandrake";

pub fn plant_of_seeds(item: &str) -> Option<&'static Plant> {
    PLANTS.iter().find(|p| p.seeds == item)
}

pub fn plant_of_crop(block: &str) -> Option<&'static Plant> {
    PLANTS.iter().find(|p| p.crop == block)
}

/// `VegetationBlock.mayPlaceOn` для наших культур: `Some(true)` — на диком
/// грунте растения; `None` — пусть решает шаблон (пашня — да, остальное — нет).
/// Чужой блок — тоже `None`.
pub fn may_stand_on(crop: &str, ground: &str) -> Option<bool> {
    let plant = plant_of_crop(crop)?;
    plant.wild_ground.contains(&ground).then_some(true)
}

fn is_air(block: &str) -> bool {
    matches!(block, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
}

/// Куда ляжет семя при клике по `pos` гранью `face`: в соседнюю клетку, как
/// ставит ванильный предмет-блок по незаменяемому блоку (пашня не заменяется).
pub fn plant_target(pos: [i32; 3], face: [i32; 3]) -> [i32; 3] {
    [pos[0] + face[0], pos[1] + face[1], pos[2] + face[2]]
}

/// Можно ли посадить: клетка пуста, под ней пашня. Так ванилла и проверяет
/// (`BlockItem.place` → `canSurvive` культуры), только без предмета-блока.
pub fn can_plant(target: &str, below: &str) -> bool {
    is_air(target) && below == FARMLAND
}

/// Что делает крик мандрагоры, когда её срывают.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scream {
    /// Не мандрагора или неспелая: молчит.
    Silent,
    /// Кричит, но уши закрыты: только звук.
    Muffled,
    /// Кричит в открытые уши: тошнота.
    Nausea,
}

/// Тошнота от крика: коротко, 6 секунд.
pub const SCREAM_NAUSEA_TICKS: i32 = 120;
/// Крик — ванильный звук с поднятым тоном: своего звука мандрагоре пока нет.
pub const SCREAM_SOUND: &str = "minecraft:entity.ghast.scream";
pub const SCREAM_PITCH: f32 = 1.7;
pub const SCREAM_VOLUME: f32 = 0.7;

/// Кричит ли сорванная культура. Ушных затычек в моде нет, поэтому уши
/// закрывает **любой предмет в слоте головы** — шлем, черепаший панцирь,
/// тыква, голова: проверка одна и дешёвая (`native_carried_stack(103)`).
pub fn scream(block: &str, age: Option<&str>, head_covered: bool) -> Scream {
    if block != MANDRAKE || age != Some(RIPE_AGE) {
        Scream::Silent
    } else if head_covered {
        Scream::Muffled
    } else {
        Scream::Nausea
    }
}

pub const SCREAM_MESSAGE: &str = "Мандрагора кричит! Без шлема в ушах звенит";
pub const SCREAM_MUFFLED: &str = "Мандрагора кричит, но шлем глушит крик";

/// Перемешивает зерно (splitmix64): у соседних id и тиков — разные броски.
pub fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Семена с умершего моба: какие и сколько (1–2), если выпали. Только с
/// убитого игроком — как редкий ванильный лут (`killed_by_player`), чтобы
/// фермы мобов не сыпали семенами. `seed` — зерно броска (id моба и тик).
pub fn mob_seed_drop(kind: &str, killed_by_player: bool, seed: u64) -> Option<(&'static str, u32)> {
    if !killed_by_player {
        return None;
    }
    let plant = PLANTS.iter().find(|p| p.dropped_by.0 == kind)?;
    let roll = mix(seed);
    (roll % plant.dropped_by.1 == 0).then(|| (plant.seeds, 1 + ((roll >> 32) % 2) as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_the_shared_ones() {
        // Эти id используют другие части мода (рецепты переработки урожая).
        let ids: Vec<[&str; 3]> = PLANTS.iter().map(|p| [p.crop, p.seeds, p.harvest]).collect();
        assert_eq!(
            ids,
            vec![
                ["biolab:valerian", "biolab:valerian_seeds", "biolab:valerian_root"],
                ["biolab:belladonna", "biolab:belladonna_seeds", "biolab:belladonna_berries"],
                ["biolab:mandrake", "biolab:mandrake_seeds", "biolab:mandrake_root"],
            ]
        );
        assert_eq!(plant_of_seeds("biolab:mandrake_seeds").map(|p| p.crop), Some(MANDRAKE));
        assert!(plant_of_seeds("minecraft:wheat_seeds").is_none());
    }

    #[test]
    fn seeds_go_only_onto_farmland_from_above() {
        assert_eq!(plant_target([0, 63, 0], [0, 1, 0]), [0, 64, 0]);
        assert!(can_plant("minecraft:air", FARMLAND));
        assert!(!can_plant("minecraft:air", "minecraft:grass_block"), "на траву не сажается");
        assert!(!can_plant("minecraft:air", "minecraft:dirt"));
        assert!(!can_plant("biolab:valerian", FARMLAND), "занято");
    }

    #[test]
    fn wild_plants_stand_on_their_ground_but_not_on_trampled_farmland() {
        assert_eq!(may_stand_on("biolab:valerian", "minecraft:grass_block"), Some(true));
        assert_eq!(may_stand_on("biolab:mandrake", "minecraft:mud"), Some(true));
        assert_eq!(may_stand_on("biolab:belladonna", "minecraft:podzol"), Some(true));
        // Вытоптанная пашня — земля: решает шаблон, и культура слетает.
        for p in &PLANTS {
            assert_eq!(may_stand_on(p.crop, "minecraft:dirt"), None, "{}", p.crop);
            assert_eq!(may_stand_on(p.crop, FARMLAND), None, "пашню разрешает шаблон");
        }
        assert_eq!(may_stand_on("minecraft:wheat", "minecraft:grass_block"), None, "чужой блок");
    }

    #[test]
    fn only_a_ripe_mandrake_screams() {
        assert_eq!(scream(MANDRAKE, Some("3"), false), Scream::Nausea);
        assert_eq!(scream(MANDRAKE, Some("3"), true), Scream::Muffled);
        assert_eq!(scream(MANDRAKE, Some("2"), false), Scream::Silent);
        assert_eq!(scream("biolab:valerian", Some("3"), false), Scream::Silent);
        assert_eq!(scream(MANDRAKE, None, false), Scream::Silent);
    }

    #[test]
    fn mobs_drop_seeds_now_and_then() {
        for p in &PLANTS {
            let (kind, one_in) = p.dropped_by;
            let drops: Vec<_> = (0..3000u64).filter_map(|s| mob_seed_drop(kind, true, s)).collect();
            let share = drops.len() as f64 / 3000.0;
            let want = 1.0 / one_in as f64;
            assert!((share - want).abs() < 0.04, "{kind}: {share} вместо {want}");
            assert!(drops.iter().all(|(id, n)| *id == p.seeds && (1..=2).contains(n)));
            assert!(drops.iter().any(|(_, n)| *n == 2));
            assert!((0..200u64).all(|s| mob_seed_drop(kind, false, s).is_none()), "только убитый игроком");
        }
        assert!((0..200u64).all(|s| mob_seed_drop("minecraft:zombie", true, s).is_none()));
    }
}
