// Урон своего типа мобу (GDD 4.26, Б; M11-17): из чистой записи таблицы
// урона (`effects.rs`, `DamageType`) — в `Damage` игры.
//
// Моб узнаёт тип урона по флагам `NamedDamage` (`docs/native-mods.md` пина,
// «Named mob damage»; `crates/horizon-client/src/mobs.rs`): имя уходит в
// запись урона моба, в лут и в сообщение о смерти, а броню, отброс и панику
// игра решает по флагам. Флаги совпадают с тегами в
// `content/data/minecraft/tags/damage_type/` — это сверяет
// `tests/effects.rs`. Ванильная магия остаётся `DamageKind::Magic`: у неё
// свои ванильные теги, и именованной её делать незачем.
//
// Игроку тип отдаётся по id (`EffectAction::Hurt(id, сила)`): игра ищет его в
// реестре типов урона мира, куда попадает и наш JSON.
//
// Модуль живёт в игре: `crate::game::biolab_damage`.

use crate::game::biolab_effects as fx;
use horizon_client::mobs::{Damage, DamageKind, NamedDamage};

/// Урон мобу этого типа.
pub fn mob(damage: &'static fx::DamageType) -> Damage {
    if damage.id == fx::DAMAGE_MAGIC.id {
        return Damage::of(DamageKind::Magic);
    }
    let mut tags = 0;
    if damage.bypasses_armor {
        tags |= NamedDamage::BYPASSES_ARMOR;
    }
    if damage.no_knockback {
        tags |= NamedDamage::NO_KNOCKBACK;
    }
    if damage.panic_causes {
        tags |= NamedDamage::PANIC_CAUSES;
    }
    Damage::of(DamageKind::Named(NamedDamage::new(damage.id, tags)))
}

/// Урон игроку, на чьём тике идёт хук: путём урона эффекта, как у
/// ванильного яда (броня по тегам типа, кадры неуязвимости, щит).
pub fn player(game: &mut crate::game::Game<'_>, damage: &'static fx::DamageType, amount: f32) {
    game.apply_effect_action(horizon_client::effects::EffectAction::Hurt(damage.id, amount));
}
