// Воронка не кладёт в машину то, с чем машина не работает (GDD 4.15): такой
// предмет остаётся в воронке, и она пробует следующий слот, а вход машины
// не забивается чужим. Слот хук не называет, только блоки, — поэтому вопрос
// про машину целиком (`hopper_accepts`); в какой слот ляжет, решают стороны
// контейнера (`machine_faces`). Из машины воронка забирает как раньше:
// источник здесь не проверяем.
fn biolab_core_hopper_filter(
    game: &crate::game::Game<'_>,
    destination: [i32; 3],
    item: &str,
    cancel: &mut bool,
) {
    use crate::game::biolab_core as m;
    if *cancel || !game.native_on(m::ID) {
        return;
    }
    let block = game.native_block(destination);
    if let Some(machine) = m::machine(&block)
        && !m::hopper_accepts(machine, item)
    {
        *cancel = true;
    }
}
