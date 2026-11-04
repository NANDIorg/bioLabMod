// Наш эффект подействовал (`MobEffect.applyEffectTick`, стабильный хук API 56).
// Игра здесь только для чтения: моб в этот момент вынут из мира, эффекты
// игрока — тоже (`docs/native-mods.md`, API 56). Поэтому действие лишь
// записывается в очередь `biolab_effects`, а выполняет его тик игрока
// (`effect_actions.rs`). Снять эффект можно сразу — `keep`.
fn biolab_core_effect_apply(
    game: &crate::game::Game<'_>,
    effect: &str,
    dimension: &str,
    entity: Option<i32>,
    amplifier: i32,
    keep: &mut bool,
) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_effects as fx;
    // Хук серверный; у клиента (`remote`) мобы и игрок — копии, их не трогаем.
    if !game.native_on(m::ID) || game.remote.is_some() {
        return;
    }
    let who = match entity {
        Some(id) => match u32::try_from(id) {
            Ok(id) => fx::Who::Mob { dimension: dimension.to_owned(), id },
            Err(_) => return,
        },
        // Игрок — тот, чьё состояние сейчас подставлено (гость сервера, в
        // одиночной игре окно — тоже гость); None — хозяин без подстановки.
        None => fx::Who::Player(game.lan_acting_uuid),
    };
    let Ok(mut state) = fx::state().lock() else {
        return;
    };
    if !state.apply(game.ticks, who, effect, amplifier) {
        *keep = false;
    }
}
