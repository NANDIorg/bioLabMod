// Своя вкладка в творческом инвентаре: все предметы и блоки мода в одном
// месте. Перестройка вкладок — приватная функция `Creative::rebuild`, после
// неё список готов, и туда дописывается наша.
fn biolab_deep_creative_tabs(creative: &mut crate::creative::Creative, value: ()) {
    use crate::biolab_tab as tab;
    // Вкладки перестраиваются при смене прав; свою добавляем заново каждый
    // раз, но только если её там ещё нет.
    if creative.tabs.iter().any(|t| t.id == tab::ID) {
        return value;
    }
    // Список берём из своей таблицы: в ванильные вкладки предметы мода не
    // попадают (проверено в игре — вкладка выходила пустой).
    let mut ids: Vec<&str> = tab::ITEMS.to_vec();
    ids.sort_by_key(|id| tab::order(id));
    let items: Vec<crate::items::ItemStack> = ids
        .iter()
        .map(|id| crate::items::ItemStack::new(id, 1))
        .collect();
    if items.is_empty() {
        return value;
    }
    let icon = tab::ICON.to_string();
    creative.tabs.push(crate::creative::Tab {
        id: tab::ID.to_string(),
        title: tab::TITLE.to_string(),
        icon,
        items,
        top: false,
        // Шестая колонка нижнего ряда занята «инвентарём выживания», и
        // вкладка пряталась под ним: встаём пятой, рядом с яйцами призыва.
        column: 5,
        search: false,
        inventory: false,
        hotbar: false,
        // Страница вкладок: ванильные на нулевой, туда же встаём и мы.
        page: 0,
    });
    creative.forget_items();
    value
}
