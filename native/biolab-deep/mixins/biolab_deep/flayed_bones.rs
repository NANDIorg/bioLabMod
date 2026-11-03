// С ноября 2026 цепляемся не к `render_visible` (в ней появился `?`, а RETURN
// его не допускает), а к обёрткам `render_with` и `render_skinned`. Матрицу
// `base` они ещё не считали, и вызывать функции прямо в привязке нельзя —
// поэтому берём `camera` и считаем ту же `base_transform`, что и сама игра.
// Проход костей: после всего тела моба дорисовываем его срезанные конечности
// тонкими и цвета кости. RETURN, а не вставка после отрисовки тела: в
// `render_visible` есть замыкание `draw_body`, и внутренние вставки туда не
// допускаются; RETURN трогает только хвостовое выражение.
// `value` (хвост `if shell_last { draw_body(out); }`) — первый параметр:
// аргументы вычисляются по порядку, и `out` для нас заимствуется уже после
// того, как хвост им попользовался.
// Атрибутов у помощника быть не может (так требует инструмент миксинов).
fn biolab_deep_flayed_bones(
    value: (),
    m: &crate::mobs::Mob,
    alpha: f32,
    camera: glam::DVec3,
    atlas: crate::mobs::render::Atlas<'_>,
    light: u32,
    data: &crate::mobs::MobData,
    out: &mut Vec<crate::mesher::Vertex>,
) {
    let base = crate::mobs::render::base_transform(m, alpha, camera);
    // Обёртки зовут отрисовку тела всегда: отдельной проверки не нужно.
    crate::biolab_surgery::draw_bones(m, alpha, base, atlas, light, data, out);
    value
}
