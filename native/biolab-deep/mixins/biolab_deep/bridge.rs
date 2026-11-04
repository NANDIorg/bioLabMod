// Мост бинарник ↔ lib для мутантов (M11).
//
// Данные мутанта (геном, операционная, гены игрока) живут на сущностях —
// attachments, а их читает только бинарник: `Host` есть у `Game`, а `Game`
// живёт в бинарнике. Что гены значат (прибавки, огонь, яды, планирование,
// регенерация) и как моб рисуется — в lib-модулях `biolab_mutants`,
// `biolab_surgery`, `biolab_players`, которые бинарнику не видны (модуль lib —
// `pub(crate)`). Этот модуль открыт наружу (`visibility = "pub"`) и только
// передаёт: в открытых функциях — открытые типы (`mobs::Mob`) и std.
//
// * `show` — окно: что пришло с сервера, целиком, раз в тик (`view_sync.rs`);
// * `apply_genome` — сервер: прибавки к атрибутам по геному. Прибавки
//   остаются модификаторами: стабильного API у них нет;
// * `mutant_tick` — сервер, каждый тик перед тиком мобов: поведение по генам.
//
// Этот файл подключается как модуль `crate::biolab_bridge` крейта
// horizon-client (lib).

/// Окно: геномы мобов, состояние операционной и гены игроков, пришедшие с
/// сервера. Каждый вызов заменяет прежнее целиком.
pub fn show(
    genomes: Vec<(u32, Vec<(usize, u8)>)>,
    surgeries: Vec<(u32, u8)>,
    own: Vec<(String, u8)>,
    others: Vec<(u32, Vec<(String, u8)>)>,
) {
    crate::biolab_mutants::replace_all(genomes);
    crate::biolab_surgery::replace_all(surgeries);
    crate::biolab_players::replace_all(own, others);
}

/// Спит ли моб под снотворным: по этому серверу ставит бит «спит» в данных
/// операционной, и окно кладёт уснувшего на землю.
pub fn sedated(m: &crate::mobs::Mob) -> bool {
    m.extra
        .effects
        .iter()
        .any(|(id, ticks, _)| &**id == crate::biolab_mutants::SEDATION && *ticks != 0)
}

/// Сервер: прибавки к атрибутам по геному `genes` (номер гена, сила).
/// Модификатор с тем же именем заменяется, так что повтор ничего не
/// складывает; прибавка, которой у генома больше нет, снимается. Здоровье
/// считается от базового здоровья вида, чтобы курица не стала крепче зомби.
/// `fill` — долить выросший максимум здоровья: мутант не должен появляться
/// израненным только потому, что ему прибавили здоровья. После загрузки мира
/// доливать нельзя — там здоровье вернётся из сейва.
pub fn apply_genome(m: &mut crate::mobs::Mob, genes: &[(usize, u8)], fill: bool) {
    use crate::biolab_mutants as mut_;
    use crate::mobs::finalize::{Attr, Op, add_modifier, has_modifier, modifier_amount, remove_modifier};
    let rows = [
        (Attr::MaxHealth, Op::AddValue),
        (Attr::MovementSpeed, Op::AddMultipliedBase),
        (Attr::AttackDamage, Op::AddValue),
        (Attr::Armor, Op::AddValue),
        (Attr::KnockbackResistance, Op::AddValue),
    ];
    // Обычный моб без наших прибавок: делать нечего, атрибуты не трогаем.
    if genes.is_empty()
        && !rows
            .iter()
            .zip(mut_::STAT_MODIFIERS)
            .any(|((attr, _), name)| has_modifier(m, *attr, name))
    {
        return;
    }
    let genome = mut_::genome_of(genes);
    let base_health = crate::mobs::finalize::base_value(m, Attr::MaxHealth);
    let stats = mut_::scaled_stats(&genome, base_health);
    let before = m.attrs.max_health;
    for (i, (attr, op)) in rows.into_iter().enumerate() {
        let name = mut_::STAT_MODIFIERS[i];
        let amount = stats[i];
        if amount != 0.0 {
            // Ставим, только если изменилось: модификатор пересчитывает атрибут.
            if modifier_amount(m, attr, name) != Some(amount) {
                add_modifier(m, attr, name, amount, op);
            }
        } else if has_modifier(m, attr, name) {
            remove_modifier(m, attr, name);
        }
    }
    let grown = m.attrs.max_health - before;
    if fill && grown > 0.0 {
        m.health = (m.health + grown).min(m.attrs.max_health);
    }
    m.health = m.health.min(m.attrs.max_health);
}

/// Сервер, каждый тик перед тиком мобов: что геном делает с мобом сам.
/// Раньше это жило в начале тика эффектов моба (`mob_tick.rs`), но там геном
/// виден только через общую память процесса; здесь его даёт бинарник из
/// данных.
pub fn mutant_tick(m: &mut crate::mobs::Mob, genes: &[(usize, u8)]) {
    use crate::biolab_mutants as mut_;
    if genes.is_empty() || !m.alive() {
        return;
    }
    let genome = mut_::genome_of(genes);
    // Термостойкость: такой мутант не горит — ни от огня, ни от солнца.
    if mut_::fireproof(&genome) {
        m.fire_ticks = 0;
    }
    // Стойкость к ядам: наши и ванильные отравы с такого мутанта сходят — до
    // того, как эффекты тикнут, так что токсикоз его не успевает ударить.
    if mut_::toxin_proof(&genome) {
        m.extra.effects.retain(|(id, _, _)| {
            !matches!(
                &**id,
                "minecraft:poison" | "poison" | "minecraft:wither" | "wither" | "biolab:toxicosis"
            )
        });
    }
    // Перепонки (GDD 4.19): мутант планирует. Ванильное медленное падение
    // Horizon мобам не применяет (его читает только физика игрока), поэтому
    // гасим скорость падения сами и сбрасываем высоту падения — с перепонками
    // моб не разбивается.
    let glide = mut_::flag_strength(&genome, "glide");
    if glide > 0 && !m.on_ground {
        m.velocity.y = mut_::glide_fall(m.velocity.y, glide);
        m.fall_distance = 0.0;
    }
    // Регенерация: раз в секунду отрастает доля здоровья. Часы — возраст
    // самого моба.
    let heal = mut_::regeneration(&genome);
    if heal > 0.0 && i64::from(m.age) % 20 == 0 {
        m.health = (m.health + heal).min(m.attrs.max_health);
    }
}
