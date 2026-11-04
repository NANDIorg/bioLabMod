// Журнал лаборанта применён: открыть карту исследований. Хук предмета
// срабатывает там, где игра решает, что предмет применён «в воздух» (блок под
// прицелом уже отказался от щелчка), — как у ванильной книги, поэтому журнал
// не перебивает машины и сундуки. Обычно это сервер (одиночная игра — тоже:
// окно подключено к своему серверу), и экран окна ему не открыть: он шлёт
// игроку `NET_JOURNAL`, а открывает `research_net.rs`. Если же игра идёт
// прямо в окне (без отдельного сервера), экран открывается здесь же.
// `claimed` ставит и `remedy.rs`; здесь — на случай, если тот бандл решит
// иначе: другим обработчикам журнал трогать незачем.
fn biolab_core_research_open(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    claimed: &mut bool,
) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) || &*stack.item != m::MANUAL_ID {
        return;
    }
    *claimed = true;
    // Окно само: экран открылся — больше ничего не нужно. На сервере
    // `native_mod_screen_open` всегда false (нет окна) и ничего не меняет.
    let already = game.native_mod_screen() == Some(m::ID);
    if game.native_mod_screen_open(m::ID) {
        if !already {
            m::open_research();
        }
        return;
    }
    let uuid = game.native_acting_id();
    let _ = game.native_net_send_to_player(uuid, m::NET_JOURNAL, &[1]);
}
