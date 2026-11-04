// Шифт-клик в меню машины (GDD 4.15, автоматизация). Из инвентаря предмет
// идёт в слот по роли: сырьё — во вход, раствор чана — в слот топлива, а не
// в первый свободный, как у сундука (раньше так он попадал и в выход, и
// машина вставала). Куда и сколько — решает `runtime.rs`
// (`quick_move_targets`, `quick_move_plan`), здесь только предметы.
// Шифт-клик по слоту самой машины не трогаем: игра кладёт из него в
// инвентарь, как из сундука, — это и нужно. Предмет, который машине не
// нужен, тоже отдаём игре: в своём меню она спросит правила слотов и
// никуда его не положит.
fn biolab_core_machine_quick_move(
    game: &mut crate::game::Game<'_>,
    entity: &mut horizon_client::chunk::block_entity::ModBlockEntity,
    container_slots: usize,
    slot: usize,
    stack: &mut horizon_client::items::ItemStack,
    handled: &mut bool,
) {
    use crate::game::biolab_core as m;
    if *handled || slot < container_slots || !game.native_on(m::ID) {
        return;
    }
    let Some(machine) = m::machine(&entity.id) else {
        return;
    };
    let targets = m::quick_move_targets(machine, &stack.item);
    if targets.is_empty() {
        return;
    }
    if entity.items.len() < machine.slots {
        entity.items.resize(machine.slots, None);
    }
    // Долить можно только такой же стак с теми же данными: две колбы с
    // разным веществом или два фрагмента с разными генами не сливаются.
    let held: Vec<Option<(bool, u32)>> = entity
        .items
        .iter()
        .map(|s| s.as_ref().map(|s| (s.same_item_same_components(stack), s.count)))
        .collect();
    // `items()` — таблица предметов игры, не хук-поинт: при обновлении
    // Horizon проверить.
    let max = game.items().get(&stack.item).map_or(64, |i| i.max_stack);
    let plan = m::quick_move_plan(&held, &targets, stack.count, max);
    if plan.is_empty() {
        return;
    }
    for (index, put) in plan {
        let place = &mut entity.items[index];
        match place.as_mut() {
            Some(held) => held.count += put,
            None => {
                *place = Some(horizon_client::items::ItemStack {
                    count: put,
                    ..stack.clone()
                })
            }
        }
        stack.count -= put;
    }
    *handled = true;
}
