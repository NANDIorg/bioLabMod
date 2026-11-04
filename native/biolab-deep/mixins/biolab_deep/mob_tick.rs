// Что происходит с мобом каждый тик, перед его эффектами: усыплённый стоит,
// а геном мутанта лечит, гасит огонь, отбивает яды и даёт планировать.
//
// Урон и поведение самих эффектов (кислота, токсикоз, кровотечение, потеря
// цели под седацией, мутаген) сюда больше не входят: их считает
// `biolab-core` на стабильных хуках API 56 (`effect_*.rs`), одной таблицей
// для игрока и моба. Здесь — только то, на что стабильного API нет:
// прибавка к атрибуту от эффекта (скорость усыплённого) и поведение мутанта
// по генам, которое к эффектам не относится.
fn biolab_deep_mob_effects(m: &mut crate::mobs::Mob) {
    use crate::biolab_mutants as mut_;
    // Часы моба — его собственный возраст: регенерация мутанта идёт по нему.
    let clock = i64::from(m.age);
    // Уснувший не ходит: скорость гасится модификатором, пока действует
    // снотворное, и снимается, как только оно кончилось. В сейв он не
    // возвращается (его нет в `our_modifier`), так что навсегда не залипнет.
    // Цель снимает `biolab-core`; модификатор — прибавка к атрибуту, для неё
    // стабильного API нет, поэтому она здесь.
    {
        use crate::mobs::finalize::{Attr, Op, add_modifier, has_modifier, remove_modifier};
        const STILL: &str = "biolab:sedation_still";
        let sedated = m
            .extra
            .effects
            .iter()
            .any(|(id, ticks, _)| &**id == mut_::SEDATION && *ticks != 0);
        if sedated {
            if !has_modifier(m, Attr::MovementSpeed, STILL) {
                add_modifier(m, Attr::MovementSpeed, STILL, -1.0, Op::AddMultipliedTotal);
            }
        } else if has_modifier(m, Attr::MovementSpeed, STILL) {
            remove_modifier(m, Attr::MovementSpeed, STILL);
        }
    }

    let Some(genome) = mut_::genome(m.id) else {
        return;
    };
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
    // Регенерация: раз в секунду отрастает доля здоровья.
    let heal = mut_::regeneration(&genome);
    if heal > 0.0 && clock % 20 == 0 && m.alive() {
        m.health = (m.health + heal).min(m.attrs.max_health);
    }
}
