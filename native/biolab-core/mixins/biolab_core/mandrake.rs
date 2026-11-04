// Крик мандрагоры (GDD 4.20): спелую мандрагору сорвали без защиты ушей —
// короткая тошнота. Затычек в моде нет, уши закрывает любой предмет в слоте
// головы (`plants.rs`, `scream`). Хук ломания блока игроком (API 14) — до того,
// как блок исчез, поэтому состояние (`age`) ещё на месте. Блок не отменяем:
// урожай выпадет своей таблицей.
fn biolab_core_mandrake_scream(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
) {
    use crate::game::biolab_plants as p;
    if !game.native_on(crate::game::biolab_core::ID) {
        return;
    }
    // Чужой блок отсекается первым же сравнением имени.
    let age = match game.pack.assets.registry.state(state) {
        Some(s) if s.name == p::MANDRAKE => s.properties.get("age").cloned(),
        _ => return,
    };
    let covered = game.native_carried_stack(p::HEAD_SLOT).is_some();
    match p::scream(p::MANDRAKE, age.as_deref(), covered) {
        p::Scream::Silent => return,
        p::Scream::Muffled => crate::game::biolab_say::say(game, p::SCREAM_MUFFLED),
        p::Scream::Nausea => {
            game.add_player_effect(horizon_client::effects::EffectInstance::new(
                "nausea",
                p::SCREAM_NAUSEA_TICKS,
                0,
            ));
            crate::game::biolab_say::say(game, p::SCREAM_MESSAGE);
        }
    }
    // Крик слышат все рядом. Очередь уровня есть только у своего сервера.
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let centre = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5];
    let _ = game.with_level(|_, level, _| {
        level.native_play_sound(p::SCREAM_SOUND, centre, p::SOURCE_BLOCK, p::SCREAM_VOLUME, p::SCREAM_PITCH)
    });
}
