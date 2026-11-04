// Мобы чанка загрузились с диска (`native_entities_load`, API 210): данные
// мутанта приходят вместе с мобом сами (кодек), а здесь — то, чего данные не
// делают.
// * Миграция мира прошлой версии: старые метки из сырого NBT (`persist.raw`)
//   — в данные, метки — прочь (`biolab_deep_data::migrate`).
// * Прибавки к атрибутам: Horizon при загрузке модификаторы не читает
//   (`mob_from_nbt` берёт только базы), поэтому ставим их по геному заново и
//   возвращаем здоровье из сейва — загрузка срезала его до базового максимума.
// Раньше то же самое делал `restore.rs` на каждом тике игрока: стабильного
// хука на загрузку тогда не было. `native_entity_join_level` с признаком
// «загружен из сейва» для нас не годится: игра зовёт его, только когда
// загружен трейт-мод (`native_joins_flush`), а наши бандлы — миксины.
// Если носителя данных в этот момент ещё нет, моба доберёт перебор прохода
// мутантов (`mutant_tick.rs`), только уже без здоровья из сейва.
// Внутренние поля `Game` (`mobs`, `persist.raw`) — не хук-поинты: при
// обновлении Horizon проверить. Атрибутов у помощника быть не может (так
// требует инструмент миксинов).
fn biolab_deep_entities_loaded(game: &mut crate::game::Game<'_>, chunk: [i32; 2]) {
    use crate::game::biolab_deep as d;
    use crate::game::biolab_deep_data as data;
    if !game.native_on(d::ID) {
        return;
    }
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let ids: Vec<u32> = game
        .mobs
        .iter()
        .filter(|m| {
            !m.removed
                && (m.position.x.floor() as i32) >> 4 == chunk[0]
                && (m.position.z.floor() as i32) >> 4 == chunk[1]
        })
        .map(|m| m.id)
        .collect();
    for id in ids {
        data::migrate(game, id);
        let genes = d::strengths(&d::genome_from_text(&data::genome(game, id)));
        if genes.is_empty() {
            continue;
        }
        let saved = data::saved_health(game, id);
        let Some(m) = game.mobs.iter_mut().find(|m| m.id == id) else {
            continue;
        };
        horizon_client::biolab_bridge::apply_genome(m, &genes, false);
        if let Some(health) = saved {
            m.health = health.min(m.attrs.max_health).max(m.health);
        }
    }
}
