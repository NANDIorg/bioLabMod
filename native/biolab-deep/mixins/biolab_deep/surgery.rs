// Операционная (GDD 4.17): как выглядит моб на столе и срезанные конечности.
//
// Логику (погрузка, стол, срез) ведёт `biolab_core`; он вешает на моба метки —
// модификаторы дальности преследования с нулевой прибавкой, как метки генов.
// Отрисовка узнаёт всё по меткам на самом мобе: в одиночной игре сервер и
// отрисовка работают с одним и тем же списком мобов (`Game::mobs`), так что
// поставленная сервером метка видна здесь в тот же кадр, без общей памяти.
//
// Что здесь сделано:
// * моб с меткой стола лежит: модель поворачивается на 90° (`lying`) так,
//   чтобы нижняя точка модели легла ровно на верхнюю грань стола;
// * срезанная конечность в основном проходе спрятана (`hide_flayed` перед
//   анимацией), а потом дорисована отдельным проходом — тонкой и цвета кости
//   (`draw_bones`): перекрасить одну часть в общем проходе нельзя, у тела
//   один цвет на все части.
//
// Чистые части (номер → часть модели, габарит модели, матрица «лёжа»)
// проверяются тестами; функции, которым нужна сама игра, отгорожены
// `#[cfg(not(test))]`.
//
// Этот файл подключается как модуль `crate::biolab_surgery` крейта
// horizon-client (там живут `mobs` и отрисовка мобов).

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Метка «моб лежит на операционном столе».
pub const TABLE_MARKER: &str = "biolab:surgery/table";

/// Метки срезанных конечностей: номер в имени — номер конечности.
pub const FLAYED_MARKERS: [&str; 4] = [
    "biolab:surgery/flayed/0",
    "biolab:surgery/flayed/1",
    "biolab:surgery/flayed/2",
    "biolab:surgery/flayed/3",
];

/// Конечности гуманоида по номерам (GDD 4.17).
pub const HUMANOID_LIMBS: [&str; 4] = ["right_arm", "left_arm", "right_leg", "left_leg"];
/// Конечности четвероногого по номерам (GDD 4.17).
pub const QUADRUPED_LIMBS: [&str; 4] =
    ["right_front_leg", "left_front_leg", "right_hind_leg", "left_hind_leg"];

/// Во сколько раз срезанная конечность тоньше по ширине и глубине: плоти нет,
/// осталась кость.
pub const THIN: f32 = 0.5;
/// Текстура кости в атласе: из модов в общий атлас попадают `textures/gui/…`.
pub const BONE_SPRITE: &str = "biolab:gui/surgery/bone";
/// Цвет кости, если текстуры в атласе не нашлось: тогда красим текстуру
/// самого моба (умножением, поэтому выйдет грязнее, но всё равно светлая кость).
pub const BONE_RGB: u32 = 0xE8DEC4;
/// Зазор над столешницей в блоках: модель вплотную к грани стола мерцала бы
/// (две грани в одной плоскости).
pub const TABLE_GAP: f32 = 0.02;

/// Подходит ли часть по запасному правилу: в имени есть `arm` или `leg`.
pub fn is_limb_name(name: &str) -> bool {
    name.contains("arm") || name.contains("leg")
}

/// Части модели для конечностей 0…3 по списку имён частей в порядке модели.
/// Гуманоид и четвероногое узнаются по полному набору своих четырёх имён;
/// иначе — первые четыре части с `arm` или `leg` в имени (у курицы их две,
/// у слизня ни одной: срезать там нечего, и рисовать тоже).
pub fn limbs<'a>(names: &[&'a str]) -> [Option<&'a str>; 4] {
    for set in [HUMANOID_LIMBS, QUADRUPED_LIMBS] {
        let found = set.map(|want| names.iter().copied().find(|n| *n == want));
        if found.iter().all(Option::is_some) {
            return found;
        }
    }
    let mut out = [None; 4];
    for (slot, name) in out
        .iter_mut()
        .zip(names.iter().copied().filter(|n| is_limb_name(n)))
    {
        *slot = Some(name);
    }
    out
}

