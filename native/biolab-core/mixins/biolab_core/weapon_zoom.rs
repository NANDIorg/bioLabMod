// Прицел пневматического ружья (GDD 4.20): пока игрок держит ПКМ с ружьём
// (использование идёт — значит, магазин не пуст, `weapon_start.rs`), вид
// приближается, как у подзорной трубы, только слабее, и мышь замедляется.
// `native_zoom` — для этого и есть: приближение мода, плавное, со своей
// чувствительностью; подзорная труба при этом главнее. `compute_fov_modifier`
// в нашем пине не стабильный хук (его нет в `native-hooks.json`), поэтому не
// он. Клиент, стабильный хук.
fn biolab_core_weapon_zoom(
    game: &crate::game::Game<'_>,
    zoom: &mut Option<horizon_client::mod_api::NativeZoom>,
) {
    use crate::game::biolab_core as m;
    if zoom.is_some() || !game.native_on(m::ID) {
        return;
    }
    let Some((slot, _)) = game.using else {
        return;
    };
    if game.stack_in(slot).is_some_and(|stack| &*stack.item == m::AIR_RIFLE_ID) {
        *zoom = Some(horizon_client::mod_api::NativeZoom { factor: m::RIFLE_ZOOM, smooth: true, turn: m::RIFLE_ZOOM_TURN });
    }
}
