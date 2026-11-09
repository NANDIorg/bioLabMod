// Экран машины: лабораторная панель, рамки слотов, заголовок с иконкой и
// прибор машины со своей шкалой. Horizon до нас рисует простую серую панель
// с рамками, а после нас — предметы; наша панель её закрывает, поэтому рамки
// слотов и инвентаря рисуем заново. Раскладка, приборы и числа — в модуле
// мода (`machine_screen`, `screen_*`), здесь только кисть. Текстуры из
// `gui/machine` необязательны: без спрайта рисуем заливками того же цвета.
fn biolab_core_screen_draw(
    game: &crate::game::Game<'_>,
    block: &str,
    data: &[i32],
    batch: &mut horizon_client::hud::HudBatch,
    origin: [f32; 2],
) {
    use crate::game::biolab_core as m;

    /// Кисть: всё в координатах от левого верхнего угла панели.
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
            if r[2] <= 0.0 || r[3] <= 0.0 {
                return true;
            }
            let dst = self.at(r);
            self.game.native_sprite(&mut *self.batch, name, src, dst)
        }

        fn text(&mut self, text: &str, at: [f32; 2], rgb: u32, shadow: bool) {
            let at = [self.origin[0] + at[0], self.origin[1] + at[1]];
            self.game.native_text(&mut *self.batch, text, at, rgb, shadow);
        }

        fn width(&self, text: &str) -> f32 {
            self.game.native_text_width(text)
        }

        fn item(&mut self, item: &str, at: [f32; 2], size: f32) -> bool {
            let at = [self.origin[0] + at[0], self.origin[1] + at[1]];
            self.game.native_item(&mut *self.batch, item, at, size)
        }

        /// Застеклённое окно прибора: тёмный кант, стекло, блик сверху.
        fn glass(&mut self, r: [f32; 4]) {
            let [x, y, w, h] = r;
            self.fill(r, m::SCREEN_EDGE);
            self.fill([x + 1.0, y + 1.0, w - 2.0, h - 2.0], m::SCREEN_GLASS);
            self.fill([x + 1.0, y + 1.0, w - 2.0, 1.0], m::SCREEN_TEAL_DIM);
        }

        /// Полоса, заполненная на долю `f`: вдоль длинной стороны, вертикальная
        /// — снизу вверх.
        fn bar(&mut self, r: [f32; 4], f: f32, argb: u32) {
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

        /// Строка, обрезанная по ширине с многоточием.
        fn fit(&self, text: &str, width: f32) -> String {
            if self.width(text) <= width {
                return text.to_string();
            }
            let mut out: String = text.to_string();
            while !out.is_empty() && self.width(&format!("{out}…")) > width {
                out.pop();
            }
            format!("{}…", out.trim_end())
        }

        /// Текст словами по ширине.
        fn wrap(&self, text: &str, width: f32) -> Vec<String> {
            let mut lines = Vec::new();
            let mut line = String::new();
            for word in text.split_whitespace() {
                let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if self.width(&candidate) <= width || line.is_empty() {
                    line = candidate;
                } else {
                    lines.push(std::mem::take(&mut line));
                    line = word.to_string();
                }
            }
            if !line.is_empty() {
                lines.push(line);
            }
            lines
        }
    }

    /// Прямоугольник, обрезанный справа по `x`: заполненная часть стрелки.
    fn clip_x(r: [f32; 4], x: f32) -> [f32; 4] {
        [r[0], r[1], (x - r[0]).clamp(0.0, r[2]), r[3]]
    }

    let Some(layout) = m::machine_screen(block) else {
        return;
    };
    let mut pen = Pen { game, batch, origin };
    let (w, h) = (layout.width, layout.height);
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let working = m::screen_working(data);
    let progress = m::screen_progress(data);

    // Панель: сталь, светлая фаска сверху-слева, тёмная снизу-справа,
    // бирюзовая подсветка под заголовком.
    let panel = if h == m::SCREEN_HEIGHT { "biolab:gui/machine/panel" } else { "biolab:gui/machine/panel_short" };
    if !pen.sprite(panel, [0.0, 0.0, w, h], [0.0, 0.0, w, h]) {
        pen.fill([0.0, 0.0, w, h], m::SCREEN_EDGE);
        pen.fill([1.0, 1.0, w - 2.0, h - 2.0], m::SCREEN_STEEL);
        pen.fill([1.0, 1.0, w - 2.0, 1.0], m::SCREEN_STEEL_LIGHT);
        pen.fill([1.0, 1.0, 1.0, h - 2.0], m::SCREEN_STEEL_LIGHT);
        pen.fill([1.0, h - 2.0, w - 2.0, 1.0], m::SCREEN_STEEL_DARK);
        pen.fill([w - 2.0, 1.0, 1.0, h - 2.0], m::SCREEN_STEEL_DARK);
        pen.fill([4.0, m::SCREEN_HEADER - 1.0, w - 8.0, 1.0], m::SCREEN_TEAL_DIM);
        pen.fill([4.0, m::SCREEN_HEADER - 1.0, 24.0, 1.0], m::SCREEN_TEAL);
        for (x, y) in [(3.0, 3.0), (w - 5.0, 3.0), (3.0, h - 5.0), (w - 5.0, h - 5.0)] {
            pen.fill([x, y, 2.0, 2.0], m::SCREEN_STEEL_DARK);
        }
    }

    // Рамки слотов: машины и все 36 инвентаря. Выходы — в бирюзовой.
    let mut frames: Vec<([f32; 4], bool)> = Vec::new();
    for (i, at) in layout.slots.iter().enumerate() {
        if let Some(at) = at {
            frames.push((m::slot_frame(*at), m::is_output_slot(block, i)));
        }
    }
    for at in m::inventory_slots(&layout) {
        frames.push((m::slot_frame(at), false));
    }
    for (r, output) in frames {
        let sprite = if output { "biolab:gui/machine/slot_out" } else { "biolab:gui/machine/slot" };
        if !pen.sprite(sprite, [0.0, 0.0, 18.0, 18.0], r) {
            let [x, y, _, _] = r;
            pen.fill([x, y, 18.0, 18.0], m::SCREEN_STEEL_LIGHT);
            pen.fill([x, y, 17.0, 17.0], m::SCREEN_EDGE);
            pen.fill([x + 1.0, y + 1.0, 16.0, 16.0], if output { 0xFF_1D3A38 } else { m::SCREEN_SLOT });
            if output {
                pen.fill([x + 1.0, y + 16.0, 16.0, 1.0], m::SCREEN_TEAL);
            }
        }
    }

    // Заголовок: иконка машины, название, лампа работы.
    pen.item(block, [5.0, 3.0], 12.0);
    let title = game.native_screen_title().unwrap_or_else(|| m::machine_title(block).to_string());
    let title = pen.fit(&title, w - 40.0);
    pen.text(&title, [20.0, 5.0], m::SCREEN_TEXT, true);
    pen.fill([w - 12.0, 5.0, 6.0, 6.0], m::SCREEN_EDGE);
    pen.fill([w - 11.0, 6.0, 4.0, 4.0], if working { m::SCREEN_GREEN } else { m::SCREEN_STEEL_DARK });
    let [ix, iy] = layout.inventory;
    pen.text("Инвентарь", [ix, iy - 11.0], 0xC8_D8DA, false);

    // Запас энергии машины-потребителя (M10, GDD 4.27 А): полоса в строке
    // подписи инвентаря, справа; без заряда — «Нет питания» на красном.
    if let (Some(r), Some(power)) = (m::power_bar(block), m::power_shown(block, data)) {
        if power.starved {
            pen.fill(r, m::SCREEN_EDGE);
            pen.fill([r[0] + 1.0, r[1] + 1.0, r[2] - 2.0, r[3] - 2.0], 0xFF_4A1C18);
            let label = pen.fit(m::NO_POWER_BADGE, r[2] - 4.0);
            let lw = pen.width(&label);
            pen.text(&label, [r[0] + ((r[2] - lw) / 2.0).floor(), r[1] + 1.0], 0xFF_7A6A, false);
        } else {
            let colour = if power.fraction() < 0.25 { m::SCREEN_AMBER } else { m::SCREEN_TEAL };
            pen.bar(r, power.fraction(), colour);
        }
    }

    let g = layout.gauge;
    let p = layout.progress;
    match block {
        // Змеевик: медная труба змейкой, по ней бегут капли дистиллята.
        "biolab:still" => {
            pen.glass(g);
            let copper = if working { 0xFF_D88A45 } else { 0xFF_8A5A34 };
            let (left, right) = (g[0] + 6.0, g[0] + g[2] - 6.0);
            for run in 0..m::COIL_RUNS {
                let row = m::coil_row(g, run);
                pen.fill([left - 1.0, row - 1.0, right - left + 2.0, 3.0], copper);
                if run + 1 < m::COIL_RUNS {
                    let x = if run % 2 == 0 { right } else { left };
                    let next = m::coil_row(g, run + 1);
                    pen.fill([x - 1.0, row - 1.0, 3.0, next - row + 3.0], copper);
                }
            }
            if working {
                let phase = m::anim_phase(true, millis, 2400);
                for k in 0..3 {
                    let at = m::coil_point(g, (phase + k as f32 / 3.0).fract());
                    pen.fill([at[0] - 1.0, at[1] - 1.0, 3.0, 3.0], m::SCREEN_TEAL);
                }
            }
            pen.bar(p, progress, m::SCREEN_TEAL);
        }
        // Инкубатор: четыре полоски штамма по пять делений.
        "biolab:incubator" => {
            pen.glass(g);
            let values = m::screen_strain(data).map(|s| s.values()).unwrap_or([0; 4]);
            let colours = [0xFF_E0604A, m::SCREEN_AMBER, 0xFF_B070E0, m::SCREEN_TEAL];
            for (k, short) in ["Зар", "Инк", "Тяж", "Стк"].iter().enumerate() {
                let y = g[1] + 3.0 + k as f32 * 10.0;
                pen.text(short, [g[0] + 3.0, y], m::SCREEN_TEXT_DIM, false);
                for s in 0..5 {
                    let on = (s as i32) < values[k];
                    let cell = [g[0] + 26.0 + s as f32 * 11.0, y + 1.0, 9.0, 6.0];
                    pen.fill(cell, if on { colours[k] } else { m::SCREEN_STEEL_DARK });
                }
            }
            pen.bar(p, progress, m::SCREEN_GREEN);
        }
        // Ферментер: бак с бражкой, уровень растёт с прогрессом, пузыри.
        "biolab:fermenter" => {
            pen.glass(g);
            let inner = [g[0] + 2.0, g[1] + 4.0, g[2] - 4.0, g[3] - 6.0];
            let level = (inner[3] * (0.35 + 0.5 * progress)).floor();
            let top = inner[1] + inner[3] - level;
            pen.fill([inner[0], top, inner[2], level], 0xFF_6E8A34);
            pen.fill([inner[0], top, inner[2], 1.0], 0xFF_A8C85A);
            if working {
                let phase = m::anim_phase(true, millis, 1800);
                for k in 0..5 {
                    let at = m::bubble_point(g, k, phase);
                    if at[1] > top + 1.0 {
                        pen.fill([at[0], at[1], 2.0, 2.0], 0xFF_D8F08A);
                    }
                }
            }
            // Люк сверху и обручи бака.
            pen.fill([g[0] + g[2] / 2.0 - 8.0, g[1] + 1.0, 16.0, 3.0], m::SCREEN_STEEL_LIGHT);
            pen.fill([g[0] + 1.0, g[1] + 14.0, 2.0, 2.0], m::SCREEN_STEEL_LIGHT);
            pen.fill([g[0] + g[2] - 3.0, g[1] + 14.0, 2.0, 2.0], m::SCREEN_STEEL_LIGHT);
            pen.bar(p, progress, 0xFF_A8C85A);
        }
        // Центрифуга: барабан крутится четырьмя кадрами, пока идёт разделение.
        "biolab:centrifuge" => {
            let frame = m::anim_frame(working, millis, 90, 4) as f32;
            if !pen.sprite("biolab:gui/machine/drum", [frame * 20.0, 0.0, 20.0, 20.0], g) {
                let [x, y, _, _] = g;
                pen.fill([x + 1.0, y + 4.0, 18.0, 12.0], m::SCREEN_EDGE);
                pen.fill([x + 4.0, y + 1.0, 12.0, 18.0], m::SCREEN_EDGE);
                pen.fill([x + 2.0, y + 5.0, 16.0, 10.0], m::SCREEN_STEEL_DARK);
                pen.fill([x + 5.0, y + 2.0, 10.0, 16.0], m::SCREEN_STEEL_DARK);
                let spoke = m::SCREEN_STEEL_LIGHT;
                match frame as u32 {
                    0 => pen.fill([x + 9.0, y + 3.0, 2.0, 14.0], spoke),
                    2 => pen.fill([x + 3.0, y + 9.0, 14.0, 2.0], spoke),
                    f => {
                        for k in 0..5 {
                            let d = 4.0 + k as f32 * 2.5;
                            let sx = if f == 1 { x + d } else { x + 18.0 - d };
                            pen.fill([sx - 1.0, y + d - 1.0, 2.0, 2.0], spoke);
                        }
                    }
                }
                pen.fill([x + 8.0, y + 8.0, 4.0, 4.0], m::SCREEN_TEAL);
            }
            pen.bar(p, progress, m::SCREEN_TEAL);
        }
        // Реактор: градусник с красной зоной перегрева, отметкой цели нагрева
        // и полосой диапазона рецепта; стрелка прогресса над ним.
        "biolab:reactor" => {
            pen.glass(g);
            let r = m::reactor_reading(data);
            let tube = m::thermo_tube(g);
            let hot = m::thermo_x(tube, m::OVERHEAT);
            pen.fill([g[0] + 3.0, tube[1] - 2.0, 10.0, 10.0], m::SCREEN_EDGE);
            pen.fill([g[0] + 4.0, tube[1] - 1.0, 8.0, 8.0], m::SCREEN_RED);
            pen.fill(tube, m::SCREEN_STEEL_DARK);
            pen.fill([hot, tube[1], tube[0] + tube[2] - hot, tube[3]], 0xFF_5A1A16);
            for k in 1..10 {
                let x = m::thermo_x(tube, k * 100);
                pen.fill([x, tube[1] + tube[3], 1.0, 1.0], m::SCREEN_STEEL_LIGHT);
            }
            let fits = m::temperature_fits(r.temperature, r.range);
            let mercury = if r.temperature > m::OVERHEAT { 0xFF_FF5A40 } else { m::SCREEN_RED };
            let tx = m::thermo_x(tube, r.temperature);
            pen.fill([tube[0], tube[1] + 1.0, tx - tube[0], tube[3] - 2.0], mercury);
            if let Some((min, max)) = r.range {
                let (a, b) = (m::thermo_x(tube, min), m::thermo_x(tube, max));
                let band = if fits { m::SCREEN_GREEN } else { m::SCREEN_AMBER };
                pen.fill([a, tube[1] + tube[3] + 1.0, (b - a).max(1.0), 2.0], band);
            }
            if let Some(target) = r.target {
                let x = m::thermo_x(tube, target).floor();
                pen.fill([x - 2.0, tube[1] - 5.0, 5.0, 1.0], m::SCREEN_AMBER);
                pen.fill([x - 1.0, tube[1] - 4.0, 3.0, 1.0], m::SCREEN_AMBER);
                pen.fill([x, tube[1] - 3.0, 1.0, 2.0], m::SCREEN_AMBER);
            }
            let reading = format!("{} °C", r.temperature);
            let colour = if r.temperature > m::OVERHEAT {
                0xFF_6A50
            } else if r.range.is_some() && fits {
                0x5A_E8B4
            } else {
                m::SCREEN_TEXT
            };
            let row = g[1] + g[3] - 10.0;
            pen.text(&reading, [g[0] + 3.0, row], colour, true);
            let goal = match r.target {
                Some(t) => format!("нагрев до {t}"),
                None => "без нагрева".to_string(),
            };
            let gw = pen.width(&goal);
            pen.text(&goal, [g[0] + g[2] - gw - 3.0, row], m::SCREEN_TEXT_DIM, false);

            // Стрелка: древко и наконечник, заполняются слева направо.
            let fill_x = p[0] + p[2] * progress;
            let shaft = [p[0], p[1] + 3.0, p[2] - 5.0, 4.0];
            pen.fill(shaft, m::SCREEN_STEEL_DARK);
            pen.fill(clip_x(shaft, fill_x), m::SCREEN_AMBER);
            for k in 0..5 {
                let piece = [p[0] + p[2] - 5.0 + k as f32, p[1] + k as f32, 1.0, p[3] - 2.0 * k as f32];
                let lit = piece[0] < fill_x;
                pen.fill(piece, if lit { m::SCREEN_AMBER } else { m::SCREEN_STEEL_DARK });
            }
        }
        // Синтезатор: табло с набранной формулой и именем в цвете статуса,
        // справа лампа статуса, число атомов и шкала сборки.
        "biolab:synthesizer" => {
            pen.glass(g);
            let status = m::screen_datum(data, m::DATA_SYNTH_STATUS, m::SYNTH_EMPTY);
            let colour = m::synth_status_colour(status);
            let formula = m::synth_formula(data);
            let atoms: u32 = formula.iter().sum();
            let inner = g[2] - 6.0;
            let bottom = g[1] + g[3] - 2.0;
            let mut row = g[1] + 4.0;
            if atoms == 0 {
                pen.text("—", [g[0] + 3.0, row], m::SCREEN_TEXT_DIM, false);
            } else {
                let text = m::formula_text(&formula);
                let mut line = String::new();
                let mut lines = Vec::new();
                for token in m::formula_tokens(&text) {
                    let candidate = format!("{line}{token}");
                    if !line.is_empty() && pen.width(&candidate) > inner {
                        lines.push(std::mem::take(&mut line));
                        line = token;
                    } else {
                        line = candidate;
                    }
                }
                if !line.is_empty() {
                    lines.push(line);
                }
                for line in lines {
                    if row + 8.0 > bottom {
                        break;
                    }
                    pen.text(&line, [g[0] + 3.0, row], colour, true);
                    row += 10.0;
                }
                let name = m::compound_name_of(&formula);
                if name != text {
                    row += 2.0;
                    for line in pen.wrap(&name, inner) {
                        if row + 8.0 > bottom {
                            break;
                        }
                        let line = pen.fit(&line, inner);
                        pen.text(&line, [g[0] + 3.0, row], m::SCREEN_TEXT, false);
                        row += 10.0;
                    }
                }
            }
            pen.glass(p);
            pen.fill([p[0] + 4.0, p[1] + 5.0, 10.0, 10.0], m::SCREEN_EDGE);
            pen.fill([p[0] + 5.0, p[1] + 6.0, 8.0, 8.0], 0xFF00_0000 | colour);
            let percent = format!("{}%", (progress * 100.0) as i32);
            pen.text(&percent, [p[0] + 18.0, p[1] + 6.0], m::SCREEN_TEXT, false);
            let count = format!("{atoms} ат.");
            pen.text(&count, [p[0] + 4.0, p[1] + 22.0], m::SCREEN_TEXT_DIM, false);
            pen.bar([p[0] + 3.0, p[1] + p[3] - 10.0, p[2] - 6.0, 7.0], progress, 0xFF00_0000 | colour);
        }
        // Пресс: станина, шток и плита, которая опускается на таблетку.
        "biolab:press" => {
            pen.glass(g);
            let cx = (g[0] + g[2] / 2.0).floor();
            pen.fill([g[0] + 3.0, g[1] + 2.0, 3.0, g[3] - 4.0], m::SCREEN_STEEL);
            pen.fill([g[0] + g[2] - 6.0, g[1] + 2.0, 3.0, g[3] - 4.0], m::SCREEN_STEEL);
            pen.fill([g[0] + 3.0, g[1] + 2.0, g[2] - 6.0, 2.0], m::SCREEN_STEEL_LIGHT);
            let plate = m::press_plate(g, progress);
            pen.fill([cx - 1.0, g[1] + 4.0, 2.0, plate[1] - g[1] - 4.0], m::SCREEN_STEEL_DARK);
            pen.fill(plate, m::SCREEN_STEEL_LIGHT);
            pen.fill([plate[0], plate[1] + plate[3] - 1.0, plate[2], 1.0], m::SCREEN_STEEL_DARK);
            let anvil = [g[0] + 8.0, g[1] + g[3] - 8.0, g[2] - 16.0, 6.0];
            pen.fill(anvil, m::SCREEN_STEEL);
            pen.fill([anvil[0], anvil[1], anvil[2], 1.0], m::SCREEN_STEEL_LIGHT);
            if working {
                pen.fill([cx - 4.0, anvil[1] - 3.0, 8.0, 3.0], 0xFF_F0ECE0);
            }
            pen.bar(p, progress, m::SCREEN_TEAL);
        }
        // Экстрактор: имя последнего гена и его сила, шкала — спираль ДНК.
        "biolab:extractor" => {
            pen.glass(g);
            pen.text("ДНК", [g[0] + 3.0, g[1] + 3.0], m::SCREEN_TEXT_DIM, false);
            match m::screen_gene(data) {
                Some((gene, strength)) => {
                    let mut row = g[1] + 13.0;
                    for line in pen.wrap(gene.title, g[2] - 6.0).into_iter().take(2) {
                        let line = pen.fit(&line, g[2] - 6.0);
                        pen.text(&line, [g[0] + 3.0, row], 0x3F_D8C4, true);
                        row += 9.0;
                    }
                    for s in 0..strength.clamp(0, 5) {
                        let pip = [g[0] + g[2] - 8.0 - s as f32 * 6.0, g[1] + 4.0, 4.0, 4.0];
                        pen.fill(pip, m::SCREEN_TEAL);
                    }
                }
                None => pen.text("гена нет", [g[0] + 3.0, g[1] + 13.0], m::SCREEN_TEXT_DIM, false),
            }
            pen.fill(p, m::SCREEN_EDGE);
            pen.fill([p[0] + 1.0, p[1] + 1.0, p[2] - 2.0, p[3] - 2.0], m::SCREEN_GLASS);
            let fill_x = p[0] + p[2] * progress;
            let shift = m::anim_frame(working, millis, 120, 4) as usize;
            let wave = [0.0, 1.0, 2.0, 1.0];
            let steps = ((p[2] - 2.0) / 4.0) as usize;
            for k in 0..steps {
                let x = p[0] + 1.0 + k as f32 * 4.0;
                let dy = wave[(k + shift) % 4];
                let lit = x < fill_x;
                let (a, b) = if lit { (m::SCREEN_TEAL, m::SCREEN_AMBER) } else { (m::SCREEN_TEAL_DIM, m::SCREEN_STEEL_DARK) };
                pen.fill([x, p[1] + 1.0 + dy, 2.0, 2.0], a);
                pen.fill([x, p[1] + p[3] - 3.0 - dy, 2.0, 2.0], b);
            }
        }
        // Клонирующий чан: зелёный раствор, силуэт клона растёт снизу вверх,
        // под чаном — ген; шкала раствора справа.
        "biolab:cloning_vat" => {
            let vat = [g[0], g[1], g[2], g[3] - 11.0];
            pen.glass(vat);
            pen.fill([vat[0] + 2.0, vat[1] + 5.0, vat[2] - 4.0, vat[3] - 7.0], 0xFF_1E4A36);
            pen.fill([vat[0] + 2.0, vat[1] + 5.0, vat[2] - 4.0, 1.0], 0xFF_3E8A62);
            let parts = m::clone_parts(g);
            for part in parts {
                pen.fill(part, 0xFF_2A5E46);
            }
            let top = parts.iter().map(|r| r[1]).fold(f32::MAX, f32::min);
            let bottom = parts.iter().map(|r| r[1] + r[3]).fold(f32::MIN, f32::max);
            let left = parts.iter().map(|r| r[0]).fold(f32::MAX, f32::min);
            let right = parts.iter().map(|r| r[0] + r[2]).fold(f32::MIN, f32::max);
            let (cw, ch) = (right - left, bottom - top);
            let grown = (ch * progress).round();
            let drawn = grown <= 0.0
                || pen.sprite(
                    "biolab:gui/machine/clone",
                    [0.0, ch - grown, cw, grown],
                    [left, bottom - grown, cw, grown],
                );
            if !drawn {
                for part in parts {
                    if let Some(r) = m::clone_grown(part, g, progress) {
                        pen.fill(r, 0xFF_A8E0B8);
                    }
                }
            }
            if working {
                let phase = m::anim_phase(true, millis, 2600);
                for k in 0..4 {
                    let at = m::bubble_point(vat, k, phase);
                    pen.fill([at[0], at[1], 2.0, 2.0], 0xFF_6EC896);
                }
            }
            let label = match m::screen_gene(data) {
                Some((gene, strength)) => format!("{} {strength}", gene.title),
                None => "ген не задан".to_string(),
            };
            let label = pen.fit(&label, g[2]);
            let lw = pen.width(&label);
            pen.text(&label, [g[0] + (g[2] - lw) / 2.0, g[1] + g[3] - 9.0], m::SCREEN_TEXT, true);
            pen.bar(p, progress, m::SCREEN_GREEN);
        }
        // Спектрометр: 18 полос по элементам, символы в два ряда, бегущая
        // линия скана; стрелка от входа к выходу.
        "biolab:spectrometer" => {
            pen.glass(g);
            let formula = m::spectrum(data);
            for (i, element) in m::ELEMENTS.iter().enumerate() {
                let bar = m::spectrum_bar(g, i);
                pen.fill(bar, 0xFF_18262A);
                let lh = (bar[3] * m::spectrum_level(&formula, i)).round();
                let mut single = m::Formula::default();
                single[i] = 1;
                let colour = 0xFF00_0000 | m::colour_of(&single);
                pen.fill([bar[0], bar[1] + bar[3] - lh, bar[2], lh], colour);
                let at = m::spectrum_label(g, i);
                let sw = pen.width(element.symbol);
                let ink = if formula[i] > 0 { m::SCREEN_TEXT } else { 0x5A_6E72 };
                pen.text(element.symbol, [at[0] - sw / 2.0, at[1]], ink, false);
            }
            if working {
                let first = m::spectrum_bar(g, 0);
                let last = m::spectrum_bar(g, m::ELEMENTS.len() - 1);
                let x = (first[0] + (last[0] + last[2] - first[0]) * progress).floor();
                pen.fill([x, first[1] - 1.0, 1.0, first[3] + 2.0], m::SCREEN_TEAL);
            }
            // Стрелка вниз: древко заполняется сверху по прогрессу.
            let shaft = [p[0] + p[2] / 2.0 - 1.0, p[1], 2.0, p[3] - 3.0];
            pen.fill(shaft, m::SCREEN_STEEL_DARK);
            pen.fill([shaft[0], shaft[1], shaft[2], (shaft[3] * progress).floor()], m::SCREEN_TEAL);
            let head = if progress >= 1.0 || working { m::SCREEN_TEAL } else { m::SCREEN_STEEL_DARK };
            for k in 0..3 {
                let k = k as f32;
                pen.fill([p[0] + 2.0 + 2.0 * k, p[1] + p[3] - 3.0 + k, p[2] - 4.0 - 4.0 * k, 1.0], head);
            }
        }
        // Лабораторный стол: табло с тем, что соберётся из выложенного, —
        // иконка и имя или «Нет такого рецепта»; справа лампа, проценты,
        // остаток времени и шкала сборки.
        "biolab:lab_bench" => {
            pen.glass(g);
            let shown = m::bench_shown(data);
            let colour = m::bench_status_colour(shown);
            let inner = g[2] - 6.0;
            // Табло узкое: строка, которая не влезает целиком, бежит по
            // кругу, а не обрезается многоточием — иначе от «Пенициллин,
            // инъекция» осталось бы «Пени…».
            fn board(pen: &Pen<'_, '_>, line: &str, inner: f32, millis: u128) -> String {
                if pen.width(line) <= inner {
                    return line.to_string();
                }
                let mut out = m::ticker(line, (millis / 250) as usize);
                while !out.is_empty() && pen.width(&out) > inner {
                    out.pop();
                }
                out
            }
            let icon = [g[0] + ((g[2] - 16.0) / 2.0).floor(), g[1] + 4.0];
            let (lines, ink): (Vec<String>, u32) = match shown {
                m::BenchShown::Item(item, name) => {
                    pen.item(item, icon, 16.0);
                    (pen.wrap(name, inner), m::SCREEN_TEXT)
                }
                m::BenchShown::Unlisted => {
                    pen.item(m::LAB_BENCH, icon, 16.0);
                    (pen.wrap(m::bench_status_text(shown), inner), colour)
                }
                m::BenchShown::NoRecipe => {
                    let qw = pen.width("?");
                    pen.text("?", [g[0] + (g[2] - qw) / 2.0, icon[1] + 4.0], colour, true);
                    (pen.wrap(m::bench_status_text(shown), inner), colour)
                }
                m::BenchShown::Empty => {
                    let dw = pen.width("—");
                    pen.text("—", [g[0] + (g[2] - dw) / 2.0, icon[1] + 4.0], m::SCREEN_TEXT_DIM, false);
                    (vec!["пусто".to_string()], m::SCREEN_TEXT_DIM)
                }
            };
            let mut row = g[1] + 24.0;
            for line in lines.iter().take(3) {
                let line = board(&pen, line, inner, millis);
                let lw = pen.width(&line);
                pen.text(&line, [g[0] + ((g[2] - lw) / 2.0).floor(), row], ink, false);
                row += 9.0;
            }

            pen.glass(p);
            pen.fill([p[0] + 4.0, p[1] + 5.0, 10.0, 10.0], m::SCREEN_EDGE);
            pen.fill([p[0] + 5.0, p[1] + 6.0, 8.0, 8.0], 0xFF00_0000 | colour);
            let percent = format!("{}%", (progress * 100.0) as i32);
            pen.text(&percent, [p[0] + 18.0, p[1] + 6.0], m::SCREEN_TEXT, false);
            if let Some(seconds) = m::seconds_left(data) {
                pen.text(&format!("{seconds} с"), [p[0] + 4.0, p[1] + 22.0], m::SCREEN_TEXT_DIM, false);
            }
            pen.bar([p[0] + 3.0, p[1] + p[3] - 10.0, p[2] - 6.0, 7.0], progress, 0xFF00_0000 | colour);
        }
        // Ректификационная колонна (GDD 4.18): слева башня снизу вверх —
        // огонь, кипятильник, пять мест под тарелки (стоящие светлые), сверху
        // конденсатор; пар поднимается по тарелкам по ходу перегонки. Справа
        // табло: собрана ли и чего не хватает. Слоты фракций сверх числа
        // тарелок закрыты тёмной заглушкой: туда колонна не кладёт.
        m::COLUMN => {
            let shown = m::multiblock_shown(data);
            let (formed, sections) = match &shown {
                m::MultiblockShown::Formed { sections, .. } => (true, *sections),
                _ => (false, 0),
            };
            pen.glass(p);
            let (x, w) = (p[0] + 4.0, p[2] - 8.0);
            let mut row = p[1] + p[3] - 2.0;
            let flame = [x, row - 4.0, w, 4.0];
            pen.fill(flame, if working { 0xFF_FF8A30 } else { 0xFF_5A3A2A });
            if working && m::anim_frame(true, millis, 150, 2) == 1 {
                pen.fill([x + 2.0, row - 6.0, w - 4.0, 2.0], 0xFF_FFD070);
            }
            row -= 5.0;
            pen.fill([x - 1.0, row - 9.0, w + 2.0, 9.0], if formed { 0xFF_C06C40 } else { 0xFF_7A4A30 });
            pen.fill([x - 1.0, row - 9.0, w + 2.0, 1.0], 0xFF_E89864);
            row -= 10.0;
            // Пар: сколько тарелок уже прогрелось, снизу вверх.
            let hot = (progress * sections as f32).ceil() as u32;
            for k in 0..m::COLUMN_TRAYS.1 {
                let tray = [x, row - 5.0, w, 5.0];
                let present = formed && k < sections;
                pen.fill(tray, if present { m::SCREEN_STEEL_LIGHT } else { m::SCREEN_STEEL_DARK });
                if present && working && k < hot {
                    pen.fill([x + 2.0, tray[1] + 2.0, w - 4.0, 1.0], m::SCREEN_TEAL);
                }
                row -= 6.0;
            }
            pen.fill([x - 1.0, row - 4.0, w + 2.0, 4.0], if formed { 0xFF_D88A45 } else { m::SCREEN_STEEL_DARK });

            // Заглушки на слотах фракций, которых при этой высоте нет.
            for (i, at) in layout.slots.iter().enumerate().skip(1) {
                if let Some([sx, sy]) = at
                    && formed
                    && (i as u32) > sections
                {
                    pen.fill([*sx, *sy, 16.0, 16.0], m::SCREEN_STEEL_DARK);
                    pen.fill([*sx + 4.0, *sy + 7.0, 8.0, 2.0], m::SCREEN_EDGE);
                }
            }

            // Табло сборки.
            pen.glass(g);
            let inner = g[2] - 6.0;
            let (head, head_ink, rest): (&str, u32, Vec<String>) = match &shown {
                m::MultiblockShown::Unknown => ("Проверка схемы…", m::SCREEN_TEXT_DIM, Vec::new()),
                m::MultiblockShown::Formed { sections, .. } => {
                    ("Собрана", 0x5A_E8B4, vec![m::column_status(*sections)])
                }
                m::MultiblockShown::Broken { rows, total } => {
                    let lines = m::multiblock(block)
                        .map(|mb| m::shortage_lines(mb, rows, *total))
                        .unwrap_or_default();
                    ("Не собрана", 0xE8_5A4A, lines.into_iter().take(1).collect())
                }
            };
            pen.text(head, [g[0] + 3.0, g[1] + 3.0], head_ink, true);
            let mut line_y = g[1] + 13.0;
            for text in &rest {
                for line in pen.wrap(text, inner) {
                    if line_y + 8.0 > g[1] + g[3] - 1.0 {
                        break;
                    }
                    let line = pen.fit(&line, inner);
                    pen.text(&line, [g[0] + 3.0, line_y], m::SCREEN_TEXT, false);
                    line_y += 10.0;
                }
            }
        }
        // Генный секвенатор (GDD 4.18, п. 3): слева три кнопки режима
        // (выбранная подсвечена), между рядами слотов — шкала цветом режима,
        // справа монитор: режим, гены, внизу итог; у несобранного — нехватка.
        m::SEQUENCER => {
            let shown = m::seq_shown(data);
            for (b, mode) in m::seq_buttons().iter().zip(m::SeqMode::ALL) {
                let on = shown.mode == mode;
                pen.fill(*b, m::SCREEN_EDGE);
                pen.fill([b[0] + 1.0, b[1] + 1.0, b[2] - 2.0, b[3] - 2.0], if on { m::SCREEN_TEAL_DIM } else { m::SCREEN_STEEL_DARK });
                pen.fill([b[0] + 1.0, b[1] + 1.0, b[2] - 2.0, 1.0], if on { m::SCREEN_TEAL } else { m::SCREEN_STEEL });
                let mark = mode.mark();
                let mw = pen.width(mark);
                let ink = if on { m::SCREEN_TEXT } else { m::SCREEN_TEXT_DIM };
                pen.text(mark, [b[0] + ((b[2] - mw) / 2.0).floor(), b[1] + 4.0], ink, on);
            }
            let colour = match shown.mode {
                m::SeqMode::Vector => m::SCREEN_TEAL,
                m::SeqMode::Antisense => m::SCREEN_RED,
                m::SeqMode::Read => m::SCREEN_AMBER,
            };
            pen.bar(p, progress, colour);
            // Ступени двойной спирали поверх шкалы: читается как цепь ДНК.
            for k in 0..((p[2] - 4.0) / 4.0) as i32 {
                let x = p[0] + 2.0 + k as f32 * 4.0;
                let up = k % 2 == 0;
                pen.fill([x, if up { p[1] + 2.0 } else { p[1] + p[3] - 3.0 }, 2.0, 1.0], m::SCREEN_EDGE);
            }

            pen.glass(g);
            let inner = g[2] - 6.0;
            match m::multiblock_shown(data) {
                m::MultiblockShown::Formed { .. } => {
                    let head = pen.fit(shown.mode.title(), inner);
                    pen.text(&head, [g[0] + 3.0, g[1] + 3.0], 0x5A_E8B4, true);
                    if working {
                        let phase = m::anim_phase(true, millis, 1800);
                        let x = (g[0] + 2.0 + (g[2] - 4.0) * phase).floor();
                        pen.fill([x, g[1] + 12.0, 1.0, g[3] - 13.0], m::SCREEN_TEAL_DIM);
                    }
                    let rows = ((g[3] - 14.0) / 9.0).floor().max(1.0) as usize;
                    for (k, (line, ink)) in m::seq_monitor(&shown, rows).iter().enumerate() {
                        let line = pen.fit(line, inner);
                        pen.text(&line, [g[0] + 3.0, g[1] + 13.0 + k as f32 * 9.0], *ink, false);
                    }
                }
                other => {
                    let (head, ink, rest): (&str, u32, Vec<String>) = match other {
                        m::MultiblockShown::Broken { rows, total } => {
                            let lines = m::multiblock(block)
                                .map(|mb| m::shortage_lines(mb, &rows, total))
                                .unwrap_or_default();
                            ("Не собран", 0xE8_5A4A, lines.into_iter().take(1).collect())
                        }
                        _ => ("Проверка схемы…", m::SCREEN_TEXT_DIM, Vec::new()),
                    };
                    pen.text(head, [g[0] + 3.0, g[1] + 3.0], ink, true);
                    let mut line_y = g[1] + 13.0;
                    for text in &rest {
                        for line in pen.wrap(text, inner) {
                            if line_y + 8.0 > g[1] + g[3] - 1.0 {
                                break;
                            }
                            let line = pen.fit(&line, inner);
                            pen.text(&line, [g[0] + 3.0, line_y], m::SCREEN_TEXT, false);
                            line_y += 10.0;
                        }
                    }
                }
            }
        }
        // Фильтр карантинной камеры (GDD 4.18 п. 4): слева патрон и под ним
        // его остаток полосой, справа табло — держит ли камера, а если не
        // закрыта — что мешает и где («Дыра в стене: x y z»).
        m::QUARANTINE => {
            let board = m::quarantine_board(data);
            // Остаток патрона.
            pen.glass(p);
            let percent = m::screen_datum(data, m::DATA_QR_FILTER, -1);
            let (label, ink) = if percent < 0 {
                ("нет", m::SCREEN_TEXT_DIM)
            } else {
                ("", if percent > 25 { m::BOARD_GREEN } else { m::BOARD_AMBER })
            };
            if percent < 0 {
                let lw = pen.width(label);
                pen.text(label, [p[0] + ((p[2] - lw) / 2.0).floor(), p[1] + 3.0], ink, false);
            } else {
                let text = format!("{percent}%");
                let lw = pen.width(&text);
                pen.text(&text, [p[0] + ((p[2] - lw) / 2.0).floor(), p[1] + 3.0], ink, false);
                pen.bar([p[0] + 3.0, p[1] + p[3] - 7.0, p[2] - 6.0, 5.0], percent as f32 / 100.0, 0xFF00_0000 | ink);
            }
            // Табло.
            pen.glass(g);
            let inner = g[2] - 6.0;
            // Лампа «держит» в углу табло.
            let lamp = if board.ink == m::BOARD_GREEN && working { board.ink } else { 0x2A_3A34 };
            pen.fill([g[0] + g[2] - 9.0, g[1] + 3.0, 6.0, 6.0], m::SCREEN_EDGE);
            pen.fill([g[0] + g[2] - 8.0, g[1] + 4.0, 4.0, 4.0], 0xFF00_0000 | lamp);
            pen.text(board.head, [g[0] + 3.0, g[1] + 3.0], board.ink, true);
            let mut line_y = g[1] + 14.0;
            for text in &board.lines {
                for line in pen.wrap(text, inner) {
                    if line_y + 8.0 > g[1] + g[3] - 1.0 {
                        break;
                    }
                    let line = pen.fit(&line, inner);
                    pen.text(&line, [g[0] + 3.0, line_y], m::SCREEN_TEXT, false);
                    line_y += 10.0;
                }
            }
        }
        // Порт установки: стрелка подачи — бирюзовая к слотам у входного,
        // янтарная от слотов у выходного; справа табличка.
        // Криобанк (GDD 4.23 п. 9): табло холода — иней по краю, крупно
        // температура, под ней что тут хранится.
        m::CRYOBANK => {
            pen.glass(g);
            let frost = 0xFF_BFE6F2;
            for k in 0..((g[2] / 6.0) as i32) {
                let x = g[0] + 2.0 + k as f32 * 6.0;
                pen.fill([x, g[1] + 1.0, 3.0, 1.0], frost);
                pen.fill([x + 2.0, g[1] + g[3] - 2.0, 3.0, 1.0], frost);
            }
            let inner = g[2] - 8.0;
            pen.text("−80 °C", [g[0] + 4.0, g[1] + 5.0], frost & 0xFF_FFFF, true);
            let line = pen.fit("Образцы здесь не портятся", inner);
            pen.text(&line, [g[0] + 4.0, g[1] + 17.0], m::SCREEN_TEXT, false);
        }
        m::INPUT_PORT | m::OUTPUT_PORT => {
            let input = block == m::INPUT_PORT;
            let colour = if input { m::SCREEN_TEAL } else { m::SCREEN_AMBER };
            pen.glass(g);
            let cy = (g[1] + g[3] / 2.0).floor();
            pen.fill([g[0] + 4.0, cy - 1.0, g[2] - 8.0, 2.0], colour);
            for k in 0..4 {
                let k = k as f32;
                let ax = if input { g[0] + g[2] - 5.0 - k } else { g[0] + 4.0 + k };
                pen.fill([ax, cy - 1.0 - k, 1.0, 2.0 + 2.0 * k], colour);
            }
            pen.glass(p);
            let label = if input { "вход" } else { "выход" };
            let lw = pen.width(label);
            let ink = if input { 0x3F_D8C4 } else { 0xE0_A040 };
            pen.text(label, [p[0] + ((p[2] - lw) / 2.0).floor(), p[1] + 6.0], ink, false);
        }
        // Биореактор (GDD 4.18, п. 2): стрелка партии от входов к выходам
        // (над слотом раствора); внизу табло во всю ширину — слева сборка
        // или состояние среды, справа градусник 20–50 °C с зелёной полосой
        // 30–40 и отметкой, до скольки греет огонь.
        m::BIOREACTOR => {
            let medium_ink = 0xFF_6EBE46;
            let fill_x = p[0] + p[2] * progress;
            let shaft = [p[0], p[1] + 3.0, p[2] - 5.0, 5.0];
            pen.fill(shaft, m::SCREEN_STEEL_DARK);
            pen.fill(clip_x(shaft, fill_x), medium_ink);
            for k in 0..5 {
                let piece = [p[0] + p[2] - 5.0 + k as f32, p[1] + k as f32, 1.0, p[3] - 2.0 * k as f32];
                let lit = piece[0] < fill_x;
                pen.fill(piece, if lit { medium_ink } else { m::SCREEN_STEEL_DARK });
            }

            pen.glass(g);
            let r = m::bio_reading(data);
            let state = m::medium(r.temperature);
            let tube = [g[0] + g[2] - 46.0, g[1] + 5.0, 42.0, 4.0];
            pen.fill([tube[0] - 1.0, tube[1] - 1.0, tube[2] + 2.0, tube[3] + 2.0], m::SCREEN_EDGE);
            pen.fill(tube, m::SCREEN_STEEL_DARK);
            let (min, max) = m::BIOREACTOR_RANGE;
            let (a, b) = (m::bio_thermo_x(tube, min), m::bio_thermo_x(tube, max));
            pen.fill([a, tube[1] + tube[3] + 1.0, (b - a).max(1.0), 2.0], m::SCREEN_GREEN);
            let mercury = match state {
                m::Medium::Cold => m::SCREEN_AMBER,
                m::Medium::Fit => m::SCREEN_GREEN,
                m::Medium::Hot => 0xFF_FF5A40,
            };
            let tx = m::bio_thermo_x(tube, r.temperature);
            pen.fill([tube[0], tube[1] + 1.0, tx - tube[0], tube[3] - 2.0], mercury);
            if let Some(t) = r.target {
                let x = m::bio_thermo_x(tube, t).floor();
                pen.fill([x, tube[1] - 3.0, 1.0, 2.0], m::SCREEN_AMBER);
            }
            let ink = match state {
                m::Medium::Cold => 0xE0_A040,
                m::Medium::Fit => 0x5A_E8B4,
                m::Medium::Hot => 0xFF_6A50,
            };
            let reading = format!("{} °C", r.temperature);
            let rx = tube[0] - 4.0 - pen.width(&reading);
            pen.text(&reading, [rx, g[1] + 4.0], ink, false);

            let (text, text_ink) = match m::multiblock_shown(data) {
                m::MultiblockShown::Unknown => ("Проверка схемы…".to_string(), m::SCREEN_TEXT_DIM),
                m::MultiblockShown::Formed { .. } => (m::bioreactor_status(&r), ink),
                m::MultiblockShown::Broken { rows, total } => {
                    let first = m::multiblock(block)
                        .and_then(|mb| m::shortage_lines(mb, &rows, total).into_iter().next())
                        .unwrap_or_else(|| "Не собрана".to_string());
                    (first, 0xE8_5A4A)
                }
            };
            let line = pen.fit(&text, rx - g[0] - 7.0);
            pen.text(&line, [g[0] + 3.0, g[1] + 4.0], text_ink, false);
        }
        _ => {}
    }

    // Панель режима (GDD 4.23 п. 3): две ручки с кнопками «−» и «+», под
    // ними лампа «окно найдено». Рисуется поверх прибора машины, в свободном
    // углу под входом — одинаково у ферментера и экстрактора.
    if m::has_protocol(block) {
        let shown = m::proto_shown(data);
        let buttons = m::proto_buttons();
        for (k, row) in m::proto_rows().iter().enumerate() {
            let Some(knob) = m::Knob::from_id(k) else {
                continue;
            };
            pen.fill(*row, m::SCREEN_EDGE);
            pen.fill([row[0] + 1.0, row[1] + 1.0, row[2] - 2.0, row[3] - 2.0], m::SCREEN_GLASS);
            let reading = m::proto_reading(knob, shown.setting(knob));
            let rw = pen.width(&reading);
            let ink = match shown.state {
                m::PROTO_FOUND => m::SCREEN_GREEN,
                m::PROTO_NO_RECIPE => m::SCREEN_TEXT_DIM,
                _ => m::SCREEN_TEXT,
            };
            pen.text(&reading, [row[0] + (row[2] - rw) / 2.0, row[1]], ink, false);
            for (half, mark) in [(0usize, "-"), (1, "+")] {
                let b = buttons[k * 2 + half];
                pen.fill(b, m::SCREEN_EDGE);
                pen.fill([b[0] + 1.0, b[1] + 1.0, b[2] - 2.0, b[3] - 2.0], m::SCREEN_STEEL_DARK);
                pen.fill([b[0] + 1.0, b[1] + 1.0, b[2] - 2.0, 1.0], m::SCREEN_STEEL);
                let mw = pen.width(mark);
                pen.text(mark, [b[0] + ((b[2] - mw) / 2.0).floor(), b[1]], m::SCREEN_TEXT, false);
            }
        }
        // Куда двигать ручку: стрелка у края строки по прошлой попытке.
        if let Some(aims) = shown.aim {
            let rows = m::proto_rows();
            for (k, aim) in [aims.0, aims.1].into_iter().enumerate() {
                let row = rows[k];
                let (colour, dx) = match aim {
                    m::Aim::In => (m::SCREEN_GREEN, 0.0),
                    m::Aim::NearLow => (m::SCREEN_AMBER, 1.0),
                    m::Aim::FarLow => (m::SCREEN_RED, 1.0),
                    m::Aim::NearHigh => (m::SCREEN_AMBER, -1.0),
                    m::Aim::FarHigh => (m::SCREEN_RED, -1.0),
                };
                let x = if dx >= 0.0 { row[0] + row[2] - 11.0 } else { row[0] + 10.0 };
                pen.fill([x, row[1] + 3.0, 2.0, 2.0], colour);
            }
        }
    }

    // Панель опознания пробы (GDD 4.23 п. 4): вместо спектра — ответы
    // проверок, остаток вариантов и четыре кнопки под ними.
    if block == m::SPECTROMETER
        && let Some(shown) = m::probe_shown(data)
    {
        pen.glass(g);
        let inner = g[2] - 6.0;
        let buttons = m::probe_buttons();
        let rows = (((buttons[0][1] - g[1] - 4.0) / 9.0).floor().max(1.0)) as usize;
        for (k, (line, ink)) in m::probe_lines(&shown, rows).iter().enumerate() {
            let line = pen.fit(line, inner);
            pen.text(&line, [g[0] + 3.0, g[1] + 3.0 + k as f32 * 9.0], *ink, false);
        }
        for (k, b) in buttons.iter().enumerate() {
            let Some(probe) = m::Probe::from_id(k) else {
                continue;
            };
            let done = shown.done & probe.bit() != 0;
            let best = shown.best == Some(probe);
            let body = if done {
                m::SCREEN_TEAL_DIM
            } else if best {
                0xFF_2A5A54
            } else {
                m::SCREEN_STEEL_DARK
            };
            pen.fill(*b, m::SCREEN_EDGE);
            pen.fill([b[0] + 1.0, b[1] + 1.0, b[2] - 2.0, b[3] - 2.0], body);
            pen.fill([b[0] + 1.0, b[1] + 1.0, b[2] - 2.0, 1.0], if best { m::SCREEN_TEAL } else { m::SCREEN_STEEL });
            let mark = pen.fit(probe.mark(), b[2] - 4.0);
            let mw = pen.width(&mark);
            let ink = if done { m::SCREEN_TEXT_DIM } else { m::SCREEN_TEXT };
            pen.text(&mark, [b[0] + ((b[2] - mw) / 2.0).floor(), b[1] + 2.0], ink, best);
            // Идущая проверка — бегущая полоска по низу кнопки.
            if shown.running == Some(probe) {
                let lw = ((b[2] - 2.0) * progress).floor();
                pen.fill([b[0] + 1.0, b[1] + b[3] - 2.0, lw, 1.0], m::SCREEN_TEAL);
            }
        }
    }
}
