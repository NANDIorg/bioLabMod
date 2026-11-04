// На чём стоят наши культуры (GDD 4.20). Шаблон свёклы держится только на
// пашне (`supports_crops`), а дикие заросли генерация ставит на траву, подзол
// и ил (`worldgen/`): без этого ответа первое же обновление соседа (поставили
// блок рядом, траву под ней съела овца) сорвало бы дикое растение. Простую
// землю не разрешаем: вытоптанная пашня — земля, и культура с неё должна
// слететь, как ванильная. Пашню решает шаблон (`None`). Хук спрашивают на
// каждом обновлении формы растения: чужой блок отсекается первым сравнением.
// Стабильный хук API 70, только у сервера; уровень — только для чтения.
fn biolab_core_plant_ground(
    level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId,
    ground: horizon_world::registry::BlockStateId,
    may: &mut Option<bool>,
) {
    let registry = level.registry;
    let Some(own) = registry.state(state) else {
        return;
    };
    if !own.name.starts_with("biolab:") {
        return;
    }
    let Some(under) = registry.state(ground) else {
        return;
    };
    if let Some(answer) = crate::game::biolab_plants::may_stand_on(&own.name, &under.name) {
        *may = Some(answer);
    }
}
