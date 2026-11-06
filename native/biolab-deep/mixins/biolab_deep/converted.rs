// Моб превратился в другой вид (`native_living_conversion_post`, API 390,
// NeoForge `LivingConversionEvent.Post`): житель в зомби-жителя, свинья от
// молнии в зомбифицированного пиглина, зомби в утопленника, грибная корова в
// корову. Геном, отметка броска и срезы старого — новому (GDD 4.26, В):
// мутант остаётся мутантом. Новый к этому моменту уже прошёл
// `native_mobs_joined` (свой бросок в `joined.rs`) — правило
// `converted_genome` решает, чей геном остаётся.
fn biolab_deep_converted(game: &mut crate::game::Game<'_>, entity: u32, converted: u32) {
    use crate::game::biolab_deep as d;
    if !game.native_on(d::ID) {
        return;
    }
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    crate::game::biolab_heredity::carry_over(game, entity, converted);
}
