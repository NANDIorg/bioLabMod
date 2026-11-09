// ПКМ по блоку энергосети (M10, GDD 4.27 А пп. 8, 9): батареей в руке —
// зарядить её от генератора или накопителя либо отдать заряд машине; пустой
// рукой по кабелю — сведения о сети, по накопителю — его заряд. Пустая рука
// до `native_use_item_on` не доходит, поэтому хук — `native_right_click_block`
// (API 21, сервер), своей инъекцией рядом с `harvest.rs`. Всё делает
// `biolab_power::click`; клик наш — блок своё действие не выполняет.
fn biolab_core_power_click(game: &mut crate::game::Game<'_>, pos: [i32; 3], canceled: &mut bool) {
    use crate::game::biolab_core as m;
    if *canceled || !game.native_on(m::ID) {
        return;
    }
    if crate::game::biolab_power::click(game, pos) {
        *canceled = true;
    }
}
