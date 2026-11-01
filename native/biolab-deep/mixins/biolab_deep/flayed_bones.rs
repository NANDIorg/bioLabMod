// Проход костей: после всего тела моба дорисовываем его срезанные конечности
// тонкими и цвета кости. RETURN, а не вставка после отрисовки тела: в
// `render_visible` есть замыкание `draw_body`, и внутренние вставки туда не
// допускаются; RETURN трогает только хвостовое выражение.
// `value` (хвост `if shell_last { draw_body(out); }`) — первый параметр:
// аргументы вычисляются по порядку, и `out` для нас заимствуется уже после
// того, как хвост им попользовался.
// Невидимое тело (`body_visible = false`: рисуются одни слои) костей не даёт.
// Атрибутов у помощника быть не может (так требует инструмент миксинов).
fn biolab_deep_flayed_bones(
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
        crate::biolab_surgery::draw_bones(m, alpha, base, atlas, light, data, out);
    }
    value
}
