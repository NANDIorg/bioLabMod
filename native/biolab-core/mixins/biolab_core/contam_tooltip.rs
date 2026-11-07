// Подсказка о загрязнении и фильтре (M9, GDD 4.23 п. 13, 4.25). Загрязнение
// подсказка **обязана** показывать, у любого стека, не только у предметов
// мода: красные строки сразу под именем («Загрязнено: 40 %», штамм, чем
// чистить). У респиратора — остаток фильтра. Вторая инъекция на
// `native_tooltip` (API 12) после `tooltip.rs`.
fn biolab_core_contam_tooltip(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    lines: &mut Vec<(horizon_client::text::Text, u32)>,
) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_suit as suit;
    if !game.native_on(m::ID) {
        return;
    }
    if &*stack.item == m::RESPIRATOR_ID {
        let left = m::respirator_filter(suit::custom_text(stack, m::SUIT_KEY).as_deref());
        lines.push((horizon_client::text::Text::literal(&m::respirator_line(left)), 0xAA_AAAA));
    }
    if let Some(c) = suit::contamination(stack) {
        let at = lines.len().min(1);
        for (i, line) in m::contamination_lines(&c).iter().enumerate() {
            lines.insert(at + i, (horizon_client::text::Text::literal(line), m::CONTAM_INK));
        }
    }
}
