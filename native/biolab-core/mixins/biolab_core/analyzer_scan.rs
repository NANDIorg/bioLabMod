// Анализатор в руке (GDD 4.26 Г): раз в тик окна снимаем данные мобов рядом
// для рамок в мире (`analyzer_marks.rs`). Хук кадра `native_world_primitives`
// — `&self`, а данные на сущности читаются через `Host` с `&mut Game`,
// поэтому чтение — здесь, в тике, а кадр берёт снимок из `runtime.rs`.
//
// Только окно (`native_window`): у сервера свои данные, а рамки рисует
// клиент, и читаем мы копию, которую прислал сервер (`biolab:genome`,
// `biolab:surgery` с `sync`) — так это работает и на выделенном сервере.
// Нет анализатора в руке — снимок стирается, и кадр не рисует ничего.
// `selected` — поле игры, не хук-поинт (как в `analyzer.rs`): при обновлении
// Horizon проверить.
fn biolab_core_analyzer_scan(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    if !game.native_window() {
        return;
    }
    if !game.native_on(m::ID) {
        m::forget_analyzer_scan();
        return;
    }
    let slot = game.selected as i32;
    let holding = game
        .native_carried_stack(slot)
        .map(|s| s.item.clone())
        .is_some_and(|item| &*item == m::ANALYZER_ID);
    let Some(player) = game.native_player().filter(|_| holding) else {
        m::forget_analyzer_scan();
        return;
    };
    let from = [player.x, player.y, player.z];
    // Запас в два блока: снимок живёт тик, а моб за это время подходит.
    let near: Vec<u32> = game
        .mobs
        .iter()
        .filter(|mob| {
            mob.alive()
                && !mob.removed
                && &*mob.kind.id != m::ANALYZER_PLAYER_KIND
                && m::analyzer_reaches(from, [mob.position.x, mob.position.y, mob.position.z], 2.0)
        })
        .map(|mob| mob.id)
        .collect();
    // Данные — после обхода мобов: внутри него `Host` не достать.
    let mut seen = Vec::new();
    for id in near {
        let genes = crate::game::biolab_core_data::genes(game, id);
        let surgery = crate::game::biolab_core_data::surgery(game, id);
        if let Some(one) = m::AnalyzerSeen::of(id, genes, surgery) {
            seen.push(one);
        }
    }
    m::remember_analyzer_scan(seen);
}
