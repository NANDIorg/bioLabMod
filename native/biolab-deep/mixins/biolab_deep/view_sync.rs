// Окно: что пришло с сервера данными (M11) — отрисовке.
//
// Геном мобов (`biolab:genome`), операционная (`biolab:surgery`) и гены
// игроков (`biolab:player_genes`) объявлены с `sync`: сервер сам шлёт их
// игрокам, которые видят носителя, а окно держит копию в своём `Game` и
// отдаёт её через `Host` (`get_existing` у окна читает эту копию). Моб же
// рисуется в lib-крейте (`mobs::render`), куда доходит только `&Mob` — ни
// `Game`, ни данных там нет. Поэтому раз в тик окна (`native_tick`: «once a
// tick, in every Game», у окна — из `tick_remote`) переписываем пришедшее в
// таблицы отрисовки целиком (`biolab_bridge::show`). Источник — только копия
// окна: так это работает и с выделенным сервером, где сервер — другой процесс.
// Раньше таблицы писал серверный тик моба по id в общей памяти процесса —
// это работало только в одиночной игре.
//
// Свой игрок — по своему uuid (`Remote::profile`); его модель в окне — моб с
// id 0. Другие игроки — модели в списке мобов окна, uuid которых окно знает
// по пакетам сущностей (`persist.uuids`).
// Внутренние поля и методы `Game` (`remote`, `Remote::profile`, `mobs`,
// `persist.uuids`) — не хук-поинты: при обновлении Horizon проверить. Атрибутов у помощника быть не может (так
// требует инструмент миксинов).
fn biolab_deep_view_sync(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_deep as d;
    use crate::game::biolab_deep_data as data;
    if !game.native_on(d::ID) {
        return;
    }
    // Только окно, подключённое к серверу (в одиночной игре — к своему):
    // у сервера свои данные, а не пришедшие. Свой uuid — тот, под которым
    // нас знает сервер (`Remote::profile`, так его берёт и `locator_bar.rs`).
    let Some(own) = game.remote.as_ref().map(|r| r.profile()) else {
        return;
    };
    let mut mobs: Vec<u32> = Vec::new();
    let mut players: Vec<(u32, u128)> = Vec::new();
    for m in game.mobs.iter() {
        if horizon_client::biolab_players::is_player(m) {
            if let Some(ints) = game.persist.uuids.get(&m.id) {
                players.push((m.id, horizon_client::lan::playerdata::ints_uuid(*ints)));
            }
        } else {
            mobs.push(m.id);
        }
    }
    let mut genomes = Vec::new();
    let mut surgeries = Vec::new();
    for id in mobs {
        let genes = d::genome_from_text(&data::genome(game, id));
        if !genes.is_empty() {
            genomes.push((id, d::strengths(&genes)));
        }
        let bits = data::surgery(game, id);
        if bits != 0 {
            surgeries.push((id, bits));
        }
    }
    let own_genes = d::named(&d::genome_from_text(&data::player_genes(game, own)));
    let mut others = Vec::new();
    for (id, uuid) in players {
        if uuid == own {
            continue;
        }
        let genes = d::named(&d::genome_from_text(&data::player_genes(game, uuid)));
        if !genes.is_empty() {
            others.push((id, genes));
        }
    }
    horizon_client::biolab_bridge::show(genomes, surgeries, own_genes, others);
}
