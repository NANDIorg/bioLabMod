// Наросты мутантов (GDD 4.19): как мутант выглядит — рога, хвост, щупальца,
// третья рука, вторая голова, шипы, глаза по телу и т. д.
//
// Наросты — своя геометрия поверх модели моба. Её рисует отдельный проход
// после тела (RETURN на `render_visible`, `growths_draw.rs`), как проход
// костей операционной (`biolab_surgery`): тело и его слои уже нарисованы, мы
// дописываем вершины в тот же буфер с той же базовой матрицей `base` — в ней
// уже и масштаб генома, и поворот «лёжа» на операционном столе, так что
// наросты ложатся вместе с мобом.
//
// Как устроено:
// * у каждой черты — список кубов (`Piece`) и цепочек (`Chain`) относительно
//   части-якоря: голова, тело, правая/левая рука, правая/левая нога (у
//   четвероногих руки — передние лапы, ноги — задние);
// * координаты задаются в «каноническом» кадре якоря, общем для всех моделей:
//   x — влево от моба, y — «вниз по телу» (у прямоходящих к ногам, у лежачих
//   тел к хвосту), z — «к спине». Так одна таблица подходит и зомби, и
//   корове, у которой тело повёрнуто на 90° по X, и пауку, у которого брюшко
//   не повёрнуто вовсе. Точка крепления — доля габарита якоря;
// * сила гена 1–3 — размер (`grow`) и число наростов (`from_strength`);
// * анимация — от возраста моба и его шага (хвост качается, щупальца
//   извиваются, пузыри кислоты пульсируют и капают, челюсть открывается);
// * третья рука и вторая голова — копии частей самой модели (с текстурой
//   моба): так они похожи на свои у любого вида;
// * текстура остальных наростов — атлас `biolab:gui/mutations/growths`
//   (`tools/mkgui_mutations.py`): блоки материалов 32×16. Плоть красится
//   оттенком мутанта, кость, рог, глаза и кислота — нет, светящиеся пятна
//   рисуются полной яркостью.
//
// Чистая часть (якоря, кадры, геометрия, развёртка UV, анимация) — под
// тестами; функции, которым нужна сама игра, отгорожены `#[cfg(not(test))]`.
//
// Этот файл подключается как модуль `crate::biolab_growths` крейта
// horizon-client (там живут `mobs` и отрисовка мобов).

use std::f32::consts::{FRAC_PI_2, PI};

/// Текстура наростов в общем атласе: из модов туда попадают `textures/gui/…`.
pub const SPRITE: &str = "biolab:gui/mutations/growths";
/// Размер текстуры в пикселях (ширина, высота).
pub const TEX: [f32; 2] = [128.0, 64.0];
/// Размер блока одного материала в текстуре.
pub const BLOCK: [f32; 2] = [32.0, 16.0];

/// Все черты, которые видны на модели: существующие гены и гены уродств.
pub const TRAITS: [&str; 17] = [
    "chitin",
    "thorns",
    "rending_claws",
    "spring_legs",
    "aquatic",
    "acid_glands",
    "bioluminescence",
    "predator",
    "dense_bone",
    "polymelia",
    "dicephaly",
    "horns",
    "tail",
    "tentacles",
    "polyopia",
    "tumor",
    "membranes",
];

/// Черты-копии: рисуются частями самой модели, а не своими кубами.
pub const COPIES: [&str; 2] = ["polymelia", "dicephaly"];

// ---- Якоря ----

/// Часть модели, к которой крепится нарост.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Anchor {
    Head,
    Body,
    RightArm,
    LeftArm,
    RightLeg,
    LeftLeg,
}

impl Anchor {
    pub const ALL: [Anchor; 6] = [
        Anchor::Head,
        Anchor::Body,
        Anchor::RightArm,
        Anchor::LeftArm,
        Anchor::RightLeg,
        Anchor::LeftLeg,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// Та же часть с другой стороны (для парных наростов).
    pub fn mirror(self) -> Self {
        match self {
            Anchor::RightArm => Anchor::LeftArm,
            Anchor::LeftArm => Anchor::RightArm,
            Anchor::RightLeg => Anchor::LeftLeg,
            Anchor::LeftLeg => Anchor::RightLeg,
            other => other,
        }
    }
}

/// Конечности гуманоида: правая/левая рука, правая/левая нога.
pub const HUMANOID: [&str; 4] = ["right_arm", "left_arm", "right_leg", "left_leg"];
/// Конечности четвероногого: передние лапы — «руки», задние — «ноги».
pub const QUADRUPED: [&str; 4] =
    ["right_front_leg", "left_front_leg", "right_hind_leg", "left_hind_leg"];
/// Тело по именам, в порядке предпочтения: у паука `body1` — брюшко, на нём
/// и растёт всё, что на спине (`body0` — маленькая грудь).
pub const BODY_NAMES: [&str; 5] = ["body", "torso", "body1", "body0", "cube"];

/// Имена частей-якорей по списку имён модели (в порядке `Anchor::ALL`).
/// Гуманоид и четвероногое узнаются по полному набору своих конечностей, как
/// в операционной (`biolab_surgery::limbs`); иначе руки — части с `arm` (или
/// крыльями, как у курицы), ноги — с `leg`, сторона — по `right`/`left` в
/// имени. Чего нет, то `None`: черта на этом якоре просто не рисуется.
pub fn anchors<'a>(names: &[&'a str]) -> [Option<&'a str>; 6] {
    let find = |want: &str| names.iter().copied().find(|n| *n == want);
    let head = find("head").or_else(|| names.iter().copied().find(|n| n.contains("head")));
    let body = BODY_NAMES
        .iter()
        .find_map(|b| find(b))
        .or_else(|| names.iter().copied().find(|n| n.starts_with("body")));
    for set in [HUMANOID, QUADRUPED] {
        let limbs = set.map(|n| find(n));
        if limbs.iter().all(Option::is_some) {
            return [head, body, limbs[0], limbs[1], limbs[2], limbs[3]];
        }
    }
    // Запасное правило: сначала по слову в имени, сторона — по имени, а части
    // без стороны идут по порядку модели (правая первой).
    let sided = |kinds: &[&str]| -> [Option<&'a str>; 2] {
        let pool: Vec<&'a str> = kinds
            .iter()
            .flat_map(|k| names.iter().copied().filter(move |n| n.contains(k)))
            .collect();
        let right = pool.iter().copied().find(|n| n.contains("right"));
        let left = pool.iter().copied().find(|n| n.contains("left"));
        let mut plain = pool
            .iter()
            .copied()
            .filter(|n| !n.contains("right") && !n.contains("left"));
        let right = right.or_else(|| plain.next());
        let left = left.or_else(|| plain.next());
        [right, left]
    };
    let [ra, la] = sided(&["arm", "wing"][..]);
    let [rl, ll] = sided(&["leg"][..]);
    [head, body, ra, la, rl, ll]
}

// ---- Немного линейной алгебры (без glam, чтобы проверять тестами) ----

/// Матрица 3×3 по строкам.
pub type M3 = [[f32; 3]; 3];
pub const I3: M3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

pub fn mul3(a: &M3, b: &M3) -> M3 {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum()))
}

pub fn apply3(a: &M3, v: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|r| a[r][0] * v[0] + a[r][1] * v[1] + a[r][2] * v[2])
}

pub fn transpose(a: &M3) -> M3 {
    std::array::from_fn(|r| std::array::from_fn(|c| a[c][r]))
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// Поворот по углам `[x, y, z]` в том же порядке, что у частей модели:
/// Rz·Ry·Rx (`ModelPart.translateAndRotate`, `glam::EulerRot::ZYX`).
pub fn rot(r: [f32; 3]) -> M3 {
    let (sx, cx) = r[0].sin_cos();
    let (sy, cy) = r[1].sin_cos();
    let (sz, cz) = r[2].sin_cos();
    let rx = [[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]];
    let ry = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let rz = [[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]];
    mul3(&rz, &mul3(&ry, &rx))
}

/// Поворот со сдвигом: точка `p` уходит в `r·p + t`. Сдвиг — в пикселях модели.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xf {
    pub r: M3,
    pub t: [f32; 3],
}

impl Xf {
    pub const ID: Xf = Xf { r: I3, t: [0.0; 3] };

    pub fn turn(r: M3) -> Xf {
        Xf { r, t: [0.0; 3] }
    }

    pub fn shift(t: [f32; 3]) -> Xf {
        Xf { r: I3, t }
    }

    /// Сначала `o`, потом `self` (как произведение матриц `self · o`).
    pub fn then(&self, o: &Xf) -> Xf {
        Xf { r: mul3(&self.r, &o.r), t: add(apply3(&self.r, o.t), self.t) }
    }

    pub fn point(&self, p: [f32; 3]) -> [f32; 3] {
        add(apply3(&self.r, p), self.t)
    }
}

/// Матрица 4×4 по столбцам (как `glam::Mat4::from_cols_array`) для отрисовки:
/// поворот с равномерным масштабом и сдвиг из пикселей в блоки (вершины
/// частей отрисовка делит на 16, а сдвиг матрицы — уже в блоках).
pub fn to_cols(xf: &Xf, scale: f32) -> [f32; 16] {
    let r = &xf.r;
    [
        r[0][0] * scale, r[1][0] * scale, r[2][0] * scale, 0.0, //
        r[0][1] * scale, r[1][1] * scale, r[2][1] * scale, 0.0, //
        r[0][2] * scale, r[1][2] * scale, r[2][2] * scale, 0.0, //
        xf.t[0] / 16.0, xf.t[1] / 16.0, xf.t[2] / 16.0, 1.0,
    ]
}

// ---- Кадр якоря ----

/// Канонический кадр части-якоря: как перевести канонические координаты в
/// систему самой части и где её габарит.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// Канонический кадр → система части (только поворот, строки).
    pub to_local: M3,
    /// Габарит части в каноническом кадре, пиксели.
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// «Вниз» мира (по модели в покое) в каноническом кадре: туда падают капли.
    pub down: [f32; 3],
}

/// Канонические оси лежачего тела в системе модели (столбцы): x — x,
/// y («к хвосту») — +Z модели (морда у мобов смотрит в −Z), z («к спине») —
/// вверх, то есть −Y модели (ось Y модели смотрит вниз).
const LYING_AXES: M3 = [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]];

