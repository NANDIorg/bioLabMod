// Поведение своих эффектов (GDD 4.7): модуль `effects.rs` в игре живёт
// отдельно от `biolab_core` (`crate::game::biolab_effects`), а здесь
// подключается как обычный модуль — его внутренние тесты идут тут же. Ниже —
// то, что он сам проверить не может: что переезд из `biolab-deep` на хуки
// API 56 не поменял ни чисел урона, ни ритма у игрока, и что имена эффектов
// совпадают с остальным `biolab_core`.

// Общее состояние (`state`) и тип урона читают только хелперы в игре.
#[allow(dead_code)]
#[path = "../mixins/biolab_core/effects.rs"]
mod effects;

use effects::{Act, Effects, Who};

/// Урон своих эффектов игроку до переезда — дословная копия
/// `biolab_deep::effect_damage` (миксин `tick.rs` на `tick_mob_effects`,
/// часы — `game.ticks`). Заморожена здесь как эталон: новое должно бить
/// ровно так же.
fn deep_effect_damage(id: &str, clock: i64, amplifier: i32) -> Option<f32> {
    let (period, base, per_level, max_shift) = match id {
        "biolab:acid_burn" => (20, 1.0, 1.0, 4),
        "biolab:toxicosis" => (40, 1.0, 0.5, 0),
        "biolab:bleeding" => (30, 1.0, 0.5, 2),
        _ => return None,
    };
    let period = (period >> amplifier.clamp(0, max_shift)).max(10);
    (clock >= 0 && clock % i64::from(period) == 0)
        .then(|| base + per_level * amplifier.clamp(0, 10) as f32)
}

/// Тик игрока с эффектом `id` по новому пути: платформа спрашивает
/// `should_apply`, при «да» зовёт `apply` (`calls` раз — тик эффектов
/// игрока зовётся чаще раза в тик), потом тик игрока разбирает очередь.
/// Ответ — урон, который получил игрок в этот тик.
fn player_tick(fx: &mut Effects, clock: u64, id: &str, amplifier: i32, calls: usize) -> Vec<f32> {
    let me = Some(0xB10_u128);
    for _ in 0..calls {
        if effects::acts(id, clock, amplifier) {
            assert!(fx.apply(clock, Who::Player(me), id, amplifier), "бьющий эффект не снимается");
        }
    }
    let due = fx.take(clock, me, "minecraft:overworld");
    assert!(due.mobs.is_empty());
    // Сила — прежняя, тип — из таблицы урона (GDD 4.26, Б).
    due.hurts
        .into_iter()
        .map(|(amount, damage)| {
            assert_eq!(damage, effects::damage_of(id), "{id}");
            amount
        })
        .collect()
}

/// Главное: числа и ритм у игрока те же, что до переезда, — по каждому
/// эффекту, уровню и тику десяти минут игры.
#[test]
fn player_damage_is_unchanged_by_the_move() {
    for effect in &effects::HURTING {
        for amp in -1..12 {
            let mut fx = Effects::default();
            for clock in 0..12_000u64 {
                let got = player_tick(&mut fx, clock, effect.id, amp, 1);
                let was = deep_effect_damage(effect.id, clock as i64, amp);
                assert_eq!(got, was.into_iter().collect::<Vec<_>>(), "{}: уровень {amp}, тик {clock}", effect.id);
            }
            assert_eq!(fx.pending(), 0);
        }
    }
}

/// Двойной вызов тика эффектов за тик (GDD 3.7) не удваивает удары.
#[test]
fn a_doubled_effect_tick_hits_once() {
    let mut fx = Effects::default();
    let hits: usize = (0..1200)
        .map(|clock| player_tick(&mut fx, clock, effects::TOXICOSIS, 0, 5).len())
        .sum();
    assert_eq!(hits, 1200 / 40);
}

/// Эталонные числа (GDD 4.7, M5): период в тиках и урон за удар по уровням
/// 0–3. Если их надо поменять — это балансная правка, а не переезд.
#[test]
fn the_numbers_of_each_effect() {
    let table: [(&str, [i32; 4], [f32; 4]); 3] = [
        ("biolab:acid_burn", [20, 10, 10, 10], [1.0, 2.0, 3.0, 4.0]),
        ("biolab:toxicosis", [40, 40, 40, 40], [1.0, 1.5, 2.0, 2.5]),
        ("biolab:bleeding", [30, 15, 10, 10], [1.0, 1.5, 2.0, 2.5]),
    ];
    for (id, periods, amounts) in table {
        let effect = effects::hurting(id).expect(id);
        for amp in 0..4 {
            assert_eq!(effects::effect_period(effect, amp), periods[amp as usize], "{id} {amp}");
            assert_eq!(effects::effect_amount(effect, amp), amounts[amp as usize], "{id} {amp}");
        }
    }
    // Перебор не разгоняет токсикоз: смерть от передозировки не мгновенная.
    assert_eq!(effects::effect_period(effects::hurting("biolab:toxicosis").unwrap(), 9), 40);
    // Часы стартуют с удара.
    assert!(effects::acts("biolab:acid_burn", 0, 0));
}

