// Экран столба и котла (M10, GDD 4.27 п. 11): та же лабораторная панель, что
// у машин (`screen_draw.rs`), рамки слотов, заголовок с лампой работы, прибор
// генератора, шкала запаса и строка статуса. Числа — данные меню
// (`EARLY_DATA_*`, их пишет тик `gen_early.rs`), раскладка — `early_screen`.
// Вторая инъекция на `native_screen_draw`: чужие блоки — сразу выход.
fn biolab_core_gen_screen_draw(
    game: &crate::game::Game<'_>,
    block: &str,
    data: &[i32],
    batch: &mut horizon_client::hud::HudBatch,
    origin: [f32; 2],
) {
    use crate::game::biolab_core as m;
    let Some(layout) = m::early_screen(block) else {
        return;
    };
    let at = |r: [f32; 4]| [origin[0] + r[0], origin[1] + r[1], r[2], r[3]];
    let fill = |batch: &mut horizon_client::hud::HudBatch, r: [f32; 4], argb: u32| {
        if r[2] > 0.0 && r[3] > 0.0 {
            game.native_fill(&mut *batch, at(r), argb);
        }
    };
    let text = |batch: &mut horizon_client::hud::HudBatch, s: &str, p: [f32; 2], rgb: u32, shadow: bool| {
        game.native_text(&mut *batch, s, [origin[0] + p[0], origin[1] + p[1]], rgb, shadow);
    };
    let get = |i: usize| data.get(i).copied().unwrap_or(0);
    let (w, h) = (layout.width, layout.height);
    let working = get(m::EARLY_DATA_LIT) > 0;

    // Панель: спрайт машин, без него — заливки тех же цветов.
    if !game.native_sprite(batch, "biolab:gui/machine/panel", [0.0, 0.0, w, h], at([0.0, 0.0, w, h])) {
        fill(&mut *batch, [0.0, 0.0, w, h], m::SCREEN_EDGE);
        fill(&mut *batch, [1.0, 1.0, w - 2.0, h - 2.0], m::SCREEN_STEEL);
        fill(&mut *batch, [1.0, 1.0, w - 2.0, 1.0], m::SCREEN_STEEL_LIGHT);
        fill(&mut *batch, [1.0, 1.0, 1.0, h - 2.0], m::SCREEN_STEEL_LIGHT);
        fill(&mut *batch, [1.0, h - 2.0, w - 2.0, 1.0], m::SCREEN_STEEL_DARK);
        fill(&mut *batch, [w - 2.0, 1.0, 1.0, h - 2.0], m::SCREEN_STEEL_DARK);
        fill(&mut *batch, [4.0, m::SCREEN_HEADER - 1.0, w - 8.0, 1.0], m::SCREEN_TEAL_DIM);
        fill(&mut *batch, [4.0, m::SCREEN_HEADER - 1.0, 24.0, 1.0], m::SCREEN_TEAL);
    }

    // Рамки слотов: свои и 36 инвентаря; выход (пустая тара) — бирюзовый.
    let mut frames: Vec<([f32; 4], bool)> = Vec::new();
    for (i, slot) in layout.slots.iter().enumerate() {
        if let Some(slot) = slot {
            frames.push((m::slot_frame(*slot), m::early_is_output(block, i)));
        }
    }
    for slot in m::inventory_slots(&layout) {
        frames.push((m::slot_frame(slot), false));
    }
    for (r, output) in frames {
        let sprite = if output { "biolab:gui/machine/slot_out" } else { "biolab:gui/machine/slot" };
        if !game.native_sprite(batch, sprite, [0.0, 0.0, 18.0, 18.0], at(r)) {
            let [x, y, _, _] = r;
            fill(&mut *batch, [x, y, 18.0, 18.0], m::SCREEN_STEEL_LIGHT);
            fill(&mut *batch, [x, y, 17.0, 17.0], m::SCREEN_EDGE);
            fill(&mut *batch, [x + 1.0, y + 1.0, 16.0, 16.0], if output { 0xFF_1D3A38 } else { m::SCREEN_SLOT });
        }
    }

    // Заголовок: иконка, название, лампа работы.
    let _ = game.native_item(batch, block, [origin[0] + 5.0, origin[1] + 3.0], 12.0);
    let title = game.native_screen_title().unwrap_or_else(|| m::early_title(block).to_string());
    text(&mut *batch, &title, [20.0, 5.0], m::SCREEN_TEXT, true);
    fill(&mut *batch, [w - 12.0, 5.0, 6.0, 6.0], m::SCREEN_EDGE);
    fill(&mut *batch, [w - 11.0, 6.0, 4.0, 4.0], if working { m::SCREEN_GREEN } else { m::SCREEN_STEEL_DARK });

    // Прибор: стекло, внутри две вертикальные шкалы — «топливо» и «бак».
    let g = layout.gauge;
    fill(&mut *batch, g, m::SCREEN_EDGE);
    fill(&mut *batch, [g[0] + 1.0, g[1] + 1.0, g[2] - 2.0, g[3] - 2.0], m::SCREEN_GLASS);
    let column = |batch: &mut horizon_client::hud::HudBatch, r: [f32; 4], share: f32, argb: u32| {
        fill(&mut *batch, r, m::SCREEN_STEEL_DARK);
        let level = ((r[3] - 2.0) * share.clamp(0.0, 1.0)).floor();
        fill(&mut *batch, [r[0] + 1.0, r[1] + r[3] - 1.0 - level, r[2] - 2.0, level], argb);
    };
    let fuel = m::early_share(data, m::EARLY_DATA_FUEL, m::EARLY_DATA_FUEL_TOTAL);
    let tank = m::early_share(data, m::EARLY_DATA_TANK, m::EARLY_DATA_TANK_TOTAL);
    let inner = [g[0] + 4.0, g[1] + 12.0, 18.0, g[3] - 16.0];
    let second = [g[0] + g[2] - 22.0, g[1] + 12.0, 18.0, g[3] - 16.0];
    if block == m::VOLTAIC_PILE {
        // Столб: пластины тают (цинк и медь полосами), рядом — кислота.
        text(&mut *batch, "Пл", [inner[0] + 2.0, g[1] + 3.0], m::SCREEN_TEXT_DIM, false);
        text(&mut *batch, "Кисл", [second[0] - 2.0, g[1] + 3.0], m::SCREEN_TEXT_DIM, false);
        fill(&mut *batch, inner, m::SCREEN_STEEL_DARK);
        let level = ((inner[3] - 2.0) * fuel).floor() as i32;
        for k in 0..level {
            let colour = if (k / 2) % 2 == 0 { 0xFF_A8B0B6 } else { 0xFF_C06C40 };
            fill(&mut *batch, [inner[0] + 1.0, inner[1] + inner[3] - 2.0 - k as f32, inner[2] - 2.0, 1.0], colour);
        }
        column(&mut *batch, second, tank, 0xFF_C8E86A);
    } else {
        // Котёл: топка (горение) и бак с водой.
        text(&mut *batch, "Огонь", [inner[0] - 2.0, g[1] + 3.0], m::SCREEN_TEXT_DIM, false);
        text(&mut *batch, "Вода", [second[0], g[1] + 3.0], m::SCREEN_TEXT_DIM, false);
        column(&mut *batch, inner, fuel, if working { 0xFF_FF9A28 } else { 0xFF_8A5A34 });
        column(&mut *batch, second, tank, 0xFF_4A8AE0);
    }

    // Шкала запаса справа и число над строкой статуса.
    let p = layout.progress;
    column(&mut *batch, p, m::early_share(data, m::EARLY_DATA_ENERGY, m::EARLY_DATA_CAPACITY), m::SCREEN_TEAL);
    let [ix, iy] = layout.inventory;
    let charge = format!("{} Вт·с", m::early_screen_charge(data));
    text(&mut *batch, &charge, [ix, iy - 22.0], m::SCREEN_TEXT, false);
    let status_colour = if get(m::EARLY_DATA_STATUS) == m::EARLY_WORKING { m::SCREEN_TEXT } else { 0xE0_A040 };
    text(&mut *batch, &m::early_screen_line(data), [ix, iy - 12.0], status_colour, false);
}