impl Frame {
    /// Кадр по повороту части в покое `rest` (система части → система модели,
    /// строки; масштаб отбрасывается) и её габариту в своей системе.
    ///
    /// Голова и конечности: канонический кадр — оси модели. Тело: если оно
    /// выше, чем длиннее (гуманоид, крипер, слизень), — тоже оси модели, спина
    /// смотрит в +Z; если длиннее (корова, паук, курица) — тело лежачее, «вниз
    /// по телу» — к хвосту, «к спине» — вверх.
    pub fn new(rest: M3, min: [f32; 3], max: [f32; 3], body: bool) -> Frame {
        let mut r = rest;
        for c in 0..3 {
            let len = (0..3).map(|k| r[k][c] * r[k][c]).sum::<f32>().sqrt();
            if len > 1e-6 {
                for row in r.iter_mut() {
                    row[c] /= len;
                }
            }
        }
        let corners = corners(min, max);
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for c in corners {
            let p = apply3(&r, c);
            for i in 0..3 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
        let upright = !body || hi[1] - lo[1] >= hi[2] - lo[2];
        let axes = if upright { I3 } else { LYING_AXES };
        let mut to_local = mul3(&transpose(&r), &axes);
        // Почти прямые углы — ровно прямые: кубы встают по осям части.
        for row in to_local.iter_mut() {
            for v in row.iter_mut() {
                for snap in [-1.0, 0.0, 1.0] {
                    if (*v - snap).abs() < 1e-3 {
                        *v = snap;
                    }
                }
            }
        }
        let back = transpose(&to_local);
        let mut cmin = [f32::MAX; 3];
        let mut cmax = [f32::MIN; 3];
        for c in corners {
            let p = apply3(&back, c);
            for i in 0..3 {
                cmin[i] = cmin[i].min(p[i]);
                cmax[i] = cmax[i].max(p[i]);
            }
        }
        Frame { to_local, min: cmin, max: cmax, down: apply3(&transpose(&axes), [0.0, 1.0, 0.0]) }
    }

    /// Точка по долям габарита (0 — минимум, 1 — максимум по каждой оси).
    pub fn point(&self, at: [f32; 3]) -> [f32; 3] {
        [0, 1, 2].map(|i| self.min[i] + (self.max[i] - self.min[i]) * at[i])
    }

    /// Из канонического кадра в систему части.
    pub fn local(&self) -> Xf {
        Xf::turn(self.to_local)
    }
}

fn corners(a: [f32; 3], b: [f32; 3]) -> [[f32; 3]; 8] {
    [
        [a[0], a[1], a[2]],
        [b[0], a[1], a[2]],
        [a[0], b[1], a[2]],
        [b[0], b[1], a[2]],
        [a[0], a[1], b[2]],
        [b[0], a[1], b[2]],
        [a[0], b[1], b[2]],
        [b[0], b[1], b[2]],
    ]
}

// ---- Материалы и развёртка ----

/// Чем окрашен нарост.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Paint {
    /// Цветом тела мутанта (с его оттенком).
    Tinted,
    /// Своим цветом из текстуры: кость, рог, глаза, кислота.
    Natural,
    /// Своим цветом и полной яркостью: светится и ночью.
    Glowing,
}

/// Материал — блок 32×16 в текстуре. Порядок — по строкам слева направо;
/// генератор текстуры (`tools/mkgui_mutations.py`) раскладывает так же.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    Chitin,
    Bone,
    Horn,
    Flesh,
    Membrane,
    Eye,
    Gill,
    Acid,
    Glow,
}

impl Material {
    pub const ALL: [Material; 9] = [
        Material::Chitin,
        Material::Bone,
        Material::Horn,
        Material::Flesh,
        Material::Membrane,
        Material::Eye,
        Material::Gill,
        Material::Acid,
        Material::Glow,
    ];

    /// Левый верхний угол блока в текстуре, пиксели.
    pub fn origin(self) -> [f32; 2] {
        let i = self as usize;
        [(i % 4) as f32 * BLOCK[0], (i / 4) as f32 * BLOCK[1]]
    }

    pub fn paint(self) -> Paint {
        match self {
            Material::Flesh | Material::Membrane => Paint::Tinted,
            Material::Glow => Paint::Glowing,
            _ => Paint::Natural,
        }
    }
}

/// Размеры развёртки куба: размеры при силе 1, округлённые вверх (не меньше 1).
pub fn net(size: [f32; 3]) -> [f32; 3] {
    size.map(|s| s.ceil().max(1.0))
}

/// Сколько места развёртка занимает в текстуре: `2·(w+d)` × `d+h`, как у
/// развёртки кубов в моделях Minecraft.
pub fn net_extent(net: [f32; 3]) -> [f32; 2] {
    [2.0 * (net[0] + net[2]), net[2] + net[1]]
}

/// Грань модели: четыре вершины по кругу (пиксели в системе якоря), UV в
/// долях текстуры и нормаль.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub pos: [[f32; 3]; 4],
    pub uv: [[f32; 2]; 4],
    pub normal: [f32; 3],
}

/// Куб `from`…`from + size` через `xf` с развёрткой `net` от угла `origin`.
/// Раскладка граней: верхний ряд — верх и низ, нижний — +X, перед (−Z), −X,
/// зад (+Z). Глаза нарисованы и на переде, и на заде (`mkgui_mutations.py`).
pub fn cuboid(
    xf: &Xf,
    from: [f32; 3],
    size: [f32; 3],
    origin: [f32; 2],
    net: [f32; 3],
    out: &mut Vec<Quad>,
) {
    let [x0, y0, z0] = from;
    let [x1, y1, z1] = add(from, size);
    let [w, h, d] = net;
    let [u, v] = origin;
    let faces: [([[f32; 3]; 4], [f32; 3], [f32; 4]); 6] = [
        (
            [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
            [0.0, -1.0, 0.0],
            [u + d, v, u + d + w, v + d],
        ),
        (
            [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
            [0.0, 1.0, 0.0],
            [u + d + w, v, u + d + 2.0 * w, v + d],
        ),
        (
            [[x1, y0, z0], [x1, y0, z1], [x1, y1, z1], [x1, y1, z0]],
            [1.0, 0.0, 0.0],
            [u, v + d, u + d, v + d + h],
        ),
        (
            [[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]],
            [0.0, 0.0, -1.0],
            [u + d, v + d, u + d + w, v + d + h],
        ),
        (
            [[x0, y0, z1], [x0, y0, z0], [x0, y1, z0], [x0, y1, z1]],
            [-1.0, 0.0, 0.0],
            [u + d + w, v + d, u + 2.0 * d + w, v + d + h],
        ),
        (
            [[x1, y0, z1], [x0, y0, z1], [x0, y1, z1], [x1, y1, z1]],
            [0.0, 0.0, 1.0],
            [u + 2.0 * d + w, v + d, u + 2.0 * d + 2.0 * w, v + d + h],
        ),
    ];
    for (corners, n, [u0, v0, u1, v1]) in faces {
        let uv = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]].map(|[a, b]| [a / TEX[0], b / TEX[1]]);
        out.push(Quad { pos: corners.map(|p| xf.point(p)), uv, normal: apply3(&xf.r, n) });
    }
}

// ---- Описание наростов ----

/// Как куб двигается.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Still,
    /// Дышит: размер чуть меняется со временем (опухоль, пузыри кислоты).
    Pulse,
    /// Капля: падает от точки крепления «вниз» мира и появляется снова.
    Drip,
    /// Челюсть: приоткрывается и распахивается при ударе.
    Jaw,
}

/// Один куб нароста.
#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub anchor: Anchor,
    /// Точка крепления — доля габарита якоря в каноническом кадре.
    pub at: [f32; 3],
    /// Угол куба от точки крепления и его размер, пиксели (при силе 1).
    pub from: [f32; 3],
    pub size: [f32; 3],
    /// Наклон куба вокруг точки крепления, радианы (Rz·Ry·Rx).
    pub rot: [f32; 3],
    pub material: Material,
    /// С какой силы гена куб появляется: так сила прибавляет число наростов.
    pub from_strength: u8,
    /// Парный: есть и зеркальная копия с другой стороны (на парном якоре).
    pub pair: bool,
    pub motion: Move,
}

/// Цепочка сегментов: хвост, щупальце, рог, крыло. Сегмент — куб вдоль своей
/// оси +Y; следующий крепится к концу предыдущего и повёрнут на `curl` плюс
/// волну `sway`.
#[derive(Clone, Copy, Debug)]
pub struct Chain {
    pub anchor: Anchor,
    pub at: [f32; 3],
    /// Сдвиг основания от точки крепления, пиксели.
    pub off: [f32; 3],
    /// Направление основания: ось +Y первого сегмента повёрнута на эти углы.
    pub rot: [f32; 3],
    /// Сегментов при силе 1 и сколько прибавляется за каждую ступень силы.
    pub segments: u8,
    pub extra: u8,
    /// Длина сегмента и толщина у основания и на конце (при силе 1).
    pub length: f32,
    pub thick: [f32; 2],
    /// Плоская: сегмент — пластина шириной `thick` (крыло), иначе брусок.
    pub flat: bool,
    /// Постоянный изгиб в каждом суставе после первого (радианы по осям).
    pub curl: [f32; 3],
    /// Размах волны по осям, её скорость (рад/тик), запаздывание к концу, фаза.
    pub sway: [f32; 3],
    pub speed: f32,
    pub lag: f32,
    pub phase: f32,
    pub material: Material,
    pub from_strength: u8,
    pub pair: bool,
}

/// Черта: ген и его наросты.
#[derive(Clone, Copy, Debug)]
pub struct Trait {
    pub gene: &'static str,
    pub pieces: &'static [Piece],
    pub chains: &'static [Chain],
}

const P: Piece = Piece {
    anchor: Anchor::Body,
    at: [0.5; 3],
    from: [0.0; 3],
    size: [1.0; 3],
    rot: [0.0; 3],
    material: Material::Flesh,
    from_strength: 1,
    pair: false,
    motion: Move::Still,
};

const C: Chain = Chain {
    anchor: Anchor::Body,
    at: [0.5; 3],
    off: [0.0; 3],
    rot: [0.0; 3],
    segments: 3,
    extra: 0,
    length: 3.0,
    thick: [2.0, 1.0],
    flat: false,
    curl: [0.0; 3],
    sway: [0.0; 3],
    speed: 0.0,
    lag: 0.0,
    phase: 0.0,
    material: Material::Flesh,
    from_strength: 1,
    pair: false,
};

/// Поворот, после которого грань +Z куба смотрит вбок, в −X (на правый бок):
/// так глаза и пятна, нарисованные на «спине» куба, встают на бока.
const SIDE: [f32; 3] = [0.0, -FRAC_PI_2, 0.0];
/// Поворот, после которого грань +Z смотрит вверх (макушка головы).
const UP: [f32; 3] = [FRAC_PI_2, 0.0, 0.0];

// Хитин: пластины внахлёст вдоль спины, наплечники.
const PLATE: Piece = Piece {
    from: [-3.0, 0.0, -0.6],
    size: [6.0, 3.0, 1.2],
    rot: [-0.25, 0.0, 0.0],
    material: Material::Chitin,
    ..P
};
const CHITIN: [Piece; 5] = [
    Piece { at: [0.5, 0.06, 1.0], ..PLATE },
    Piece { at: [0.5, 0.28, 1.0], ..PLATE },
    Piece { at: [0.5, 0.5, 1.0], from_strength: 2, ..PLATE },
    Piece { at: [0.5, 0.72, 1.0], from_strength: 3, ..PLATE },
    Piece {
        anchor: Anchor::RightArm,
        at: [0.5, 0.0, 0.5],
        from: [-2.6, -1.4, -2.6],
        size: [5.2, 1.6, 5.2],
        rot: [0.0, 0.0, 0.25],
        pair: true,
        ..PLATE
    },
];

// Костяные шипы: гребень вдоль спины (назад к хвосту), шипы на руках наружу.
const SPIKE: Piece = Piece {
    from: [-0.5, -0.5, -0.3],
    size: [1.0, 1.0, 3.0],
    rot: [-0.5, 0.0, 0.0],
    material: Material::Bone,
    ..P
};
const ARM_SPIKE: Piece = Piece {
    anchor: Anchor::RightArm,
    from: [-2.5, -0.5, -0.5],
    size: [2.5, 1.0, 1.0],
    rot: [0.0, 0.0, -0.4],
    material: Material::Bone,
    pair: true,
    ..P
};
const THORNS: [Piece; 7] = [
    Piece { at: [0.5, 0.1, 1.0], ..SPIKE },
    Piece { at: [0.5, 0.3, 1.0], ..SPIKE },
    Piece { at: [0.5, 0.5, 1.0], from_strength: 2, ..SPIKE },
    Piece { at: [0.5, 0.7, 1.0], from_strength: 2, ..SPIKE },
    Piece { at: [0.5, 0.9, 1.0], size: [1.0, 1.0, 2.5], from_strength: 3, ..SPIKE },
    Piece { at: [0.0, 0.3, 0.5], ..ARM_SPIKE },
    Piece { at: [0.0, 0.65, 0.5], from_strength: 2, ..ARM_SPIKE },
];

