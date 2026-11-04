// Раз в кадр (`native_frame`, API 1): то, что хуки экрана сделать не могут.
// Им игра дана только на чтение, поэтому закрыть экран (E, Backspace) и
// щёлкнуть звуком кнопки они просят через `ResearchInput`, а делает это
// здесь, с игрой на запись, — в следующем кадре, задержки не видно.
// Экран закрылся сам (Escape, другой экран поверх) — сбрасываем вид и ввод,
// чтобы следующий журнал открылся на карте, а не с полузажатой кнопкой.
fn biolab_core_research_frame(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    if game.native_mod_screen() != Some(m::ID) {
        if m::research_view().open {
            m::close_research();
        }
        m::with_research_input(|input| *input = m::ResearchInput::default());
        return;
    }
    let (close, click) = m::with_research_input(|input| input.take()).unwrap_or((false, false));
    if click && let Some(p) = game.native_player() {
        // Звук только этому игроку, у его головы: щелчок интерфейса.
        let _ = game.native_play_local_sound("minecraft:ui.button.click", [p.x, p.y + 1.6, p.z], 0.25, 1.0, 0);
    }
    if close {
        game.native_mod_screen_close(m::ID);
        m::close_research();
    }
}
