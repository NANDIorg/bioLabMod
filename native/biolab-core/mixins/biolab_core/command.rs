// /biolab — состояние мода. Команды проверок: infect N, blood, cure,
// journal и name <имя> для вещества в руке.
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
        Some("book") => {
            // Новая книга: пригодится, если потеряли. Изученное хранится у
            // игрока, а не в книге, так что новая знает всё то же.
            let book = horizon_client::items::ItemStack::new(m::MANUAL_ID, 1);
            let mut placed = false;
            for slot in 0..36 {
                if let Some(place) = game.native_carried_slot(slot)
                    && place.is_none()
                {
                    *place = Some(book.clone());
                    placed = true;
                    break;
                }
            }
            game.native_chat(&if placed {
                "Выдан журнал лаборанта".to_string()
            } else {
                "Некуда положить книгу: освободите слот".to_string()
            });
            return;
        }
        Some("journal") => {
            // Весь журнал открытий: что собрано, как названо, что не опознано.
            let world = game.world_root.clone();
            for line in m::journal_lines(&world) {
                game.native_chat(&line);
            }
            return;
        }
        Some("name") => {
            // Своё имя веществу в колбе: `/biolab name <имя>`.
            let name = command
                .split_once(" name ")
                .map(|(_, rest)| rest.trim().to_string())
                .unwrap_or_default();
            let slot = game.selected as i32;
            let contents = game
                .native_carried_stack(slot)
                .filter(|stack| &*stack.item == m::FLASK_ID)
                .and_then(|stack| {
                    let extra = stack.extra.as_ref()?;
                    let (_, data) = extra
                        .other
                        .iter()
                        .find(|(key, _)| key == "minecraft:custom_data")?;
                    match data.get(m::CONTENTS_KEY) {
                        Some(horizon_world::nbt::Tag::String(s)) => Some(s.to_string_lossy()),
                        _ => None,
                    }
                });
            let Some(contents) = contents else {
                game.native_chat("Возьмите в руку колбу с веществом");
                return;
            };
            let world = game.world_root.clone();
            if m::rename_compound(&world, &contents, &name) {
                game.native_chat(&format!("{contents} теперь «{name}»"));
            } else {
                game.native_chat("Сначала опознайте вещество в спектрометре");
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
