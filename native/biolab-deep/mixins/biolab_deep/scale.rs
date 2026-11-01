// Размер мутанта: гены растягивают модель поверх ванильного масштаба.
fn biolab_deep_scale(m: &crate::mobs::Mob, value: glam::Mat4) -> glam::Mat4 {
    match crate::biolab_mutants::genome(m.id) {
        Some(g) => {
            let s = crate::biolab_mutants::scale(&g);
            value * glam::Mat4::from_scale(glam::Vec3::splat(s))
        }
        None => value,
    }
}
