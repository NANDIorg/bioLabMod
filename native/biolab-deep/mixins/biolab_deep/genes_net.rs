// Гены игроков пришли с сервера (GDD 4.19, этап «игрок»): `biolab-core` раз в
// секунду шлёт их каналом `biolab_core:genes`. Разбирает и хранит их lib
// (`biolab_players`) — там же, где их читает отрисовка. Отсюда — только то,
// что знает одно окно: какой uuid у какой сущности (`persist.uuids`, его
// заполняет приём сущностей с сервера), чтобы найти модели других игроков.
// Свой игрок — первая запись пакета, его модель окно держит отдельно.
// Хук стабильный (API 40); `persist` — внутреннее поле `Game`: при обновлении
// Horizon проверить. Атрибутов у помощника быть не может (так требует
// инструмент миксинов).
fn biolab_deep_player_genes(game: &crate::game::Game<'_>, channel: &str, data: &[u8]) {
    if channel != horizon_client::biolab_players::CHANNEL {
        return;
    }
    let mut entities: Vec<(u32, u128)> = Vec::with_capacity(game.persist.uuids.len());
    for (id, ints) in game.persist.uuids.iter() {
        entities.push((*id, horizon_client::lan::playerdata::ints_uuid(*ints)));
    }
    horizon_client::biolab_players::receive(data, &entities);
}
