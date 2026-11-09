// Живые генераторы (M10, GDD 4.27 В): микробный топливный элемент,
// фотобиореактор, глюкозный элемент, электроцитная батарея.
//
// Таблица буферов и правила — раздел «M10: живые генераторы» в `runtime.rs`.
// Выработку класть `crate::game::biolab_power::generate` — в свой буфер;
// из него тянут машины и накопители. Модуль игры
// (`crate::game::biolab_gen_living`), его тик зовёт `power_tick.rs`.

use horizon_client::chunk::block_entity::ModBlockEntity;

/// Тик блок-сущности. Не свой блок — сразу выход.
pub fn tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let _ = (game, pos, entity);
}
