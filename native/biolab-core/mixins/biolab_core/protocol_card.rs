// Карта протокола по машине (GDD 4.23 п. 3): ПКМ пустой картой по
// ферментеру или экстрактору записывает найденное окно того сырья, что стоит
// во входе, а записанной — ставит ручки. Клик забираем себе (`canceled`),
// иначе поверх сразу открылось бы меню машины.
//
// Хук — `native_right_click_block` (API 21, на сервере), а не
// `native_use_item_on`: тот зовут уже после того, как блок пропустил клик, а
// машина его не пропускает — она открывает меню. Вторая инъекция на этот хук
// рядом с `harvest.rs`. Решения — раздел «M9: машины» в `runtime.rs`,
// работа — `crate::game::biolab_protocol`.
fn biolab_core_protocol_card(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    canceled: &mut bool,
) {
    use crate::game::biolab_core as m;
    if *canceled || !game.native_on(m::ID) {
        return;
    }
    // Карта в руке: основная, иначе левая. Клонируем — дальше игра нужна
    // изменяемой, а сам стек правит уже `use_card`.
    let main = game.selected as i32;
    let card = [main, -106].into_iter().find_map(|hand| {
        game.native_carried_stack(hand)
            .filter(|held| &*held.item == m::PROTOCOL_CARD)
            .map(|held| held.clone())
    });
    let Some(card) = card else {
        return;
    };
    if crate::game::biolab_protocol::use_card(game, pos, &card) {
        *canceled = true;
    }
}
