// Игрок начал видеть сущность (`native_player_start_tracking`, API 141):
// разослать её данные мутанта заново, чтобы они дошли и до него.
//
// Пин сам шлёт клиенту все синхронизированные данные носителя, когда тот
// начинает его видеть, — но только если загружен трейт-мод:
// `hub.sync_attachments = !mod_api::mods().is_empty()` (`game/lan.rs`), и без
// этого `take_tracking`/`send_level_info` (`lan/hub.rs`) снимков не ставят.
// Наши бандлы — миксины, `mods()` у нас пуст. Поэтому после перезахода
// данные загруженного мутанта до окна не доходили: они уходят только при
// записи (проверено в игре 04.11.2026 — рога после перезахода пропали, а в
// сейве `genome` был). `sync_data` ставит значение в очередь рассылки
// (`flush_attachment_sync` в конце того же прохода LAN, после этого хука) —
// всем, кто сейчас видит носителя, а значит и этому игроку.
// Другой игрок — тоже сущность: для него рассылаем его гены. Себя игрок не
// видит никогда — свои гены ему переотправляет `biolab-core` раз в секунду
// (`biolab_core_data::set_player_genes`).
// Атрибутов у помощника быть не может (так требует инструмент миксинов).
fn biolab_deep_start_tracking(game: &mut crate::game::Game<'_>, entity: u32, uuid: u128, kind: &str) {
    use crate::game::biolab_deep as d;
    use crate::game::biolab_deep_data as data;
    if !game.native_on(d::ID) {
        return;
    }
    if kind == "minecraft:player" {
        data::resync_player(game, uuid);
    } else {
        data::resync_entity(game, entity);
    }
}
