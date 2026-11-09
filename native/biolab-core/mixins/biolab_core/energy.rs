// Буфер энергии блоков мода (M10, GDD 4.27 п. 4): `Capabilities.Energy.BLOCK`.
// Платформа спрашивает раз на блок при постройке таблицы тикающих; запас —
// целое `energy` в сейве блок-сущности. Ёмкость и пределы — `energy_spec`
// (все таблицы агентов). Ввод и вывод — через любую грань.
fn biolab_core_block_entity_energy(
    block: &str,
    energy: &mut Option<horizon_client::containers::ModEnergyBuffer>,
) {
    use crate::game::biolab_core as m;
    if energy.is_some() {
        return;
    }
    if let Some(spec) = m::energy_spec(block) {
        *energy = Some(horizon_client::containers::ModEnergyBuffer {
            capacity: spec.capacity,
            max_insert: spec.max_in,
            max_extract: spec.max_out,
            insert_faces: [true; 6],
            extract_faces: [true; 6],
        });
    }
}
