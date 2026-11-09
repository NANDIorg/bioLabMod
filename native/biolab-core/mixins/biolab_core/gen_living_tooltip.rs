// Подсказки живой энергетики (M10, GDD 4.27 В): у культуры — что она даст
// в микробном топливном элементе и сколько ей там жить («Отдача в МТЭ: N
// Вт·с/тик»), у колбы с водорослями и фермента — остаток ресурса, у ткани
// мутанта — её гены (срез их записывает) и сколько она горит в батарее.
// Строки — раздел «M10: живые генераторы» в `runtime.rs`. Ещё одна инъекция
// на `native_tooltip` (API 12), после `tooltip.rs`: строки дописываются в
// конец, под штаммом и подсказкой предмета.
fn biolab_core_living_tooltip(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    lines: &mut Vec<(horizon_client::text::Text, u32)>,
) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_gen_living::text_of;
    if !game.native_on(m::ID) {
        return;
    }
    let ours: Vec<String> = match &*stack.item {
        m::CULTURE_ID => {
            let strain = text_of(stack, m::STRAIN_KEY).and_then(|t| m::strain_from_text(&t)).unwrap_or(m::WILD);
            m::culture_mfc_lines(&strain, text_of(stack, m::LIFE_KEY).as_deref())
        }
        m::ALGAE_FLASK | m::ENZYME_ID => m::living_item_lines(&stack.item, text_of(stack, m::LIFE_KEY).as_deref()),
        m::TISSUE_ID => m::tissue_lines(text_of(stack, m::TISSUE_GENOME_KEY).as_deref()),
        _ => return,
    };
    for line in ours {
        lines.push((horizon_client::text::Text::literal(&line), 0x7F_C8E0));
    }
}
