// Нажимная пластина (M10, GDD 4.27 Б1): сущность стоит на блоке мода
// (`Block.stepOn`, хук `native_block_step_on`, API 73, сервер). Хук без
// `self` и без имени блока — только место; записываем, кто касался, а
// считает шаги тик пластины (`take_piezo_steps`, раздел «M10: генераторы
// 0–2» в `runtime.rs`). Пара — `piezo_inside.rs`: тонкую пластину сущность
// не столько «топчет», сколько стоит внутри её клетки.
fn biolab_core_piezo_step(dimension: &str, entity: i32, position: [i32; 3]) {
    crate::game::biolab_core::note_piezo_contact(dimension, position, entity);
}