/// Имена срезанных частей по маске срезов (номер → срезана ли).
pub fn flayed_parts<'a>(names: &[&'a str], mask: [bool; 4]) -> Vec<&'a str> {
    limbs(names)
        .iter()
        .zip(mask)
        .filter_map(|(name, cut)| if cut { *name } else { None })
        .collect()
}

// ---- Габарит модели ----
//
// Модель описана частями: у каждой поза — сдвиг в пикселях, поворот по осям
// (как `ModelPart.translateAndRotate`: Z, потом Y, потом X) и масштаб, — и
// вершины в пикселях в системе части. Здесь та же арифметика без glam, чтобы
// её можно было проверить тестами.

/// Аффинное преобразование: матрица 3×4 по строкам (поворот с масштабом и
/// сдвиг в последнем столбце).
pub type Affine = [f32; 12];

pub const IDENTITY: Affine = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];

fn mul(a: &Affine, b: &Affine) -> Affine {
    let mut out = [0.0; 12];
    for row in 0..3 {
        for col in 0..4 {
            let mut v = 0.0;
            for k in 0..3 {
                v += a[row * 4 + k] * b[k * 4 + col];
            }
            if col == 3 {
                v += a[row * 4 + 3];
            }
            out[row * 4 + col] = v;
        }
    }
    out
}

