// Запасной путь к семенам (GDD 4.20): ведьма роняет мандрагору, лиса —
// валериану, зомби-житель — белладонну. Ванильные таблицы мобов мод менять не
// может (вне `data/minecraft/tags/` файлы `minecraft` не читаются), поэтому —
// хуком выпадения (API 75): добавляем стопку к тому, что уже выкатилось.
// Только с убитого игроком и с моба со своей таблицей (не детёныш, правило
// `mob_drops` включено) — как редкий ванильный лут. Странствующему торговцу
// товар не добавить: хуки торговли в пине только слушают.
fn biolab_core_plant_drops(
    game: &mut crate::game::Game<'_>,
    context: &horizon_client::mod_api::NativeLootDrops<'_>,
    drops: &mut Vec<horizon_client::items::ItemStack>,
) {
    if !game.native_on(crate::game::biolab_core::ID) || context.used || context.table.is_empty() {
        return;
    }
    let Some((id, kind)) = context.entity else {
        return;
    };
    // Зерно — id моба и тик: бросок повторяется, если повторить смерть.
    let seed = (u64::from(id) << 32) ^ game.ticks;
    if let Some((item, count)) =
        crate::game::biolab_plants::mob_seed_drop(kind, context.player.is_some(), seed)
    {
        drops.push(horizon_client::items::ItemStack {
            item: item.into(),
            count,
            damage: 0,
            extra: None,
        });
    }
}
