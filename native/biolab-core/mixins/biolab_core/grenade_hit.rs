// Газовая граната разбилась (GDD 4.20, M5). Летит она копией снежка — тип
// сущности мода `data/biolab/horizon/entity_type/grenade_<газ>.json`, — и
// игра сама её бросает и разбивает крошками предмета. Здесь — облако газа
// радиусом 3 на 5 секунд в точке удара, хоть в моба, хоть в блок; дальше им
// занимается тик (`sleep_tick.rs`). Отдельная инъекция на тот же
// `native_projectile_impact`, что у дротика (`dart_hit.rs`), — тот не
// трогаем. Попадание не отменяем: снежок с уроном 0 разбивается сам.
// Стабильный API 136; слушать попадания включает `dart_hears.rs`.
fn biolab_core_grenade_hit(
    game: &mut crate::game::Game<'_>,
    dimension: &str,
    kind: &str,
    at: [f64; 3],
    canceled: &mut bool,
) {
    use crate::game::biolab_core as m;
    if *canceled || !game.native_on(m::ID) {
        return;
    }
    let Some(sleeper) = m::grenade_projectile(kind) else {
        return;
    };
    crate::game::biolab_sleep::release(
        game,
        dimension,
        sleeper,
        at,
        m::GRENADE_RADIUS,
        m::GRENADE_TICKS,
        m::Cue::GrenadeBreak,
    );
}
