// Тик игрока выполняет то, что наши эффекты записали в очередь
// (`effect_apply.rs`): там игра была только для чтения, здесь — изменяема,
// моб на месте, а игрок — тот, кто сейчас тикает. Урон и действия — те же,
// что раньше считали миксины `biolab-deep` (`tick.rs`, `mob_tick.rs`).
//
// Почему тик игрока: `native_level_tick_post` в одиночной игре не вызывается
// (GDD 3.8), а тик игрока идёт на сервере каждый тик, на уровне этого
// игрока, сразу после тика его эффектов (`tick_guests`: `guest_server_tick`,
// потом `native_player_tick`). Урон игроку приходит в тот же тик, мобам — не
// позже следующего; ритм от этого не меняется.
fn biolab_core_effect_actions(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_effects as fx;
    if !game.native_on(m::ID) || game.remote.is_some() {
        return;
    }
    let due = match fx::state().lock() {
        Ok(mut state) => state.take(game.ticks, game.lan_acting_uuid, game.dim.id()),
        Err(_) => return,
    };
    // Урон игроку — путём урона эффекта, как раньше в `tick.rs`: броня и
    // кадры неуязвимости работают как у ванильного яда. Тип — свой у каждого
    // эффекта (`fx::damage_of`, GDD 4.26, Б).
    for (amount, damage) in due.hurts {
        crate::game::biolab_damage::player(game, damage, amount);
    }
    for (id, act) in due.mobs {
        // Мутаген пишет геном в данные моба: для этого нужна вся игра, а не
        // только моб, поэтому этот случай — до выбора моба.
        if let fx::Act::Mutate = act {
            let Some((mob_id, age)) = game
                .mobs
                .iter()
                .find(|mob| mob.id == id && !mob.removed && mob.alive())
                .map(|mob| (mob.id, mob.age))
            else {
                continue;
            };
            // Сила генов по порядку `GENES` — так их ждёт выбор гена.
            let have = crate::game::biolab_core_data::genes(game, mob_id);
            let mut strengths = vec![0u8; m::GENES.len()];
            for (name, strength) in &have {
                if let Some(i) = m::GENES.iter().position(|g| g.name == *name) {
                    strengths[i] = (*strength).clamp(0, 3) as u8;
                }
            }
            let seed = fx::mutation_seed(mob_id, u64::from(age));
            let Some((index, strength)) = fx::mutation(&strengths, seed) else {
                continue;
            };
            let Some(kind) = m::GENES.get(index) else {
                continue;
            };
            // Ген добавляется или усиливается: строка данных хранит по гену
            // одну, сильнейшую силу. Прибавки выставит `biolab-deep`.
            let mut genes = have;
            genes.push((kind.name, strength));
            crate::game::biolab_core_data::set_genes(game, mob_id, &genes);
            continue;
        }
        let Some(mob) = game
            .mobs
            .iter_mut()
            .find(|mob| mob.id == id && !mob.removed && mob.alive())
        else {
            continue;
        };
        match act {
            fx::Act::Hurt(amount, damage) => {
                mob.damage(amount, crate::game::biolab_damage::mob(damage));
            }
            // Седативное: моб забывает, кого преследовал. Стоять на месте его
            // заставляет модификатор скорости — он в `biolab-deep`
            // (`mob_tick.rs`): у прибавок к атрибутам стабильного API нет.
            fx::Act::Calm => mob.ai.target = None,
            // Мутаген разобран выше: он пишет геном в данные моба.
            fx::Act::Mutate => {}
        }
    }
}
