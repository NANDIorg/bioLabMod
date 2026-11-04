// Поза руки с оружием в использовании, от первого лица (GDD 4.20): пустой
// арбалет натягивается — поза лука (`ItemPose::Bow`: оттянут и дрожит, когда
// натянут); заряженный арбалет и ружьё в прицеле держатся неподвижно
// (`Crossbow` у предмета мода пин рисует как `Using`). Данные — со стека:
// когда сервер зарядил арбалет, клиент получает новый стек, и поза
// сменяется сама. Клиент, стабильный API 90.
fn biolab_core_use_animation(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    animation: &mut horizon_world::consumable::UseAnimation,
) {
    use crate::game::biolab_core as m;
    use horizon_world::consumable::UseAnimation;
    let Some(weapon) = m::dart_weapon(&stack.item) else {
        return;
    };
    if !game.native_on(m::ID) {
        return;
    }
    let state = crate::game::biolab_weapons::state_of(stack);
    *animation = match weapon.loading {
        m::Loading::Charge if state.load.is_empty() => UseAnimation::Bow,
        m::Loading::Charge | m::Loading::Magazine => UseAnimation::Crossbow,
        m::Loading::Each => return,
    };
}
