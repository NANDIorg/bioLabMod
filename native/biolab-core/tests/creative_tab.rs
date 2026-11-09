// Вкладка творческого инвентаря (GDD 4.14) — файл контентного мода
// `data/biolab/horizon/creative_tab/lab.json`, кода у неё нет. Поэтому её
// согласие с остальным контентом некому проверить, кроме этого теста.
//
// Зачем это важно: предмет, которого нет в списке вкладки, игра молча не
// покажет (в творчестве его найдёт только поиск), а id, которого нет в
// контенте, молча выбросит. Ни то ни другое в логе не видно.

use std::path::{Path, PathBuf};

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/data/biolab/horizon")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Строки-элементы массива `key` из JSON. Своего разбора JSON у крейта нет
/// (зависимостей ноль), а в файле вкладки массив — только id в кавычках.
fn strings_of(json: &str, key: &str) -> Vec<String> {
    let start = json.find(&format!("\"{key}\"")).unwrap_or_else(|| panic!("нет ключа {key}"));
    let open = start + json[start..].find('[').expect("не массив");
    let close = open + json[open..].find(']').expect("массив не закрыт");
    json[open + 1..close]
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Значение строкового ключа `"key": "value"`.
fn string_of(json: &str, key: &str) -> String {
    let start = json.find(&format!("\"{key}\"")).unwrap_or_else(|| panic!("нет ключа {key}"));
    let rest = &json[start + key.len() + 2..];
    let open = rest.find('"').expect("значение не строка") + 1;
    let close = open + rest[open..].find('"').expect("строка не закрыта");
    rest[open..close].to_string()
}

/// Есть ли у блока предмет: `"item": true` в его файле (без ключа — нет, как
/// считает `tools/check_models.py`).
fn block_has_item(json: &str) -> bool {
    json.find("\"item\"")
        .is_some_and(|i| json[i + 6..].trim_start_matches([' ', ':', '\n']).starts_with("true"))
}

/// Все предметы мода: `item/*.json` и блоки с предметом.
fn mod_items() -> Vec<String> {
    let mut out = Vec::new();
    for (dir, only_with_item) in [("item", false), ("block", true)] {
        for entry in std::fs::read_dir(data().join(dir)).expect("нет папки контента") {
            let path = entry.unwrap().path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".json")) else {
                continue;
            };
            if !only_with_item || block_has_item(&read(&path)) {
                out.push(format!("biolab:{name}"));
            }
        }
    }
    out.sort();
    out
}

