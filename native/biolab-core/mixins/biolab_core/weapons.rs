// Оружие для дротиков в игре (GDD 4.20): духовая трубка, арбалет-транквилизатор
// и пневматическое ружьё. Решения — в `runtime.rs` («Оружие для дротиков»),
// здесь — исполнение на типах игры: стек оружия, дротики в инвентаре, запуск
// снаряда, звук и сообщение. Модуль живёт в игре (`crate::game::biolab_weapons`):
// хуки (`remedy.rs`, `weapon_*.rs`) зовут его, а не держат каждый свою копию.
//
// Как выпускается снаряд. В пине нет помощника «выпусти снаряд с такой
// скоростью» (`docs/native-mods.md`: `native_projectile_launch` только
// сообщает о запуске, `player_launch_projectile` — только гостям LAN и только
// ванильные). Но модуль — потомок `crate::game`, и ему видно то же, чем игра
// бросает снежок сама (`game/creatures.rs`, `throw_item`):
// `Projectile::new(ProjectileKind::Thrown(..))` + `shoot_from_rotation`
// (`projectiles.rs`, публичные) и `add_projectile` (`pub(super)` в
// `game::creatures`). Скорость и разброс — свои, из `DART_WEAPONS`. Это
// внутренний API игры, а не хук-поинт: при обновлении Horizon проверить
// `throw_item` и сигнатуры. `add_projectile` сам зовёт хук запуска
// (`dart_launch.rs`) и раздаёт снаряд клиентам, как брошенный рукой.
//
// Где работает. Клик (`native_use_item`) — сервер. Тик натяжения и отпускание
// идут на обеих сторонах, но мир меняет только тот `Game`, что ведёт мир
// (`native_window()` ложно): на клиенте сервера стек и снаряды придут от
// сервера. Отказ начать использование — на обеих, одним решением
// (`weapon_use_may_start`), иначе клиент предсказал бы натяжение зря.

use crate::game::biolab_core as m;
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

/// Левая рука в нумерации `native_carried_stack`.
pub const OFFHAND: i32 = -106;

/// Слот оружия `item` у действующего игрока: правая рука, иначе левая.
pub(super) fn weapon_slot(game: &mut crate::game::Game<'_>, item: &str) -> Option<i32> {
    let selected = game.selected as i32;
    [selected, OFFHAND]
        .into_iter()
        .find(|&slot| game.native_carried_stack(slot).is_some_and(|s| &*s.item == item))
}

/// Данные оружия со стека (`custom_data`, ключ `WEAPON_KEY`).
pub fn state_of(stack: &ItemStack) -> m::WeaponState {
    let text = stack
        .extra
        .as_ref()
        .and_then(|extra| {
            extra
                .other
                .iter()
                .find(|(key, _)| key == "minecraft:custom_data")
                .and_then(|(_, data)| match data.get(m::WEAPON_KEY) {
                    Some(Tag::String(s)) => Some(s.to_string_lossy()),
                    _ => None,
                })
        })
        .unwrap_or_default();
    m::weapon_state(&text)
}

/// Записать данные оружия в стек. Своего в `custom_data` у оружия больше
/// нет, поэтому запись целиком заменяет его. Сохраняется с миром и едет
/// клиенту вместе со стеком (GDD 3.9).
pub fn store(stack: &mut ItemStack, state: &m::WeaponState) {
    let data = stack.extra.get_or_insert_with(Default::default);
    data.other.retain(|(key, _)| key != "minecraft:custom_data");
    data.other.push((
        "minecraft:custom_data".to_string(),
        Tag::compound([(m::WEAPON_KEY, Tag::String(m::weapon_text(state).as_str().into()))]),
    ));
    data.other.sort_by(|a, b| a.0.cmp(&b.0));
}

/// Записать данные в оружие, что лежит в `slot`.
fn store_in(game: &mut crate::game::Game<'_>, slot: i32, item: &str, state: &m::WeaponState) {
    if let Some(stack) = game.native_carried_stack(slot)
        && &*stack.item == item
    {
        store(stack, state);
    }
}

/// Стопки дротиков у действующего игрока в порядке, каким их берёт оружие:
/// левая рука, потом инвентарь по номерам слотов (как `getProjectile` у
/// лука: сначала руки, потом инвентарь).
pub(super) fn dart_stacks(game: &mut crate::game::Game<'_>) -> Vec<(i32, &'static str, u32)> {
    let slots = game.inventory.len() as i32;
    let mut out = Vec::new();
    for slot in std::iter::once(OFFHAND).chain(0..slots) {
        if let Some(stack) = game.native_carried_stack(slot)
            && stack.count > 0
            && let Some(dart) = m::dart_projectile(&stack.item)
        {
            out.push((slot, dart, stack.count));
        }
    }
    out
}

