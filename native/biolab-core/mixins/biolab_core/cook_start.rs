// Печь начала готовить (M9, GDD 4.23 п. 13): выход печи — новый стек, так
// что готовка и так снимает загрязнение. Термостойкий штамм (стойкость 4–5)
// оставляет половину: запоминаем, что останется от входа (`CookBook`), пока
// вход ещё лежит в печи. Хук `native_furnace_start_smelt` (API 230) только
// читает: остаток кладёт на выход тик костюма (`biolab_suit::tick`).
// Вход читаем из блок-сущности печи напрямую, как фон частиц (`ambient.rs`).
fn biolab_core_cook_start(game: &crate::game::Game<'_>, pos: [i32; 3]) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) {
        return;
    }
    let input = match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
        Some(horizon_client::chunk::block_entity::BlockEntity::Furnace(f)) => {
            f.items[0].as_ref().and_then(crate::game::biolab_suit::contamination)
        }
        _ => None,
    };
    if let Ok(mut book) = m::cook_book().lock() {
        book.start(game.dim.id(), pos, input);
    }
}
