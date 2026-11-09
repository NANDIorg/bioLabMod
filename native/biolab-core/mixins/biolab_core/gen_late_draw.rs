// Экран двигателя-генератора и изотопной батареи (M10, GDD 4.27 п. 11): та
// же лабораторная панель, что у машин (`screen_draw.rs`), слот топлива со
// шкалой горения над ним, табло (выработка, заряд, остаток топлива) и шкала
// заряда. Раскладка и строки — `late_screen`, `gen_screen_lines` в
// `runtime.rs`, числа — данные меню (`gen_data`, пишет тик генератора).
// Вторая инъекция на `native_screen_draw`: `screen_draw.rs` на генераторах
// выходит сразу (их нет в `machine_screen`). Только клиент.
fn biolab_core_gen_late_draw(
    game: &crate::game::Game<'_>,
    block: &str,
    data: &[i32],
    batch: &mut horizon_client::hud::HudBatch,
    origin: [f32; 2],
) {
    use crate::game::biolab_core as m;

    /// Кисть: всё в координатах от левого верхнего угла панели (как в
    /// `screen_draw.rs`).
    struct Pen<'a, 'g> {
        game: &'a crate::game::Game<'g>,
        batch: &'a mut horizon_client::hud::HudBatch,
        origin: [f32; 2],
    }

    impl Pen<'_, '_> {
        fn at(&self, r: [f32; 4]) -> [f32; 4] {
            [self.origin[0] + r[0], self.origin[1] + r[1], r[2], r[3]]
        }

        fn fill(&mut self, r: [f32; 4], argb: u32) {
            if r[2] <= 0.0 || r[3] <= 0.0 {
                return;
            }
            let dst = self.at(r);
            self.game.native_fill(&mut *self.batch, dst, argb);
        }

        fn sprite(&mut self, name: &str, src: [f32; 4], r: [f32; 4]) -> bool {
            let dst = self.at(r);
            self.game.native_sprite(&mut *self.batch, name, src, dst)
        }

        fn text(&mut self, text: &str, at: [f32; 2], rgb: u32, shadow: bool) {
            let at = [self.origin[0] + at[0], self.origin[1] + at[1]];
            self.game.native_text(&mut *self.batch, text, at, rgb, shadow);
        }

        fn item(&mut self, item: &str, at: [f32; 2], size: f32) -> bool {
            let at = [self.origin[0] + at[0], self.origin[1] + at[1]];
            self.game.native_item(&mut *self.batch, item, at, size)
        }

        /// Полоса, заполненная на долю `f`: горизонтальная — слева,
        /// вертикальная — снизу.
        fn bar(&mut self, r: [f32; 4], f: f32, argb: u32) {
            use crate::game::biolab_core as m;
            let [x, y, w, h] = r;
            self.fill(r, m::SCREEN_EDGE);
            self.fill([x + 1.0, y + 1.0, w - 2.0, h - 2.0], m::SCREEN_STEEL_DARK);
            let f = f.clamp(0.0, 1.0);
            if h > w {
                let lh = ((h - 2.0) * f).floor();
                self.fill([x + 1.0, y + h - 1.0 - lh, w - 2.0, lh], argb);
            } else {
                let lw = ((w - 2.0) * f).floor();
                self.fill([x + 1.0, y + 1.0, lw, h - 2.0], argb);
            }
        }
    }

    let Some(layout) = m::late_screen(block) else {
        return;
    };
    let mut pen = Pen { game, batch, origin };
    let (w, h) = (layout.width, layout.height);
    let get = |i: usize| data.get(i).copied().unwrap_or(0);
    let working = get(m::GEN_DATA_RATE) > 0;

    // Панель: спрайт машин, без него — заливки тех же цветов.
    if !pen.sprite("biolab:gui/machine/panel", [0.0, 0.0, w, h], [0.0, 0.0, w, h]) {
        pen.fill([0.0, 0.0, w, h], m::SCREEN_EDGE);
        pen.fill([1.0, 1.0, w - 2.0, h - 2.0], m::SCREEN_STEEL);
        pen.fill([1.0, 1.0, w - 2.0, 1.0], m::SCREEN_STEEL_LIGHT);
        pen.fill([1.0, 1.0, 1.0, h - 2.0], m::SCREEN_STEEL_LIGHT);
        pen.fill([1.0, h - 2.0, w - 2.0, 1.0], m::SCREEN_STEEL_DARK);
        pen.fill([w - 2.0, 1.0, 1.0, h - 2.0], m::SCREEN_STEEL_DARK);
        pen.fill([4.0, m::SCREEN_HEADER - 1.0, w - 8.0, 1.0], m::SCREEN_TEAL_DIM);
        pen.fill([4.0, m::SCREEN_HEADER - 1.0, 24.0, 1.0], m::SCREEN_TEAL);
    }

    // Рамки слотов: топливо и 36 инвентаря.
    let mut frames: Vec<[f32; 2]> = layout.slots.iter().flatten().copied().collect();
    frames.extend(m::inventory_slots(&layout));
    for slot in frames {
        let r = m::slot_frame(slot);
        if !pen.sprite("biolab:gui/machine/slot", [0.0, 0.0, 18.0, 18.0], r) {
            let [x, y, _, _] = r;
            pen.fill([x, y, 18.0, 18.0], m::SCREEN_STEEL_LIGHT);
            pen.fill([x, y, 17.0, 17.0], m::SCREEN_EDGE);
            pen.fill([x + 1.0, y + 1.0, 16.0, 16.0], m::SCREEN_SLOT);
        }
    }

    // Заголовок: иконка, название, лампа работы.
    pen.item(block, [5.0, 3.0], 12.0);
    let title = game.native_screen_title().unwrap_or_else(|| m::late_title(block).to_string());
    pen.text(&title, [20.0, 5.0], m::SCREEN_TEXT, true);
    pen.fill([w - 12.0, 5.0, 6.0, 6.0], m::SCREEN_EDGE);
    pen.fill([w - 11.0, 6.0, 4.0, 4.0], if working { m::SCREEN_GREEN } else { m::SCREEN_STEEL_DARK });

    // Шкала горения текущей единицы — над слотом топлива.
    let flame = if block == m::RTG { 0xFF_7CF03C } else { m::SCREEN_AMBER };
    pen.bar(m::GEN_FUEL_BAR, get(m::GEN_DATA_FUEL) as f32 / 1000.0, flame);

    // Табло: три строки.
    let g = layout.gauge;
    pen.fill(g, m::SCREEN_EDGE);
    pen.fill([g[0] + 1.0, g[1] + 1.0, g[2] - 2.0, g[3] - 2.0], m::SCREEN_GLASS);
    for (i, line) in m::gen_screen_lines(block, data).iter().enumerate() {
        let colour = if i == 0 && working { m::SCREEN_GREEN } else { m::SCREEN_TEXT };
        pen.text(line, [g[0] + 4.0, g[1] + 3.0 + i as f32 * 11.0], colour, false);
    }

    // Шкала заряда.
    let capacity = get(m::GEN_DATA_CAPACITY).max(1) as f32;
    pen.bar(layout.progress, get(m::GEN_DATA_STORED) as f32 / capacity, m::SCREEN_TEAL);

    let [ix, iy] = layout.inventory;
    pen.text("Инвентарь", [ix, iy - 11.0], 0xC8_D8DA, false);
}