/// Взять `count` дротиков из слота. Последний ушедший освобождает слот:
/// стопка с 0 очищается в конце тика инвентаря (`native_carried_stack`).
fn take_from(game: &mut crate::game::Game<'_>, slot: i32, count: u32) {
    if let Some(stack) = game.native_carried_stack(slot) {
        stack.count = stack.count.saturating_sub(count);
    }
}

/// Слот полного баллона CO₂, если он есть.
fn canister_slot(game: &mut crate::game::Game<'_>) -> Option<i32> {
    let slots = game.inventory.len() as i32;
    std::iter::once(OFFHAND)
        .chain(0..slots)
        .find(|&slot| game.native_carried_stack(slot).is_some_and(|s| &*s.item == m::CO2_CANISTER_ID && s.count > 0))
}

/// Подключить баллон: полный становится пустым прямо в слоте. Выдача в
/// инвентарь из хука ненадёжна (`remedy.rs`, шприц от вакцины), поэтому
/// полный баллон не стопкуется и меняется на месте.
fn spend_canister(game: &mut crate::game::Game<'_>, slot: i32) {
    if let Some(stack) = game.native_carried_stack(slot) {
        if stack.count <= 1 {
            stack.item = m::CO2_CANISTER_EMPTY_ID.into();
            stack.count = 1;
            stack.damage = 0;
            stack.extra = None;
        } else {
            stack.count -= 1;
        }
    }
}

/// Звук события мода в точке `at` всем рядом — как в `remedy.rs`: очередь
/// уровня (`native_play_sound`, API 25) и пакет от сервера. Без своего сервера
/// (клиент) звук не берётся. `with_level` — внутренний API игры.
pub(super) fn play_at(game: &mut crate::game::Game<'_>, cue: m::Cue, at: [f64; 3]) {
    let Some(sound) = m::sound(cue) else {
        return;
    };
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let _ = game.with_level(|_, level, _| level.native_play_sound(sound.event, at, sound.source, sound.volume, sound.pitch));
}

/// Звук у головы действующего игрока.
fn play(game: &mut crate::game::Game<'_>, cue: m::Cue) {
    if let Some(player) = game.native_player() {
        play_at(game, cue, [player.x, player.y + 1.0, player.z]);
    }
}

/// Сообщение действующему игроку над хотбаром (API 21): оно доходит и до
/// гостя LAN. Нет такого игрока на сервере (мир ведёт само окно) — в чат.
fn say(game: &mut crate::game::Game<'_>, text: &str) {
    let uuid = game.native_acting_id();
    if !game.native_action_bar(uuid, horizon_client::text::Text::literal(text)) {
        crate::game::biolab_say::say(game, text);
    }
}

/// Пауза оружия: группа кулдауна — id предмета, как у ванильного
/// (`cooldowns::group` без компонента `use_cooldown`). Гостю идёт пакетом.
fn cool_down(game: &mut crate::game::Game<'_>, item: &str, ticks: i32) {
    if ticks > 0 {
        game.add_cooldown(item.into(), ticks);
    }
}

/// Выпустить дротик `dart` из оружия `weapon` от глаз действующего игрока по
/// взгляду. Тот же снаряд, что у броска рукой (`throw_item`): старт на 0,1
/// ниже глаз, к скорости прибавляется движение стрелка. Дальность действия
/// взводится до `add_projectile`, и хук запуска запоминает её с точкой
/// (`dart_launch.rs`). Ложь — снаряд не вылетел (нет игрока, тип дротика не
/// зарегистрирован).
pub(super) fn shoot(game: &mut crate::game::Game<'_>, weapon: &m::DartWeapon, dart: &str) -> bool {
    use horizon_client::projectiles::{Projectile, ProjectileKind};
    let Some(kind) = horizon_client::projectiles::thrown_type(dart) else {
        return false;
    };
    let Some(eye) = game.eye_position() else {
        return false;
    };
    let Some(on_ground) = game.player.as_ref().map(|p| p.ground.on_ground) else {
        return false;
    };
    let movement = game.known_player_movement();
    let (pitch, yaw) = (game.pitch, game.yaw);
    let start = horizon_core::geometry::Vec3::new(eye[0], eye[1] - 0.1, eye[2]);
    let mut projectile = Projectile::new(ProjectileKind::Thrown(kind), start, Some(horizon_client::mobs::Who::Player));
    projectile.shoot_from_rotation(&mut game.world.random, pitch, yaw, 0.0, weapon.speed, weapon.spread, movement, on_ground);
    m::arm_dart_range(weapon.range);
    game.add_projectile(projectile);
    // Хук запуска забрал дальность; не позвали его — снимаем сами, чтобы
    // она не досталась следующему броску рукой.
    let _ = m::take_armed_range();
    // Тихое оружие не будит сенсоры скалка и стража: игровое событие
    // `projectile_shoot` шлём только у громкого.
    if !weapon.quiet {
        game.player_projectile_shot();
    }
    play_at(game, weapon.cue, eye);
    true
}

