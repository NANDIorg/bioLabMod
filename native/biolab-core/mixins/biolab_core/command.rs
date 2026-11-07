// /biolab — состояние мода. Команды проверок: infect N, blood, cure,
// journal, name <имя> для вещества в руке и age <дней> — состарить часы гена
// у мутантов рядом (предохранители, GDD 4.23 п. 10).
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
        crate::game::biolab_say::say(game, &format!("Заражение: {}%", level.min(100)));
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
                crate::game::biolab_say::say(game, &line);
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
            crate::game::biolab_say::say(game, &if placed {
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
                crate::game::biolab_say::say(game, &line);
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
                crate::game::biolab_say::say(game, "Возьмите в руку колбу с веществом");
                return;
            };
            let world = game.world_root.clone();
            if m::rename_compound(&world, &contents, &name) {
                crate::game::biolab_say::say(game, &format!("{contents} теперь «{name}»"));
            } else {
                crate::game::biolab_say::say(game, "Сначала опознайте вещество в спектрометре");
            }
            return;
        }
        Some("age") => {
            // Состарить предохранители (GDD 4.23 п. 10): часы гена у мутантов
            // в 8 блоках сдвигаются на N игровых дней назад, и следующий тик
            // мира выносит приговор. Иначе проверку пришлось бы ждать сутками
            // игрового времени: `game_time` командой `/time` не двигается.
            let days: i64 = words.next().and_then(|w| w.parse().ok()).unwrap_or(1);
            let shift = days * m::GAME_DAY;
            let Some(player) = game.native_player() else {
                return;
            };
            let spot = [player.x, player.y, player.z];
            let near: Vec<u32> = game
                .mobs
                .iter()
                .filter(|mob| mob.alive() && m::within([mob.position.x, mob.position.y, mob.position.z], spot, 8.0))
                .map(|mob| mob.id)
                .collect();
            let mut aged = 0;
            for id in near {
                let mut clock = crate::game::biolab_core_data::clock(game, id);
                if clock.set == 0 {
                    continue;
                }
                clock.set -= shift;
                if clock.dose > 0 {
                    clock.dose -= shift;
                }
                if crate::game::biolab_core_data::set_clock(game, id, &clock) {
                    aged += 1;
                }
            }
            crate::game::biolab_say::say(game, &format!("Часы гена сдвинуты на {days} дн.: мутантов {aged}"));
            return;
        }
        Some("cure") => {
            // Вылечить начисто: болезнь, кровь и ломку.
            let world = game.world_root.clone();
            let uuid = game.native_acting_id();
            m::cure_world_player(&world, uuid);
            crate::game::biolab_say::say(game, "Вылечено: кровь чистая, болезни нет");
            return;
        }
        _ => {}
    }

    for line in m::status_lines() {
        crate::game::biolab_say::say(game, &line);
    }
}
