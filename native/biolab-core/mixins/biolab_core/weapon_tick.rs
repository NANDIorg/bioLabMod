// Тик зажатого ПКМ с арбалетом-транквилизатором (GDD 4.20): натяжение своё,
// потому что ванильное в пине привязано к id `minecraft:crossbow`
// (`game/crossbow.rs`). Звуки на 20 и 50 %, на 1,25 с дротик из инвентаря
// уходит в арбалет (`weapons.rs`, `charge_tick`). Зовётся на обеих сторонах;
// мир меняет только тот `Game`, что его ведёт: клиент сервера получит
// заряженный стек от сервера. Стабильный API 131.
fn biolab_core_weapon_tick(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    entity: Option<i32>,
    remaining: i32,
) {
    use crate::game::biolab_core as m;
    let Some(weapon) = m::dart_weapon(&stack.item) else {
        return;
    };
    if weapon.loading != m::Loading::Charge || entity.is_some() || !game.native_on(m::ID) || game.native_window() {
        return;
    }
    crate::game::biolab_weapons::charge_tick(game, weapon, stack, remaining);
}
