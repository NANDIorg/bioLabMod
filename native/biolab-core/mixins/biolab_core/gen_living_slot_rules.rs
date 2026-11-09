// Правила слотов меню живых генераторов (M10, GDD 4.27 В): то же правило,
// что у воронки (`living_slot_accepts`), — ткань без электрических органов и
// колба не с глюкозой в слот не ложатся. Хук — свободная функция lib-крейта
// (`menu.rs`), видит `runtime.rs` как `crate::biolab_core` (модуль
// `runtime-menu`); стек тот же, `custom_data` читаем сами. Тип меню
// называется как блок (`data/biolab/horizon/menu/<блок>.json`).
fn biolab_core_living_slot_rules(menu: &str, slot: usize, stack: &crate::items::ItemStack, may_place: &mut bool) {
    use crate::biolab_core as m;
    if m::living_kind(menu).is_none() {
        return;
    }
    let data = |key: &str| -> Option<String> {
        let extra = stack.extra.as_ref()?;
        let (_, tag) = extra.other.iter().find(|(name, _)| name == "minecraft:custom_data")?;
        match tag.get(key) {
            Some(horizon_world::nbt::Tag::String(s)) => Some(s.to_string_lossy()),
            _ => None,
        }
    };
    if let Some(place) = m::living_slot_accepts(menu, slot, &stack.item, &data) {
        *may_place = *may_place && place;
    }
}
