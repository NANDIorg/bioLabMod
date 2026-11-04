// Тесты модулей, которые в игре живут в других крейтах (мутанты, наросты,
// вкладка — в lib horizon-client).
// Здесь они подключаются как обычные модули; функции, которым нужна сама игра,
// отгорожены `#[cfg(not(test))]`, а их внутренние тесты прогоняются тут.

// Таблица своих эффектов живёт в `biolab-core` (модуль игры
// `biolab_effects`): сверяем с ней то, что осталось в этом бандле.
#[allow(dead_code)]
#[path = "../../biolab-core/mixins/biolab_core/effects.rs"]
mod core_effects;
#[path = "../mixins/biolab_deep/mutants.rs"]
mod mutants;
#[path = "../mixins/biolab_deep/surgery.rs"]
mod surgery;
#[path = "../mixins/biolab_deep/growths.rs"]
mod growths;
#[path = "../mixins/biolab_deep/players.rs"]
mod players;

/// Данные на сущностях (M11): `biolab-core` и `biolab-deep` объявляют одни и
/// те же типы у себя (`biolab_core_data`, `biolab_deep_data`). Ключ — пара
/// «пространство имён, имя», и значение, записанное одним бандлом, второй
/// читает как своё. Поэтому ключи, поля и биты обязаны совпадать до байта.
#[test]
fn core_and_deep_name_the_same_data() {
    use biolab_core as c;
    use biolab_deep as d;
    assert_eq!(c::DATA_NAMESPACE, d::DATA_NAMESPACE);
    assert_eq!(c::GENOME_DATA, d::GENOME_DATA);
    assert_eq!(c::SURGERY_DATA, d::SURGERY_DATA);
    assert_eq!(c::CHECKED_DATA, d::CHECKED_DATA);
    assert_eq!(c::PLAYER_GENES_DATA, d::PLAYER_GENES_DATA);
    assert_eq!(c::GENES_FIELD, d::GENES_FIELD);
    assert_eq!(c::CHECKED_FIELD, d::CHECKED_FIELD);
    assert_eq!(
        [c::FLAYED_FIELD, c::TABLE_FIELD, c::ASLEEP_FIELD],
        [d::FLAYED_FIELD, d::TABLE_FIELD, d::ASLEEP_FIELD]
    );
    assert_eq!([c::FLAYED_BITS, c::TABLE_BIT, c::ASLEEP_BIT], [d::FLAYED_BITS, d::TABLE_BIT, d::ASLEEP_BIT]);
    // И отрисовка читает те же биты.
    assert_eq!(
        [surgery::FLAYED_BITS, surgery::TABLE_BIT, surgery::ASLEEP_BIT],
        [d::FLAYED_BITS, d::TABLE_BIT, d::ASLEEP_BIT]
    );
    // Маска срезов операционной core ложится в нижние биты как есть.
    assert_eq!(c::FLAYED_BITS, (1u8 << c::SURGERY_FLAYED.len()) - 1);
    assert_eq!(c::STAT_MODIFIERS, d::STAT_MODIFIERS);
    assert_eq!(d::STAT_MODIFIERS, mutants::STAT_MODIFIERS);
}

