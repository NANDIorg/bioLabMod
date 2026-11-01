// Размер мутанта: гены растягивают модель поверх ванильного масштаба.
// Моб на операционном столе ещё и лежит (GDD 4.17): поворот «лёжа» ставится
// поверх всего масштаба, чтобы на столешницу лёг уже итоговый габарит модели.
// Модель игрока — не моб: геном мобов по id к ней не относится (у своей
// модели id 0, как у первого моба сервера); стол её тоже не касается
// (`biolab_surgery::on_table`).
fn biolab_deep_scale(m: &crate::mobs::Mob, value: glam::Mat4) -> glam::Mat4 {
    let genome = if crate::biolab_players::is_player(m) {
        None
    } else {
        crate::biolab_mutants::genome(m.id)
    };
    let value = match genome {
        Some(g) => {
            let s = crate::biolab_mutants::scale(&g);
            value * glam::Mat4::from_scale(glam::Vec3::splat(s))
        }
        None => value,
    };
    crate::biolab_surgery::lay_on_table(m, value)
}
