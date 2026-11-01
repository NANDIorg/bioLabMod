// Иконки эффектов модов в атласе HUD.
//
// Движок рисует иконку эффекта спрайтом `mob_effect/<id>`, а у эффекта мода id
// всегда с пространством имён, то есть ключ выходит `mob_effect/biolab:acid_burn`.
// В атлас же из модов попадают только `textures/gui/...`, под ключом
// `biolab:gui/mob_effect/acid_burn` — порядок частей другой, поэтому ванильный
// поиск спрайта промахивается и рамка эффекта рисуется пустой.
//
// Чинится это одной строкой данных, а не отрисовкой поверх: после сборки атласа
// мы добавляем псевдоним с тем ключом, который ищет движок. Он указывает на тот
// же прямоугольник, поэтому иконка появляется везде сразу — и в HUD, и в
// инвентаре, — и работает для любого мода, а не только для нашего.
//
// Этот файл подключается как модуль `crate::biolab_icons` крейта
// horizon-client-render.

/// Путь, по которому мод кладёт иконку эффекта внутри своего пространства имён.
pub const ICON_PREFIX: &str = "gui/mob_effect/";

/// Ключ, под которым движок ищет иконку эффекта `<ns>:<path>`.
pub fn alias_for(name: &str) -> Option<String> {
    let (namespace, rest) = name.split_once(':')?;
    let path = rest.strip_prefix(ICON_PREFIX)?;
    (!namespace.is_empty() && !path.is_empty())
        .then(|| format!("mob_effect/{namespace}:{path}"))
}

/// Добавить в атлас псевдонимы иконок эффектов всех модов.
/// Существующие ключи не трогаем: ванильная иконка всегда главнее.
#[cfg(not(test))]
pub fn alias_effect_icons(atlas: &mut crate::hud::HudAtlas) {
    let aliases: Vec<(String, crate::hud::Rect)> = atlas
        .sprites
        .iter()
        .filter_map(|(name, rect)| Some((alias_for(name)?, *rect)))
        .collect();
    for (name, rect) in aliases {
        atlas.sprites.entry(name).or_insert(rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_key_the_engine_looks_for() {
        assert_eq!(
            alias_for("biolab:gui/mob_effect/acid_burn").as_deref(),
            Some("mob_effect/biolab:acid_burn")
        );
    }

    #[test]
    fn ignores_other_sprites() {
        assert_eq!(alias_for("mob_effect/poison"), None);
        assert_eq!(alias_for("biolab:gui/container/machine"), None);
        assert_eq!(alias_for("biolab:gui/mob_effect/"), None);
    }
}