/// Форма данных одна: строку генома, записанную core (укол, чан, носилки,
/// мутаген), deep читает теми же генами той же силы, и наоборот (бросок
/// при рождении). Поля операционной — те же байты.
#[test]
fn core_and_deep_codecs_agree() {
    // Все гены таблицы, разные силы, вперемешку и с повтором — обе стороны
    // приводят строку к одному виду.
    let core_genes: Vec<(&'static str, i32)> = biolab_core::GENES
        .iter()
        .enumerate()
        .rev()
        .map(|(i, g)| (g.name, 1 + (i % 3) as i32))
        .chain([("horns", 1)])
        .collect();
    let from_core = biolab_core::genome_data(&core_genes);
    let deep_genes = biolab_deep::genome_from_text(&from_core);
    assert_eq!(deep_genes.len(), biolab_core::GENES.len());
    assert_eq!(biolab_deep::genome_text(&deep_genes), from_core, "deep пишет ту же строку");
    for (i, strength) in &deep_genes {
        let name = biolab_deep::GENE_NAMES[*i];
        let core_strength = biolab_core::genes_of_data(&from_core)
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, s)| *s);
        assert_eq!(core_strength, Some(*strength), "{name}");
    }
    // Бросок deep → строка → core видит тот же геном.
    for draw in 0..3000u32 {
        let rolled = biolab_deep::roll_for("minecraft:zombie", draw.wrapping_mul(2_654_435_761));
        let text = biolab_deep::genome_text(&rolled);
        let seen: Vec<(String, i32)> = biolab_core::genes_of_data(&text)
            .into_iter()
            .map(|(n, s)| (n.to_owned(), s))
            .collect();
        let mut expected: Vec<(String, i32)> = rolled
            .iter()
            .map(|(i, s)| (biolab_deep::GENE_NAMES[*i].to_owned(), *s))
            .collect();
        expected.sort_by_key(|(n, _)| biolab_deep::GENE_NAMES.iter().position(|x| *x == n.as_str()));
        assert_eq!(seen, expected, "бросок {draw}");
        assert_eq!(biolab_core::genome_data(&biolab_core::genes_of_data(&text)), text);
    }
    for bits in 0..=(biolab_deep::FLAYED_BITS | biolab_deep::TABLE_BIT | biolab_deep::ASLEEP_BIT) {
        assert_eq!(biolab_core::surgery_fields(bits), biolab_deep::surgery_fields(bits));
        let fields = biolab_core::surgery_fields(bits);
        let get = |name: &str| fields.iter().find(|(f, _)| *f == name).map(|(_, v)| *v);
        assert_eq!(biolab_deep::surgery_from_fields(get), bits);
        assert_eq!(biolab_core::surgery_from_fields(get), bits);
        // Отрисовка понимает то, что записал core (стол, срезы), и «спит» от deep.
        let s = surgery::Surgery::from_bits(bits);
        assert_eq!(s.on_table, bits & biolab_core::TABLE_BIT != 0);
        assert_eq!(s.asleep, bits & biolab_deep::ASLEEP_BIT != 0);
        for limb in 0..4 {
            assert_eq!(s.flayed[limb], bits & (1 << limb) != 0);
        }
    }
    // Запись стола или среза в core не сбрасывает «спит», поставленный deep.
    let asleep = biolab_deep::ASLEEP_BIT;
    assert_eq!(biolab_core::surgery_bits(true, 0b11, asleep) & asleep, asleep);
}

/// Миграция старого мира: метки, которые писали прошлые версии core (укол,
/// чан, стол, срез) и deep (бросок, отметка), deep переносит в данные, и core
/// читает из них тот же геном и то же состояние стола.
#[test]
fn old_markers_migrate_into_the_same_data() {
    let mut ids: Vec<&str> = Vec::new();
    for (name, strength) in [("chitin", 2), ("horns", 3), ("membranes", 1)] {
        ids.push(biolab_core::marker_for(name, strength).expect(name));
    }
    ids.push(biolab_core::SURGERY_TABLE);
    ids.push(biolab_core::SURGERY_FLAYED[1]);
    ids.push(biolab_core::SURGERY_FLAYED[3]);
    ids.push(biolab_core::SURGERY_SLOW);
    ids.push(biolab_deep::CHECKED_MARKER);
    ids.push(biolab_core::STAT_MODIFIERS[0]);
    let legacy = biolab_deep::read_legacy(ids.iter().copied()).expect("метки нашлись");
    assert_eq!(biolab_core::genes_of_data(&legacy.genome), vec![("chitin", 2), ("horns", 3), ("membranes", 1)]);
    assert_eq!(legacy.surgery, biolab_core::TABLE_BIT | 0b1010);
    assert!(legacy.checked);
    // Снимаются только метки; прибавки (скорость за срезы, здоровье генома)
    // остаются модификаторами.
    let kept: Vec<&str> = ids.iter().copied().filter(|id| !biolab_deep::is_legacy_marker(id)).collect();
    assert_eq!(kept, vec![biolab_core::SURGERY_SLOW, biolab_core::STAT_MODIFIERS[0]]);
    // Старые списки меток двух бандлов совпадают: миграция deep узнаёт всё,
    // что когда-то ставил core.
    assert_eq!(biolab_core::GENE_MARKERS, biolab_deep::GENE_MARKERS);
    assert_eq!(biolab_core::SURGERY_FLAYED, biolab_deep::FLAYED_MARKERS);
    assert_eq!(biolab_core::SURGERY_TABLE, biolab_deep::TABLE_MARKER);
    for marker in biolab_core::GENE_MARKERS {
        let (name, strength) = biolab_core::gene_of_marker(marker).unwrap();
        let legacy = biolab_deep::read_legacy([marker]).unwrap();
        assert_eq!(biolab_core::genes_of_data(&legacy.genome), vec![(name, strength)], "{marker}");
    }
}