/// Преобразование части внутри родителя по её позе
/// `[x, y, z, xRot, yRot, zRot, xScale, yScale, zScale]`: сдвиг, поворот
/// Rz·Ry·Rx, масштаб — как `draw_normals` в отрисовке мобов.
pub fn part_affine(parent: &Affine, pose: &[f32; 9]) -> Affine {
    let (sx, cx) = pose[3].sin_cos();
    let (sy, cy) = pose[4].sin_cos();
    let (sz, cz) = pose[5].sin_cos();
    let rx: Affine = [1.0, 0.0, 0.0, 0.0, 0.0, cx, -sx, 0.0, 0.0, sx, cx, 0.0];
    let ry: Affine = [cy, 0.0, sy, 0.0, 0.0, 1.0, 0.0, 0.0, -sy, 0.0, cy, 0.0];
    let rz: Affine = [cz, -sz, 0.0, 0.0, sz, cz, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let scale: Affine = [pose[6], 0.0, 0.0, 0.0, 0.0, pose[7], 0.0, 0.0, 0.0, 0.0, pose[8], 0.0];
    let shift: Affine = [1.0, 0.0, 0.0, pose[0], 0.0, 1.0, 0.0, pose[1], 0.0, 0.0, 1.0, pose[2]];
    let local = mul(&shift, &mul(&rz, &mul(&ry, &mul(&rx, &scale))));
    mul(parent, &local)
}

/// Точка через преобразование.
pub fn apply(a: &Affine, p: [f32; 3]) -> [f32; 3] {
    let row = |r: usize| a[r * 4] * p[0] + a[r * 4 + 1] * p[1] + a[r * 4 + 2] * p[2] + a[r * 4 + 3];
    [row(0), row(1), row(2)]
}

/// Габаритный параллелепипед по осям.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Bounds {
    pub const EMPTY: Self = Self { min: [f32::MAX; 3], max: [f32::MIN; 3] };

    pub fn is_empty(&self) -> bool {
        (0..3).any(|i| self.min[i] > self.max[i])
    }

    pub fn grow(&mut self, p: [f32; 3]) {
        for i in 0..3 {
            self.min[i] = self.min[i].min(p[i]);
            self.max[i] = self.max[i].max(p[i]);
        }
    }

    pub fn corners(&self) -> [[f32; 3]; 8] {
        let (a, b) = (self.min, self.max);
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
}

/// Габарит модели в покое по моделям видов: модели не меняются, считаем один раз.
fn bounds_table() -> &'static Mutex<HashMap<String, Bounds>> {
    static TABLE: OnceLock<Mutex<HashMap<String, Bounds>>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Габарит модели `key`, посчитанный `compute` при первом обращении.
pub fn cached_bounds(key: &str, compute: impl FnOnce() -> Bounds) -> Bounds {
    if let Some(b) = bounds_table().lock().ok().and_then(|t| t.get(key).copied()) {
        return b;
    }
    let b = compute();
    if let Ok(mut t) = bounds_table().lock() {
        t.insert(key.to_string(), b);
    }
    b
}

/// Матрица «лёжа» (4×4 по столбцам, как `glam::Mat4::from_cols_array`).
///
/// Действует там же, где масштаб вида (`species_scale`): в системе модели,
/// где ось Y смотрит вниз, ноги на y = 0, морда — в сторону −Z. `b` — габарит
/// модели в этой системе.
/// * Высокая модель (гуманоид, курица, крипер) ложится на спину: поворот на
///   90° вокруг X, морда — вверх, голова уходит назад.
/// * Длинная (корова, паук) ложится на бок: поворот на 90° вокруг Z.
///
/// Потом модель сдвигается так, чтобы её нижняя точка легла на уровень ног
/// (на верхнюю грань стола, с зазором `TABLE_GAP`), а середина габарита по X и Z
/// встала над точкой моба — над серединой стола.
pub fn lying(b: &Bounds) -> [f32; 16] {
    let height = b.max[1] - b.min[1];
    let length = b.max[2] - b.min[2];
    // Строки поворота: (x, y, z) → (x, z, −y) на спину, (−y, x, z) на бок.
    let r: [[f32; 3]; 3] = if height >= length {
        [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]]
    } else {
        [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]
    };
    let mut turned = Bounds::EMPTY;
    for c in b.corners() {
        turned.grow([0, 1, 2].map(|row| r[row][0] * c[0] + r[row][1] * c[1] + r[row][2] * c[2]));
    }
    let shift = [
        -(turned.min[0] + turned.max[0]) / 2.0,
        -turned.max[1] - TABLE_GAP,
        -(turned.min[2] + turned.max[2]) / 2.0,
    ];
    [
        r[0][0], r[1][0], r[2][0], 0.0, //
        r[0][1], r[1][1], r[2][1], 0.0, //
        r[0][2], r[1][2], r[2][2], 0.0, //
        shift[0], shift[1], shift[2], 1.0,
    ]
}

/// Точка через матрицу 4×4 по столбцам.
pub fn transform(m: &[f32; 16], p: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|row| m[row] * p[0] + m[4 + row] * p[1] + m[8 + row] * p[2] + m[12 + row])
}

// ---- Проход костей ----
//
// Анимация (`anim::setup`) вызывается и для тела, и для каждого слоя (броня,
// шерсть, одежда). Срезанную конечность прячем во всех: на кости нет ни
// плоти, ни рукава. А в своём проходе костей анимацию зовём ещё раз — и там
// прятать нельзя. Отличаем свой проход флагом потока: отрисовка однопоточная,
// а флаг потока не перепутается и при нескольких.

thread_local! {
    static BONE_PASS: Cell<bool> = const { Cell::new(false) };
}

/// Идёт ли сейчас свой проход костей.
pub fn bone_pass() -> bool {
    BONE_PASS.with(Cell::get)
}

pub fn set_bone_pass(on: bool) {
    BONE_PASS.with(|f| f.set(on));
}

// ---- Связь с игрой ----

// Метки ставит сервер на своей копии моба, а модификатор `FollowRange`
// клиенту не приходит: у моба, которого рисует окно, меток нет (проверено в
// игре — корова на столе стояла). Поэтому, как геном (`biolab_mutants`),
// состояние операционной сервер записывает в общую память процесса по id
// моба на каждом его тике (`spawn.rs`), а отрисовка читает отсюда.

