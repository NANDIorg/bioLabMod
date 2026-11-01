// После сборки атласа HUD добавляем псевдонимы иконок эффектов модов.
fn biolab_deep_atlas_icons(value: crate::Result<crate::hud::HudAtlas>) -> crate::Result<crate::hud::HudAtlas> {
    let mut value = value;
    if let Ok(atlas) = value.as_mut() {
        crate::biolab_icons::alias_effect_icons(atlas);
    }
    value
}
