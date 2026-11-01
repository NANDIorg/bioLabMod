// Срезанная конечность в общем проходе не рисуется: её дорисует проход костей
// (`flayed_bones.rs`) — тонкой и цвета кости. Прячем перед анимацией: после
// `anim::setup` в `render_visible` точки вставки нет (там замыкание
// `draw_body`, а внутренние вставки в функциях с замыканиями запрещены), а
// HEAD самой анимации допустим и срабатывает для тела и каждого его слоя.
fn biolab_deep_flay_hide(m: &crate::mobs::Mob, p: &mut crate::mobs::anim::Poses) {
    crate::biolab_surgery::hide_flayed(m, p);
}