// Рвущие когти: из конца руки (лапы), загнуты вперёд.
const CLAW: Piece = Piece {
    anchor: Anchor::RightArm,
    at: [0.5, 1.0, 0.5],
    size: [0.6, 3.5, 0.6],
    rot: [-0.45, 0.0, 0.0],
    material: Material::Horn,
    pair: true,
    ..P
};
const CLAWS: [Piece; 3] = [
    Piece { from: [-1.6, -0.5, -0.3], ..CLAW },
    Piece { from: [1.0, -0.5, -0.3], ..CLAW },
    Piece { from: [-0.3, -0.5, -0.3], size: [0.6, 4.0, 0.6], from_strength: 2, ..CLAW },
];

// Пружинные ноги: обратное колено (выступ назад), сухожилие к пятке, шпора.
const LEG: Piece = Piece { anchor: Anchor::RightLeg, pair: true, ..P };
const SPRING_LEGS: [Piece; 3] = [
    Piece { at: [0.5, 0.5, 1.0], from: [-1.0, -0.5, -0.5], size: [2.0, 1.5, 3.0], rot: [0.35, 0.0, 0.0], ..LEG },
    Piece { at: [0.5, 0.5, 1.0], from: [-0.6, 0.0, 1.0], size: [1.2, 5.0, 1.2], rot: [-0.3, 0.0, 0.0], ..LEG },
    Piece {
        at: [0.5, 1.0, 1.0],
        from: [-0.5, -1.5, -0.2],
        size: [1.0, 1.0, 2.0],
        rot: [0.3, 0.0, 0.0],
        material: Material::Bone,
        from_strength: 2,
        ..LEG
    },
];

// Жабры: щели на боках головы, ближе к затылку и низу.
const GILL: Piece = Piece {
    anchor: Anchor::Head,
    at: [0.0, 0.7, 0.55],
    size: [0.6, 3.0, 0.8],
    material: Material::Gill,
    pair: true,
    ..P
};
const GILLS: [Piece; 3] = [
    Piece { from: [-0.5, -1.5, -0.4], ..GILL },
    Piece { from: [-0.5, -1.5, 0.8], from_strength: 2, ..GILL },
    Piece { from: [-0.5, -1.5, -1.6], from_strength: 3, ..GILL },
];

// Кислотные железы: пульсирующие пузыри на боках и спине, с них капает.
const BUBBLE: Piece = Piece {
    from: [-1.5, -1.0, -1.0],
    size: [2.0, 2.0, 2.0],
    material: Material::Acid,
    motion: Move::Pulse,
    ..P
};
const DROP: Piece = Piece {
    from: [-1.0, -0.4, -0.4],
    size: [0.8, 1.2, 0.8],
    material: Material::Acid,
    motion: Move::Drip,
    ..P
};
const ACID: [Piece; 6] = [
    Piece { at: [0.0, 0.35, 0.5], ..BUBBLE },
    Piece { at: [0.0, 0.35, 0.5], ..DROP },
    Piece { at: [0.35, 0.55, 1.0], from: [-1.0, -1.0, -0.5], ..BUBBLE },
    // Левый бок — со второй силы: записан отдельно, уже зеркальным.
    Piece { at: [1.0, 0.45, 0.5], from: [-0.5, -1.0, -1.0], from_strength: 2, ..BUBBLE },
    Piece { at: [1.0, 0.45, 0.5], from: [0.2, -0.4, -0.4], from_strength: 2, ..DROP },
    Piece { at: [0.7, 0.25, 1.0], from: [-1.2, -1.2, -0.5], size: [2.4, 2.4, 2.4], from_strength: 3, ..BUBBLE },
];

// Биолюминесценция: светящиеся пятна на спине, боках и голове.
const SPOT: Piece = Piece {
    from: [-1.0, -1.0, -0.2],
    size: [2.0, 2.0, 0.6],
    material: Material::Glow,
    ..P
};
const SMALL_SPOT: Piece = Piece { from: [-0.8, -0.8, -0.2], size: [1.6, 1.6, 0.6], ..SPOT };
const GLOW: [Piece; 8] = [
    Piece { at: [0.3, 0.25, 1.0], ..SPOT },
    Piece { at: [0.7, 0.5, 1.0], ..SPOT },
    Piece { anchor: Anchor::Head, at: [0.0, 0.45, 0.5], rot: SIDE, pair: true, ..SPOT },
    Piece { at: [0.4, 0.75, 1.0], from_strength: 2, ..SPOT },
    Piece { at: [0.0, 0.45, 0.5], rot: SIDE, pair: true, from_strength: 2, ..SPOT },
    Piece { at: [0.65, 0.12, 1.0], from_strength: 3, ..SMALL_SPOT },
    Piece { at: [0.0, 0.75, 0.65], rot: SIDE, pair: true, from_strength: 3, ..SMALL_SPOT },
    Piece { anchor: Anchor::Head, at: [0.5, 0.0, 0.5], rot: UP, from_strength: 3, ..SPOT },
];

// Хищник: выдвинутая нижняя челюсть с клыками.
const JAW: Piece = Piece { anchor: Anchor::Head, at: [0.5, 1.0, 0.0], motion: Move::Jaw, ..P };
const FANG: Piece = Piece { material: Material::Bone, pair: true, ..JAW };
const PREDATOR: [Piece; 4] = [
    Piece { from: [-3.0, -1.2, -3.0], size: [6.0, 2.0, 4.0], ..JAW },
    Piece { from: [-2.6, -3.0, -2.7], size: [0.8, 2.0, 0.8], ..FANG },
    // Верхние клыки сидят в голове, челюсть их не двигает.
    Piece { from: [-1.6, -0.6, -1.2], size: [0.7, 1.8, 0.7], from_strength: 2, motion: Move::Still, ..FANG },
    Piece { from: [-3.2, -2.2, -1.2], size: [0.7, 2.5, 0.7], rot: [0.0, 0.0, 0.3], from_strength: 3, ..FANG },
];

// Плотная кость: наросты на плечах, потом на коленях, потом на локтях.
const KNOB: Piece = Piece { material: Material::Bone, pair: true, ..P };
const DENSE_BONE: [Piece; 3] = [
    Piece { anchor: Anchor::RightArm, at: [0.0, 0.05, 0.5], from: [-1.2, -1.2, -1.3], size: [2.2, 2.2, 2.6], ..KNOB },
    Piece {
        anchor: Anchor::RightLeg,
        at: [0.5, 0.5, 0.0],
        from: [-1.2, -1.0, -1.0],
        size: [2.4, 2.0, 1.6],
        from_strength: 2,
        ..KNOB
    },
    Piece {
        anchor: Anchor::RightArm,
        at: [0.0, 0.55, 0.6],
        from: [-1.0, -0.8, -0.8],
        size: [1.6, 1.6, 1.6],
        from_strength: 3,
        ..KNOB
    },
];

// Многоглазие: глаза на спине, груди, боках и голове (развёртка 3×3×1 —
// глаз нарисован на переде и заде куба).
const EYE: Piece = Piece {
    from: [-1.5, -1.5, -0.2],
    size: [3.0, 3.0, 1.0],
    material: Material::Eye,
    ..P
};
const POLYOPIA: [Piece; 7] = [
    Piece { at: [0.5, 0.3, 1.0], ..EYE },
    Piece { anchor: Anchor::Head, at: [0.5, 0.2, 0.0], from: [-1.5, -1.5, -0.8], ..EYE },
    Piece { at: [0.3, 0.6, 1.0], from_strength: 2, ..EYE },
    Piece { at: [0.7, 0.6, 1.0], from_strength: 2, ..EYE },
    Piece { at: [0.5, 0.35, 0.0], from: [-1.5, -1.5, -0.8], from_strength: 2, ..EYE },
    Piece { at: [0.0, 0.4, 0.5], rot: SIDE, pair: true, from_strength: 3, ..EYE },
    Piece { anchor: Anchor::Head, at: [0.0, 0.3, 0.4], rot: SIDE, pair: true, from_strength: 3, ..EYE },
];

// Опухоль: горб, потом раздутый бок, потом ещё шишка. Дышит.
const LUMP: Piece = Piece { motion: Move::Pulse, ..P };
const TUMOR: [Piece; 3] = [
    Piece { at: [0.55, 0.15, 1.0], from: [-2.5, -2.0, -0.8], size: [5.0, 4.0, 3.5], ..LUMP },
    Piece { at: [0.0, 0.55, 0.5], from: [-2.2, -2.5, -2.0], size: [3.0, 5.0, 4.0], from_strength: 2, ..LUMP },
    Piece { at: [0.3, 0.4, 1.0], from: [-1.5, -1.5, -0.5], size: [3.0, 3.0, 2.5], from_strength: 3, ..LUMP },
];

// Перепонки на руках: кожистый лоскут от руки к телу (со второй силы).
const MEMBRANE_FLAPS: [Piece; 1] = [Piece {
    anchor: Anchor::RightArm,
    at: [1.0, 0.1, 0.8],
    from: [0.0, 0.0, -0.25],
    size: [4.0, 8.0, 0.5],
    material: Material::Membrane,
    pair: true,
    from_strength: 2,
    ..P
}];

// Рога: пара от макушки, вверх и в стороны, загибаются назад.
const HORNS: [Chain; 1] = [Chain {
    anchor: Anchor::Head,
    at: [0.25, 0.0, 0.45],
    rot: [0.3, 0.0, 2.62],
    segments: 2,
    extra: 1,
    length: 2.2,
    thick: [2.2, 0.7],
    curl: [0.35, 0.0, 0.0],
    material: Material::Horn,
    pair: true,
    ..C
}];

// Хвост: от низа спины (у лежачих — от конца тела), качается из стороны в сторону.
const TAIL: [Chain; 1] = [Chain {
    at: [0.5, 1.0, 0.8],
    off: [0.0, -0.5, 0.0],
    rot: [0.6, 0.0, 0.0],
    segments: 3,
    extra: 1,
    length: 3.0,
    thick: [2.4, 0.9],
    curl: [0.18, 0.0, 0.0],
    sway: [0.05, 0.0, 0.3],
    speed: 0.12,
    lag: 0.6,
    ..C
}];

// Щупальца: 2–4 из спины, извиваются, концы загибаются вверх (к голове).
const TENTACLE: Chain = Chain {
    segments: 5,
    length: 2.2,
    thick: [1.8, 0.6],
    curl: [0.15, 0.0, 0.0],
    sway: [0.3, 0.0, 0.25],
    speed: 0.14,
    lag: 0.7,
    ..C
};
const TENTACLES: [Chain; 3] = [
    Chain { at: [0.3, 0.2, 1.0], rot: [FRAC_PI_2, -0.5, 0.0], pair: true, ..TENTACLE },
    Chain {
        at: [0.5, 0.45, 1.0],
        rot: [FRAC_PI_2 - 0.3, 0.0, 0.0],
        phase: 2.1,
        from_strength: 2,
        ..TENTACLE
    },
    Chain {
        at: [0.5, 0.7, 1.0],
        rot: [FRAC_PI_2 + 0.3, 0.0, 0.0],
        phase: 4.2,
        from_strength: 3,
        ..TENTACLE
    },
];

// Перепонки: пара кожистых крыльев от лопаток в стороны, медленно машут.
const WINGS: [Chain; 1] = [Chain {
    at: [0.3, 0.12, 1.0],
    off: [0.0, 0.0, 0.3],
    rot: [0.45, 0.0, FRAC_PI_2],
    segments: 2,
    length: 5.0,
    thick: [7.0, 5.0],
    flat: true,
    curl: [0.35, 0.0, 0.0],
    sway: [0.25, 0.0, 0.0],
    speed: 0.18,
    lag: 0.4,
    material: Material::Membrane,
    pair: true,
    ..C
}];

