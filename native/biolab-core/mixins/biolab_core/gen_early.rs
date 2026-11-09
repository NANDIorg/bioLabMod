// Генераторы тиров 0–2 (M10, GDD 4.27 Б1): электрофорная машина, нажимная
// пластина, вольтов столб, термопара, паровой котёл с динамо, водяное колесо,
// ветрогенератор.
//
// Таблица буферов и правила — раздел «M10: генераторы 0–2» в `runtime.rs`.
// Выработку класть `crate::game::biolab_power::generate` — в свой буфер;
// из него тянут машины и накопители. Модуль игры
// (`crate::game::biolab_gen_early`), его тик зовёт `power_tick.rs`.

use horizon_client::chunk::block_entity::ModBlockEntity;

/// Тик блок-сущности. Не свой блок — сразу выход.
pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let _ = (game, pos, entity);
}
