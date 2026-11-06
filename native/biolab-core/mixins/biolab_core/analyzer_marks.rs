// Анализатор в руке (GDD 4.26 Г): рамки в мире вокруг заражённых и мутантов
// в 16 блоках, над рамкой — короткая подпись, цвет — что с мобом. Только
// клиент, раз в кадр; рисует сама игра (`world_primitives`), без wgpu.
//
// Знаем только то, что окну и так приходит (подробно — `runtime.rs`, раздел
// «M11-Г»): мутант — по геному из данных моба (снимок `analyzer_scan.rs`,
// раз в тик окна), заражённый — по виду: зомби, как в правиле заражения
// близостью (`infection.rs`), и `biolab:infected`. Больных штаммом мобов
// окно не видит: чужих эффектов клиенту не шлют.
//
// Рамка и подпись видны сквозь стены: это прибор, а радиус небольшой.
// Положение моба — живое и сглаженное по доле тика, так что рамка идёт за
// мобом без рывков. `mobs`, `kind`, `previous`, `removed` — поля игры, не
// хук-поинты: при обновлении Horizon проверить.
fn biolab_core_analyzer_marks(
    game: &crate::game::Game<'_>,
    partial: f32,
    primitives: &mut horizon_client::world_primitives::WorldPrimitives,
) {
    use crate::game::biolab_core as m;
    if !game.native_on(m::ID) {
        return;
    }
    // None — анализатора в руке нет (или тик окна ещё не прошёл).
    let Some(scan) = m::shown_analyzer_scan() else {
        return;
    };
    let Some(player) = game.native_player() else {
        return;
    };
    let from = [player.x, player.y, player.z];
    for mob in game.mobs.iter() {
        if !mob.alive() || mob.removed || &*mob.kind.id == m::ANALYZER_PLAYER_KIND {
            continue;
        }
        let feet = m::analyzer_lerp(
            [mob.previous.x, mob.previous.y, mob.previous.z],
            [mob.position.x, mob.position.y, mob.position.z],
            partial,
        );
        if !m::analyzer_reaches(from, feet, 0.0) {
            continue;
        }
        let seen = scan.get(&mob.id);
        let genes: &[(&'static str, i32)] = seen.map(|s| s.genes.as_slice()).unwrap_or(&[]);
        let zombie = mob.species() == horizon_client::mobs::species::Species::Zombie;
        let Some(mark) = m::analyzer_mark(m::analyzer_carrier(&mob.kind.id, zombie), genes) else {
            continue;
        };
        let color = m::analyzer_color(mark);
        let (min, max) = m::analyzer_box(
            feet,
            mob.kind.width,
            mob.kind.height,
            m::body_scale(genes),
            seen.is_some_and(|s| s.lying),
        );
        primitives.box_outline(min, max, color, m::ANALYZER_LINE).through_walls();
        let label = m::analyzer_label(mark, genes, seen.is_some_and(|s| s.asleep));
        primitives
            .text([feet[0], max[1] + m::ANALYZER_TEXT_GAP, feet[2]], label, color, m::ANALYZER_TEXT_SCALE)
            .through_walls()
            .scale_with_distance()
            .background();
    }
}
