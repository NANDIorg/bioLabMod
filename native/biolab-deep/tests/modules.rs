// Тесты модулей, которые в игре живут в других крейтах (иконки — в
// horizon-client-render, мутанты и эффекты мобов — в lib horizon-client).
// Здесь они подключаются как обычные модули; функции, которым нужна сама игра,
// отгорожены `#[cfg(not(test))]`, а их внутренние тесты прогоняются тут.

#[path = "../mixins/biolab_deep/atlas.rs"]
mod atlas;
#[path = "../mixins/biolab_deep/effects.rs"]
mod effects;
#[path = "../mixins/biolab_deep/mutants.rs"]
mod mutants;
#[path = "../mixins/biolab_deep/tab.rs"]
mod tab;
#[path = "../mixins/biolab_deep/surgery.rs"]
mod surgery;
#[path = "../mixins/biolab_deep/growths.rs"]
mod growths;

/// Наросты читают силу гена по имени: каждая видимая черта (GDD 4.19) должна
/// называть ген из таблицы генов, иначе она молча не нарисуется.
#[test]
fn growth_traits_name_real_genes() {
    for gene in growths::TRAITS {
        assert!(mutants::gene_index(gene).is_some(), "{gene} нет в таблице генов");
    }
    // Сила гена приходит из генома так же, как её читает `growths::strength`.
    let mut genome = mutants::Genome::default();
    assert!(genome.add(mutants::Gene { index: mutants::gene_index("tail").unwrap(), strength: 2 }));
    assert_eq!(genome.strength_of("tail").min(3), 2);
    assert_eq!(genome.strength_of("horns"), 0);
}

/// Урон своих эффектов у игрока (`runtime.rs`, бинарник) и у мобов
/// (`effects.rs`, библиотека) считается двумя копиями одной логики: они не
/// должны разойтись ни по одному эффекту.
#[test]
fn player_and_mob_damage_agree() {
    assert_eq!(biolab_deep::HURTING.len(), effects::HURTING.len());
    for effect in &biolab_deep::HURTING {
        assert!(effects::hurting(effect.id).is_some(), "{} только у игрока", effect.id);
        for amp in -1..12 {
            for ticks in -1..200 {
                assert_eq!(
                    biolab_deep::effect_damage(effect.id, ticks, amp),
                    effects::effect_damage(effect.id, ticks, amp),
                    "{}: уровень {amp}, тиков {ticks}",
                    effect.id
                );
            }
        }
    }
    assert_eq!(biolab_deep::SEDATION, effects::SEDATION);
}

#[test]
fn icon_alias_matches_the_engine_key() {
    assert_eq!(
        atlas::alias_for("biolab:gui/mob_effect/acid_burn").as_deref(),
        Some("mob_effect/biolab:acid_burn")
    );
}

/// Геном выдаёт бинарник (`runtime.rs`), а читает по меткам lib
/// (`mutants.rs`): имена генов, их цена и метки в обеих копиях должны
/// совпадать, иначе мутант, записанный в сейв, перестанет узнаваться.
#[test]
fn binary_and_lib_genes_agree() {
    assert_eq!(biolab_deep::GENE_NAMES.len(), mutants::GENE_NAMES.len());
    assert_eq!(biolab_deep::GENE_MARKERS.len(), mutants::GENE_MARKERS.len());
    // Предел числа генов и стабильности — только у природного броска в
    // бинарнике; геном lib держит все гены таблицы (GDD 4.19).
    assert!(biolab_deep::MAX_GENES < mutants::GENE_COUNT);
    for (i, name) in biolab_deep::GENE_NAMES.iter().enumerate() {
        assert_eq!(*name, mutants::GENE_NAMES[i], "ген {i}");
        assert_eq!(biolab_deep::GENE_COSTS[i], mutants::gene_cost(i), "{name}");
        for strength in 1..=3 {
            let marker = biolab_deep::marker(i, strength).expect(name);
            let gene = mutants::decode_marker(marker).expect(marker);
            assert_eq!(gene.index, i);
            assert_eq!(gene.strength, strength);
            assert_eq!(mutants::marker_of(gene), Some(marker));
        }
    }
}

/// Любой брошенный бинарником геном lib должна принять целиком: и по числу
/// генов, и по стабильности.
#[test]
fn every_rolled_genome_fits_the_lib() {
    use biolab_deep::Temperament as T;
    for (draw, temper) in (0..4000).flat_map(|d| [(d, T::Hostile), (d, T::Neutral), (d, T::Passive)]) {
        let rolled = biolab_deep::roll(draw, 100, temper);
        let mut genome = mutants::Genome::default();
        for (gene, strength) in &rolled {
            assert!(
                genome.add(mutants::Gene { index: *gene, strength: *strength }),
                "бросок {draw}: lib не приняла {rolled:?}"
            );
        }
        assert_eq!(genome.list().count(), rolled.len());
        assert_eq!(mutants::cost(&genome), biolab_deep::cost(&rolled));
        assert_eq!(mutants::from_text(&mutants::to_text(&genome)), Some(genome));
    }
}

/// Лабораторный геном больше природного: метки, которые поставил укол или
/// чан, lib читает все до одной, сколько бы их ни было (GDD 4.19).
#[test]
fn a_lab_genome_beyond_the_natural_limit_is_read_whole() {
    let mut genome = mutants::Genome::default();
    let mut markers = Vec::new();
    for gene in 0..biolab_deep::GENE_NAMES.len() {
        let strength = 1 + (gene % 3) as i32;
        markers.push(biolab_deep::marker(gene, strength).expect("метка есть"));
    }
    // Так геном собирает `spawn.rs`: по меткам на мобе.
    for marker in &markers {
        let gene = mutants::decode_marker(marker).expect(marker);
        assert!(genome.add(gene), "{marker}");
    }
    assert_eq!(genome.count(), biolab_deep::GENE_NAMES.len());
    assert!(biolab_deep::cost(&genome.list().map(|g| (g.index, g.strength)).collect::<Vec<_>>()) > biolab_deep::STABILITY);
    for (gene, name) in biolab_deep::GENE_NAMES.iter().enumerate() {
        assert_eq!(i32::from(genome.strength_of(name)), 1 + (gene % 3) as i32, "{name}");
    }
}
