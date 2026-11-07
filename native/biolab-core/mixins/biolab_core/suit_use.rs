// ПКМ для спецодежды и загрязнения (M9, GDD 4.25, 4.23 п. 13):
// * фильтрующий патрон в руке и респиратор на голове — новый фильтр
//   (`FILTER_FULL`), патрон тратится;
// * хлор (`DISINFECTANT`) — чистит предмет в левой руке и всё надетое, одна
//   штука хлора тратится. Чистить нечего — клик не забираем.
// Ещё один перехват на `native_use_item` (стабильный API 13), как
// `gas_use.rs`; патрон и хлор других действий по клику в воздух не имеют.
fn biolab_core_suit_use(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    claimed: &mut bool,
) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_suit as suit;

    /// Одна штука из руки, в которой этот предмет: основной или левой.
    fn spend_one(game: &mut crate::game::Game<'_>, item: &str) {
        let main = game.selected as i32;
        let slot = if game.native_carried_stack(main).is_some_and(|held| &*held.item == item) {
            main
        } else {
            -106
        };
        if let Some(held) = game.native_carried_stack(slot)
            && &*held.item == item
        {
            held.count = held.count.saturating_sub(1);
        }
    }

    if !game.native_on(m::ID) || *claimed {
        return;
    }
    let item: &str = &stack.item;
    if item == m::FILTER_CARTRIDGE {
        let left = match game.native_carried_stack(103) {
            Some(head) if &*head.item == m::RESPIRATOR_ID => {
                m::respirator_filter(suit::custom_text(head, m::SUIT_KEY).as_deref())
            }
            _ => {
                crate::game::biolab_say::say(game, m::FILTER_NO_RESPIRATOR);
                return;
            }
        };
        *claimed = true;
        if left >= m::FILTER_FULL {
            crate::game::biolab_say::say(game, m::FILTER_ALREADY_FULL);
            return;
        }
        if let Some(head) = game.native_carried_stack(103) {
            suit::set_custom_text(head, m::SUIT_KEY, Some(&m::suit_text(m::FILTER_FULL)));
        }
        spend_one(game, m::FILTER_CARTRIDGE);
        crate::game::biolab_say::say(game, m::FILTER_REFILLED);
    } else if item == m::DISINFECTANT {
        let mut cleaned = 0;
        for slot in [-106, 100, 101, 102, 103] {
            if let Some(held) = game.native_carried_stack(slot)
                && suit::contamination(held).is_some()
            {
                suit::set_contamination(held, None);
                cleaned += 1;
            }
        }
        if cleaned == 0 {
            return;
        }
        *claimed = true;
        spend_one(game, m::DISINFECTANT);
        crate::game::biolab_say::say(game, &m::disinfect_message(cleaned));
    }
}