/// Черты, которые рисуются своими кубами (копии частей — `extra_arms`,
/// `second_head`).
pub const TABLE: [Trait; 15] = [
    Trait { gene: "chitin", pieces: &CHITIN, chains: &[] },
    Trait { gene: "thorns", pieces: &THORNS, chains: &[] },
    Trait { gene: "rending_claws", pieces: &CLAWS, chains: &[] },
    Trait { gene: "spring_legs", pieces: &SPRING_LEGS, chains: &[] },
    Trait { gene: "aquatic", pieces: &GILLS, chains: &[] },
    Trait { gene: "acid_glands", pieces: &ACID, chains: &[] },
    Trait { gene: "bioluminescence", pieces: &GLOW, chains: &[] },
    Trait { gene: "predator", pieces: &PREDATOR, chains: &[] },
    Trait { gene: "dense_bone", pieces: &DENSE_BONE, chains: &[] },
    Trait { gene: "horns", pieces: &[], chains: &HORNS },
    Trait { gene: "tail", pieces: &[], chains: &TAIL },
    Trait { gene: "tentacles", pieces: &[], chains: &TENTACLES },
    Trait { gene: "polyopia", pieces: &POLYOPIA, chains: &[] },
    Trait { gene: "tumor", pieces: &TUMOR, chains: &[] },
    Trait { gene: "membranes", pieces: &MEMBRANE_FLAPS, chains: &WINGS },
];

// ---- Сборка ----

/// Во сколько раз наросты крупнее при силе гена 1–3.
pub fn grow(strength: u8) -> f32 {
    0.75 + 0.25 * f32::from(strength.clamp(1, 3))
}

/// Что анимации нужно знать о мобе: возраст в тиках (с долей кадра), шаг
/// (`WalkAnimationState`: пройденное и скорость) и замах удара 0…1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Motion {
    pub age: f32,
    pub walk: f32,
    pub speed: f32,
    pub swing: f32,
}

/// Грани одного якоря одной окраски: рисуются одним вызовом.
#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    pub anchor: Anchor,
    pub paint: Paint,
    pub quads: Vec<Quad>,
}

/// Зеркальная копия куба: отражение через середину якоря по X. Отражение
/// поворота Rz·Ry·Rx — те же углы с обратным знаком у Y и Z.
pub fn mirrored(p: &Piece) -> Piece {
    Piece {
        anchor: p.anchor.mirror(),
        at: [1.0 - p.at[0], p.at[1], p.at[2]],
        from: [-p.from[0] - p.size[0], p.from[1], p.from[2]],
        rot: [p.rot[0], -p.rot[1], -p.rot[2]],
        ..*p
    }
}

fn batch(out: &mut Vec<Batch>, anchor: Anchor, paint: Paint) -> &mut Vec<Quad> {
    let i = match out.iter().position(|b| b.anchor == anchor && b.paint == paint) {
        Some(i) => i,
        None => {
            out.push(Batch { anchor, paint, quads: Vec::new() });
            out.len() - 1
        }
    };
    &mut out[i].quads
}

/// Небольшая разница фаз между кубами, чтобы пузыри и капли не дышали хором.
fn phase_of(at: [f32; 3]) -> f32 {
    at[0] * 5.1 + at[1] * 7.3 + at[2] * 3.7
}

fn piece_quads(p: &Piece, frame: &Frame, strength: u8, m: &Motion, out: &mut Vec<Quad>) {
    let mut k = grow(strength);
    let phase = phase_of(p.at);
    if p.motion == Move::Pulse {
        k *= 1.0 + 0.06 * (m.age * 0.2 + phase).sin();
    }
    let mut at = frame.point(p.at);
    if p.motion == Move::Drip {
        // Капля проходит 5 пикселей вниз примерно за 20 тиков и начинается снова.
        let fall = (m.age * 0.05 + phase).rem_euclid(1.0) * 5.0 * k;
        at = add(at, frame.down.map(|d| d * fall));
    }
    let mut r = p.rot;
    if p.motion == Move::Jaw {
        r[0] += 0.12 + 0.1 * (m.age * 0.08).sin() + 0.5 * (m.swing.clamp(0.0, 1.0) * PI).sin();
    }
    let xf = frame.local().then(&Xf::shift(at)).then(&Xf::turn(rot(r)));
    cuboid(&xf, p.from.map(|v| v * k), p.size.map(|v| v * k), p.material.origin(), net(p.size), out);
}

/// Число сегментов цепочки при этой силе.
pub fn chain_segments(c: &Chain, strength: u8) -> u8 {
    c.segments + c.extra * (strength.clamp(1, 3) - 1)
}

/// Размер первого сегмента цепочки при силе 1: по нему развёртка в текстуре.
pub fn chain_net(c: &Chain) -> [f32; 3] {
    if c.flat {
        net([c.thick[0], c.length, 0.5])
    } else {
        net([c.thick[0], c.length, c.thick[0]])
    }
}

fn chain_quads(c: &Chain, frame: &Frame, strength: u8, left: bool, m: &Motion, out: &mut Vec<Quad>) {
    let k = grow(strength);
    let sign = if left { -1.0 } else { 1.0 };
    let at = if left { [1.0 - c.at[0], c.at[1], c.at[2]] } else { c.at };
    let off = [c.off[0] * sign, c.off[1], c.off[2]];
    let base = [c.rot[0], c.rot[1] * sign, c.rot[2] * sign];
    let mut xf = frame
        .local()
        .then(&Xf::shift(add(frame.point(at), off)))
        .then(&Xf::turn(rot(base)));
    let n = chain_segments(c, strength);
    let len = c.length * k;
    let thick_k = k.sqrt();
    let uv_net = chain_net(c);
    for i in 0..n {
        let t = m.age * c.speed + c.phase - f32::from(i) * c.lag;
        let curl = if i == 0 { [0.0; 3] } else { c.curl };
        let joint = [
            curl[0] + c.sway[0] * t.sin(),
            (curl[1] + c.sway[1] * (t + 1.3).sin()) * sign,
            (curl[2] + c.sway[2] * t.cos()) * sign,
        ];
        xf = xf.then(&Xf::turn(rot(joint)));
        let f = if n > 1 { f32::from(i) / f32::from(n - 1) } else { 0.0 };
        let thick = (c.thick[0] + (c.thick[1] - c.thick[0]) * f) * thick_k;
        let (from, size) = if c.flat {
            // Пластина свисает от сустава в сторону +X сегмента; у зеркальной — в −X.
            let x = if left { 1.0 - thick } else { -1.0 };
            ([x, 0.0, -0.25], [thick, len, 0.5])
        } else {
            ([-thick / 2.0, 0.0, -thick / 2.0], [thick, len, thick])
        };
        cuboid(&xf, from, size, c.material.origin(), uv_net, out);
        xf = xf.then(&Xf::shift([0.0, len, 0.0]));
    }
}

/// Все наросты своей текстуры по силе генов. `strength` — сила гена по имени
/// (0 — гена нет); `frames` — кадры якорей в порядке `Anchor::ALL`, `None` —
/// такой части нет (или она спрятана), и черта там не рисуется. Вершины — в
/// пикселях в системе части-якоря.
pub fn build(strength: &dyn Fn(&str) -> u8, frames: &[Option<Frame>; 6], m: Motion) -> Vec<Batch> {
    let mut out = Vec::new();
    for t in &TABLE {
        let s = strength(t.gene).min(3);
        if s == 0 {
            continue;
        }
        for p in t.pieces.iter().filter(|p| p.from_strength <= s) {
            let sides = if p.pair { vec![*p, mirrored(p)] } else { vec![*p] };
            for p in sides {
                let Some(frame) = frames[p.anchor.index()] else { continue };
                piece_quads(&p, &frame, s, &m, batch(&mut out, p.anchor, p.material.paint()));
            }
        }
        for c in t.chains.iter().filter(|c| c.from_strength <= s) {
            let sides: &[bool] = if c.pair { &[false, true] } else { &[false] };
            for &left in sides {
                let anchor = if left { c.anchor.mirror() } else { c.anchor };
                let Some(frame) = frames[anchor.index()] else { continue };
                chain_quads(c, &frame, s, left, &m, batch(&mut out, anchor, c.material.paint()));
            }
        }
    }
    out.retain(|b| !b.quads.is_empty());
    out
}

// ---- Копии частей модели ----

/// Взмах лишней руки (радианы вокруг оси X): шаг со своим сдвигом фазы,
/// лёгкое покачивание на месте и рывок при ударе.
pub fn arm_swing(m: &Motion, phase: f32) -> f32 {
    let walk = (m.walk * 0.6662 + phase).cos() * 1.2 * m.speed;
    let idle = (m.age * 0.09 + phase).sin() * 0.12;
    let strike = (m.swing.clamp(0.0, 1.0) * PI).sin() * 1.1;
    walk + idle + strike
}

/// Лишние руки (полимелия): (левая ли, куда поставить копию руки в системе
/// тела, масштаб). Рука растёт из спины, отведена назад и в сторону и машет
/// со сдвигом фазы на четверть шага; с силой крупнее, при силе 3 — вторая,
/// зеркальная, в противофазе. Копия руки — в своей системе (плечо в начале
/// координат, рука висит по +Y), как часть модели без своего сдвига.
pub fn extra_arms(strength: u8, body: &Frame, m: &Motion) -> Vec<(bool, Xf, f32)> {
    let s = strength.min(3);
    if s == 0 {
        return Vec::new();
    }
    let scale = [0.75, 0.9, 1.0][usize::from(s) - 1];
    let mut out = Vec::new();
    for left in [false, true] {
        if left && s < 3 {
            break;
        }
        let sign = if left { -1.0 } else { 1.0 };
        let mut p = body.point([if left { 0.65 } else { 0.35 }, 0.15, 1.0]);
        p[2] -= 0.5;
        let phase = FRAC_PI_2 + if left { PI } else { 0.0 };
        let theta = 0.9 + arm_swing(m, phase);
        let r = mul3(&rot([0.0, 0.0, 0.3 * sign]), &rot([theta, 0.0, 0.0]));
        out.push((left, body.local().then(&Xf::shift(p)).then(&Xf::turn(r)), scale));
    }
    out
}

/// Вторая голова (дицефалия): куда поставить копию головы в системе самой
/// головы (она уже повёрнута взглядом моба) и её масштаб. Сидит справа и
/// ниже — на плече, клонится наружу и сама озирается.
pub fn second_head(strength: u8, head: &Frame, m: &Motion) -> Option<(Xf, f32)> {
    let s = strength.min(3);
    if s == 0 {
        return None;
    }
    let k = [0.6, 0.8, 1.0][usize::from(s) - 1];
    let p = [head.min[0] - k * head.max[0] - 0.5, (head.max[1] - head.min[1]) * 0.35, 0.0];
    let look = [0.08 * (m.age * 0.07 + 1.0).sin(), 0.35 * (m.age * 0.045).sin(), -0.3];
    // Копия головы — в системе головы: переводим в канонический кадр и обратно.
    let xf = head
        .local()
        .then(&Xf::shift(p))
        .then(&Xf::turn(rot(look)))
        .then(&Xf::turn(transpose(&head.to_local)));
    Some((xf, k))
}

// ---- Игрок от первого лица ----
//
// От первого лица игрок видит только руку (её рисует игра, когда рука пуста
// или держит карту) и то, что само лезет в кадр. На руке — те же наросты, что
// на модели: когти, лоскут перепонки, хитиновый наплечник, шипы, костяные
// наросты. Отдельно от руки — третья рука у плеча и щупальца снизу кадра.
// Рога, хвост, вторая голова, жабры, глаза, челюсть, горб и крылья за спиной
// из глаз не видно — их нет.

/// Черты, которые видны на руке в кадре.
pub const FIRST_PERSON_ARM: [&str; 5] =
    ["rending_claws", "membranes", "chitin", "thorns", "dense_bone"];

