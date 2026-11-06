// Стабильный хук разведения (`native_baby_entity_spawn`, API 395, NeoForge
// `BabyEntitySpawnEvent`): запомнить геномы родителей (GDD 4.26, В). Не
// отменяем — детёныш нужен. Игра зовёт хук только при загруженном
// трейт-моде, поэтому то же делает и HEAD на `breed` (`breed.rs`); два
// вызова одного рождения дают одну запись (`Births::expect`).
fn biolab_deep_baby_spawn(game: &mut crate::game::Game<'_>, parent: u32, partner: u32) {
    use crate::game::biolab_deep as d;
    if !game.native_on(d::ID) {
        return;
    }
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    crate::game::biolab_heredity::expect_birth(game, parent, partner);
}
