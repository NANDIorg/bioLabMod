// Наросты в кадре от первого лица, не привязанные к руке игрока (GDD 4.19,
// этап «игрок»): третья рука у плеча (бьёт следом за рукой) и щупальца снизу
// кадра. Их видно, что бы игрок ни держал, поэтому они — после всех рук и
// предметов.
// Куда вставать: в саму `hand_geometry` нельзя — в ней вложенный `use`
// (инструмент миксинов такие функции не берёт, проверено `horizon-mixins
// plan`). Зато `render` сразу после неё отдаёт тот же буфер кадра рук
// (`&mut hand`) в `item_activation_geometry` (тотем): HEAD там и дописывает
// в него. Других вызовов у неё нет; на экране заглавия и в подзорной трубе
// буфер пуст.
// Пустой буфер — руки в кадре нет вовсе (третье лицо, F1, сон, смерть,
// наблюдатель): тогда и наростов нет. Невидимый игрок без предмета в руке
// тоже даёт пустой буфер; с предметом — проверяем невидимость сами.
// Кадр рук (вид, покачивание), свет и замах считаются так же, как в
// `hand_geometry`; `partial` — тот же `camera_alpha`. Внутренние поля и
// методы `Game` — не хук-поинты: при обновлении Horizon проверить.
// Атрибутов у помощника быть не может (так требует инструмент миксинов).
fn biolab_deep_view_growths(
    game: &crate::game::Game<'_>,
    partial: f32,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) {
    if out.is_empty() || !game.camera_type.first_person() || game.own_invisible() {
        return;
    }
    let a = partial;
    let swing = game.swing.animation(a);
    let view = horizon_client::camera::view_matrix(game.yaw, game.pitch);
    let [sway_x, sway_y] = game.sway.rotation(a, game.pitch, game.yaw);
    let to_world = view.inverse()
        * glam::Mat4::from_rotation_x(sway_x.to_radians())
        * glam::Mat4::from_rotation_y(sway_y.to_radians());
    let eye = game.eye_position().unwrap_or_default();
    let light = game.world.light(
        eye[0].floor() as i32,
        eye[1].floor() as i32,
        eye[2].floor() as i32,
    );
    let packed = (u32::from(light >> 4) * 16) | ((u32::from(light & 15) * 16) << 8);
    let model = game.player_model.as_ref();
    let slim = model.is_some_and(|p| p.slim);
    let age = model.map_or(0.0, |p| p.age as f32) + a;
    let sprites = &game.gui.atlas.sprites;
    // Скин — как у руки игрока: свой, а без места в атласе — Стив.
    let skin = match model.and_then(|p| p.skin.as_deref()).and_then(|s| sprites.get(s)) {
        Some(r) => Some([r.u0, r.v0, r.u1, r.v1]),
        None => sprites
            .get("entity/player/wide/steve")
            .map(|r| [r.u0, r.v0, r.u1, r.v1]),
    };
    let growth = sprites
        .get(horizon_client::biolab_players::SPRITE)
        .map(|r| [r.u0, r.v0, r.u1, r.v1]);
    let arms = [
        game.res.player_arm(slim, false).as_ref(),
        game.res.player_arm(slim, true).as_ref(),
    ];
    let start = out.len();
    horizon_client::biolab_players::draw_view(arms, to_world, swing, age, packed, skin, growth, out);
    // Как в самой `hand_geometry`: рука и предметы в кадре — без улучшенной
    // прозрачности.
    for vertex in &mut out[start..] {
        vertex.layer &= !horizon_client::gfx::OIT_ENTITY;
    }
}
