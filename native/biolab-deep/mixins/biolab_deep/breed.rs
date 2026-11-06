// Два животных вот-вот родят детёныша (`Game::breed`, ванильный
// `Animal.spawnChildFromBreeding`): запомнить геномы родителей, чтобы
// детёныш получил их смесь вместо природного броска (GDD 4.26, В).
//
// Стабильный хук для этого — `native_baby_entity_spawn` (API 395,
// `baby_spawn.rs`), но игра зовёт его только при загруженном трейт-моде, а
// наши бандлы — миксины. Поэтому HEAD на саму внутреннюю функцию: она
// получает родителя и партнёра первыми аргументами. При обновлении Horizon
// проверить. Запись одного рождения двумя хуками сливается в одну.
fn biolab_deep_breed(game: &mut crate::game::Game<'_>, parent: u32, partner: u32) {
    use crate::game::biolab_deep as d;
    if !game.native_on(d::ID) {
        return;
    }
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    crate::game::biolab_heredity::expect_birth(game, parent, partner);
}
