// Проход мутантов на сервере, перед тиком мобов (M11): прибавки к атрибутам
// и поведение по генам из данных на мобах (`biolab:genome`).
//
// Раньше это делали lib-хуки на тике самого моба (`spawn.rs` на `base_tick`,
// `mob_tick.rs`) и узнавали геном по меткам на мобе. Данные видны только
// бинарнику (`Host` есть у `Game`), поэтому проход — здесь. Встаём в
// `native_entity_tick_listened` (API 146): игра спрашивает его на сервере
// перед тиком каждого семейства сущностей (мобы, предметы, вагонетки,
// лодки), в одиночной игре тоже — в отличие от `native_level_tick_post`.
// Семейств несколько, поэтому проход отсекается «раз за тик уровня»
// (`first_pass`). `listened` не трогаем: слушать сами тики сущностей нам не
// нужно, и игра не заводит их список.
//
// Раз в `SCAN_EVERY` тиков — перебор всех мобов уровня: миграция старых
// меток (страховка к `loaded.rs`), прибавки по геному, бит «спит» в данных
// операционной и выжимка мутантов. Каждый тик — поведение мутантов из
// выжимки: огонь, яды, планирование, регенерация.
// Внутренние поля `Game` (`mobs`, `ticks`, `dim`, `serving`) — не
// хук-поинты: при обновлении Horizon проверить. Атрибутов у помощника быть
// не может (так требует инструмент миксинов).
fn biolab_deep_mutant_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_deep as d;
    use crate::game::biolab_deep_data as data;
    if !game.native_on(d::ID) {
        return;
    }
    // Хук и так только серверный; страховка для клиента чужого сервера.
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let ticks = game.ticks;
    let dimension = game.dim.id().to_string();
    if !d::first_pass(&dimension, ticks) {
        return;
    }

    if d::scan_due(ticks) {
        let ids: Vec<u32> = game
            .mobs
            .iter()
            .filter(|m| !m.removed)
            .map(|m| m.id)
            .collect();
        let mut mutants = Vec::new();
        for id in ids {
            data::migrate(game, id);
            let genes = d::strengths(&d::genome_from_text(&data::genome(game, id)));
            let Some(index) = game.mobs.iter().position(|m| m.id == id) else {
                continue;
            };
            // Прибавки ставятся и снимаются здесь: ген, вживлённый живому
            // мобу, начинает работать без перезахода.
            horizon_client::biolab_bridge::apply_genome(&mut game.mobs[index], &genes, true);
            // Спит под снотворным — окно кладёт его на землю. Бит пишем,
            // только когда он меняется: каждая запись уходит клиентам.
            let asleep = horizon_client::biolab_bridge::sedated(&game.mobs[index]);
            let bits = data::surgery(game, id);
            let want = if asleep { bits | d::ASLEEP_BIT } else { bits & !d::ASLEEP_BIT };
            if want != bits {
                data::set_surgery(game, id, want);
            }
            if !genes.is_empty() {
                mutants.push((id, genes));
            }
        }
        d::remember_mutants(&dimension, mutants);
    }

    for (id, genes) in d::mutants(&dimension) {
        if let Some(m) = game.mobs.iter_mut().find(|m| m.id == id && !m.removed) {
            horizon_client::biolab_bridge::mutant_tick(m, &genes);
        }
    }
}