/// Состояние операционной моба: лежит ли на столе, спит ли под снотворным и
/// какие конечности срезаны.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Surgery {
    pub on_table: bool,
    /// Под снотворным (`biolab:sedation`): моб лежит на земле. Иначе не
    /// понять, подействовал ли укол — ванильного «сна» у мобов нет.
    pub asleep: bool,
    pub flayed: [bool; 4],
}

impl Surgery {
    pub fn is_plain(&self) -> bool {
        !self.on_table && !self.asleep && !self.flayed.iter().any(|f| *f)
    }

    /// Рисовать ли моба лёжа: на столе или уснувшим на земле.
    pub fn lying(&self) -> bool {
        self.on_table || self.asleep
    }
}

fn states() -> &'static std::sync::Mutex<std::collections::HashMap<u32, Surgery>> {
    static STATES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u32, Surgery>>> =
        std::sync::OnceLock::new();
    STATES.get_or_init(Default::default)
}

/// Записать состояние моба. Обычного моба в памяти не держим.
pub fn remember(id: u32, state: Surgery) {
    if let Ok(mut t) = states().lock() {
        if state.is_plain() {
            t.remove(&id);
        } else {
            t.insert(id, state);
        }
    }
}

pub fn state_of(id: u32) -> Surgery {
    states().lock().ok().and_then(|t| t.get(&id).copied()).unwrap_or_default()
}

/// Состояние по меткам на самом мобе — так его читает сервер.
#[cfg(not(test))]
pub fn read_markers(m: &crate::mobs::Mob) -> Surgery {
    use crate::mobs::finalize::{Attr, has_modifier};
    Surgery {
        on_table: has_modifier(m, Attr::FollowRange, TABLE_MARKER),
        asleep: m
            .extra
            .effects
            .iter()
            .any(|(id, ticks, _)| &**id == "biolab:sedation" && *ticks != 0),
        flayed: FLAYED_MARKERS.map(|marker| has_modifier(m, Attr::FollowRange, marker)),
    }
}

/// Лежит ли моб на операционном столе. Модель игрока — не моб: у своей id 0,
/// как у первого моба сервера, и без проверки она легла бы вместе с ним.
#[cfg(not(test))]
pub fn on_table(m: &crate::mobs::Mob) -> bool {
    !crate::biolab_players::is_player(m) && state_of(m.id).on_table
}

/// Какие конечности срезаны: маска по номерам (у модели игрока — никаких).
#[cfg(not(test))]
pub fn flayed(m: &crate::mobs::Mob) -> [bool; 4] {
    if crate::biolab_players::is_player(m) {
        return [false; 4];
    }
    state_of(m.id).flayed
}

/// Модель тела, которой моб рисуется сейчас (с её ключом), и его текстура —
/// так же, как их выбирает `render_visible`.
#[cfg(not(test))]
fn body_model(m: &crate::mobs::Mob) -> (String, std::sync::Arc<crate::mobs::Part>, String) {
    let (key, texture) = crate::mobs::render::body(m);
    let model = m
        .kind
        .layer(&key)
        .cloned()
        .unwrap_or_else(|| m.kind.model.clone());
    (key, model, texture)
}

/// Имена частей в порядке модели (обход в глубину).
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

/// Габарит модели в покое, в пикселях модели.
#[cfg(not(test))]
fn model_bounds(model: &crate::mobs::Part) -> Bounds {
    fn walk(part: &crate::mobs::Part, parent: &Affine, b: &mut Bounds) {
        let a = part_affine(parent, &part.pose);
        for poly in &part.polygons {
            for v in poly.vertices.iter() {
                b.grow(apply(&a, [v[0], v[1], v[2]]));
            }
        }
        for child in &part.children {
            walk(child, &a, b);
        }
    }
    let mut b = Bounds::EMPTY;
    walk(model, &IDENTITY, &mut b);
    b
}

