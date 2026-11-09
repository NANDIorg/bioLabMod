// Чанк загружен (API 57, сервер): заражённость и журнал порчи из данных
// чанка — в зеркало (`chunk_contamination` отвечает из него).
fn biolab_core_contam_chunk_load(game: &mut crate::game::Game<'_>, dimension: &str, chunk: [i32; 2]) {
    if !game.native_on(crate::game::biolab_core::ID) {
        return;
    }
    crate::game::biolab_world_contam::chunk_load(game, dimension, chunk);
}
