// Внешность своих видов мода (GDD 4.26 А). Модуль lib-крейта
// (`crate::biolab_infected_look`): его читает оттенок тела (`color.rs`,
// RETURN на `mobs::render::body_color`), куда доходит только `&Mob`.
//
// Заражённый (`biolab:infected`) — модель и текстура зомби, но тело
// зеленовато-бледное: каналы ванильного цвета умножаются на оттенок
// (`biolab_mutants::tint_over`), так что текстура остаётся, а мутант поверх
// докрашивается своим геном.

/// Оттенок тела своих видов, RGB.
pub const SPECIES_TINTS: [(&str, u32); 1] = [("biolab:infected", 0xB8_E0_B0)];

/// Оттенок тела вида `kind` (`biolab:infected`) в RGB; None — у вида своего
/// оттенка нет.
pub fn species_tint(kind: &str) -> Option<u32> {
    SPECIES_TINTS.iter().find(|(k, _)| *k == kind).map(|(_, rgb)| *rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infected_is_pale_green() {
        let rgb = species_tint("biolab:infected").expect("у заражённого свой оттенок");
        let (r, g, b) = ((rgb >> 16) & 255, (rgb >> 8) & 255, rgb & 255);
        assert!(g > r && g > b, "зеленее: {rgb:06X}");
        assert!(r >= 0x90 && b >= 0x90, "бледный, а не тёмный: {rgb:06X}");
        assert_eq!(species_tint("minecraft:zombie"), None, "ванильный зомби не перекрашивается");
        assert_eq!(species_tint("zombie"), None);
    }
}
