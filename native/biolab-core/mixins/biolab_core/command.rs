// /biolab — состояние мода; /biolab infect N — уровень заражения для проверок.
// Журнал показывает панель анализатора: запрос клиент → сервер в одиночной
// игре не доходит, а канал сервер → клиент работает.
fn biolab_core_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) || m::command_name(command) != "biolab" {
        return;
    }
    *ran = true;
    let mut words = command.split(' ').skip(1);
    if words.next() == Some("infect") {
        let level: u32 = words.next().and_then(|w| w.parse().ok()).unwrap_or(50);
        let world = game.world_root.clone();
        let uuid = game.native_acting_id();
        m::set_infection(&world, uuid, level);
        game.native_chat(&format!("Заражение: {}%", level.min(100)));
        return;
    }
    for line in m::status_lines() {
        game.native_chat(&line);
    }
}
