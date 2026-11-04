// Игрок ударил моба: скальпель оставляет кровоточащую рану, а шипы мутанта
// ранят в ответ и его ядовитая кровь брызжет в лицо.
fn biolab_core_hurt_enemy(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    entity: u32,
) {
    use crate::game::biolab_core as m;
    use crate::game::biolab_particles as fx;
    if !game.native_on(m::ID) {
        return;
    }
    // Скальпель оставляет рану, которая кровоточит, и кровь брызжет сразу
    // (`native_send_particles`, API 103; шлёт сервер, как и звук). Дальше
    // капает кровотечение (`ambient.rs`).
    let mut wound = None;
    if &*stack.item == m::SCALPEL_ID
        && let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == entity)
    {
        mob.add_effect(m::BLEEDING, m::BLEEDING_TICKS, 0);
        let feet = [mob.position.x, mob.position.y, mob.position.z];
        wound = Some(fx::over_body(&fx::CUT, feet, f64::from(mob.height())));
    }
    if let Some(at) = wound
        && (game.serving.is_some() || game.options.dedicated.is_some())
    {
        let seed = u64::from(entity) ^ game.ticks;
        let options = horizon_client::particles::ParticleOptions::named(fx::CUT.name);
        let _ = game.with_level(|_, level, _| {
            for s in fx::sends(&fx::CUT, at, seed) {
                let _ = level.native_send_particles(&options, None, s.at, s.spread, s.speed, s.count);
            }
        });
    }
    // Рогатость у игрока (GDD 4.19): удар отбрасывает моба сильнее. Толчок
    // поверх ванильного: `knockback` толкает против направления (`xd`, `zd`),
    // а оно — от моба к игроку, то есть моб летит прочь.
    let world = game.world_root.clone();
    let horns = m::world_player_gene(&world, game.native_acting_id(), "horns");
    if horns > 0
        && let Some(player) = game.native_player()
        && let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == entity && mob.alive())
    {
        let (xd, zd) = (player.x - mob.position.x, player.z - mob.position.z);
        // Рогами бьёт сам игрок: источником идёт его удар.
        let source = crate::mobs::Damage::player([player.x, player.y, player.z]);
        mob.knockback(m::player_ram_push(horns), xd, zd, &source);
    }
    if !game.mobs.iter().any(|mob| mob.id == entity) {
        return;
    }
    // Геном — данные на мобе (M11).
    let genes = crate::game::biolab_core_data::genes(game, entity);
    if genes.is_empty() {
        return;
    }
    let thorns = m::behaviour_strength(&genes, "thorns");
    if thorns > 0 {
        game.apply_effect_action(horizon_client::effects::EffectAction::Hurt(
            "minecraft:magic",
            m::thorns_damage(thorns),
        ));
    }
    if m::behaviour_strength(&genes, "toxic_blood") > 0 {
        game.add_player_effect(horizon_client::effects::EffectInstance::new(
            "poison", 80, 0,
        ));
    }
}