/// ПКМ оружием (`native_use_item`, сервер): трубка стреляет, заряженный
/// арбалет стреляет, пустое ружьё заряжается, нечем — осечка. Ложь — клик не
/// забираем: игра начнёт использование (натяжение арбалета, прицел ружья).
pub(super) fn click(game: &mut crate::game::Game<'_>, weapon: &'static m::DartWeapon, stack: &ItemStack) -> bool {
    let Some(slot) = weapon_slot(game, &stack.item) else {
        return false;
    };
    let mut state = state_of(stack);
    let stacks = dart_stacks(game);
    let creative = game.creative;
    match m::weapon_click(weapon, &state, m::first_dart(&stacks), creative) {
        m::WeaponClick::Use => return false,
        m::WeaponClick::Shoot(dart) => {
            if !shoot(game, weapon, dart) {
                return true;
            }
            match weapon.loading {
                // Трубка: выстрелила первым дротиком инвентаря — он и уходит.
                m::Loading::Each => {
                    if !creative && let Some(&(from, _, _)) = stacks.first() {
                        take_from(game, from, 1);
                    }
                }
                // Арбалет: заряд ушёл.
                m::Loading::Charge | m::Loading::Magazine => {
                    state.load.clear();
                    store_in(game, slot, weapon.item, &state);
                }
            }
            cool_down(game, weapon.item, weapon.cooldown);
        }
        m::WeaponClick::Reload => {
            for (from, count) in m::reload_magazine(&mut state, &stacks, creative) {
                take_from(game, from, count);
            }
            store_in(game, slot, weapon.item, &state);
            play(game, m::Cue::WeaponLoaded);
            say(game, &m::magazine_message(&state));
            cool_down(game, weapon.item, m::RIFLE_RELOAD_COOLDOWN);
        }
        m::WeaponClick::NoDarts => {
            play(game, m::Cue::WeaponEmpty);
            say(game, m::no_darts_message());
        }
    }
    true
}

/// Начать ли использование оружия (`living_entity_use_item_start`, обе
/// стороны): тот же ответ, что у клика.
pub(super) fn may_start(game: &mut crate::game::Game<'_>, weapon: &m::DartWeapon, stack: &ItemStack) -> bool {
    let state = state_of(stack);
    let stacks = dart_stacks(game);
    let creative = game.creative;
    m::weapon_use_may_start(weapon, &state, m::first_dart(&stacks), creative)
}

/// Тик натяжения арбалета (`on_use_tick`, мир ведёт этот `Game`): звуки на
/// 20 и 50 %, на 1,25 с — дротик из инвентаря в арбалет.
pub(super) fn charge_tick(game: &mut crate::game::Game<'_>, weapon: &m::DartWeapon, stack: &ItemStack, remaining: i32) {
    let mut state = state_of(stack);
    if !state.load.is_empty() {
        return;
    }
    match m::crossbow_charge(m::weapon_held(remaining)) {
        None => {}
        Some(m::ChargeStep::Start | m::ChargeStep::Middle) => play(game, m::Cue::WeaponCharge),
        Some(m::ChargeStep::Loaded) => {
            let Some(slot) = weapon_slot(game, weapon.item) else {
                return;
            };
            let stacks = dart_stacks(game);
            let creative = game.creative;
            let dart = match (stacks.first(), creative) {
                (Some(&(from, dart, _)), false) => {
                    take_from(game, from, 1);
                    dart
                }
                (Some(&(_, dart, _)), true) => dart,
                (None, true) => m::DARTS[0].0,
                // Дротик ушёл из инвентаря, пока натягивали: не заряжаем.
                (None, false) => {
                    play(game, m::Cue::WeaponEmpty);
                    return;
                }
            };
            state.load = vec![dart];
            store_in(game, slot, weapon.item, &state);
            play(game, m::Cue::WeaponLoaded);
        }
    }
}

/// Отпущен прицел ружья (`release_using`, мир ведёт этот `Game`): выстрел
/// первым дротиком магазина, газ из баллона.
pub(super) fn rifle_release(game: &mut crate::game::Game<'_>, weapon: &m::DartWeapon, stack: &ItemStack) {
    let Some(slot) = weapon_slot(game, weapon.item) else {
        return;
    };
    let mut state = state_of(stack);
    let canister = canister_slot(game);
    let creative = game.creative;
    match m::rifle_fire(&mut state, canister.is_some(), creative) {
        m::RifleShot::Empty => {}
        m::RifleShot::NoGas => {
            play(game, m::Cue::WeaponEmpty);
            say(game, m::no_gas_message());
        }
        m::RifleShot::Fired { dart, connected } => {
            // Снаряд не вылетел — магазин и газ остаются как были.
            if !shoot(game, weapon, dart) {
                return;
            }
            if connected && let Some(at) = canister {
                spend_canister(game, at);
                play(game, m::Cue::GasConnect);
                say(game, &m::canister_message());
            }
            store_in(game, slot, weapon.item, &state);
            cool_down(game, weapon.item, weapon.cooldown);
        }
    }
}
