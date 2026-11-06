// Моб вот-вот превратится (`native_living_conversion_pre`, API 391, NeoForge
// `LivingConversionEvent.Pre`). Запоминаем его геном и срезы, чтобы отдать
// их новому мобу в `joined.rs` (GDD 4.26, В). Нужно ради молнии: свинья в
// зомбифицированного пиглина и житель в ведьму в пине идут без `post`
// (`lightning.rs`: новый моб ставится событием, мимо `convert_to`), и
// `converted.rs` для них не срабатывает. Проверено в игре 06.11.2026.
// Отмену превращения другим модом мы не видим (HEAD): запись тогда просто
// устареет через `CONVERSION_TICKS`.
fn biolab_deep_conversion_pre(game: &mut crate::game::Game<'_>, entity: u32, outcome: &str) {
    use crate::game::biolab_deep as d;
    if !game.native_on(d::ID) {
        return;
    }
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    crate::game::biolab_heredity::expect_conversion(game, entity, outcome);
}
