// Экран живого генератора (M10, GDD 4.27 В): лабораторная панель, рамки
// слотов, заголовок с иконкой и лампой, слева — шкала порции (сколько ещё
// горит органика, колба или ткань), посередине табло (состояние, ток, заряд,
// ресурс культуры или водорослей), справа — шкала заряда буфера. Числа — из
// слотов данных (`living_reading`), строки — `living_board`; здесь только
// кисть. Вторая инъекция на `native_screen_draw` после `screen_draw.rs` (тот
// знает только машины и на наших блоках ничего не рисует).
fn biolab_core_living_draw(
    game: &crate::game::Game<'_>,
    block: &str,
    data: &[i32],
    batch: &mut horizon_client::hud::HudBatch,
    origin: [f32; 2],
) {
    use crate::game::biolab_core as m;

    let Some(kind) = m::living_kind(block) else {
        return;
    };
    let Some(layout) = m::living_screen(block) else {
        return;
    };
    let at = |r: [f32; 4]| [origin[0] + r[0], origin[1] + r[1], r[2], r[3]];
    let fill = |batch: &mut horizon_client::hud::HudBatch, r: [f32; 4], argb: u32| {
        if r[2] > 0.0 && r[3] > 0.0 {
            game.native_fill(batch, at(r), argb);
        }
    };
    let text = |batch: &mut horizon_client::hud::HudBatch, s: &str, p: [f32; 2], rgb: u32, shadow: bool| {
        game.native_text(batch, s, [origin[0] + p[0], origin[1] + p[1]], rgb, shadow);
    };
    // Строка, обрезанная по ширине с многоточием.
    let fit = |s: &str, width: f32| -> String {
        if game.native_text_width(s) <= width {
            return s.to_string();
        }
        let mut out = s.to_string();
        while !out.is_empty() && game.native_text_width(&format!("{out}…")) > width {
            out.pop();
        }
        format!("{}…", out.trim_end())
    };
    // Полоса, заполненная на долю `f`: вертикальная — снизу вверх.
    let bar = |batch: &mut horizon_client::hud::HudBatch, r: [f32; 4], f: f32, argb: u32| {
        let [x, y, w, h] = r;
        fill(batch, r, m::SCREEN_EDGE);
        fill(batch, [x + 1.0, y + 1.0, w - 2.0, h - 2.0], m::SCREEN_STEEL_DARK);
        let lh = ((h - 2.0) * f.clamp(0.0, 1.0)).floor();
        fill(batch, [x + 1.0, y + h - 1.0 - lh, w - 2.0, lh], argb);
    };

    let r = m::living_reading(data);
    let (w, h) = (layout.width, layout.height);

    // Панель: сталь с фаской, бирюзовая подсветка под заголовком.
    let panel = "biolab:gui/machine/panel";
    if !game.native_sprite(batch, panel, [0.0, 0.0, w, h], at([0.0, 0.0, w, h])) {
        fill(batch, [0.0, 0.0, w, h], m::SCREEN_EDGE);
        fill(batch, [1.0, 1.0, w - 2.0, h - 2.0], m::SCREEN_STEEL);
        fill(batch, [1.0, 1.0, w - 2.0, 1.0], m::SCREEN_STEEL_LIGHT);
        fill(batch, [1.0, 1.0, 1.0, h - 2.0], m::SCREEN_STEEL_LIGHT);
        fill(batch, [1.0, h - 2.0, w - 2.0, 1.0], m::SCREEN_STEEL_DARK);
        fill(batch, [w - 2.0, 1.0, 1.0, h - 2.0], m::SCREEN_STEEL_DARK);
        fill(batch, [4.0, m::SCREEN_HEADER - 1.0, w - 8.0, 1.0], m::SCREEN_TEAL_DIM);
        fill(batch, [4.0, m::SCREEN_HEADER - 1.0, 24.0, 1.0], m::SCREEN_TEAL);
    }

    // Рамки слотов генератора и 36 слотов инвентаря.
    let mut frames: Vec<[f32; 4]> = layout.slots.iter().flatten().map(|p| m::slot_frame(*p)).collect();
    frames.extend(m::inventory_slots(&layout).into_iter().map(m::slot_frame));
    for f in frames {
        if !game.native_sprite(batch, "biolab:gui/machine/slot", [0.0, 0.0, 18.0, 18.0], at(f)) {
            let [x, y, _, _] = f;
            fill(batch, [x, y, 18.0, 18.0], m::SCREEN_STEEL_LIGHT);
            fill(batch, [x, y, 17.0, 17.0], m::SCREEN_EDGE);
            fill(batch, [x + 1.0, y + 1.0, 16.0, 16.0], m::SCREEN_SLOT);
        }
    }

    // Заголовок: иконка, название, лампа работы.
    let working = r.state == m::LivingState::Working;
    let _ = game.native_item(batch, block, [origin[0] + 5.0, origin[1] + 3.0], 12.0);
    let title = game.native_screen_title().unwrap_or_else(|| m::living_title(block).to_string());
    let title = fit(&title, w - 40.0);
    text(batch, &title, [20.0, 5.0], m::SCREEN_TEXT, true);
    fill(batch, [w - 12.0, 5.0, 6.0, 6.0], m::SCREEN_EDGE);
    fill(batch, [w - 11.0, 6.0, 4.0, 4.0], if working { m::SCREEN_GREEN } else { m::SCREEN_STEEL_DARK });
    let [ix, iy] = layout.inventory;
    text(batch, "Инвентарь", [ix, iy - 11.0], 0xC8_D8DA, false);

    // Шкала порции: янтарная, тает сверху вниз.
    let burn = if r.burn_total > 0 { r.burn as f32 / r.burn_total as f32 } else { 0.0 };
    bar(batch, m::LIVING_BURN_BAR, burn, m::SCREEN_AMBER);

    // Табло: стекло, строка состояния её цветом, ниже числа.
    let g = m::LIVING_BOARD;
    fill(batch, g, m::SCREEN_EDGE);
    fill(batch, [g[0] + 1.0, g[1] + 1.0, g[2] - 2.0, g[3] - 2.0], m::SCREEN_GLASS);
    fill(batch, [g[0] + 1.0, g[1] + 1.0, g[2] - 2.0, 1.0], m::SCREEN_TEAL_DIM);
    let inner = g[2] - 6.0;
    let head = fit(r.state.text(kind), inner);
    text(batch, &head, [g[0] + 3.0, g[1] + 3.0], r.state.ink(), true);
    let mut y = g[1] + 14.0;
    for line in m::living_board(block, &r) {
        if y + 8.0 > g[1] + g[3] - 1.0 {
            break;
        }
        let line = fit(&line, inner);
        text(batch, &line, [g[0] + 3.0, y], m::SCREEN_TEXT, false);
        y += 9.0;
    }

    // Шкала заряда: бирюзовая, растёт снизу вверх; искра, пока идёт ток.
    let cap = m::energy_spec(block).map_or(1, |s| s.capacity.max(1));
    bar(batch, m::LIVING_CHARGE_BAR, r.stored as f32 / cap as f32, m::SCREEN_TEAL);
    if working {
        let c = m::LIVING_CHARGE_BAR;
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let phase = m::anim_phase(true, millis, 900);
        let sy = c[1] + c[3] - 3.0 - (c[3] - 6.0) * phase;
        fill(batch, [c[0] + 3.0, sy.floor(), c[2] - 6.0, 2.0], 0xFF_E8FCFF);
    }
}
