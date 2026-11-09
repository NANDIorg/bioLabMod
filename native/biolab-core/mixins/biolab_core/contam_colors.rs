// Цвета биомов в заражённой зоне (M10, GDD 4.27 п. 1): цвет задаётся на
// биом целиком, поэтому оттенок — по зоне вокруг игрока, а не по чанку
// («мир блекнет, когда входишь в очаг»). Ступень оттенка ставит
// `contam_view.rs`. Хук клиента API 107.
fn biolab_core_contam_colors(game: &crate::game::Game<'_>, colors: &mut horizon_client::biome::BiomeColors) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) {
        return;
    }
    let stage = m::applied_tint();
    if stage == 0 {
        return;
    }
    let [grass, foliage, dry, water] =
        m::tint_colors(stage, [colors.grass, colors.foliage, colors.dry_foliage, colors.water]);
    colors.grass = grass;
    colors.foliage = foliage;
    colors.dry_foliage = dry;
    colors.water = water;
}