/// Группы вкладки в порядке показа: стол и журнал, машины, инструменты,
/// элементы, сырьё и вещества, лекарства, генетика, жидкости. Так вкладка
/// читается как путь игрока, а не как россыпь (GDD 4.14).
fn group(item: &str) -> usize {
    let name = item.strip_prefix("biolab:").unwrap_or(item);
    match name {
        "lab_bench" | "lab_manual" => 0,
        "still" | "incubator" | "fermenter" | "centrifuge" | "synthesizer" | "reactor" | "press"
        | "extractor" | "cloning_vat" | "spectrometer" | "pen" | "console" | "operating_table" => 1,
        // Многоблочные установки (GDD 4.18): общие части и колонна — к машинам.
        "casing" | "casing_glass" | "input_port" | "output_port" | "column" | "column_tray"
        | "column_condenser" => 1,
        // Генный секвенатор (GDD 4.18, п. 3): консоль, стойка, монитор.
        "sequencer" | "sequencer_rack" | "sequencer_monitor" => 1,
        // Биореактор (GDD 4.18, п. 2).
        "bioreactor" => 1,
        // Карантинная камера: фильтр, панели и патрон фильтра — к машинам.
        "quarantine" | "clean_panel" | "filter_cartridge" => 1,
        // Криобанк (GDD 4.23 п. 9) — к машинам, термобокс — к инструментам.
        "cryobank" => 1,
        // Генераторы тиров 3–5 (M10, Б2) — к машинам.
        "solar_panel" | "engine_generator" | "geothermal_tap" | "rtg" => 1,
        // Генераторы тиров 0–2 (M10, GDD 4.27 Б1) и динамо котла — к машинам;
        // пластины столба — сырьё.
        "electrophore" | "piezo_plate" | "voltaic_pile" | "thermocouple" | "steam_boiler" | "dynamo"
        | "water_wheel" | "wind_turbine" => 1,
        // Живые генераторы (M10, GDD 4.27 В) — к машинам; их колба с
        // водорослями и фермент — к веществам (по умолчанию).
        "microbial_cell" | "photobioreactor" | "glucose_cell" | "electrocyte_battery" => 1,
        "thermobox" => 2,
        "flask" | "swab" | "agar" | "analyzer" | "microscope" | "syringe" | "scalpel" | "bandage"
        | "blueprint" | "stretcher" => 2,
        // Карта протокола (GDD 4.23 п. 3) — тоже снаряжение лаборанта.
        "protocol_card" => 2,
        // Оружие для дротиков и баллоны к ружью (GDD 4.20) — тоже инструменты.
        "blowpipe" | "tranq_crossbow" | "air_rifle" | "co2_canister" | "co2_canister_empty" => 2,
        // Спецодежда (GDD 4.25) — тоже снаряжение лаборанта.
        "lab_coat" | "lab_pants" | "lab_shoes" | "hazmat_respirator" | "hazmat_suit" | "hazmat_pants"
        | "hazmat_boots" => 2,
        _ if name.starts_with("element_") => 3,
        "culture" | "culture_weakened" | "mold" | "nutrient_broth" | "mutagen" => 4,
        _ if name.starts_with("extract_") => 4,
        // Препараты; с M5 — ещё гранаты, тряпка и отвар (GDD 4.20).
        _ if ["pill_", "syringe_", "spray_", "dart_", "ointment_", "grenade_", "rag_"]
            .iter()
            .any(|p| name.starts_with(p))
            || name == "remedy"
            || name == "calming_brew" =>
        {
            5
        }
        "mutant_tissue" | "dna_fragment" | "gm_plot" | "gm_crop" => 6,
        // Векторы секвенатора — тоже генетика.
        "gene_vector" | "antisense_vector" => 6,
        _ if name.ends_with("_bucket") => 7,
        // Энергосеть (M10, GDD 4.27): кабели, накопители, батарея — после
        // жидкостей, своей группой; генераторы — сюда же.
        "copper_cable" | "power_cable" | "cable_repeater" | "leyden_jar" | "lead_battery"
        | "capacitor_bank" | "battery" => 8,
        // Новый предмет без группы падает к веществам — тест ниже скажет, если
        // он встал не на своё место.
        _ => 4,
    }
}

#[test]
fn the_tab_holds_every_mod_item_once() {
    let tab = read(&data().join("creative_tab/lab.json"));
    let items = strings_of(&tab, "items");
    let all = mod_items();
    assert!(all.len() > 70, "контент не нашёлся: {} предметов", all.len());

    let missing: Vec<_> = all.iter().filter(|i| !items.contains(i)).collect();
    assert!(missing.is_empty(), "нет во вкладке: {missing:?}");
    let unknown: Vec<_> = items.iter().filter(|i| !all.contains(i)).collect();
    assert!(unknown.is_empty(), "во вкладке, но нет в контенте: {unknown:?}");
    let mut seen = items.clone();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), items.len(), "предмет во вкладке дважды");

    // Иконка — предмет мода (иначе на корешке пусто) и первый во вкладке.
    let icon = string_of(&tab, "icon");
    assert!(all.contains(&icon), "иконка {icon} — не предмет мода");
    assert_eq!(items.first(), Some(&icon));
}

#[test]
fn the_tab_goes_group_by_group() {
    let items = strings_of(&read(&data().join("creative_tab/lab.json")), "items");
    for pair in items.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        assert!(
            (group(a), a) < (group(b), b),
            "{b} стоит после {a}: порядок — по группам, внутри группы по имени"
        );
    }
}

#[test]
fn the_tab_title_is_translated() {
    let tab = read(&data().join("creative_tab/lab.json"));
    assert_eq!(string_of(&tab, "title"), "BioLab");
    // Ключ `itemGroup.<мод>.<файл>` игра ищет раньше самого `title`.
    let lang = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/assets/biolab/lang");
    for file in ["en_us.json", "ru_ru.json"] {
        assert!(read(&lang.join(file)).contains("\"itemGroup.biolab.lab\""), "нет заголовка в {file}");
    }
}
