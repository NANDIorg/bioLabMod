// Отпущен ПКМ с пневматическим ружьём (GDD 4.20): выстрел из прицела —
// первый дротик магазина, газ из баллона (`weapons.rs`, `rifle_release`).
// Арбалету отпускание ничего не делает: он заряжен натяжением, а стреляет
// следующим кликом. `used` не ставим: пауза ружья — своя (`add_cooldown`),
// компонента `use_cooldown` у предмета нет. Обе стороны; мир меняет только
// тот `Game`, что его ведёт. Стабильный API 131.
fn biolab_core_weapon_release(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    entity: Option<i32>,
) {
    use crate::game::biolab_core as m;
    let Some(weapon) = m::dart_weapon(&stack.item) else {
        return;
    };
    if weapon.loading != m::Loading::Magazine || entity.is_some() || !game.native_on(m::ID) || game.native_window() {
        return;
    }
    crate::game::biolab_weapons::rifle_release(game, weapon, stack);
}
