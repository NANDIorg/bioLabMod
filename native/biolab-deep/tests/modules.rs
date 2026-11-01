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
    assert_eq!(biolab_deep::MAX_GENES, mutants::MAX_GENES);
    assert_eq!(biolab_deep::STABILITY, mutants::STABILITY);
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
