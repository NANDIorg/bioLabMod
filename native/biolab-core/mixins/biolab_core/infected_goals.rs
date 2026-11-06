// Заражённый (GDD 4.26 А): цели зомби остаются (шаблон вида — зомби, его
// цели уже добавлены), сверху — охота на скот. Заражённый идёт к стаду, а не
// только к игроку: цель «ближайший из видов» с приоритетом 4 — после игрока
// (2) и жителей с големом (3), до черепашат (5). Атакует её ванильная цель
// зомби (`MeleeAttack`), удар по скоту разбирает `infected_tick.rs`.
//
// Хук — свободная функция lib-крейта `mobs::ai::native_register_goals`:
// зовётся для каждого нового моба, после целей его вида и до чтения сейва,
// у клиента тоже (для копий), а цели тикают только там, где идёт ИИ.
// `crate::biolab_core` здесь — копия `runtime.rs` в lib-крейте (модуль
// `runtime-menu` манифеста). Виды добычи — те же, что `INFECTED_PREY`.
fn biolab_core_infected_goals(mob: &mut crate::mobs::Mob) {
    use crate::mobs::Species;
    use crate::mobs::goals::NearestTarget;
    if !crate::biolab_core::is_infected(&mob.kind.id) {
        return;
    }
    mob.ai.targets.add(
        4,
        NearestTarget::species(&[Species::Cow, Species::Pig, Species::Sheep, Species::Chicken], true),
    );
}
