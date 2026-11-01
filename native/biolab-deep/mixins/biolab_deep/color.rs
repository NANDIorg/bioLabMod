// Цвет мутанта: геном красит всё тело цветом первого гена, у которого он есть.
fn biolab_deep_tint(m: &crate::mobs::Mob, value: u32) -> u32 {
    match crate::biolab_mutants::genome(m.id).and_then(|g| crate::biolab_mutants::tint_rgb(&g)) {
        Some(rgb) => crate::mobs::render::abgr(rgb),
        None => value,
    }
}
