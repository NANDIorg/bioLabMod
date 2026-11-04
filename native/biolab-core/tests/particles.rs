// Свои частицы (GDD 3.9): таблица `particles.rs` в игре живёт отдельным
// модулем бандла (`crate::game::biolab_particles`), а здесь подключается как
// обычный модуль — его внутренние тесты идут тут же. Ниже — то, что модуль
// сам проверить не может: файлы частиц в контентном моде и согласие таблицы
// с остальным `biolab_core`.
//
// Зачем это важно: частицу без `particles/<имя>.json` клиент Horizon просто
// не рисует, а сервер всё равно отвечает «принято» — ошибки нигде не будет.

// Константы цикла фона (`EVERY`, `SCAN`…) читает только хелпер `ambient.rs`,
// в тестах они не нужны.
#[allow(dead_code)]
#[path = "../mixins/biolab_core/particles.rs"]
mod particles;

use std::path::{Path, PathBuf};

fn content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/assets/biolab")
}

/// Строки массива `"textures": [...]` из JSON спрайт-сета. Своего разбора
/// JSON у крейта нет (зависимостей ноль), а формат здесь простой.
fn textures_of(json: &str) -> Vec<String> {
    let start = json.find("\"textures\"").expect("нет ключа textures");
    let open = start + json[start..].find('[').expect("textures не массив");
    let close = open + json[open..].find(']').expect("массив не закрыт");
    json[open + 1..close]
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Ширина и высота PNG по заголовку IHDR.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let be = |i: usize| u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    Some((be(16), be(20)))
}

#[test]
fn every_particle_the_code_sends_has_its_sprite_set_and_textures() {
    for name in particles::ALL {
        let path = name.strip_prefix("biolab:").expect("частица мода — в biolab");
        let file = content().join("particles").join(format!("{path}.json"));
        let json = std::fs::read_to_string(&file)
            .unwrap_or_else(|_| panic!("{name}: нет {}", file.display()));
        let textures = textures_of(&json);
        assert!(!textures.is_empty(), "{name}: пустой спрайт-сет");
        for texture in textures {
            // `biolab:<путь>` ищется в атласе как `biolab:particle/<путь>`,
            // то есть файл `textures/particle/<путь>.png`.
            let rel = texture.strip_prefix("biolab:").unwrap_or_else(|| panic!("{name}: {texture} не из biolab"));
            for suffix in ["_n", "_s", "_e"] {
                assert!(!rel.ends_with(suffix), "{texture}: суффикс {suffix} атлас примет за служебную карту");
            }
            let png = content().join("textures/particle").join(format!("{rel}.png"));
            let bytes = std::fs::read(&png).unwrap_or_else(|_| panic!("{name}: нет {}", png.display()));
            assert_eq!(png_size(&bytes), Some((8, 8)), "{}", png.display());
        }
    }
}

#[test]
fn no_sprite_set_lies_unused() {
    let dir = content().join("particles");
    for entry in std::fs::read_dir(&dir).expect("папка particles") {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "json") {
            let stem = path.file_stem().unwrap().to_string_lossy();
            let name = format!("biolab:{stem}");
            assert!(particles::ALL.contains(&name.as_str()), "{name}: файл есть, в ALL нет");
        }
    }
}

#[test]
fn sources_match_the_rest_of_the_mod() {
    // Лужи — те же блоки, что в таблице луж мода.
    for (block, _) in particles::PUDDLES {
        assert!(biolab_core::fluid_of_block(block).is_some(), "{block} не лужа");
    }
    // Машины — наши машины.
    for (block, _) in particles::MACHINES {
        assert!(biolab_core::machine(block).is_some(), "{block} не машина");
    }
    // Эффекты — наши эффекты, под теми же именами, что выдаёт код.
    let acid = biolab_core::fluid_of_block("biolab:acid_puddle").unwrap().effect;
    for effect in [biolab_core::SEDATION_EFFECT, biolab_core::BLEEDING, acid] {
        assert!(particles::of_effect(effect).is_some(), "{effect} без частиц");
    }
    for (effect, _) in particles::EFFECTS {
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../content/data/biolab/horizon/mob_effect/{}.json",
            effect.strip_prefix("biolab:").unwrap()
        ));
        assert!(file.is_file(), "{effect}: нет такого эффекта в контенте");
    }
    // Флаги поведения — те, что знают гены.
    for (flag, _) in particles::BEHAVIOURS {
        assert!(
            biolab_core::GENE_BEHAVIOUR.iter().any(|flags| flags.contains(&flag)),
            "{flag}: ни у одного гена нет такого поведения"
        );
    }
}

#[test]
fn particles_are_named_only_through_the_table() {
    // Имя частицы строкой прямо в хелпере обошло бы проверку файлов выше.
    let native = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut calls = 0;
    for bundle in ["biolab-core/mixins/biolab_core", "biolab-deep/mixins/biolab_deep"] {
        for entry in std::fs::read_dir(native.join(bundle)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for (i, _) in text.match_indices("ParticleOptions::named(") {
                calls += 1;
                let arg = text[i + "ParticleOptions::named(".len()..].trim_start();
                assert!(!arg.starts_with('"'), "{}: имя частицы строкой, а не из таблицы", path.display());
            }
        }
    }
    assert!(calls >= 3, "вызовы частиц пропали: {calls}");
}
