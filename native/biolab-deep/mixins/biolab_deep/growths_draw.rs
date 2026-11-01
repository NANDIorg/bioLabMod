// Проход наростов (GDD 4.19): после всего тела моба дорисовываем наросты его
// генома — рога, хвост, щупальца, третью руку, вторую голову и т. д.
// RETURN, как у прохода костей (`flayed_bones.rs`): в `render_visible` есть
// замыкание `draw_body`, и внутренние вставки туда не допускаются; RETURN
// трогает только хвостовое выражение. Два RETURN на одной функции
// складываются в цепочку (`$value` — результат предыдущего).
// `value` — первый параметр: аргументы вычисляются по порядку, и `out` для нас
// заимствуется уже после того, как хвост им попользовался.
// Невидимое тело (`body_visible = false`: рисуются одни слои) наростов не даёт.
// Атрибутов у помощника быть не может (так требует инструмент миксинов).
fn biolab_deep_growths(
    value: (),
    m: &crate::mobs::Mob,
    alpha: f32,
    base: glam::Mat4,
    atlas: crate::mobs::render::Atlas<'_>,
    light: u32,
    data: &crate::mobs::MobData,
    body_visible: bool,
    out: &mut Vec<crate::mesher::Vertex>,
) {
    if body_visible {
        crate::biolab_growths::draw_growths(m, alpha, base, atlas, light, data, out);
    }
    value
}
