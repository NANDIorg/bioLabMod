// Мутанта убили: с него достаётся мутантная ткань, а нестабильный геном
// оставляет после себя лужу кислоты.
fn biolab_core_mob_killed(game: &mut crate::game::Game<'_>, entity: u32) {
    use crate::game::biolab_core as m;
    use horizon_client::mobs::finalize::{Attr, has_modifier};
    /// Звук события мода в точке `at` всем игрокам рядом. Звук встаёт в
    /// очередь уровня (`native_play_sound`, API 25), и сервер рассылает его
    /// пакетом — в одиночной игре тоже: окно подключено к своему серверу как
    /// игрок. Без своего сервера (клиент) `native_play_sound` звук не берёт,
    /// поэтому там выходим сразу. `with_level` — внутренний API игры, не
    /// хук-поинт: при обновлении Horizon проверить.
    fn play_at(
        game: &mut crate::game::Game<'_>,
        cue: crate::game::biolab_core::Cue,
        at: [f64; 3],
    ) {
        let Some(sound) = crate::game::biolab_core::sound(cue) else {
            return;
        };
        if game.serving.is_none() && game.options.dedicated.is_none() {
            return;
        }
        let _ = game.with_level(|_, level, _| {
            level.native_play_sound(sound.event, at, sound.source, sound.volume, sound.pitch)
        });
    }

    if !game.native_on(m::ID) {
        return;
    }
    let Some(mob) = game.mobs.iter().find(|mob| mob.id == entity) else {
        return;
    };
    let genes: Vec<(&'static str, i32)> = m::GENE_MARKERS
        .iter()
        .filter(|marker| has_modifier(mob, Attr::FollowRange, marker))
        .filter_map(|marker| m::gene_of_marker(marker))
        .collect();
    if genes.is_empty() {
        return;
    }
    let spot = [
        mob.position.x.floor() as i32,
        mob.position.y.floor() as i32,
        mob.position.z.floor() as i32,
    ];
    let volatile = m::behaviour_strength(&genes, "volatile") > 0;
    let necrosis = m::behaviour_strength(&genes, "necrosis") > 0;
    let count = m::tissue_drop(genes.len());

    // Ткань кладём прямо в рюкзак убийце. `native_give` здесь не годится:
    // он работает только в творческом режиме (проверено в игре — в выживании
    // ткань пропадала), а предметную сущность хуки создавать не умеют.
    let mut left = count;
    for slot in 0..36 {
        if left == 0 {
            break;
        }
        let Some(place) = game.native_carried_slot(slot) else {
            continue;
        };
        match place {
            Some(stack) if &*stack.item == m::TISSUE_ID && stack.count < 64 => {
                let add = left.min(64 - stack.count);
                stack.count += add;
                left -= add;
            }
            None => {
                *place = Some(horizon_client::items::ItemStack {
                    item: m::TISSUE_ID.into(),
                    count: left,
                    damage: 0,
                    extra: None,
                });
                left = 0;
            }
            _ => {}
        }
    }
    game.native_chat(&m::tissue_message(count - left, &genes));
    // Срезанная ткань слышна там, где лежит мутант.
    if count > left {
        let at = [
            f64::from(spot[0]) + 0.5,
            f64::from(spot[1]) + 0.5,
            f64::from(spot[2]) + 0.5,
        ];
        play_at(game, m::Cue::Tissue, at);
    }

    // Нестабильный геном оставляет кислоту, некроз — заразную слизь.
    if volatile || necrosis {
        let fluid = if volatile { m::FLUIDS[0].block } else { m::FLUIDS[2].block };
        let dimension = game.dim.id().to_string();
        if let Some(state) = game.pack.assets.registry.default_state(fluid) {
            game.native_set_block(
                &dimension,
                spot,
                state,
                horizon_world::level::flags::UPDATE_ALL,
            );
            game.native_set_block_property(spot, "layers", &m::FLUID_LAYERS.to_string());
        }
    }
}