/// Гены игрока — данные `biolab:player_genes`: строку собирает core из
/// состояния игрока, окно (`view_sync.rs`) разбирает её по именам и
/// раскладывает по моделям (`biolab_players`). Проверяем настоящим кодом обеих
/// сторон (GDD 4.19, этап «игрок»).
#[test]
fn player_genes_from_core_reach_the_player_models() {
    use biolab_core::{PlayerState, Splice, splice_player};
    let mut me = PlayerState::default();
    assert_eq!(splice_player(&mut me, "tentacles", 2), Splice::Done);
    assert_eq!(splice_player(&mut me, "polymelia", 3), Splice::Done);
    let mut other = PlayerState::default();
    assert_eq!(splice_player(&mut other, "horns", 1), Splice::Done);
    let mine = biolab_core::genome_data(&biolab_core::player_genes(&me));
    let theirs = biolab_core::genome_data(&biolab_core::player_genes(&other));
    let none = biolab_core::genome_data(&biolab_core::player_genes(&PlayerState::default()));

    // Так их раскладывает окно: свои — модели 0, чужие — по id сущности.
    let named = |text: &str| biolab_deep::named(&biolab_deep::genome_from_text(text));
    players::replace_all(named(&mine), vec![(100, named(&theirs)), (101, named(&theirs)), (102, named(&none))]);
    let own = players::own().expect("свои гены");
    assert_eq!(own.strength_of("tentacles"), 2);
    assert_eq!(own.strength_of("polymelia"), 3);
    assert_eq!(own.strength_of("horns"), 0);
    assert_eq!(players::of_model(players::OWN_MODEL).map(|g| g.strength_of("tentacles")), Some(2));
    assert_eq!(players::of_model(100).map(|g| g.strength_of("horns")), Some(1));
    assert_eq!(players::of_model(101), players::of_model(100));
    assert!(players::of_model(102).is_none(), "без генов модель рисуется как есть");

    // Все черты наростов — гены, которые core знает по имени: иначе игроку
    // они не придут никогда.
    for gene in growths::TRAITS {
        assert!(biolab_core::GENES.iter().any(|g| g.name == gene), "{gene} нет у core");
    }
    // Рука в кадре ищет в атласе ту же текстуру наростов, что и модель.
    assert_eq!(players::SPRITE, growths::SPRITE);

    // Генов не стало — данные пустые, наросты снимаются.
    players::replace_all(named(&none), Vec::new());
    assert!(players::own().is_none());
    assert!(players::of_model(players::OWN_MODEL).is_none());
    assert!(players::of_model(100).is_none());
}

/// Сила гена у игрока — как у мутанта: 0–3, нулевые гены не хранятся.
#[test]
fn player_genes_read_like_a_genome() {
    let genes = players::PlayerGenes::new([("tail".to_owned(), 5), ("horns".to_owned(), 0)]);
    assert_eq!(genes.strength_of("tail"), 3);
    assert_eq!(genes.strength_of("horns"), 0);
    assert!(!genes.is_empty());
    assert!(players::PlayerGenes::new([("horns".to_owned(), 0)]).is_empty());
}

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

