// Рамка маски на экране (M9, GDD 4.25): респиратор на голове — по краям
// экрана тёмные полосы, как край маски; забитый фильтр — с красноватым
// краем. От первого лица, как ванильная тыква. Рисуем перед хотбаром
// (`native_gui_layer_pre`, API 139), чтобы хотбар, здоровье и чат лежали
// поверх рамки. Только клиент; броня — та, что прислал сервер.
fn biolab_core_suit_mask(
    game: &mut crate::game::Game<'_>,
    layer: &str,
    batch: &mut horizon_client::hud::HudBatch,
) {
    use crate::game::biolab_core as m;
    if layer != "minecraft:hotbar" || !game.native_on(m::ID) || !game.camera_type.first_person() {
        return;
    }
    let dry = match game.armor[0].as_ref() {
        Some(head) if &*head.item == m::RESPIRATOR_ID => {
            m::respirator_filter(crate::game::biolab_suit::custom_text(head, m::SUIT_KEY).as_deref()) == 0
        }
        _ => return,
    };
    let (width, height) = (batch.width as f32, batch.height as f32);
    for (rect, argb) in m::mask_frame(width, height, dry) {
        game.native_fill(batch, rect, argb);
    }
}
