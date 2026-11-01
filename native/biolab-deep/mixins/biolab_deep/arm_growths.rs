// Наросты на руке в кадре (GDD 4.19, этап «игрок»): когти, лоскут перепонки,
// хитиновый наплечник, шипы и костяные наросты — на той руке, которую рисует
// игра (пустая рука и руки под картой, `first_person_arm`). HEAD, а не RETURN:
// в теле функции есть замыкание позы, а HEAD с ним допустим; порядок граней
// для руки неважен — она рисуется с буфером глубины.
// Рука та же, что берёт игра (`res.player_arm`, тонкая для тонкого скина), и
// поза у неё та же — `m` и шарнир плеча из модели; гены — свои, из пакета
// сервера (`biolab_players::own`). Невидимый игрок до сюда не доходит: игра
// тогда руку не рисует и функцию не зовёт.
// Внутренние поля и методы `Game` (`res`, `player_model`, `gui`) — не
// хук-поинты: при обновлении Horizon проверить. Атрибутов у помощника быть
// не может (так требует инструмент миксинов).
fn biolab_deep_arm_growths(
    game: &crate::game::Game<'_>,
    left: bool,
    m: glam::Mat4,
    light: u32,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) {
    let model = game.player_model.as_ref();
    let slim = model.is_some_and(|p| p.slim);
    let Some(arm) = game.res.player_arm(slim, left).as_ref() else {
        return;
    };
    let age = model.map_or(0.0, |p| p.age as f32);
    let uv = game
        .gui
        .atlas
        .sprites
        .get(horizon_client::biolab_players::SPRITE)
        .map(|r| [r.u0, r.v0, r.u1, r.v1]);
    horizon_client::biolab_players::draw_arm(arm, left, m, light, age, uv, out);
}
