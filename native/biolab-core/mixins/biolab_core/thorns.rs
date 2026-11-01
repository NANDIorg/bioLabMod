// Игрок ударил моба: скальпель оставляет кровоточащую рану, а шипы мутанта
// ранят в ответ и его ядовитая кровь брызжет в лицо.
fn biolab_core_hurt_enemy(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    entity: u32,
) {
    use crate::game::biolab_core as m;
    use horizon_client::mobs::finalize::{Attr, has_modifier};
    if !game.native_on(m::ID) {
        return;
    }
    // Скальпель оставляет рану, которая кровоточит.
    if &*stack.item == m::SCALPEL_ID
        && let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == entity)
    {
        mob.add_effect(m::BLEEDING, m::BLEEDING_TICKS, 0);
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
        mob.knockback(m::player_ram_push(horns), xd, zd);
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
