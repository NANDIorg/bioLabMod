// Мобы появились в мире (родились, призваны или загрузились с диска).
// Загруженному мутанту возвращаем наши модификаторы из сырого NBT: ванилла
// пишет их в сейв, но обратно не читает. Новому мобу любого вида без меток
// бросаем геном: шанс и подходящие гены — по таблице видов
// (`biolab_deep::roll_for`). Вид и поведение в lib-крейте сами узнают геном
// по меткам на мобе.
fn biolab_deep_mobs_joined(game: &mut crate::game::Game<'_>, from: usize) {
    use crate::game::biolab_deep as d;
    use horizon_client::mobs::finalize::{Attr, Op, add_modifier, has_modifier};
    use horizon_world::nbt::Tag;
    if !game.native_on(d::ID) {
        return;
    }
    for index in from..game.mobs.len() {
        let id = game.mobs[index].id;

        // 1. Загруженный мутант: что из наших модификаторов лежит в сейве.
        let mut restored: Vec<(Attr, &'static str, f64, Op)> = Vec::new();
        let mut saved_health: Option<f32> = None;
        if let Some(raw) = game.persist.raw.get(&id) {
            if let Some(Tag::Float(bits)) = raw.get("Health") {
                saved_health = Some(f32::from_bits(*bits));
            }
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
        }

        let m = &mut game.mobs[index];
        if !restored.is_empty() {
            for (attr, known, amount, op) in restored {
                add_modifier(m, attr, known, amount, op);
            }
            // Ванилла при загрузке срезала здоровье до базового максимума:
            // возвращаем сохранённое, теперь уже под нашим максимумом.
            if let Some(health) = saved_health {
                m.health = health.min(m.attrs.max_health).max(m.health);
            }
            continue;
        }

        // 2. Моб с диска без наших меток — обычный, и таким остаётся. Бросать
        // ему заново нельзя: id сессионный, и каждый перезаход давал бы стаду
        // у дома новый шанс, пока мутантами не стали бы все.
        if game.persist.raw.contains_key(&id) {
            continue;
        }
        // 3. Новый моб любого вида. Уже помеченного (клон из чана, укол
        // вектором) не трогаем — геном ему задали нарочно.
        let m = &mut game.mobs[index];
        if d::GENE_MARKERS
            .iter()
            .any(|marker| has_modifier(m, Attr::FollowRange, marker))
        {
            continue;
        }
        // Вид так, как его называют хук-поинты (`minecraft:zombie`).
        let kind = ["minecraft:", m.kind.id.trim_start_matches("minecraft:")].concat();
        let genome = d::roll_for(&kind, m.id.wrapping_mul(2_654_435_761));
        if genome.is_empty() {
            continue;
        }
        // Метки с нулевой прибавкой: по ним геном найдут вид, поведение и
        // сейв. Прибавки к атрибутам ставит lib по этим же меткам
        // (`spawn.rs`), поэтому здесь их нет.
        for (gene, strength) in genome {
            if let Some(marker) = d::marker(gene, strength) {
                add_modifier(m, Attr::FollowRange, marker, 0.0, Op::AddValue);
            }
        }
    }
}
