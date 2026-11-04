// Начало использования оружия (GDD 4.20): заряженный арбалет не натягивают —
// он стреляет кликом (`remedy.rs`), пустое ружьё не целится — оно
// заряжается кликом, а без дротиков нечего ни натягивать, ни заряжать.
// Сервер до сюда доходит, только если клик не забран; клиент же начинает
// использование сам, своим предсказанием (`remote_use_item` пина), не
// спрашивая `native_use_item`. Поэтому отказ — здесь, одним решением на
// обеих сторонах (`weapon_use_may_start`): иначе клиент рисовал бы
// натяжение, пока сервер уже выстрелил. Стабильный API 90.
fn biolab_core_weapon_start(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    entity: Option<i32>,
    canceled: &mut bool,
) {
    use crate::game::biolab_core as m;
    let Some(weapon) = m::dart_weapon(&stack.item) else {
        return;
    };
    // Моб (`entity`) оружием мода не пользуется; чужой мод уже отказал.
    if entity.is_some() || *canceled || !game.native_on(m::ID) || weapon.loading == m::Loading::Each {
        return;
    }
    if !crate::game::biolab_weapons::may_start(game, weapon, stack) {
        *canceled = true;
    }
}