/// Перед анимацией: спрятать срезанные конечности (кроме своего прохода
/// костей). Прячем до анимации, потому что после неё точки вставки нет; сама
/// анимация видимость конечностей мобов не возвращает (только у игроков).
#[cfg(not(test))]
pub fn hide_flayed(m: &crate::mobs::Mob, poses: &mut crate::mobs::anim::Poses) {
    if bone_pass() {
        return;
    }
    let mask = flayed(m);
    if !mask.contains(&true) {
        return;
    }
    let (_, model, _) = body_model(m);
    let mut names = Vec::new();
    part_names(&model, &mut names);
    for name in flayed_parts(&names, mask) {
        poses.visible(name, false);
    }
}

/// Масштаб вида с поворотом «лёжа», если моб на столе.
#[cfg(not(test))]
pub fn lay_on_table(m: &crate::mobs::Mob, value: glam::Mat4) -> glam::Mat4 {
    // Лёжа рисуется и пациент стола, и уснувший под снотворным: поворот один
    // и тот же, нижняя точка модели встаёт туда, где стоит моб.
    if !state_of(m.id).lying() {
        return value;
    }
    let (key, model, _) = body_model(m);
    let rest = cached_bounds(&key, || model_bounds(&model));
    if rest.is_empty() {
        return value;
    }
    // Габарит в той системе, где действует матрица вида: пиксели в блоки,
    // сдвиг модели `(0, −1.501, 0)` (он идёт после масштаба вида) и сам масштаб.
    let mut b = Bounds::EMPTY;
    for c in rest.corners() {
        let p = glam::Vec3::new(c[0] / 16.0, c[1] / 16.0 - 1.501, c[2] / 16.0);
        b.grow(value.transform_point3(p).to_array());
    }
    glam::Mat4::from_cols_array(&lying(&b)) * value
}

