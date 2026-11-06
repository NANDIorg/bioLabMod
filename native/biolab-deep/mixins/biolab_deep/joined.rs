// Новые мобы в мире (родились, призваны, вышли из чана): бросок генома.
//
// Геном — данные на мобе (`biolab:genome`, M11), а не метки. Шанс и
// подходящие гены — по таблице видов (`biolab_deep::roll_for`). Бросок
// делается один раз за жизнь моба: отметка `biolab:mutation_checked` уходит
// в сейв вместе с ним, и перезаход не даёт стаду у дома новый шанс.
// Загруженных с диска этот хук не видит (их принимает `loaded.rs`).
//
// Призванный командой `/summon` с NBT старого формата (метки на атрибуте)
// сначала проходит миграцию: метки — в данные. Раньше такие метки
// возвращались, только если у них была сумма-дробь (`amount:0.0d`) и
// операция строкой: `amount:0` из команды — целое, и моб молча оставался
// обычным; по меткам он «был» мутантом (их показывал `data get`), а наростов
// не получал до первого укола. Миграция узнаёт метки по имени.
// Моб с уже записанными данными (клон чана, пациент стола, призыв с
// `horizon:attachments` и UUID) не бросает: геном ему задали нарочно — те,
// кто его создаёт, всё равно перепишут данные сразу после появления.
// Прибавки к атрибутам выставит проход мутантов (`mutant_tick.rs`).
// Внутренние поля `Game` — не хук-поинты: при обновлении Horizon проверить.
fn biolab_deep_mobs_joined(game: &mut crate::game::Game<'_>, from: usize) {
    use crate::game::biolab_deep as d;
    use crate::game::biolab_deep_data as data;
    if !game.native_on(d::ID) {
        return;
    }
    // Бросает только сервер: у клиента данные — копия, и бросок там ничего
    // бы не значил.
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    let fresh: Vec<(u32, String)> = game
        .mobs
        .iter()
        .skip(from)
        .map(|m| {
            // Вид так, как его называют хук-поинты (`minecraft:zombie`); свой
            // вид мода (`biolab:infected`) — со своим пространством имён.
            (m.id, d::hook_kind(&m.kind.id))
        })
        .collect();
    for (id, kind) in fresh {
        data::migrate(game, id);
        if data::checked(game, id) || !data::genome(game, id).is_empty() {
            continue;
        }
        data::set_checked(game, id);
        let genome = d::roll_for(&kind, id.wrapping_mul(2_654_435_761));
        if !genome.is_empty() {
            data::set_genome(game, id, &d::genome_text(&genome));
        }
    }
}
