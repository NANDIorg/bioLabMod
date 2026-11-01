// Загруженные с диска мобы: вернуть наши модификаторы из сырого NBT.
//
// Horizon при загрузке модификаторы атрибутов не читает (`mob_from_nbt`
// берёт только базы), а хук «мобы появились» (`native_mobs_joined`) для
// загруженных не вызывается — только для новых. Поэтому раньше мутант после
// перезахода терял геном, метки операционной и отметку броска, а следующий
// сейв затирал их насовсем (проверено в игре: корова со стола встала и ушла).
// Здесь, на тике игрока, каждый моб, у которого есть сырые данные сейва, один
// раз за сессию получает свои модификаторы обратно.
fn biolab_deep_restore_loaded(game: &mut crate::game::Game<'_>) {
    use crate::game::biolab_deep as d;
    use horizon_client::mobs::finalize::{Attr, Op, add_modifier};
    use horizon_world::nbt::Tag;
    if !game.native_on(d::ID) {
        return;
    }
    for index in 0..game.mobs.len() {
        let id = game.mobs[index].id;
        if !d::first_restore(id) {
            continue;
        }
        let Some(raw) = game.persist.raw.get(&id) else {
            continue;
        };
        let mut restored: Vec<(Attr, &'static str, f64, Op)> = Vec::new();
        let saved_health = match raw.get("Health") {
            Some(Tag::Float(bits)) => Some(f32::from_bits(*bits)),
            _ => None,
        };
        if let Some(Tag::List(attrs)) = raw.get("attributes") {
            for entry in attrs {
                let Some(Tag::String(name)) = entry.get("id") else {
                    continue;
                };
                let name = name.to_string_lossy();
                let Some(attr) = Attr::ALL
                    .into_iter()
                    .find(|a| format!("minecraft:{}", a.name()) == name)
                else {
                    continue;
                };
                let Some(Tag::List(mods)) = entry.get("modifiers") else {
                    continue;
                };
                for md in mods {
                    let Some(Tag::String(mid)) = md.get("id") else {
                        continue;
                    };
                    let Some(known) = d::our_modifier(&mid.to_string_lossy()) else {
                        continue;
                    };
                    let Some(Tag::Double(bits)) = md.get("amount") else {
                        continue;
                    };
                    let op = match md.get("operation") {
                        Some(Tag::String(o)) => match o.to_string_lossy().as_str() {
                            "add_value" => Op::AddValue,
                            "add_multiplied_base" => Op::AddMultipliedBase,
                            "add_multiplied_total" => Op::AddMultipliedTotal,
                            _ => continue,
                        },
                        _ => continue,
                    };
                    restored.push((attr, known, f64::from_bits(*bits), op));
                }
            }
        }
        if restored.is_empty() {
            continue;
        }
        let m = &mut game.mobs[index];
        for (attr, known, amount, op) in restored {
            add_modifier(m, attr, known, amount, op);
        }
        // Загрузка срезала здоровье до базового максимума: возвращаем
        // сохранённое, теперь уже под нашим максимумом.
        if let Some(health) = saved_health {
            m.health = health.min(m.attrs.max_health).max(m.health);
        }
    }
}
