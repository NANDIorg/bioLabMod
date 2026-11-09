// Чанк выгружен (API 57, сервер): из зеркала долой. Данные чанка к этому
// мигу уже записаны тиком заражения (выгруженного чанка в мире нет).
fn biolab_core_contam_chunk_unload(game: &mut crate::game::Game<'_>, dimension: &str, chunk: [i32; 2]) {
    crate::game::biolab_world_contam::chunk_unload(game, dimension, chunk);
}
