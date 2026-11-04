// Что происходит с мобом каждый тик, перед его эффектами: усыплённый стоит.
//
// Урон и поведение самих эффектов (кислота, токсикоз, кровотечение, потеря
// цели под седацией, мутаген) сюда не входят: их считает `biolab-core` на
// стабильных хуках API 56 (`effect_*.rs`), одной таблицей для игрока и моба.
// Поведение мутанта по генам (огонь, яды, планирование, регенерация) с M11
// тоже не здесь: геном — данные на мобе, их видит только бинарник, и он
// выставляет поведение перед тиком мобов (`mutant_tick.rs` →
// `biolab_bridge::mutant_tick`). Здесь осталось то, на что стабильного API нет
// и для чего данные не нужны: прибавка к атрибуту от эффекта (скорость
// усыплённого).
fn biolab_deep_mob_effects(m: &mut crate::mobs::Mob) {
    use crate::mobs::finalize::{Attr, Op, add_modifier, has_modifier, remove_modifier};
    // Уснувший не ходит: скорость гасится модификатором, пока действует
    // снотворное, и снимается, как только оно кончилось. Цель снимает
    // `biolab-core`; модификатор — прибавка к атрибуту, для неё стабильного
    // API нет, поэтому она здесь.
    // Паралич кураре (GDD 4.20) держит на месте тем же модификатором: моб
    // не спит и не лежит, но и не ходит. Свой модификатор ему не нужен —
    // сон и паралич вместе всё равно дают «стоять».
    const STILL: &str = "biolab:sedation_still";
    if crate::biolab_bridge::sedated(m) || crate::biolab_bridge::paralysed(m) {
        if !has_modifier(m, Attr::MovementSpeed, STILL) {
            add_modifier(m, Attr::MovementSpeed, STILL, -1.0, Op::AddMultipliedTotal);
        }
    } else if has_modifier(m, Attr::MovementSpeed, STILL) {
        remove_modifier(m, Attr::MovementSpeed, STILL);
    }
}