/// Моб бьётся той же таблицей и тем же ритмом, что игрок: одна таблица,
/// одни часы. До переезда у моба были свои часы (`m.age`) — теперь
/// общие, поэтому удары синхронны, а их частота и сила прежние.
#[test]
fn mobs_and_players_share_one_rhythm() {
    for effect in &effects::HURTING {
        for amp in 0..4 {
            let mut fx = Effects::default();
            let mut mob_hits = Vec::new();
            let mut player_hits = Vec::new();
            for clock in 0..2400u64 {
                if effects::acts(effect.id, clock, amp) {
                    fx.apply(clock, Who::Mob { dimension: "minecraft:overworld".into(), id: 11 }, effect.id, amp);
                    fx.apply(clock, Who::Player(None), effect.id, amp);
                }
                let due = fx.take(clock, None, "minecraft:overworld");
                for (id, act) in due.mobs {
                    assert_eq!(id, 11);
                    let Act::Hurt(amount, damage) = act else { panic!("{act:?}") };
                    mob_hits.push((clock, amount, damage.id));
                }
                player_hits.extend(due.hurts.into_iter().map(|(a, damage)| (clock, a, damage.id)));
            }
            assert_eq!(mob_hits, player_hits, "{} {amp}", effect.id);
            assert_eq!(mob_hits.len() as i32, 2400 / effects::effect_period(effect, amp));
        }
    }
}

/// Имена эффектов — те же, что выдают шприцы, лужи и дротики `biolab_core`.
#[test]
fn effect_names_match_the_core() {
    assert_eq!(effects::TOXICOSIS, biolab_core::TOXICOSIS_EFFECT);
    assert_eq!(effects::SEDATION, biolab_core::SEDATION_EFFECT);
    assert_eq!(effects::BLEEDING, biolab_core::BLEEDING_EFFECT);
    assert_eq!(effects::BLEEDING, biolab_core::BLEEDING);
    assert_eq!(effects::MUTAGENESIS, biolab_core::MUTAGENESIS_EFFECT);
    assert_eq!(effects::PARALYSIS, biolab_core::PARALYSIS_EFFECT);
    // Кислота: эффект лужи кислоты.
    assert!(biolab_core::FLUIDS.iter().any(|f| f.effect == effects::ACID_BURN));
}

/// Мутаген выбирает ген по номеру в таблице `GENES`, а метку тик игрока
/// берёт из `GENE_MARKERS` по номеру и силе: порядок меток — три на ген.
#[test]
fn a_mutation_names_a_real_marker() {
    for (index, gene) in biolab_core::GENES.iter().enumerate() {
        for strength in 1..=3 {
            assert_eq!(
                biolab_core::GENE_MARKERS.get(index * 3 + strength - 1).copied(),
                biolab_core::marker_for(gene.name, strength as i32),
                "{}/{strength}",
                gene.name
            );
        }
    }
    let empty = vec![0u8; biolab_core::GENES.len()];
    let (index, strength) = effects::mutation(&empty, effects::mutation_seed(42, 1000)).unwrap();
    assert!(index < biolab_core::GENES.len() && (1..=3).contains(&strength));
}

// ---- Свои типы урона (GDD 4.26, Б) ----

fn content(path: &str) -> String {
    let full = format!("{}/../../content/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{full}: {e}"))
}

/// Строковое поле плоского JSON (`"key": "value"`), без своего парсера.
fn string_field(json: &str, key: &str) -> Option<String> {
    let rest = &json[json.find(&format!("\"{key}\""))? + key.len() + 2..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;
    Some(rest[..rest.find('"')?].to_owned())
}

/// У каждого нашего типа урона есть описание в данных, теги совпадают с
/// флагами, которыми бьётся моб (`NamedDamage`), а сообщения о смерти есть
/// на обоих языках. Иначе игрок не получил бы урона вовсе (игра не нашла бы
/// тип в реестре), а моб и игрок по-разному считали бы броню.
#[test]
fn our_damage_types_are_in_the_data_pack() {
    let tag = |name: &str| content(&format!("data/minecraft/tags/damage_type/{name}.json"));
    let (armor, knockback, panic) = (tag("bypasses_armor"), tag("no_knockback"), tag("panic_causes"));
    let langs = [content("assets/biolab/lang/en_us.json"), content("assets/biolab/lang/ru_ru.json")];
    for damage in effects::OUR_DAMAGE {
        let (namespace, name) = damage.id.split_once(':').unwrap();
        let json = content(&format!("data/{namespace}/damage_type/{name}.json"));
        let message = string_field(&json, "message_id").expect(damage.id);
        assert_eq!(message, format!("{namespace}.{name}"), "{}", damage.id);
        let scaling = string_field(&json, "scaling").expect(damage.id);
        assert!(
            ["never", "when_caused_by_living_non_player", "always"].contains(&scaling.as_str()),
            "{}: scaling {scaling}",
            damage.id
        );
        assert!(json.contains("\"exhaustion\""), "{}: exhaustion", damage.id);
        if let Some(effects) = string_field(&json, "effects") {
            assert!(
                ["hurt", "thorns", "drowning", "burning", "poking", "freezing"].contains(&effects.as_str()),
                "{}: effects {effects}",
                damage.id
            );
        }
        let quoted = format!("\"{}\"", damage.id);
        assert_eq!(armor.contains(&quoted), damage.bypasses_armor, "{}: bypasses_armor", damage.id);
        assert_eq!(knockback.contains(&quoted), damage.no_knockback, "{}: no_knockback", damage.id);
        assert_eq!(panic.contains(&quoted), damage.panic_causes, "{}: panic_causes", damage.id);
        for lang in &langs {
            for key in [format!("death.attack.{message}"), format!("death.attack.{message}.player")] {
                assert!(lang.contains(&format!("\"{key}\"")), "{key}");
            }
        }
    }
    // Теги дописывают ванильные, а не заменяют их.
    for tag in [&armor, &knockback, &panic] {
        assert!(!tag.contains("\"replace\": true"), "{tag}");
    }
    // Эффект болезни в таблице урона — тот же, что выдаёт заражение.
    assert_eq!(effects::INFECTION, biolab_core::INFECTION_EFFECT);
}