/// Поведение своих эффектов переехало в `biolab-core` (хуки API 56). Здесь
/// остались модификатор скорости усыплённого (`mob_tick.rs`) и бит «спит»
/// (проход мутантов), а геном с M11 — данные; они должны понимать то, что
/// пишет core:
/// * id седации — тот же, что выдают шприц, дротик и стол;
/// * ген, который мутаген core (`effect_actions.rs`) выбрал по номеру в своей
///   таблице и вписал в строку данных, deep читает как тот же ген той же силы,
///   и геном отрисовки от этого меняется.
#[test]
fn core_effects_speak_the_deep_genome() {
    assert_eq!(mutants::SEDATION, biolab_core::SEDATION_EFFECT);
    assert_eq!(mutants::SEDATION, core_effects::SEDATION);
    // Паралич кураре (M5): моб стоит тем же модификатором, что усыплённый.
    assert_eq!(mutants::PARALYSIS, biolab_core::PARALYSIS_EFFECT);
    assert_eq!(mutants::PARALYSIS, core_effects::PARALYSIS);
    // Ген устойчивости к снотворному — тот, чью силу core прибавляет к массе.
    assert!(mutants::gene_index(biolab_core::RESISTANCE_GENE).is_some());
    assert_eq!(mutants::GENE_STATS[mutants::gene_index(biolab_core::RESISTANCE_GENE).unwrap()], [0.0; 5]);
    assert_eq!(biolab_core::GENES.len(), mutants::GENE_NAMES.len());
    for (i, gene) in biolab_core::GENES.iter().enumerate() {
        assert_eq!(gene.name, mutants::GENE_NAMES[i], "ген {i}");
        assert_eq!(gene.cost, mutants::gene_cost(i), "{}", gene.name);
    }
    assert_eq!(biolab_core::GENE_MARKERS, mutants::GENE_MARKERS);

    // Мутаген: строка данных → силы по порядку `GENES`, как их собирает
    // `effect_actions.rs` → выбор гена → строка данных → геном отрисовки.
    // Он обязан измениться.
    let start = biolab_core::genome_data(&[("regeneration", 2), ("acid_glands", 3)]);
    let to_lib = |text: &str| mutants::genome_of(&biolab_deep::strengths(&biolab_deep::genome_from_text(text)));
    let before = to_lib(&start);
    for seed in 0..2000u64 {
        let have = biolab_core::genes_of_data(&start);
        let mut strengths = vec![0u8; biolab_core::GENES.len()];
        for (name, strength) in &have {
            let i = biolab_core::GENES.iter().position(|g| g.name == *name).unwrap();
            strengths[i] = *strength as u8;
        }
        let (index, strength) = core_effects::mutation(&strengths, seed).expect("мутировать есть чему");
        let mut genes = have;
        genes.push((biolab_core::GENES[index].name, strength));
        let after = to_lib(&biolab_core::genome_data(&genes));
        assert_eq!(i32::from(after.strength(index)), strength.max(i32::from(before.strength(index))));
        assert_ne!(after, before, "мутация ничего не изменила: ген {index}, сила {strength}");
    }
}

/// Геном выдаёт и хранит бинарник (`runtime.rs`, данные), а что гены значат,
/// знает lib (`mutants.rs`): имена генов, их цена и старые метки (для
/// миграции) в обеих копиях должны совпадать, иначе мутант, записанный в
/// сейв, перестанет узнаваться.
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

/// Лабораторный геном больше природного: гены, которые вписали укол или чан,
/// доходят до отрисовки все до одного, сколько бы их ни было (GDD 4.19) —
/// и из данных, и из старых меток при миграции.
#[test]
fn a_lab_genome_beyond_the_natural_limit_is_read_whole() {
    let mut lab = Vec::new();
    let mut markers = Vec::new();
    for gene in 0..biolab_deep::GENE_NAMES.len() {
        let strength = 1 + (gene % 3) as i32;
        lab.push((biolab_deep::GENE_NAMES[gene], strength));
        markers.push(biolab_deep::marker(gene, strength).expect("метка есть"));
    }
    // Так геном доходит до окна: строка данных core → разбор deep → lib.
    let text = biolab_core::genome_data(&lab);
    let genome = mutants::genome_of(&biolab_deep::strengths(&biolab_deep::genome_from_text(&text)));
    // Старый мир: метки → миграция → та же строка.
    let migrated = biolab_deep::read_legacy(markers.iter().copied()).expect("метки есть");
    assert_eq!(migrated.genome, text);
    assert_eq!(genome.count(), biolab_deep::GENE_NAMES.len());
    assert!(biolab_deep::cost(&genome.list().map(|g| (g.index, g.strength)).collect::<Vec<_>>()) > biolab_deep::STABILITY);
    for (gene, name) in biolab_deep::GENE_NAMES.iter().enumerate() {
        assert_eq!(i32::from(genome.strength_of(name)), 1 + (gene % 3) as i32, "{name}");
    }
}