/// Наросты на руке в кадре: та же таблица, что у модели, но только черты
/// руки и только на той руке, которую рисует игра (`frame` — её кадр).
pub fn first_person_arm(st: &dyn Fn(&str) -> u8, frame: Frame, left: bool, m: Motion) -> Vec<Batch> {
    let mut frames = [None; 6];
    let anchor = if left { Anchor::LeftArm } else { Anchor::RightArm };
    frames[anchor.index()] = Some(frame);
    let arm_only = |name: &str| if FIRST_PERSON_ARM.contains(&name) { st(name) } else { 0 };
    build(&arm_only, &frames, m)
}

/// Третья рука в кадре: поза — как у руки игрока (`player_arm_pose` с этой
/// стороны и своим замахом), сдвинутая в пространстве вида и уменьшенная.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewArm {
    pub left: bool,
    /// Сдвиг в пространстве вида, блоки: x — вправо, y — вверх, z — назад.
    pub offset: [f32; 3],
    pub scale: f32,
    /// Замах 0…1 для `player_arm_pose`.
    pub attack: f32,
}

/// Где третья рука в кадре относительно своей руки: выше и чуть дальше —
/// растёт из-за плеча и нависает над рукой игрока, не закрывая середину.
pub const VIEW_ARM_OFFSET: [f32; 3] = [0.06, 0.24, 0.1];

/// Замах, запоздавший на долю `delay`: третья рука бьёт следом за своей.
pub fn delayed(swing: f32, delay: f32) -> f32 {
    ((swing.clamp(0.0, 1.0) - delay) / (1.0 - delay)).clamp(0.0, 1.0)
}

/// Лишние руки в кадре (полимелия): правая с первой силы, при силе 3 и левая —
/// как на модели (`extra_arms`). Чуть покачиваются на месте, при ударе бьют
/// следом за рукой игрока; размер — как на модели.
pub fn view_arms(strength: u8, swing: f32, age: f32) -> Vec<ViewArm> {
    let s = strength.min(3);
    if s == 0 {
        return Vec::new();
    }
    let scale = [0.75, 0.9, 1.0][usize::from(s) - 1];
    let mut out = Vec::new();
    for left in [false, true] {
        if left && s < 3 {
            break;
        }
        let sign = if left { -1.0 } else { 1.0 };
        let phase = if left { PI } else { 0.0 };
        let [x, y, z] = VIEW_ARM_OFFSET;
        out.push(ViewArm {
            left,
            offset: [
                sign * (x + 0.01 * (age * 0.07 + phase).cos()),
                y + 0.015 * (age * 0.09 + phase).sin(),
                z,
            ],
            scale,
            attack: delayed(swing, if left { 0.3 } else { 0.15 }),
        });
    }
    out
}

/// Щупальце в кадре: основание (пиксели в пространстве вида: x — вправо,
/// y — вверх, z — назад) и направление первого сегмента. Растёт снизу из-за
/// края кадра вверх, к середине и вперёд; левое — зеркало правого.
#[derive(Clone, Copy, Debug)]
pub struct ViewTentacle {
    pub off: [f32; 3],
    pub rot: [f32; 3],
    pub left: bool,
    pub phase: f32,
    pub from_strength: u8,
}

/// Щупальце в кадре: потолще, чем на модели, — оно ближе к глазам и должно
/// читаться; концы загибаются вперёд, от игрока.
const VIEW_TENTACLE: Chain = Chain {
    segments: 5,
    length: 2.6,
    thick: [2.6, 0.8],
    curl: [-0.08, 0.0, 0.0],
    sway: [0.12, 0.0, 0.1],
    speed: 0.14,
    lag: 0.5,
    ..C
};

/// Щупальца в кадре: два с первой силы (по углам), третье и четвёртое — ниже
/// и шире. Столько же, сколько на модели (`TENTACLES`).
pub const VIEW_TENTACLES: [ViewTentacle; 4] = [
    ViewTentacle { off: [15.0, -15.0, -13.0], rot: [-0.3, 0.0, 0.35], left: false, phase: 0.0, from_strength: 1 },
    ViewTentacle { off: [15.0, -15.0, -13.0], rot: [-0.3, 0.0, 0.35], left: true, phase: 1.3, from_strength: 1 },
    ViewTentacle { off: [20.0, -17.0, -10.0], rot: [-0.2, 0.0, 0.55], left: false, phase: 2.1, from_strength: 2 },
    ViewTentacle { off: [20.0, -17.0, -10.0], rot: [-0.2, 0.0, 0.55], left: true, phase: 4.2, from_strength: 3 },
];

/// Грани щупалец в кадре, в пикселях пространства вида.
pub fn view_tentacles(strength: u8, m: &Motion) -> Vec<Quad> {
    let s = strength.min(3);
    let mut out = Vec::new();
    if s == 0 {
        return out;
    }
    // Точка крепления — начало координат вида: всё место задаёт `off`.
    let origin = Frame { to_local: I3, min: [0.0; 3], max: [0.0; 3], down: [0.0, -1.0, 0.0] };
    for t in VIEW_TENTACLES.iter().filter(|t| t.from_strength <= s) {
        let c = Chain { off: t.off, rot: t.rot, phase: t.phase, ..VIEW_TENTACLE };
        chain_quads(&c, &origin, s, t.left, m, &mut out);
    }
    out
}

// ---- Связь с игрой ----

/// Сила гена у мутанта 0…3. Единственная точка, где наросты читают геном.
#[cfg(not(test))]
pub fn strength(g: &crate::biolab_mutants::Genome, name: &str) -> u8 {
    g.strength_of(name).min(3)
}

#[cfg(not(test))]
fn part_names<'a>(part: &'a crate::mobs::Part, out: &mut Vec<&'a str>) {
    out.push(part.name.as_str());
    for child in &part.children {
        part_names(child, out);
    }
}

#[cfg(not(test))]
fn find_part<'a>(part: &'a crate::mobs::Part, name: &str) -> Option<&'a crate::mobs::Part> {
    if part.name == name {
        return Some(part);
    }
    part.children.iter().find_map(|c| find_part(c, name))
}

/// Поворот части в покое в системе модели (по позам частей от корня).
#[cfg(not(test))]
fn rest_rotation(
    part: &crate::mobs::Part,
    name: &str,
    parent: &crate::biolab_surgery::Affine,
) -> Option<M3> {
    let a = crate::biolab_surgery::part_affine(parent, &part.pose);
    if part.name == name {
        return Some([[a[0], a[1], a[2]], [a[4], a[5], a[6]], [a[8], a[9], a[10]]]);
    }
    part.children.iter().find_map(|c| rest_rotation(c, name, &a))
}

/// Габарит части в своей системе: её собственные грани, а если их нет
/// (пустой шарнир) — грани детей в позе покоя.
#[cfg(not(test))]
fn local_bounds(part: &crate::mobs::Part) -> Option<([f32; 3], [f32; 3])> {
    use crate::biolab_surgery::{Affine, Bounds, IDENTITY, apply, part_affine};
    fn walk(part: &crate::mobs::Part, a: &Affine, b: &mut Bounds) {
        for poly in &part.polygons {
            for v in poly.vertices.iter() {
                b.grow(apply(a, [v[0], v[1], v[2]]));
            }
        }
        for child in &part.children {
            walk(child, &part_affine(a, &child.pose), b);
        }
    }
    let mut b = Bounds::EMPTY;
    for poly in &part.polygons {
        for v in poly.vertices.iter() {
            b.grow([v[0], v[1], v[2]]);
        }
    }
    if b.is_empty() {
        walk(part, &IDENTITY, &mut b);
    }
    (!b.is_empty()).then_some((b.min, b.max))
}

/// Свои грани — одной частью модели без позы: её рисует обычный `draw`.
#[cfg(not(test))]
fn to_part(quads: &[Quad]) -> crate::mobs::Part {
    crate::mobs::Part {
        name: String::new(),
        pose: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        polygons: quads
            .iter()
            .map(|q| crate::mobs::Polygon {
                normal: q.normal,
                vertices: std::array::from_fn(|i| {
                    [q.pos[i][0], q.pos[i][1], q.pos[i][2], q.uv[i][0], q.uv[i][1]]
                }),
                // Наросты видны с обеих сторон: щупальце или перепонку
                // игрок обходит кругом, отсекать задние грани нельзя.
                cull_back: false,
            })
            .collect(),
        children: Vec::new(),
    }
}

/// Проход наростов: после тела моба дорисовать наросты его генома.
///
/// Поза — своя (`anim::setup` ещё раз, как в проходе костей): так наросты
/// идут за головой, руками и ногами в этом же кадре. Срезанные на
/// операционном столе конечности анимация прячет (`flay_hide.rs`), и якорем
/// они не служат: на кости когтей нет. `base` — та же матрица, что у тела
/// (с масштабом генома и поворотом «лёжа»).
#[cfg(not(test))]
#[allow(clippy::too_many_arguments)]
pub fn draw_growths(
    m: &crate::mobs::Mob,
    alpha: f32,
    base: glam::Mat4,
    atlas: crate::mobs::render::Atlas<'_>,
    light: u32,
    data: &crate::mobs::MobData,
    out: &mut Vec<crate::mesher::Vertex>,
) {
    // Игрок (своя модель, чужие игроки, превью в инвентаре) — не моб: его
    // гены приходят пакетом с сервера (`biolab_players`), а геном мобов по id
    // к нему не относится. Модель игрока — гуманоид, и крепления те же.
    let player = crate::biolab_players::is_player(m);
    let genome = if player { None } else { crate::biolab_mutants::genome(m.id) };
    let genes = if player { crate::biolab_players::of_model(m.id) } else { None };
    if genome.is_none() && genes.is_none() {
        return;
    }
    let st = |name: &str| match (&genome, &genes) {
        (Some(g), _) => strength(g, name),
        (None, Some(p)) => p.strength_of(name),
        (None, None) => 0,
    };
    if !TRAITS.iter().any(|t| st(*t) > 0) {
        return;
    }
    let (key, texture) = crate::mobs::render::body(m);
    let model = m
        .kind
        .layer(&key)
        .cloned()
        .unwrap_or_else(|| m.kind.model.clone());
    let s = crate::mobs::anim::state(m, alpha);
    let mut poses = crate::mobs::anim::Poses::new(&model);
    crate::mobs::anim::setup(m, &s, &mut poses, &data.animations);
    let mut names = Vec::new();
    part_names(&model, &mut names);
    let found = anchors(&names);
    let mut frames: [Option<Frame>; 6] = [None; 6];
    let mut mats: [Option<glam::Mat4>; 6] = [None; 6];
    for (i, name) in found.iter().enumerate() {
        let Some(name) = *name else { continue };
        let Some(part) = find_part(&model, name) else { continue };
        if poses.is_hidden(part) {
            continue;
        }
        let Some(rest) = rest_rotation(&model, name, &crate::biolab_surgery::IDENTITY) else {
            continue;
        };
        let Some((min, max)) = local_bounds(part) else { continue };
        let Some(mat) = crate::mobs::render::part_matrix(&model, base, &poses, name, 0.0) else {
            continue;
        };
        frames[i] = Some(Frame::new(rest, min, max, i == Anchor::Body.index()));
        mats[i] = Some(mat);
    }
    let motion = Motion { age: s.age, walk: s.pos, speed: s.speed, swing: s.swing };
    // Красная вспышка от удара — как у тела.
    let overlay = crate::overlay::living(0.0, m.hurt_time > 0 || m.death_time > 0);
    // Цвет тела уже с оттенком мутанта (RETURN на `body_color`, `color.rs`).
    // У игрока это белый: плоть и перепонка остаются цвета текстуры
    // (нейтральная розоватая кожа), а не красятся под скин.
    let tint = crate::mobs::render::body_color(m, alpha);
    let still = crate::mobs::anim::Poses::default();
    if let Some(uv) = atlas(SPRITE) {
        for b in build(&st, &frames, motion) {
            let Some(mat) = mats[b.anchor.index()] else { continue };
            draw_batch(&b, mat, uv, tint, light, overlay, out);
        }
    }
    // Копии частей самой модели: её текстура и цвет тела. Скин игрока без
    // места в атласе (не скачан) игра рисует скином по умолчанию — так же и
    // его третью руку и вторую голову.
    let Some(uv) = atlas(&texture).or_else(|| if player { atlas(&m.kind.texture) } else { None }) else {
        return;
    };
    let arms = st("polymelia");
    if arms > 0 {
        if let (Some(body), Some(body_m)) = (frames[Anchor::Body.index()], mats[Anchor::Body.index()]) {
            for (left, xf, scale) in extra_arms(arms, &body, &motion) {
                let order = if left {
                    [Anchor::LeftArm, Anchor::RightArm]
                } else {
                    [Anchor::RightArm, Anchor::LeftArm]
                };
                let Some(name) = order.iter().find_map(|a| found[a.index()]) else { continue };
                let Some(part) = find_part(&model, name) else { continue };
                let mut arm = part.clone();
                // Место руки — точка на спине: сдвиг части убираем, наклон в покое оставляем.
                arm.pose[0] = 0.0;
                arm.pose[1] = 0.0;
                arm.pose[2] = 0.0;
                let mat = body_m * glam::Mat4::from_cols_array(&to_cols(&xf, scale));
                crate::mobs::render::draw(&arm, mat, &still, uv, tint, light, overlay, out);
            }
        }
    }
    let heads = st("dicephaly");
    if heads > 0 {
        let head = frames[Anchor::Head.index()];
        let head_m = mats[Anchor::Head.index()];
        let part = found[Anchor::Head.index()].and_then(|name| find_part(&model, name));
        if let (Some(head), Some(head_m), Some(part)) = (head, head_m, part) {
            if let Some((xf, scale)) = second_head(heads, &head, &motion) {
                let mut copy = part.clone();
                // Поза головы уже в `head_m`: у копии своей позы нет.
                copy.pose = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
                let mat = head_m * glam::Mat4::from_cols_array(&to_cols(&xf, scale));
                crate::mobs::render::draw(&copy, mat, &still, uv, tint, light, overlay, out);
            }
        }
    }
}