/// Проход костей: срезанные конечности моба тонкими и цвета кости.
#[cfg(not(test))]
#[allow(clippy::too_many_arguments)]
pub fn draw_bones(
    m: &crate::mobs::Mob,
    alpha: f32,
    base: glam::Mat4,
    atlas: crate::mobs::render::Atlas<'_>,
    light: u32,
    data: &crate::mobs::MobData,
    out: &mut Vec<crate::mesher::Vertex>,
) {
    let mask = flayed(m);
    if !mask.contains(&true) {
        return;
    }
    let (_, model, texture) = body_model(m);
    let mut names = Vec::new();
    part_names(&model, &mut names);
    let parts = flayed_parts(&names, mask);
    if parts.is_empty() {
        return;
    }
    let s = crate::mobs::anim::state(m, alpha);
    let mut poses = crate::mobs::anim::Poses::new(&model);
    set_bone_pass(true);
    crate::mobs::anim::setup(m, &s, &mut poses, &data.animations);
    set_bone_pass(false);
    // Конечность, которую спрятала сама анимация (руки разбойника, сложенные
    // крестом), не рисуем и костью.
    let shown: Vec<&str> = parts
        .into_iter()
        .filter(|name| find_part(&model, name).is_some_and(|p| !poses.is_hidden(p)))
        .collect();
    if shown.is_empty() {
        return;
    }
    for name in &shown {
        let pose = poses.get(name);
        pose[6] *= THIN;
        pose[8] *= THIN;
    }
    // Всё, кроме срезанных конечностей (и того, что к ним крепится), — без
    // своих граней: так обход дерева частей сохраняет позы родителей.
    fn skip_others(
        part: &crate::mobs::Part,
        poses: &mut crate::mobs::anim::Poses,
        keep: &[&str],
        inside: bool,
    ) {
        let inside = inside || keep.contains(&part.name.as_str());
        if !inside {
            poses.skip_draw(&part.name);
        }
        for child in &part.children {
            skip_others(child, poses, keep, inside);
        }
    }
    skip_others(&model, &mut poses, &shown, false);
    let (uv, color) = match atlas(BONE_SPRITE) {
        Some(uv) => (uv, 0xFFFF_FFFF),
        None => match atlas(&texture) {
            Some(uv) => (uv, crate::mobs::render::abgr(BONE_RGB)),
            None => return,
        },
    };
    // Красная вспышка от удара — и на костях, как на остальном теле.
    let overlay = crate::overlay::living(0.0, m.hurt_time > 0 || m.death_time > 0);
    crate::mobs::render::draw(&model, base, &poses, uv, color, light, overlay, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZOMBIE: [&str; 8] =
        ["root", "head", "hat", "right_arm", "left_leg", "left_arm", "right_leg", "body"];
    const COW: [&str; 7] =
        ["root", "head", "right_front_leg", "right_hind_leg", "left_hind_leg", "body", "left_front_leg"];

    #[test]
    fn humanoid_limbs_follow_the_table() {
        assert_eq!(
            limbs(&ZOMBIE),
            [Some("right_arm"), Some("left_arm"), Some("right_leg"), Some("left_leg")]
        );
    }

    #[test]
    fn quadruped_limbs_follow_the_table() {
        assert_eq!(
            limbs(&COW),
            [Some("right_front_leg"), Some("left_front_leg"), Some("right_hind_leg"), Some("left_hind_leg")]
        );
        // У паука восемь ног, но четыре из них зовутся как у четвероногого.
        let spider = [
            "root",
            "head",
            "right_front_leg",
            "right_hind_leg",
            "left_middle_front_leg",
            "body0",
            "left_hind_leg",
            "right_middle_hind_leg",
            "left_front_leg",
        ];
        assert_eq!(limbs(&spider)[1], Some("left_front_leg"));
        assert_eq!(limbs(&spider)[3], Some("left_hind_leg"));
    }

    #[test]
    fn others_take_parts_named_arm_or_leg_in_model_order() {
        let chicken = ["root", "head", "beak", "left_leg", "right_leg", "right_wing", "body"];
        assert_eq!(limbs(&chicken), [Some("left_leg"), Some("right_leg"), None, None]);
        let villager = ["root", "head", "nose", "arms", "left_leg", "body", "right_leg"];
        assert_eq!(limbs(&villager), [Some("arms"), Some("left_leg"), Some("right_leg"), None]);
        let bee = ["root", "bone", "front_legs", "middle_legs", "back_legs", "stinger"];
        assert_eq!(limbs(&bee), [Some("front_legs"), Some("middle_legs"), Some("back_legs"), None]);
        let slime = ["root", "cube", "right_eye", "left_eye", "mouth"];
        assert_eq!(limbs(&slime), [None; 4], "у слизня резать нечего");
        // Неполный набор гуманоида — не гуманоид: идём по запасному правилу.
        let half = ["root", "right_arm", "body", "left_leg", "tail_leg", "extra_arm"];
        assert_eq!(
            limbs(&half),
            [Some("right_arm"), Some("left_leg"), Some("tail_leg"), Some("extra_arm")]
        );
    }

    #[test]
    fn the_mask_picks_the_cut_limbs() {
        assert_eq!(flayed_parts(&ZOMBIE, [false; 4]), Vec::<&str>::new());
        assert_eq!(flayed_parts(&ZOMBIE, [true, false, false, true]), vec!["right_arm", "left_leg"]);
        let chicken = ["root", "left_leg", "right_leg"];
        assert_eq!(flayed_parts(&chicken, [true; 4]), vec!["left_leg", "right_leg"]);
    }

    #[test]
    fn markers_follow_the_contract() {
        for (i, marker) in FLAYED_MARKERS.iter().enumerate() {
            assert_eq!(*marker, format!("biolab:surgery/flayed/{i}"));
        }
        assert_eq!(TABLE_MARKER, "biolab:surgery/table");
        assert!(THIN > 0.3 && THIN < 0.8, "тоньше, но не исчезает");
    }

    /// Текстура кости ищется в общем атласе под ключом `biolab:gui/…`: туда
    /// попадают только `textures/gui/` модов. Файл должен быть на месте.
    #[test]
    fn the_bone_texture_is_where_the_atlas_takes_it_from() {
        let path = BONE_SPRITE.strip_prefix("biolab:gui/").expect("ключ атласа из textures/gui");
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/assets/biolab/textures/gui")
            .join(format!("{path}.png"));
        assert!(file.is_file(), "нет {}", file.display());
        // Запасной цвет — светлый: кость, а не плоть.
        let [r, g, b] = [BONE_RGB >> 16 & 0xFF, BONE_RGB >> 8 & 0xFF, BONE_RGB & 0xFF];
        assert!(r > 200 && g > 190 && b > 160, "{BONE_RGB:06X}");
    }

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-4)
    }

    #[test]
    fn a_part_is_moved_turned_and_scaled_like_the_renderer_does() {
        // Только сдвиг: пиксели складываются.
        let a = part_affine(&IDENTITY, &[1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        assert!(close(apply(&a, [1.0, 1.0, 1.0]), [2.0, 3.0, 4.0]));
        // Поворот на 90° вокруг Z: ось X уходит в Y.
        let q = std::f32::consts::FRAC_PI_2;
        let a = part_affine(&IDENTITY, &[0.0, 0.0, 0.0, 0.0, 0.0, q, 1.0, 1.0, 1.0]);
        assert!(close(apply(&a, [1.0, 0.0, 0.0]), [0.0, 1.0, 0.0]));
        // Вокруг X: Y уходит в Z; вокруг Y: Z уходит в X.
        let a = part_affine(&IDENTITY, &[0.0, 0.0, 0.0, q, 0.0, 0.0, 1.0, 1.0, 1.0]);
        assert!(close(apply(&a, [0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]));
        let a = part_affine(&IDENTITY, &[0.0, 0.0, 0.0, 0.0, q, 0.0, 1.0, 1.0, 1.0]);
        assert!(close(apply(&a, [0.0, 0.0, 1.0]), [1.0, 0.0, 0.0]));
        // Порядок как у ModelPart: сначала X, потом Y, потом Z (Rz·Ry·Rx).
        let a = part_affine(&IDENTITY, &[0.0, 0.0, 0.0, q, q, 0.0, 1.0, 1.0, 1.0]);
        // Y → (X) → Z → (Y) → X.
        assert!(close(apply(&a, [0.0, 1.0, 0.0]), [1.0, 0.0, 0.0]));
        // Масштаб — до поворота и сдвига, ребёнок — внутри родителя.
        let parent = part_affine(&IDENTITY, &[0.0, 10.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        let child = part_affine(&parent, &[2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 1.0, 1.0]);
        assert!(close(apply(&child, [1.0, 1.0, 0.0]), [4.0, 11.0, 0.0]));
    }

    #[test]
    fn bounds_grow_and_know_when_empty() {
        let mut b = Bounds::EMPTY;
        assert!(b.is_empty());
        b.grow([1.0, -2.0, 3.0]);
        b.grow([-1.0, 2.0, 0.0]);
        assert!(!b.is_empty());
        assert_eq!(b.min, [-1.0, -2.0, 0.0]);
        assert_eq!(b.max, [1.0, 2.0, 3.0]);
        assert_eq!(b.corners().len(), 8);
        let k = "test:model";
        assert_eq!(cached_bounds(k, || b), b);
        // Второй раз не считается заново.
        assert_eq!(cached_bounds(k, || Bounds::EMPTY), b);
    }

    fn turned(m: &[f32; 16], b: &Bounds) -> Bounds {
        let mut out = Bounds::EMPTY;
        for c in b.corners() {
            out.grow(transform(m, c));
        }
        out
    }

    /// Зомби в системе вида: Y вниз, ноги на 0, макушка на −2, грудь 0,25 в глубину.
    const STANDING_ZOMBIE: Bounds = Bounds { min: [-0.5, -2.0, -0.125], max: [0.5, 0.0, 0.125] };

    #[test]
    fn a_tall_mob_lies_on_its_back_on_the_table_top() {
        let m = lying(&STANDING_ZOMBIE);
        let b = turned(&m, &STANDING_ZOMBIE);
        // Лежит: в высоту — глубина груди, в длину — рост.
        assert!((b.max[1] - b.min[1] - 0.25).abs() < 1e-4, "{b:?}");
        assert!((b.max[2] - b.min[2] - 2.0).abs() < 1e-4, "{b:?}");
        // Нижняя точка — на столешнице (с зазором), не ниже и не выше.
        assert!((b.max[1] + TABLE_GAP).abs() < 1e-4, "{b:?}");
        // Над серединой стола.
        assert!((b.min[0] + b.max[0]).abs() < 1e-4 && (b.min[2] + b.max[2]).abs() < 1e-4, "{b:?}");
        // На спине: морда (−Z) смотрит вверх (−Y в системе вида).
        let face = transform(&m, [0.0, -1.75, -0.125]);
        let back = transform(&m, [0.0, -1.75, 0.125]);
        assert!(face[1] < back[1], "морда выше затылка: {face:?} {back:?}");
    }

    #[test]
    fn a_long_mob_lies_on_its_side() {
        // Корова: в длину больше, чем в высоту.
        let cow = Bounds { min: [-0.4, -1.4, -0.9], max: [0.4, 0.0, 0.6] };
        let m = lying(&cow);
        let b = turned(&m, &cow);
        // Длина осталась вдоль Z, высота ушла вбок, ширина стала высотой.
        assert!((b.max[2] - b.min[2] - 1.5).abs() < 1e-4, "{b:?}");
        assert!((b.max[0] - b.min[0] - 1.4).abs() < 1e-4, "{b:?}");
        assert!((b.max[1] - b.min[1] - 0.8).abs() < 1e-4, "{b:?}");
        assert!((b.max[1] + TABLE_GAP).abs() < 1e-4, "{b:?}");
        assert!((b.min[2] + b.max[2]).abs() < 1e-4, "{b:?}");
    }

    #[test]
    fn lying_is_a_pure_turn_and_shift() {
        // Без масштаба: столбцы поворота единичные и взаимно перпендикулярны.
        for b in [STANDING_ZOMBIE, Bounds { min: [-1.0, -0.5, -2.0], max: [1.0, 0.0, 2.0] }] {
            let m = lying(&b);
            let cols = [[m[0], m[1], m[2]], [m[4], m[5], m[6]], [m[8], m[9], m[10]]];
            for (i, a) in cols.iter().enumerate() {
                for (j, c) in cols.iter().enumerate() {
                    let dot: f32 = (0..3).map(|k| a[k] * c[k]).sum();
                    assert!((dot - if i == j { 1.0 } else { 0.0 }).abs() < 1e-6);
                }
            }
            assert_eq!([m[3], m[7], m[11], m[15]], [0.0, 0.0, 0.0, 1.0]);
        }
    }

    #[test]
    fn the_bone_pass_flag_is_per_call() {
        assert!(!bone_pass());
        set_bone_pass(true);
        assert!(bone_pass());
        set_bone_pass(false);
        assert!(!bone_pass());
    }
}

#[cfg(test)]
mod memory_tests {
    use super::*;

    #[test]
    fn the_server_state_reaches_the_renderer_by_id() {
        let id = 9_000_001;
        assert_eq!(state_of(id), Surgery::default());
        let lying = Surgery { on_table: true, asleep: false, flayed: [true, false, false, false] };
        remember(id, lying);
        assert_eq!(state_of(id), lying);
        remember(id, Surgery::default());
        assert_eq!(state_of(id), Surgery::default(), "обычного моба забываем");
        let asleep = Surgery { asleep: true, ..Surgery::default() };
        assert!(asleep.lying() && !asleep.is_plain(), "уснувший лежит");
    }
}
