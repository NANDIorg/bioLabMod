// Геном моба: узнаём его по меткам — модификаторам с именем гена, которые
// вешает бинарник (`joined.rs`), укол вектором или клонирующий чан. Здесь же
// геному выдаются прибавки к атрибутам: так ген, вживлённый уже живому мобу,
// начинает работать сразу, без перезахода в мир.
fn biolab_deep_mutate(m: &mut crate::mobs::Mob) {
    use crate::biolab_mutants as mut_;
    use crate::mobs::finalize::{Attr, Op, add_modifier, has_modifier};
    // Операционная: метки видит только сервер, отрисовка берёт их из общей
    // памяти по id (см. `biolab_surgery`). Пишем до выхода «генома нет»:
    // на стол кладут и обычных мобов.
    // Пишет только серверная копия моба: у неё есть отметка броска мутации
    // (её ставит каждому мобу `joined.rs`). Клиентской копии метки не
    // приходят, и, если её тик тоже доходит сюда, она стёрла бы состояние.
    if crate::mobs::finalize::has_modifier(
        m,
        crate::mobs::finalize::Attr::FollowRange,
        "biolab:mutation_checked",
    ) {
        crate::biolab_surgery::remember(m.id, crate::biolab_surgery::read_markers(m));
    }
    let mut genome = mut_::Genome::default();
    for marker in mut_::GENE_MARKERS {
        if has_modifier(m, Attr::FollowRange, marker)
            && let Some(gene) = mut_::decode_marker(marker)
        {
            genome.add(gene);
        }
    }
    if genome.list().next().is_none() {
        // Меток нет: обычный моб. Загруженному их могут вернуть позже, но
        // проверка дешёвая, а кандидатов мало.
        return;
    }
    if mut_::genome(m.id) == Some(genome) {
        return;
    }
    mut_::remember(m.id, genome);

    // Прибавки ставим заново: модификатор с тем же именем заменяется, так что
    // усиленный ген не складывается сам с собой.
    // Мутант бывает любого вида: здоровье считаем от базового здоровья вида
    // (без модификаторов), чтобы курица не стала крепче зомби.
    let base_health = crate::mobs::finalize::base_value(m, Attr::MaxHealth);
    let stats = mut_::scaled_stats(&genome, base_health);
    let rows = [
        (Attr::MaxHealth, mut_::STAT_MODIFIERS[0], stats[0], Op::AddValue),
        (
            Attr::MovementSpeed,
            mut_::STAT_MODIFIERS[1],
            stats[1],
            Op::AddMultipliedBase,
        ),
        (Attr::AttackDamage, mut_::STAT_MODIFIERS[2], stats[2], Op::AddValue),
        (Attr::Armor, mut_::STAT_MODIFIERS[3], stats[3], Op::AddValue),
        (
            Attr::KnockbackResistance,
            mut_::STAT_MODIFIERS[4],
            stats[4],
            Op::AddValue,
        ),
    ];
    let before = m.attrs.max_health;
    for (attr, name, amount, op) in rows {
        if amount != 0.0 {
            add_modifier(m, attr, name, amount, op);
        }
    }
    // Выросший максимум здоровья заполняем: мутант не должен появляться
    // израненным только потому, что ему прибавили здоровья.
    let grown = m.attrs.max_health - before;
    if grown > 0.0 {
        m.health = (m.health + grown).min(m.attrs.max_health);
    }
}
