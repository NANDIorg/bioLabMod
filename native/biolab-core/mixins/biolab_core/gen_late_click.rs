// ПКМ пустой рукой по солнечной панели или геотермальному съёмнику (M10,
// GDD 4.27 п. 11): у них нет слотов и экрана — состояние сообщением,
// «Солнечная панель: 12 Вт·с/тик · заряд 1 200 / 2 400 · небо открыто». Клик
// забираем, иначе шаблон печки полез бы открывать меню. Ещё одна инъекция на
// `native_right_click_block` (API 21, сервер; зовётся всегда, без условия на
// моды), рядом с `harvest.rs` и `protocol_card.rs`. Работа —
// `crate::game::biolab_gen_late::click`.
fn biolab_core_gen_late_click(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    canceled: &mut bool,
) {
    use crate::game::biolab_core as m;
    if *canceled || !game.native_on(m::ID) {
        return;
    }
    crate::game::biolab_gen_late::click(game, pos, canceled);
}
