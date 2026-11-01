// /biolab — состояние мода; /biolab infect N — уровень заражения для проверок.
// Журнал показывает панель анализатора: запрос клиент → сервер в одиночной
// игре не доходит, а канал сервер → клиент работает.
fn biolab_core_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) || m::command_name(command) != "biolab" {
        return;
    }
    *ran = true;
    let mut first = command.split(' ').skip(1);
    if first.next() == Some("infect") {
        let level: u32 = first.next().and_then(|w| w.parse().ok()).unwrap_or(50);
        let world = game.world_root.clone();
        let uuid = game.native_acting_id();
        m::set_infection(&world, uuid, level);
        game.native_chat(&format!("Заражение: {}%", level.min(100)));
        return;
    }
    let mut words = command.split(' ').skip(1);
    match words.next() {
        Some("blood") => {
            // Что сейчас в крови: то же, что показывает анализатор.
            let world = game.world_root.clone();
            let uuid = game.native_acting_id();
            let Some(state) = m::player_tick(&world, uuid) else {
                return;
            };
            let lines = m::analyzer_lines_with(
                state.infection,
                m::journal_count(&world),
                &state.drugs,
                &m::strain_or_wild(&state),
            );
            for line in lines {
                game.native_chat(&line);
            }
            return;
        }
        Some("cure") => {
            // Вылечить начисто: болезнь, кровь и ломку.
            let world = game.world_root.clone();
            let uuid = game.native_acting_id();
            m::cure_world_player(&world, uuid);
            game.native_chat("Вылечено: кровь чистая, болезни нет");
            return;
        }
        _ => {}
    }

    for line in m::status_lines() {
        game.native_chat(&line);
    }
}
