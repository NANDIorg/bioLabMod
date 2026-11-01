// Цвет мутанта: геном красит всё тело цветом первого гена, у которого он есть.
// Красим поверх ванильного цвета, а не вместо: у волка своя тень, у
// тропической рыбы — краситель, и мутант любого вида их не теряет.
fn biolab_deep_tint(m: &crate::mobs::Mob, value: u32) -> u32 {
    match crate::biolab_mutants::genome(m.id).and_then(|g| crate::biolab_mutants::tint_rgb(&g)) {
        Some(rgb) => crate::biolab_mutants::tint_over(value, crate::mobs::render::abgr(rgb)),
        None => value,
    }
}
