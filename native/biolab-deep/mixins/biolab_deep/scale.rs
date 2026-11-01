// Размер мутанта: гены растягивают модель поверх ванильного масштаба.
// Моб на операционном столе ещё и лежит (GDD 4.17): поворот «лёжа» ставится
// поверх всего масштаба, чтобы на столешницу лёг уже итоговый габарит модели.
fn biolab_deep_scale(m: &crate::mobs::Mob, value: glam::Mat4) -> glam::Mat4 {
    let value = match crate::biolab_mutants::genome(m.id) {
        Some(g) => {
            let s = crate::biolab_mutants::scale(&g);
            value * glam::Mat4::from_scale(glam::Vec3::splat(s))
        }
        None => value,
    };
    crate::biolab_surgery::lay_on_table(m, value)
}
