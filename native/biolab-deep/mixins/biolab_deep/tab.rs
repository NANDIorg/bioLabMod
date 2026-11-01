// Порядок и подписи своей вкладки творческого инвентаря.
//
// Этот файл подключается как модуль `crate::biolab_tab` крейта
// horizon-client: вкладки живут там же, где `creative`.

/// Идентификатор и заголовок вкладки.
pub const ID: &str = "biolab:lab";
pub const TITLE: &str = "BioLab";
/// Чем вкладка отмечена на корешке.
pub const ICON: &str = "biolab:lab_bench";
/// Пространство имён мода: по нему предметы и отбираются.
pub const PREFIX: &str = "biolab:";

/// Предметы и блоки мода для вкладки. Список собирается скриптом из файлов
/// контента (`scratchpad/m7_tab.py`): в творческие вкладки игры предметы мода
/// не попадают, брать их оттуда нечего.
pub const ITEMS: [&str; 77] = [
    "biolab:centrifuge",
    "biolab:cloning_vat",
    "biolab:console",
    "biolab:extractor",
    "biolab:fermenter",
    "biolab:gm_plot",
    "biolab:incubator",
    "biolab:lab_bench",
    "biolab:operating_table",
    "biolab:pen",
    "biolab:press",
    "biolab:reactor",
    "biolab:spectrometer",
    "biolab:still",
    "biolab:synthesizer",
    "biolab:acid_puddle_bucket",
    "biolab:agar",
    "biolab:analyzer",
    "biolab:bandage",
    "biolab:blueprint",
    "biolab:culture",
    "biolab:culture_weakened",
    "biolab:dart_sedative",
    "biolab:dart_venom",
    "biolab:dna_fragment",
    "biolab:element_calcium",
    "biolab:element_carbon",
    "biolab:element_chlorine",
    "biolab:element_copper",
    "biolab:element_glaurium",
    "biolab:element_gold",
    "biolab:element_hydrogen",
    "biolab:element_iodine",
    "biolab:element_iron",
    "biolab:element_magnesium",
    "biolab:element_nitrogen",
    "biolab:element_oxygen",
    "biolab:element_phosphorus",
    "biolab:element_potassium",
    "biolab:element_redstonium",
    "biolab:element_silicon",
    "biolab:element_sodium",
    "biolab:element_sulfur",
    "biolab:extract_caffeine",
    "biolab:extract_penicillin",
    "biolab:extract_raw",
    "biolab:extract_salicylate",
    "biolab:extract_sedative",
    "biolab:extract_venom",
    "biolab:flask",
    "biolab:gm_crop",
    "biolab:microscope",
    "biolab:mold",
    "biolab:mutagen",
    "biolab:mutagen_bucket",
    "biolab:mutant_tissue",
    "biolab:nutrient_broth",
    "biolab:ointment_analgesic",
    "biolab:pill_analgesic",
    "biolab:pill_antidote",
    "biolab:pill_penicillin",
    "biolab:pill_stimulant",
    "biolab:remedy",
    "biolab:scalpel",
    "biolab:slime_bucket",
    "biolab:sludge_bucket",
    "biolab:spray_analgesic",
    "biolab:spray_stimulant",
    "biolab:stretcher",
    "biolab:swab",
    "biolab:syringe",
    "biolab:syringe_mutagen",
    "biolab:syringe_penicillin",
    "biolab:syringe_sedative",
    "biolab:syringe_stimulant",
    "biolab:syringe_vaccine",
    "biolab:syringe_venom",
];

/// Группы в порядке показа: сначала инструменты и машины, потом сырьё,
/// вещества, лекарства и в конце — генетика. Так вкладка читается как путь
/// игрока, а не как случайная россыпь.
pub const GROUPS: [&str; 8] = [
    "bench", "machine", "tool", "element", "compound", "medicine", "gene", "fluid",
];

/// К какой группе отнести предмет и каким он в ней идёт.
pub fn order(item: &str) -> (usize, String) {
    let name = item.strip_prefix(PREFIX).unwrap_or(item);
    let group = match name {
        "lab_bench" => 0,
        "still" | "incubator" | "fermenter" | "centrifuge" | "synthesizer" | "reactor"
        | "press" | "extractor" | "cloning_vat" | "spectrometer" | "pen" | "console"
        | "operating_table" => 1,
        "flask" | "swab" | "agar" | "analyzer" | "microscope" | "syringe" | "scalpel"
        | "bandage" | "blueprint" | "stretcher" => 2,
        _ if name.starts_with("element_") => 3,
        "culture" | "culture_weakened" | "mold" | "nutrient_broth" | "mutagen" => 4,
        _ if name.starts_with("extract_") => 4,
        _ if name.starts_with("pill_")
            || name.starts_with("syringe_")
            || name.starts_with("spray_")
            || name.starts_with("dart_")
            || name.starts_with("ointment_")
            || name == "remedy" =>
        {
            5
        }
        "mutant_tissue" | "dna_fragment" | "gm_plot" | "gm_crop" => 6,
        _ if name.ends_with("_bucket") || name.ends_with("_puddle") => 7,
        _ => 4,
    };
    (group, name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_fall_into_their_groups() {
        let group = |item: &str| order(item).0;
        assert_eq!(group("biolab:lab_bench"), 0);
        assert_eq!(group("biolab:reactor"), 1);
        assert_eq!(group("biolab:microscope"), 2);
        assert_eq!(group("biolab:element_carbon"), 3);
        assert_eq!(group("biolab:extract_penicillin"), 4);
        assert_eq!(group("biolab:pill_antidote"), 5);
        assert_eq!(group("biolab:dart_venom"), 5);
        assert_eq!(group("biolab:dna_fragment"), 6);
        assert_eq!(group("biolab:acid_puddle_bucket"), 7);
        // Незнакомый предмет не теряется, а падает к веществам.
        assert!(group("biolab:something_new") < GROUPS.len());
    }

    #[test]
    fn the_order_is_stable_and_by_name() {
        let mut items = vec![
            "biolab:element_sulfur",
            "biolab:lab_bench",
            "biolab:element_carbon",
            "biolab:reactor",
        ];
        items.sort_by_key(|i| order(i));
        assert_eq!(
            items,
            [
                "biolab:lab_bench",
                "biolab:reactor",
                "biolab:element_carbon",
                "biolab:element_sulfur"
            ]
        );
    }
}
