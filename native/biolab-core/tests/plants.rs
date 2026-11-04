// Свои растения (GDD 4.20): таблица `plants.rs` в игре живёт отдельным
// модулем бандла (`crate::game::biolab_plants`), а здесь подключается как
// обычный модуль — его внутренние тесты идут тут же. Ниже — то, что модуль
// сам проверить не может: согласие таблицы с данными контентного мода.
//
// Зачем это важно: заросли ставит генерация по файлам `worldgen/`, а держит на
// месте хук `plant_ground.rs` по таблице. Разойдутся грунты — дикое растение
// слетит при первом обновлении соседа, и никто этого не заметит.
// Структуру worldgen-файлов (типы, шаги, ванильные биомы) проверяет
// `tools/check_worldgen.py`.

// Константы крика и посадки читают только хелперы бандла.
#[allow(dead_code)]
#[path = "../mixins/biolab_core/plants.rs"]
mod plants;

use biolab_core::Json;
use std::path::{Path, PathBuf};

fn content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

fn json(rel: &str) -> Json {
    let path = content().join(rel);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Json::parse(&text).unwrap_or_else(|| panic!("{rel}: не JSON"))
}

fn short(id: &str) -> &str {
    id.strip_prefix("biolab:").expect("id мода")
}

fn strings(v: Option<&Json>) -> Vec<String> {
    match v {
        Some(Json::Str(s)) => vec![s.clone()],
        Some(Json::Arr(list)) => list.iter().filter_map(|j| j.str().map(str::to_string)).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn every_plant_is_a_crop_with_seeds_and_harvest() {
    for p in &plants::PLANTS {
        let block = json(&format!("data/biolab/horizon/block/{}.json", short(p.crop)));
        assert_eq!(block.get("template").and_then(Json::str), Some(plants::CROP_TEMPLATE), "{}", p.crop);
        assert_eq!(block.get("item"), Some(&Json::Bool(false)), "{}: семена — отдельный предмет", p.crop);
        let seeds = json(&format!("data/biolab/horizon/item/{}.json", short(p.seeds)));
        assert_eq!(seeds.get("template").and_then(Json::str), Some(plants::SEEDS_TEMPLATE), "{}", p.seeds);
        assert!(content().join(format!("data/biolab/horizon/item/{}.json", short(p.harvest))).exists());
        for item in [p.seeds, p.harvest] {
            assert!(content().join(format!("assets/biolab/items/{}.json", short(item))).exists(), "{item}");
        }
        // Четыре стадии шаблона — четыре варианта в blockstate.
        let states = json(&format!("assets/biolab/blockstates/{}.json", short(p.crop)));
        for age in 0..4 {
            assert!(states.get("variants").and_then(|v| v.get(&format!("age={age}"))).is_some(), "{} age={age}", p.crop);
        }
    }
}

#[test]
fn ripe_crop_drops_harvest_and_seeds_unripe_only_seeds() {
    for p in &plants::PLANTS {
        let table = json(&format!("data/biolab/loot_table/blocks/{}.json", short(p.crop)));
        let text = format!("{table:?}");
        assert!(text.contains(p.harvest) && text.contains(p.seeds), "{}", p.crop);
        let pools = table.get("pools").and_then(Json::arr).expect("pools");
        // Первый пул: урожай, если спелое, иначе семена.
        let children = pools[0].get("entries").and_then(Json::arr).unwrap()[0]
            .get("children")
            .and_then(Json::arr)
            .expect("alternatives");
        assert_eq!(children[0].get("name").and_then(Json::str), Some(p.harvest));
        assert_eq!(children[1].get("name").and_then(Json::str), Some(p.seeds));
        let ripe = children[0].get("condition").expect("условие спелости");
        assert_eq!(ripe.get("type").and_then(Json::str), Some("minecraft:match_block"));
        assert_eq!(ripe.get("blocks").and_then(Json::str), Some(p.crop));
        assert_eq!(ripe.get("state").and_then(|s| s.get("age")).and_then(Json::str), Some(plants::RIPE_AGE));
        // Второй пул — семена со спелого, тем же условием.
        assert_eq!(pools[1].get("condition"), Some(ripe));
        assert_eq!(
            table.get("random_sequence").and_then(Json::str),
            Some(format!("biolab:blocks/{}", short(p.crop)).as_str())
        );
    }
}

#[test]
fn wild_patches_grow_where_the_table_says() {
    for p in &plants::PLANTS {
        let patch = format!("{}_patch", short(p.crop));
        let modifier = json(&format!("data/biolab/neoforge/biome_modifier/{patch}.json"));
        assert_eq!(modifier.get("type").and_then(Json::str), Some("neoforge:add_features"));
        assert_eq!(modifier.get("step").and_then(Json::str), Some("vegetal_decoration"));
        assert_eq!(strings(modifier.get("features")), vec![format!("biolab:{patch}")]);
        assert_eq!(strings(modifier.get("biomes")), p.biomes.to_vec(), "{patch}: биомы");

        let placed = json(&format!("data/biolab/worldgen/placed_feature/{patch}.json"));
        assert_eq!(placed.get("feature").and_then(Json::str), Some(format!("biolab:{patch}").as_str()));
        // Грунт фильтра размещения = грунт, на котором хук держит растение.
        let filter = placed
            .get("placement")
            .and_then(Json::arr)
            .unwrap()
            .iter()
            .find(|m| m.get("type").and_then(Json::str) == Some("minecraft:block_predicate_filter"))
            .expect("фильтр по грунту");
        let ground: Vec<String> = filter
            .get("predicate")
            .and_then(|p| p.get("predicates"))
            .and_then(Json::arr)
            .unwrap()
            .iter()
            .filter(|q| q.get("type").and_then(Json::str) == Some("minecraft:matching_blocks"))
            .flat_map(|q| strings(q.get("blocks")))
            .collect();
        assert_eq!(ground, p.wild_ground.to_vec(), "{patch}: грунт генерации и хука разошёлся");
        for g in p.wild_ground {
            assert_eq!(plants::may_stand_on(p.crop, g), Some(true));
        }

        // Ставится спелым и без проверки выживания: `simple_block` спросил бы
        // правило культуры (пашня под ней) и не поставил бы ничего.
        let feature = json(&format!("data/biolab/worldgen/configured_feature/{patch}.json"));
        assert_eq!(feature.get("type").and_then(Json::str), Some("minecraft:block_column"));
        let provider = feature.get("layers").and_then(Json::arr).unwrap()[0].get("provider").unwrap();
        assert_eq!(provider.get("id").and_then(Json::str), Some(p.crop));
        assert_eq!(provider.get("properties").and_then(|v| v.get("age")).and_then(Json::str), Some(plants::RIPE_AGE));
    }
}

#[test]
fn the_bundle_wires_every_helper() {
    let manifest = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("mixins/biolab_core/horizon-mixin.toml"),
    )
    .unwrap();
    for (helper, target) in [
        ("plant_seed.rs", "native_use_item_on"),
        ("plant_ground.rs", "GameLevel<'_>::native_may_place_on"),
        ("mandrake.rs", "native_destroy_block"),
        ("plant_drops.rs", "native_loot_drops"),
    ] {
        let at = manifest.find(&format!("helper = \"{helper}\"")).unwrap_or_else(|| panic!("{helper} не подключён"));
        let block = &manifest[..at];
        let target_line = block.rfind("target = ").unwrap();
        assert!(block[target_line..].contains(target), "{helper}: не тот хук");
    }
    assert!(manifest.contains("source = \"plants.rs\""));
}
