// Строка мода в Options > Mods.
fn biolab_core_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::biolab_core as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: Vec::new(),
    });
}
