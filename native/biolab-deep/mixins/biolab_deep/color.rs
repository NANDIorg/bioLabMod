// Цвет мутанта: геном красит всё тело цветом первого гена, у которого он есть.
// Красим поверх ванильного цвета, а не вместо: у волка своя тень, у
// тропической рыбы — краситель, и мутант любого вида их не теряет.
// Модель игрока — не моб: геном мобов по id к ней не относится (у своей
// модели id 0, как у первого моба сервера).
// Свой вид мода (заражённый, GDD 4.26 А) сначала получает оттенок вида,
// мутант — свой поверх него.
fn biolab_deep_tint(m: &crate::mobs::Mob, value: u32) -> u32 {
    if crate::biolab_players::is_player(m) {
        return value;
    }
    let value = match crate::biolab_infected_look::species_tint(&m.kind.id) {
        Some(rgb) => crate::biolab_mutants::tint_over(value, crate::mobs::render::abgr(rgb)),
        None => value,
    };
    match crate::biolab_mutants::genome(m.id).and_then(|g| crate::biolab_mutants::tint_rgb(&g)) {
        Some(rgb) => crate::biolab_mutants::tint_over(value, crate::mobs::render::abgr(rgb)),
        None => value,
    }
}
