// Что происходит с мобом каждый тик: наши эффекты бьют, а геном мутанта
// лечит, гасит огонь и отбивает седативное.
fn biolab_deep_mob_effects(m: &mut crate::mobs::Mob) {
    use crate::biolab_effects as fx;
    use crate::biolab_mutants as mut_;
    // Часы моба — его собственный возраст: он тикает сам и не зависит от
    // того, как часто эффект подновляют.
    let clock = i64::from(m.age);
    let hits: Vec<f32> = m
        .extra
        .effects
        .iter()
        .filter(|(_, ticks, _)| *ticks != 0)
        .filter_map(|(id, _, amp)| fx::effect_damage(id, clock, *amp))
        .collect();
    for amount in hits {
        m.damage(
            amount,
            crate::mobs::Damage::of(crate::mobs::DamageKind::Magic),
        );
    }
    // Седативное: моб забывает, кого преследовал. Ванильного эффекта с таким
    // поведением нет, а дротик для отлова мутантов без него не работает.
    let sedated = m
        .extra
        .effects
        .iter()
        .any(|(id, ticks, _)| &**id == fx::SEDATION && *ticks != 0);
    if sedated {
        m.ai.target = None;
    }

    // Мутаген копит нагрузку, и на пороге организм срывается: добавляется
    // ген. Метку вешаем здесь, а прибавки к атрибутам выставит `spawn.rs` —
    // он замечает, что геном моба изменился.
    let under_mutagen = m
        .extra
        .effects
        .iter()
        .any(|(id, ticks, _)| &**id == mut_::MUTAGENESIS && *ticks != 0);
    if under_mutagen && mut_::add_load(m.id, 1) {
        let genome = mut_::genome(m.id).unwrap_or_default();
        let seed = u64::from(m.id).wrapping_mul(2_654_435_761) ^ u64::from(m.age);
        if let Some(gene) = mut_::mutation(&genome, seed)
            && let Some(marker) = mut_::marker_of(gene)
        {
            crate::mobs::finalize::add_modifier(
                m,
                crate::mobs::finalize::Attr::FollowRange,
                marker,
                0.0,
                crate::mobs::finalize::Op::AddValue,
            );
        }
        // Мутация потратила мутаген: эффект уходит.
        m.extra.effects.retain(|(id, _, _)| &**id != mut_::MUTAGENESIS);
    }

    let Some(genome) = mut_::genome(m.id) else {
        return;
    };
    // Термостойкость: такой мутант не горит — ни от огня, ни от солнца.
    if mut_::fireproof(&genome) {
        m.fire_ticks = 0;
    }
    // Стойкость к ядам: наши и ванильные отравы с такого мутанта сходят.
    if mut_::toxin_proof(&genome) {
        m.extra.effects.retain(|(id, _, _)| {
            !matches!(
                &**id,
                "minecraft:poison" | "poison" | "minecraft:wither" | "wither" | "biolab:toxicosis"
            )
        });
    }
    // Регенерация: раз в секунду отрастает доля здоровья.
    let heal = mut_::regeneration(&genome);
    if heal > 0.0 && clock % 20 == 0 && m.alive() {
        m.health = (m.health + heal).min(m.attrs.max_health);
    }
}