/// Грани одного якоря одной окраски — одной частью модели под матрицей
/// якоря `mat`. Плоть и перепонка — цветом `tint`, остальное своим цветом,
/// свечение — полной яркостью.
#[cfg(not(test))]
fn draw_batch(
    b: &Batch,
    mat: glam::Mat4,
    uv: [f32; 4],
    tint: u32,
    light: u32,
    overlay: u32,
    out: &mut Vec<crate::mesher::Vertex>,
) {
    let still = crate::mobs::anim::Poses::default();
    let (color, lit) = match b.paint {
        Paint::Tinted => (tint, light),
        Paint::Natural => (0xFFFF_FFFF, light),
        Paint::Glowing => (0xFFFF_FFFF, 240 | 240 << 8),
    };
    let start = out.len();
    crate::mobs::render::draw(&to_part(&b.quads), mat, &still, uv, color, lit, overlay, out);
    if b.paint == Paint::Glowing {
        // Как светящиеся слои (глаза паука): без затенения гранью и с
        // отметкой яркого для HDR.
        for v in &mut out[start..] {
            v.normal = [0; 4];
            v.layer |= u32::from(crate::mesher::BRIGHT) << 16;
        }
    }
}

/// Поза части руки в кадре — так же, как её ставит игра
/// (`Game::first_person_arm`): шарнир плеча из модели, без наклона, только
/// поворот наружу на 0,1 рад.
#[cfg(not(test))]
fn first_person_arm_pose(arm: &crate::mobs::Part, left: bool) -> [f32; 9] {
    let mut pose = arm.pose;
    pose[3] = 0.0;
    pose[4] = 0.0;
    pose[5] = if left { -0.1 } else { 0.1 };
    pose
}

/// Наросты на руке в кадре: в HEAD `Game::first_person_arm`, с той же позой
/// руки `m`, что у игры. `arm` — часть руки игрока (её кадр — по её граням),
/// `uv` — текстура наростов в атласе. Свет — как у руки, без вспышки удара:
/// руку в кадре игра тоже рисует без неё.
#[cfg(not(test))]
#[allow(clippy::too_many_arguments)]
pub fn draw_first_person_arm(
    st: &dyn Fn(&str) -> u8,
    arm: &crate::mobs::Part,
    left: bool,
    m: glam::Mat4,
    light: u32,
    age: f32,
    uv: [f32; 4],
    out: &mut Vec<crate::mesher::Vertex>,
) {
    let Some((min, max)) = local_bounds(arm) else {
        return;
    };
    let motion = Motion { age, ..Motion::default() };
    let batches = first_person_arm(st, Frame::new(I3, min, max, false), left, motion);
    if batches.is_empty() {
        return;
    }
    let p = first_person_arm_pose(arm, left);
    // Как `render_model`: сдвиг шарнира (пиксели → блоки), поворот Rz·Ry·Rx, масштаб.
    let mat = m
        * glam::Mat4::from_translation(glam::Vec3::new(p[0], p[1], p[2]) / 16.0)
        * glam::Mat4::from_quat(glam::Quat::from_euler(glam::EulerRot::ZYX, p[5], p[4], p[3]))
        * glam::Mat4::from_scale(glam::Vec3::new(p[6], p[7], p[8]));
    for b in &batches {
        draw_batch(b, mat, uv, 0xFFFF_FFFF, light, 0, out);
    }
}

/// Наросты в кадре, не привязанные к руке: в RETURN `Game::hand_geometry`.
/// Третья рука — копия руки игрока (`arms`: правая, левая) с его скином
/// (`skin`) в позе руки в кадре со своим сдвигом и замахом; щупальца — свои
/// кубы с текстурой наростов (`growth`). `to_world` — кадр рук (вид с
/// покачиванием), как у игры.
#[cfg(not(test))]
#[allow(clippy::too_many_arguments)]
pub fn draw_first_person_view(
    st: &dyn Fn(&str) -> u8,
    arms: [Option<&crate::mobs::Part>; 2],
    to_world: glam::Mat4,
    swing: f32,
    age: f32,
    light: u32,
    skin: Option<[f32; 4]>,
    growth: Option<[f32; 4]>,
    out: &mut Vec<crate::mesher::Vertex>,
) {
    let still = crate::mobs::anim::Poses::default();
    if let Some(uv) = skin {
        for a in view_arms(st("polymelia"), swing, age) {
            let Some(arm) = arms[usize::from(a.left)] else { continue };
            let mut copy = arm.clone();
            copy.pose = first_person_arm_pose(arm, a.left);
            // Масштаб — после позы руки: рука уменьшается на своём месте, а
            // не съезжает к середине кадра.
            let mat = to_world
                * glam::Mat4::from_translation(glam::Vec3::from_array(a.offset))
                * crate::first_person::player_arm_pose(a.left, 0.0, a.attack)
                * glam::Mat4::from_scale(glam::Vec3::splat(a.scale));
            crate::mobs::render::draw(&copy, mat, &still, uv, 0xFFFF_FFFF, light, 0, out);
        }
    }
    if let Some(uv) = growth {
        let motion = Motion { age, swing, ..Motion::default() };
        let quads = view_tentacles(st("tentacles"), &motion);
        if !quads.is_empty() {
            let b = Batch { anchor: Anchor::Body, paint: Material::Flesh.paint(), quads };
            draw_batch(&b, to_world, uv, 0xFFFF_FFFF, light, 0, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZOMBIE: [&str; 8] =
        ["root", "head", "hat", "right_arm", "left_leg", "left_arm", "right_leg", "body"];
    const COW: [&str; 7] =
        ["root", "head", "right_front_leg", "right_hind_leg", "left_hind_leg", "body", "left_front_leg"];
    const SPIDER: [&str; 12] = [
        "root",
        "head",
        "body0",
        "body1",
        "right_hind_leg",
        "left_hind_leg",
        "right_middle_hind_leg",
        "left_middle_hind_leg",
        "right_middle_front_leg",
        "left_middle_front_leg",
        "right_front_leg",
        "left_front_leg",
    ];
    const SLIME: [&str; 5] = ["root", "cube", "right_eye", "left_eye", "mouth"];

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-4)
    }

    // Кадры якорей, как их соберёт игра по позам покоя ванильных моделей.
    fn zombie_frames() -> [Option<Frame>; 6] {
        [
            Some(Frame::new(I3, [-4.0, -8.0, -4.0], [4.0, 0.0, 4.0], false)),
            Some(Frame::new(I3, [-4.0, 0.0, -2.0], [4.0, 12.0, 2.0], true)),
            Some(Frame::new(I3, [-3.0, -2.0, -2.0], [1.0, 10.0, 2.0], false)),
            Some(Frame::new(I3, [-1.0, -2.0, -2.0], [3.0, 10.0, 2.0], false)),
            Some(Frame::new(I3, [-2.0, 0.0, -2.0], [2.0, 12.0, 2.0], false)),
            Some(Frame::new(I3, [-2.0, 0.0, -2.0], [2.0, 12.0, 2.0], false)),
        ]
    }

    /// Тело коровы: куб (−6, −10, −7)…(6, 8, 3), повёрнут на 90° по X.
    fn cow_body() -> Frame {
        Frame::new(rot([FRAC_PI_2, 0.0, 0.0]), [-6.0, -10.0, -7.0], [6.0, 8.0, 3.0], true)
    }

    fn cow_frames() -> [Option<Frame>; 6] {
        let leg = Frame::new(I3, [-2.0, 0.0, -2.0], [2.0, 12.0, 2.0], false);
        [
            Some(Frame::new(I3, [-4.0, -4.0, -6.0], [4.0, 4.0, 0.0], false)),
            Some(cow_body()),
            Some(leg),
            Some(leg),
            Some(leg),
            Some(leg),
        ]
    }

    /// Паук: брюшко `body1` не повёрнуто, длиннее, чем выше; ноги повёрнуты.
    fn spider_frames() -> [Option<Frame>; 6] {
        let leg = |z: f32| Frame::new(rot([0.0, 0.0, z]), [-15.0, -1.0, -1.0], [1.0, 1.0, 1.0], false);
        [
            Some(Frame::new(I3, [-4.0, -4.0, -8.0], [4.0, 4.0, 0.0], false)),
            Some(Frame::new(I3, [-5.0, -4.0, -6.0], [5.0, 4.0, 6.0], true)),
            Some(leg(-0.785)),
            Some(leg(0.785)),
            Some(leg(-0.785)),
            Some(leg(0.785)),
        ]
    }

    fn slime_frames() -> [Option<Frame>; 6] {
        [None, Some(Frame::new(I3, [-3.0, 17.0, -3.0], [3.0, 23.0, 3.0], true)), None, None, None, None]
    }

    fn only(gene: &'static str, s: u8) -> impl Fn(&str) -> u8 {
        move |name: &str| if name == gene { s } else { 0 }
    }

    fn quads(b: &[Batch]) -> usize {
        b.iter().map(|b| b.quads.len()).sum()
    }

    /// Площадь граней — мерило «насколько крупно».
    fn area(b: &[Batch]) -> f32 {
        let mut sum = 0.0;
        for q in b.iter().flat_map(|b| &b.quads) {
            let e1 = [0, 1, 2].map(|i| q.pos[1][i] - q.pos[0][i]);
            let e2 = [0, 1, 2].map(|i| q.pos[3][i] - q.pos[0][i]);
            let c = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            sum += (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
        }
        sum
    }

    #[test]
    fn the_trait_list_matches_the_gdd() {
        assert_eq!(TRAITS.len(), TABLE.len() + COPIES.len());
        for t in &TABLE {
            assert!(TRAITS.contains(&t.gene), "{}", t.gene);
            assert!(!t.pieces.is_empty() || !t.chains.is_empty(), "у {} нет геометрии", t.gene);
        }
        for c in &COPIES {
            assert!(TRAITS.contains(c));
            assert!(TABLE.iter().all(|t| t.gene != *c), "{c} рисуется копией, а не кубами");
        }
    }

    #[test]
    fn humanoid_anchors() {
        assert_eq!(
            anchors(&ZOMBIE),
            [Some("head"), Some("body"), Some("right_arm"), Some("left_arm"), Some("right_leg"), Some("left_leg")]
        );
    }

    #[test]
    fn quadruped_anchors_use_front_legs_as_arms() {
        assert_eq!(
            anchors(&COW),
            [
                Some("head"),
                Some("body"),
                Some("right_front_leg"),
                Some("left_front_leg"),
                Some("right_hind_leg"),
                Some("left_hind_leg")
            ]
        );
    }

    #[test]
    fn the_spider_grows_on_its_abdomen() {
        let a = anchors(&SPIDER);
        assert_eq!(a[Anchor::Head.index()], Some("head"));
        assert_eq!(a[Anchor::Body.index()], Some("body1"), "на брюшке, не на груди");
        assert_eq!(a[Anchor::RightArm.index()], Some("right_front_leg"));
        assert_eq!(a[Anchor::LeftLeg.index()], Some("left_hind_leg"));
    }

    #[test]
    fn the_slime_has_only_a_body() {
        assert_eq!(anchors(&SLIME), [None, Some("cube"), None, None, None, None]);
    }

    #[test]
    fn other_models_go_by_part_names() {
        let chicken = ["root", "head", "beak", "left_leg", "right_leg", "right_wing", "left_wing", "body"];
        assert_eq!(
            anchors(&chicken),
            [Some("head"), Some("body"), Some("right_wing"), Some("left_wing"), Some("right_leg"), Some("left_leg")]
        );
        let villager = ["root", "head", "nose", "arms", "left_leg", "body", "right_leg"];
        let a = anchors(&villager);
        assert_eq!(a[Anchor::RightArm.index()], Some("arms"));
        assert_eq!(a[Anchor::LeftArm.index()], None);
        assert_eq!(a[Anchor::RightLeg.index()], Some("right_leg"));
    }

    #[test]
    fn an_upright_body_keeps_the_model_axes() {
        let f = zombie_frames()[1].unwrap();
        assert_eq!(f.to_local, I3);
        assert_eq!(f.down, [0.0, 1.0, 0.0]);
        // Спина зомби — +Z модели.
        assert!(close(f.point([0.5, 0.0, 1.0]), [0.0, 0.0, 2.0]));
    }

    #[test]
    fn the_cow_body_is_turned_but_its_back_is_still_up() {
        let f = cow_body();
        // Повёрнутое тело коровы в каноническом кадре совпадает со своим.
        assert_eq!(f.to_local, I3);
        assert_eq!(f.down, [0.0, 0.0, -1.0], "вниз мира — к животу");
        // Точка «на спине» уходит вверх модели, «к хвосту» — в +Z модели.
        let world = rot([FRAC_PI_2, 0.0, 0.0]);
        let back = apply3(&world, apply3(&f.to_local, f.point([0.5, 0.5, 1.0])));
        let belly = apply3(&world, apply3(&f.to_local, f.point([0.5, 0.5, 0.0])));
        assert!(back[1] < belly[1], "спина выше живота: {back:?} {belly:?}");
        let tail = apply3(&world, apply3(&f.to_local, f.point([0.5, 1.0, 0.5])));
        let neck = apply3(&world, apply3(&f.to_local, f.point([0.5, 0.0, 0.5])));
        assert!(tail[2] > neck[2], "хвост сзади: {tail:?} {neck:?}");
    }

    #[test]
    fn an_unturned_long_body_lies_too() {
        // Брюшко паука: не повёрнуто, но длиннее, чем выше.
        let f = spider_frames()[1].unwrap();
        // «Вниз по телу» — к хвосту (+Z части), «к спине» — вверх (−Y части).
        assert_eq!(apply3(&f.to_local, [0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
        assert_eq!(apply3(&f.to_local, [0.0, 0.0, 1.0]), [0.0, -1.0, 0.0]);
        assert_eq!(f.min, [-5.0, -6.0, -4.0]);
        assert_eq!(f.max, [5.0, 6.0, 4.0]);
    }

    #[test]
    fn every_trait_has_geometry_on_a_humanoid() {
        let frames = zombie_frames();
        for t in &TABLE {
            for s in 1..=3 {
                let b = build(&only(t.gene, s), &frames, Motion::default());
                assert!(quads(&b) > 0, "{} при силе {s}", t.gene);
                assert_eq!(quads(&b) % 6, 0, "кубы по шесть граней");
            }
        }
        let body = frames[1].unwrap();
        let head = frames[0].unwrap();
        for s in 1..=3 {
            assert!(!extra_arms(s, &body, &Motion::default()).is_empty());
            assert!(second_head(s, &head, &Motion::default()).is_some());
        }
        assert!(extra_arms(0, &body, &Motion::default()).is_empty());
        assert!(second_head(0, &head, &Motion::default()).is_none());
    }

    #[test]
    fn no_gene_no_growths() {
        assert!(build(&|_: &str| 0u8, &zombie_frames(), Motion::default()).is_empty());
    }

    #[test]
    fn uv_stays_inside_the_texture_and_its_material_block() {
        let check = |what: &str, material: Material, net: [f32; 3]| {
            let [w, h] = net_extent(net);
            let [u, v] = material.origin();
            assert!(w <= BLOCK[0] && h <= BLOCK[1], "{what}: развёртка {w}×{h} больше блока");
            assert!(u + BLOCK[0] <= TEX[0] && v + BLOCK[1] <= TEX[1], "{what}: блок вне текстуры");
        };
        for t in &TABLE {
            for p in t.pieces {
                check(t.gene, p.material, net(p.size));
            }
            for c in t.chains {
                check(t.gene, c.material, chain_net(c));
            }
        }
        // И в собранных гранях — UV в долях текстуры.
        for t in &TABLE {
            let b = build(&only(t.gene, 3), &zombie_frames(), Motion { age: 37.0, ..Motion::default() });
            for q in b.iter().flat_map(|b| &b.quads) {
                for [u, v] in q.uv {
                    assert!((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v), "{}: {u} {v}", t.gene);
                }
            }
        }
    }

    #[test]
    fn materials_tile_the_texture_without_overlap() {
        for (i, a) in Material::ALL.iter().enumerate() {
            for b in &Material::ALL[i + 1..] {
                assert_ne!(a.origin(), b.origin(), "{a:?} и {b:?}");
            }
        }
        assert_eq!(Material::Glow.paint(), Paint::Glowing);
        assert_eq!(Material::Flesh.paint(), Paint::Tinted);
        assert_eq!(Material::Bone.paint(), Paint::Natural);
    }

    #[test]
    fn eyes_look_out_with_their_painted_faces() {
        // Глаз нарисован на переде (−Z) и заде (+Z) куба 3×3×1: у развёртки
        // от угла блока это клетки (1, 1) и (5, 1).
        let mut out = Vec::new();
        cuboid(&Xf::ID, [0.0; 3], [3.0, 3.0, 1.0], Material::Eye.origin(), net([3.0, 3.0, 1.0]), &mut out);
        let [u, v] = Material::Eye.origin();
        let front = out.iter().find(|q| q.normal == [0.0, 0.0, -1.0]).unwrap();
        let back = out.iter().find(|q| q.normal == [0.0, 0.0, 1.0]).unwrap();
        assert_eq!(front.uv[0], [(u + 1.0) / TEX[0], (v + 1.0) / TEX[1]]);
        assert_eq!(back.uv[0], [(u + 5.0) / TEX[0], (v + 1.0) / TEX[1]]);
        // Глаз на боку смотрит наружу: грань +Z после поворота — в −X.
        assert!(close(apply3(&rot(SIDE), [0.0, 0.0, 1.0]), [-1.0, 0.0, 0.0]));
        assert!(close(apply3(&rot(UP), [0.0, 0.0, 1.0]), [0.0, -1.0, 0.0]));
    }

    #[test]
    fn growths_grow_with_strength() {
        let frames = zombie_frames();
        let m = Motion::default();
        for t in &TABLE {
            let a = [1, 2, 3].map(|s| area(&build(&only(t.gene, s), &frames, m)));
            assert!(a[0] < a[1] && a[1] < a[2], "{}: {a:?}", t.gene);
        }
        let body = frames[1].unwrap();
        let arms = [1, 2, 3].map(|s| extra_arms(s, &body, &m));
        assert!(arms[0][0].2 < arms[1][0].2 && arms[1][0].2 <= arms[2][0].2);
        assert!(arms[2].len() > arms[0].len(), "при силе 3 рук больше");
        let head = frames[0].unwrap();
        let heads = [1, 2, 3].map(|s| second_head(s, &head, &m).unwrap().1);
        assert!(heads[0] < heads[1] && heads[1] < heads[2]);
        assert!(grow(1) < grow(2) && grow(2) < grow(3));
        assert_eq!(grow(1), 1.0);
    }

    #[test]
    fn the_cow_spider_and_slime_get_what_fits() {
        for (name, frames) in [("корова", cow_frames()), ("паук", spider_frames())] {
            for t in &TABLE {
                let b = build(&only(t.gene, 2), &frames, Motion::default());
                assert!(quads(&b) > 0, "{name}: {}", t.gene);
            }
        }
        // У слизня одно тело: черты головы и конечностей пропускаются, тела — рисуются.
        let frames = slime_frames();
        for gene in ["horns", "aquatic", "predator", "rending_claws", "spring_legs", "dense_bone"] {
            let b = build(&only(gene, 3), &frames, Motion::default());
            assert!(b.is_empty(), "у слизня нет якоря для {gene}");
        }
        for gene in ["tail", "tentacles", "tumor", "chitin", "acid_glands", "bioluminescence", "membranes"] {
            let b = build(&only(gene, 3), &frames, Motion::default());
            assert!(quads(&b) > 0, "слизень: {gene}");
            assert!(b.iter().all(|b| b.anchor == Anchor::Body));
        }
        // Многоглазие: глаза на голове пропускаются, на теле остаются.
        let b = build(&only("polyopia", 3), &frames, Motion::default());
        assert!(b.iter().all(|b| b.anchor == Anchor::Body) && quads(&b) > 0);
    }

    #[test]
    fn pairs_are_mirror_images() {
        let frames = zombie_frames();
        let b = build(&only("horns", 2), &frames, Motion::default());
        let head: Vec<&Quad> = b.iter().filter(|b| b.anchor == Anchor::Head).flat_map(|b| &b.quads).collect();
        let half = head.len() / 2;
        assert!(half > 0);
        // Голова зомби симметрична относительно x = 0: у каждой грани правого
        // рога есть отражённая грань левого (грань +X отражается в −X, поэтому
        // ищем по центру, а не по порядку).
        let centre = |q: &Quad| q.pos.iter().fold([0.0; 3], |a, p| add(a, p.map(|v| v / 4.0)));
        for r in &head[..half] {
            let rc = centre(r);
            assert!(
                head[half..].iter().any(|l| close([-rc[0], rc[1], rc[2]], centre(l))),
                "нет отражения для {rc:?}"
            );
        }
        // Рога растут вверх (−Y головы), а не вниз.
        let top = head.iter().flat_map(|q| q.pos).map(|p| p[1]).fold(f32::MAX, f32::min);
        assert!(top < -8.0, "рог выше макушки: {top}");
        // Зеркальный куб — на парном якоре.
        let p = mirrored(&CLAWS[0]);
        assert_eq!(p.anchor, Anchor::LeftArm);
        assert_eq!(p.at[0], 0.5);
        assert_eq!(p.from[0], 1.0);
    }

    #[test]
    fn the_tail_sways_and_the_hump_breathes() {
        let frames = zombie_frames();
        let tip = |age: f32| {
            let b = build(&only("tail", 3), &frames, Motion { age, ..Motion::default() });
            b[0].quads.last().unwrap().pos[0]
        };
        assert!((tip(0.0)[0] - tip(13.0)[0]).abs() > 0.3, "хвост качается");
        // Хвост растёт от спины назад: конец позади тела (+Z).
        assert!(tip(0.0)[2] > 2.0, "{:?}", tip(0.0));
        let hump = |age: f32| area(&build(&only("tumor", 1), &frames, Motion { age, ..Motion::default() }));
        assert!((hump(0.0) - hump(8.0)).abs() > 0.01, "опухоль дышит");
        // Капли кислоты падают вниз мира (за эти 5 тиков капля не обрывается:
        // её фаза 0,4 → 0,65 круга). Вторые шесть граней — первая капля.
        let drop = |age: f32| {
            let b = build(&only("acid_glands", 1), &frames, Motion { age, ..Motion::default() });
            b.iter().flat_map(|b| &b.quads).nth(6).unwrap().pos[0]
        };
        assert!(drop(5.0)[1] > drop(0.0)[1] + 1.0, "{:?} {:?}", drop(0.0), drop(5.0));
    }

    #[test]
    fn tentacles_come_out_of_the_back() {
        let frames = zombie_frames();
        for (s, count) in [(1, 2), (2, 3), (3, 4)] {
            let b = build(&only("tentacles", s), &frames, Motion::default());
            assert_eq!(quads(&b), count * 5 * 6, "{count} щупальца по 5 сегментов");
        }
        let b = build(&only("tentacles", 1), &frames, Motion::default());
        let far = b[0].quads.iter().flat_map(|q| q.pos).map(|p| p[2]).fold(f32::MIN, f32::max);
        assert!(far > 6.0, "щупальца торчат назад: {far}");
    }

    #[test]
    fn wings_flap_out_to_both_sides() {
        let frames = zombie_frames();
        let b = build(&only("membranes", 1), &frames, Motion::default());
        let xs: Vec<f32> = b.iter().flat_map(|b| &b.quads).flat_map(|q| q.pos).map(|p| p[0]).collect();
        let lo = xs.iter().copied().fold(f32::MAX, f32::min);
        let hi = xs.iter().copied().fold(f32::MIN, f32::max);
        assert!(lo < -8.0 && hi > 8.0, "крылья шире плеч: {lo} {hi}");
    }

    #[test]
    fn the_extra_arm_grows_from_the_back_and_swings_on_its_own() {
        let body = zombie_frames()[1].unwrap();
        let (_, xf, _) = extra_arms(1, &body, &Motion::default())[0];
        // Плечо — на спине.
        assert!((xf.t[2] - 1.5).abs() < 1e-4, "{:?}", xf.t);
        // Стоя рука отведена назад: её конец (+Y копии) — за спиной.
        let hand = xf.point([0.0, 10.0, 0.0]);
        assert!(hand[2] > 4.0, "{hand:?}");
        let m = Motion { walk: 3.0, speed: 0.8, ..Motion::default() };
        // Фаза своя: не совпадает с ванильным взмахом руки зомби.
        let vanilla = (m.walk * 0.6662).cos() * 2.0 * m.speed * 0.5;
        assert!((arm_swing(&m, FRAC_PI_2) - vanilla).abs() > 0.1);
        // Вторая рука — в противофазе.
        let arms = extra_arms(3, &body, &m);
        assert_eq!(arms.len(), 2);
        assert!(arms[1].0 && !arms[0].0);
    }

    #[test]
    fn the_second_head_sits_beside_the_first() {
        let head = zombie_frames()[0].unwrap();
        let (xf, k) = second_head(1, &head, &Motion::default()).unwrap();
        // Вся копия головы правее своей (−X) и ниже макушки.
        let mut max_x = f32::MIN;
        for c in corners(head.min, head.max) {
            let p = xf.point(c.map(|v| v * k));
            max_x = max_x.max(p[0]);
        }
        assert!(max_x < head.min[0] + 0.5, "копия не влезает в голову: {max_x}");
        assert!(xf.t[1] > 0.0, "ниже — на плече");
    }

    /// Рука игрока в кадре: куб (−3, −2, −2)…(1, 10, 2) у правой, зеркально у
    /// левой (широкий скин).
    fn first_person_frames() -> [Frame; 2] {
        [
            Frame::new(I3, [-3.0, -2.0, -2.0], [1.0, 10.0, 2.0], false),
            Frame::new(I3, [-1.0, -2.0, -2.0], [3.0, 10.0, 2.0], false),
        ]
    }

    #[test]
    fn the_first_person_arm_shows_only_what_grows_on_the_arm() {
        let [right, left] = first_person_frames();
        for gene in TRAITS {
            let b = first_person_arm(&only(gene, 3), right, false, Motion::default());
            if FIRST_PERSON_ARM.contains(&gene) {
                assert!(quads(&b) > 0, "{gene} на руке в кадре");
                assert!(b.iter().all(|b| b.anchor == Anchor::RightArm), "{gene}");
                // На левой руке — зеркальные наросты той же черты.
                let l = first_person_arm(&only(gene, 3), left, true, Motion::default());
                assert!(quads(&l) > 0 && l.iter().all(|b| b.anchor == Anchor::LeftArm), "{gene} слева");
            } else {
                assert!(b.is_empty(), "{gene} из глаз не видно");
            }
        }
        // Рога, хвост, вторая голова от первого лица не рисуются.
        for gene in ["horns", "tail", "dicephaly", "polymelia", "tentacles"] {
            assert!(!FIRST_PERSON_ARM.contains(&gene));
        }
        // Когти торчат из кисти дальше конца руки.
        let b = first_person_arm(&only("rending_claws", 1), right, false, Motion::default());
        let tip = b.iter().flat_map(|b| &b.quads).flat_map(|q| q.pos).map(|p| p[1]).fold(f32::MIN, f32::max);
        assert!(tip > 10.5, "когти за кистью: {tip}");
        // Лоскут перепонки на руке — со второй силы, как на модели.
        assert!(first_person_arm(&only("membranes", 1), right, false, Motion::default()).is_empty());
    }

    #[test]
    fn the_third_arm_in_view_strikes_after_the_hand() {
        assert!(view_arms(0, 0.5, 0.0).is_empty());
        for s in 1..=2 {
            let a = view_arms(s, 0.0, 0.0);
            assert_eq!(a.len(), 1);
            assert!(!a[0].left, "первая — справа, как на модели");
        }
        let three = view_arms(3, 0.0, 10.0);
        assert_eq!(three.len(), 2);
        assert!(three[1].left);
        // Над своей рукой и по своей стороне кадра.
        assert!(three[0].offset[0] > 0.0 && three[1].offset[0] < 0.0);
        assert!(three.iter().all(|a| a.offset[1] > 0.1));
        let scales = [1, 2, 3].map(|s| view_arms(s, 0.0, 0.0)[0].scale);
        assert!(scales[0] < scales[1] && scales[1] < scales[2]);
        // Замах — следом за рукой: начинается позже, кончается вместе с ней.
        assert_eq!(view_arms(1, 0.0, 0.0)[0].attack, 0.0);
        assert_eq!(view_arms(1, 1.0, 0.0)[0].attack, 1.0);
        assert_eq!(view_arms(1, 0.1, 0.0)[0].attack, 0.0);
        let mid = view_arms(1, 0.5, 0.0)[0].attack;
        assert!(mid > 0.0 && mid < 0.5, "{mid}");
        // Покачивается на месте.
        assert_ne!(view_arms(1, 0.0, 0.0)[0].offset, view_arms(1, 0.0, 20.0)[0].offset);
    }

    #[test]
    fn view_tentacles_rise_from_below_into_view() {
        let m = Motion::default();
        assert!(view_tentacles(0, &m).is_empty());
        for (s, count) in [(1, 2), (2, 3), (3, 4)] {
            assert_eq!(view_tentacles(s, &m).len(), count * 5 * 6, "{count} щупальца по 5 сегментов");
        }
        let q = view_tentacles(1, &m);
        let (right, left) = q.split_at(q.len() / 2);
        for t in [right, left] {
            // Основание — под нижним краем кадра, конец — выше основания,
            // всё целиком — перед глазами (вид смотрит в −Z).
            let base = t[0].pos.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
            let tip = t.last().unwrap().pos.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
            assert!(base < -12.0, "{base}");
            assert!(tip > base + 8.0, "растёт вверх: {base} → {tip}");
            assert!(t.iter().flat_map(|q| q.pos).all(|p| p[2] < 0.0), "перед глазами");
        }
        // Входят в кадр: основание за нижним краем (угол обзора рук — 70° по
        // вертикали, tg 35° ≈ 0,7), конец — в кадре, как бы ни извивались.
        let slope = |p: [f32; 3]| p[1] / -p[2];
        for (s, age) in (1..=3).flat_map(|s| (0..120).step_by(5).map(move |a| (s, a))) {
            let q = view_tentacles(s, &Motion { age: age as f32, ..m });
            for t in q.chunks(5 * 6) {
                assert!(slope(t[0].pos[0]) < -0.7, "основание видно: {:?}", t[0].pos[0]);
                let tip = t.last().unwrap().pos[2];
                assert!(slope(tip).abs() < 0.7, "конец за кадром на {age}: {tip:?}");
            }
        }
        // Правое справа, левое слева.
        assert!(right[0].pos[0][0] > 5.0 && left[0].pos[0][0] < -5.0);
        // Извиваются.
        let later = view_tentacles(1, &Motion { age: 15.0, ..m });
        assert!((q[29].pos[0][0] - later[29].pos[0][0]).abs() > 0.2);
        // Развёртка щупальца в кадре влезает в блок своего материала.
        let [w, h] = net_extent(chain_net(&VIEW_TENTACLE));
        assert!(w <= BLOCK[0] && h <= BLOCK[1], "{w}×{h}");
    }

    #[test]
    fn the_draw_matrix_turns_pixels_into_blocks() {
        let xf = Xf { r: rot([0.0, FRAC_PI_2, 0.0]), t: [16.0, -8.0, 32.0] };
        let m = to_cols(&xf, 2.0);
        assert_eq!([m[12], m[13], m[14], m[15]], [1.0, -0.5, 2.0, 1.0]);
        // Столбцы — повёрнутые оси с масштабом.
        assert!(close([m[0], m[1], m[2]], apply3(&xf.r, [2.0, 0.0, 0.0])));
        assert!(close([m[8], m[9], m[10]], apply3(&xf.r, [0.0, 0.0, 2.0])));
    }

    #[test]
    fn rotations_follow_the_model_order() {
        let q = FRAC_PI_2;
        // Как у частей модели (`biolab_surgery::part_affine`): сначала X, потом Y.
        assert!(close(apply3(&rot([q, q, 0.0]), [0.0, 1.0, 0.0]), [1.0, 0.0, 0.0]));
        let a = Xf { r: rot([0.3, -0.7, 1.1]), t: [1.0, 2.0, 3.0] };
        let b = Xf { r: rot([-0.2, 0.4, 0.9]), t: [-4.0, 0.5, 2.0] };
        let p = [0.7, -1.3, 2.2];
        assert!(close(a.then(&b).point(p), a.point(b.point(p))));
    }

    /// Текстура наростов ищется в общем атласе под ключом `biolab:gui/…`:
    /// файл должен быть на месте и нужного размера.
    #[test]
    fn the_texture_is_where_the_atlas_takes_it_from() {
        let path = SPRITE.strip_prefix("biolab:gui/").expect("ключ атласа из textures/gui");
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/assets/biolab/textures/gui")
            .join(format!("{path}.png"));
        let bytes = std::fs::read(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let be = |i: usize| u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        assert_eq!(&bytes[12..16], b"IHDR");
        assert_eq!([be(16) as f32, be(20) as f32], TEX, "размер текстуры");
    }
}
