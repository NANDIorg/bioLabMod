// M9: пропорции тела (GDD 4.24) — девять генов, которые меняют не набор частей
// модели, а их форму: длинные или тяжёлые руки, большая или маленькая
// голова, короткие или ходульные ноги, раздутый или впалый корпус, одна
// сторона крупнее другой.
//
// У каждой части модели своя поза из девяти чисел (`x, y, z, xRot, yRot,
// zRot, xScale, yScale, zScale`, `mobs.rs` пина), и эти позы лежат в таблице
// по именам частей (`anim::Poses`). Поза правится HEAD'ом на `anim::setup`
// (`proportions_pose.rs`, рядом с прятками срезанных конечностей): анимация
// крутит части (`xRot`…), а масштаб не трогает (ключевые кадры его только
// прибавляют), поэтому множители переживают её. `setup` зовут и тело, и
// каждый его слой, и проход костей, и проход наростов — форма у всех одна.
//
// Правила GDD 4.24:
// * ген меняет множитель, а не абсолютный размер: складывается с
//   гигантизмом (тот тянет всю модель, `scale.rs`), множители генов
//   перемножаются;
// * множитель по каждой оси — в пределах 0,5…2,5 (`LIMITS`);
// * хитбокс не меняется: длинные руки дают дальность удара числом
//   (`biolab-core`, `proportions.rs`), а не «потому что видно»;
// * у вида без такой части ген ничего не делает: ни вида, ни чисел (рук у
//   коровы нет — её передние лапы здесь ноги).
//
// Ноги длиннее или короче — значит, ступни уходят в землю или висят над ней:
// вся модель сдвигается по вертикали (поза корня `root`) так, чтобы нижняя
// ступня стояла на земле. Корпус шире — руки гуманоида отходят в стороны
// вместе с плечами.
//
// Чистая часть (таблица множителей, план тела по именам частей, сдвиги,
// прибавки к атрибутам) проверяется тестами (`tests/modules.rs`); функции,
// которым нужна игра, отгорожены `#[cfg(not(test))]`.
//
// Этот файл подключается как модуль `crate::biolab_proportions` крейта
// horizon-client (lib): там живут `mobs`, отрисовка и мост `biolab_bridge`.

/// Гены пропорций — в порядке таблицы генов (`GENE_NAMES`). Тот же список,
/// что `PROPORTION_GENES` в бинарнике и в `biolab-core`.
pub const GENES: [&str; 9] = [
    "long_arms",
    "heavy_arms",
    "macrocephaly",
    "microcephaly",
    "short_legs",
    "stilt_legs",
    "bloated_torso",
    "sunken_chest",
    "asymmetry",
];

/// Силы генов пропорций, по порядку `GENES` (0 — гена нет).
pub type Strengths = [u8; 9];

/// Предел множителя по одной оси: меньше — часть выворачивается в точку,
/// больше — модель рвётся (GDD 4.24).
pub const LIMITS: (f32, f32) = (0.5, 2.5);

/// Сила гена 1–3 → во сколько раз сильнее отклонение от нормы, как размер
/// наростов (GDD 4.19, `biolab_growths::grow`): ×1 / 1,25 / 1,5.
pub fn grow(strength: u8) -> f32 {
    match strength {
        0 => 0.0,
        1 => 1.0,
        2 => 1.25,
        _ => 1.5,
    }
}

/// Множитель, у которого для каждой силы своё число (GDD даёт их явно).
fn stepped(strength: u8, steps: [f32; 3]) -> f32 {
    match strength {
        0 => 1.0,
        s => steps[usize::from(s.min(3)) - 1],
    }
}

/// Отклонение от единицы, усиленное по правилу `grow`.
fn grown(strength: u8, deviation: f32) -> f32 {
    1.0 + deviation * grow(strength)
}

// ---- План тела ----

/// Чем часть модели служит для генов пропорций.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Head,
    Body,
    Arm,
    Leg,
    /// Остальное (хвост, крылья, уши): меняет только асимметрия.
    Other,
}

/// Сторона части по имени.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Right,
    Left,
    Middle,
}

/// Сторона по имени части: `right_arm`, `left_hind_leg`.
pub fn side_of(name: &str) -> Side {
    if name.contains("right") {
        Side::Right
    } else if name.contains("left") {
        Side::Left
    } else {
        Side::Middle
    }
}

/// Корпус по именам, в порядке предпочтения — как якорь тела наростов
/// (`biolab_growths::BODY_NAMES`).
pub const BODY_NAMES: [&str; 5] = ["body", "torso", "body1", "body0", "cube"];
/// Полный набор четвероногого: передние лапы у него — ноги, а не руки.
pub const QUADRUPED: [&str; 4] =
    ["right_front_leg", "left_front_leg", "right_hind_leg", "left_hind_leg"];

/// Роли частей модели по их именам (порядок модели, повторы отброшены).
/// * голова — `head`, иначе первая часть со словом `head`;
/// * корпус — первое из `BODY_NAMES`, иначе первая часть на `body`;
/// * руки — части со словом `arm` (у жителя и разбойника — одна часть
///   `arms`), но у четвероногого рук нет: его лапы — ноги;
/// * ноги — части со словом `leg`;
/// * остальное — `Other`: его меняет только асимметрия.
pub fn body_plan<'a>(names: &[&'a str]) -> Vec<(&'a str, Role, Side)> {
    let mut seen: Vec<&'a str> = Vec::new();
    for name in names {
        if !name.is_empty() && *name != "root" && !seen.contains(name) {
            seen.push(name);
        }
    }
    let head = seen
        .iter()
        .copied()
        .find(|n| *n == "head")
        .or_else(|| seen.iter().copied().find(|n| n.contains("head")));
    let body = BODY_NAMES
        .iter()
        .find_map(|b| seen.iter().copied().find(|n| n == b))
        .or_else(|| seen.iter().copied().find(|n| n.starts_with("body")));
    let quadruped = QUADRUPED.iter().all(|q| seen.contains(q));
    seen.iter()
        .map(|name| {
            let role = if Some(*name) == head {
                Role::Head
            } else if Some(*name) == body {
                Role::Body
            } else if name.contains("leg") {
                Role::Leg
            } else if name.contains("arm") && !quadruped {
                Role::Arm
            } else {
                Role::Other
            };
            (*name, role, side_of(name))
        })
        .collect()
}

// ---- Таблица множителей ----

/// Множитель одной части по трём осям (x — вбок, y — вдоль части, z — вглубь)
/// от одного гена. Числа GDD 4.24; где GDD даёт одно число, сила его
/// усиливает по правилу `grow` (×1 / 1,25 / 1,5 к отклонению):
/// * долгорукость — руки по `y` ×1,3 / 1,6 / 2,0;
/// * тяжёлые руки — руки по `x` и `z` ×1,4 / 1,5 / 1,6;
/// * большеголовость — голова ×1,3 / 1,5 / 1,8;
/// * микроцефалия — голова ×0,7 / 0,625 / 0,55;
/// * коротконогость — ноги по `y` ×0,6 / 0,55 / 0,5 (ниже предела 0,5 не
///   идём), по `x` и `z` ×1,3 / 1,375 / 1,45;
/// * ходульные ноги — ноги по `y` ×1,5 / 1,8 / 2,0;
/// * раздутый торс — корпус по `x` и `z` ×1,4 / 1,5 / 1,6;
/// * впалая грудь — корпус по `x` и `z` ×0,75 / 0,69 / 0,63 (рост корпуса
///   тот же: голова остаётся на плечах);
/// * асимметрия — все правые части ×1,3 / 1,375 / 1,45, левые
///   ×0,8 / 0,75 / 0,7.
pub fn gene_scale(gene: usize, strength: u8, role: Role, side: Side) -> [f32; 3] {
    const ONE: [f32; 3] = [1.0, 1.0, 1.0];
    if strength == 0 {
        return ONE;
    }
    let s = strength;
    match (gene, role) {
        (0, Role::Arm) => [1.0, stepped(s, [1.3, 1.6, 2.0]), 1.0],
        (1, Role::Arm) => {
            let k = grown(s, 0.4);
            [k, 1.0, k]
        }
        (2, Role::Head) => [stepped(s, [1.3, 1.5, 1.8]); 3],
        (3, Role::Head) => [grown(s, -0.3); 3],
        (4, Role::Leg) => {
            let k = grown(s, 0.3);
            [k, stepped(s, [0.6, 0.55, 0.5]), k]
        }
        (5, Role::Leg) => [1.0, stepped(s, [1.5, 1.8, 2.0]), 1.0],
        (6, Role::Body) => {
            let k = grown(s, 0.4);
            [k, 1.0, k]
        }
        (7, Role::Body) => {
            let k = grown(s, -0.25);
            [k, 1.0, k]
        }
        (8, _) => match side {
            Side::Right => [grown(s, 0.3); 3],
            Side::Left => [grown(s, -0.2); 3],
            Side::Middle => ONE,
        },
        _ => ONE,
    }
}

/// Итоговый множитель части: гены перемножаются, каждая ось — в `LIMITS`.
pub fn part_scale(st: &Strengths, role: Role, side: Side) -> [f32; 3] {
    let mut k = [1.0f32; 3];
    for (gene, strength) in st.iter().enumerate() {
        let g = gene_scale(gene, *strength, role, side);
        for axis in 0..3 {
            k[axis] *= g[axis];
        }
    }
    k.map(|v| v.clamp(LIMITS.0, LIMITS.1))
}

/// Есть ли у генома хоть одна черта пропорций.
pub fn any(st: &Strengths) -> bool {
    st.iter().any(|s| *s > 0)
}

// ---- Сдвиги ----

/// Нога для сдвига модели: длина в пикселях модели (от шарнира вниз) и
/// множитель длины.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LegLength {
    pub length: f32,
    pub scale: f32,
}

/// На сколько пикселей опустить модель (минус — поднять), чтобы нижняя
/// ступня встала на землю: длинная нога иначе ушла бы в землю, короткая —
/// повисла бы. У модели без ног (или ноги не вертикальны — паук) сдвига нет.
/// Ось `y` модели смотрит вниз.
pub fn ground_shift(legs: &[LegLength]) -> f32 {
    legs.iter()
        .map(|l| l.length * (l.scale - 1.0))
        .fold(None, |best: Option<f32>, d| Some(best.map_or(d, |b| b.max(d))))
        .map_or(0.0, |longest| -longest)
}

/// На сколько пикселей отвести плечо гуманоида наружу, когда корпус шире
/// (или уже) в `scale` раз: половина ширины корпуса растёт на столько же.
/// `pivot_x` — шарнир руки; рука по центру (руки жителя) не сдвигается.
pub fn shoulder_shift(body_half_width: f32, scale: f32, pivot_x: f32) -> f32 {
    if pivot_x.abs() < 0.5 {
        return 0.0;
    }
    body_half_width * (scale - 1.0) * pivot_x.signum()
}

/// Вертикальна ли часть в покое: так висят ноги гуманоида и четвероногого.
/// Ноги паука растут вбок (`zRot` ±45°) — по ним модель не ставится на землю.
pub fn upright(pose: &[f32; 9]) -> bool {
    pose[3].abs() < 0.35 && pose[5].abs() < 0.35
}

// ---- Вид моба: у кого есть руки и ноги ----
//
// Виды по id, как их называет игра (без `minecraft:`). Те же списки — в
// `biolab-core` (дальний удар длинных рук, подсказка «у этого вида нет
// рук»); сверяет `tests/modules.rs`.

/// Виды с руками: гуманоиды, големы, жители и разбойники.
pub const ARMED_KINDS: [&str; 30] = [
    "player", "zombie", "husk", "drowned", "zombie_villager", "skeleton", "stray",
    "bogged", "parched", "wither_skeleton", "piglin", "piglin_brute", "zombified_piglin",
    "enderman", "iron_golem", "snow_golem", "copper_golem", "witch", "villager",
    "wandering_trader", "pillager", "vindicator", "evoker", "illusioner", "vex", "allay",
    "creaking", "warden", "giant", "biolab:infected",
];

/// Виды без ног: слизни, гасты, рыбы, кальмары, летуны без лап.
pub const LEGLESS_KINDS: [&str; 26] = [
    "slime", "magma_cube", "sulfur_cube", "ghast", "happy_ghast", "squid", "glow_squid",
    "cod", "salmon", "tropical_fish", "pufferfish", "tadpole", "dolphin", "guardian",
    "elder_guardian", "shulker", "phantom", "bat", "silverfish", "endermite", "blaze",
    "breeze", "nautilus", "zombie_nautilus", "vex", "allay",
];

/// Вид без `minecraft:`; свой вид мода — как есть.
pub fn bare_kind(kind: &str) -> &str {
    kind.strip_prefix("minecraft:").unwrap_or(kind)
}

/// Есть ли у вида руки.
pub fn has_arms(kind: &str) -> bool {
    ARMED_KINDS.contains(&bare_kind(kind))
}

/// Есть ли у вида ноги.
pub fn has_legs(kind: &str) -> bool {
    !LEGLESS_KINDS.contains(&bare_kind(kind))
}

// ---- Числа ----

/// Прибавки к атрибутам: здоровье, скорость (доля базы), урон, броня,
/// сопротивление отбрасыванию, дальность взгляда (блоки), сила прыжка.
pub const ATTRS: usize = 7;

/// Модификаторы прибавок пропорций — по одному на атрибут, в порядке
/// `ATTRS`. Свои имена, а не `biolab:genome_*`: те ставит `mutants::stats`
/// по таблице, где вид не учитывается.
pub const MODIFIERS: [&str; ATTRS] = [
    "biolab:proportion_health",
    "biolab:proportion_speed",
    "biolab:proportion_damage",
    "biolab:proportion_armor",
    "biolab:proportion_knockback",
    "biolab:proportion_follow",
    "biolab:proportion_jump",
];

/// Прибавки одного гена за единицу силы (порядок `ATTRS`). Побочки GDD 4.24:
/// * долгорукость — чисел нет: дальний удар реже обычного (`biolab-core`);
/// * тяжёлые руки — +1 урона, −4 % скорости;
/// * большеголовость — взгляд +4 блока (и замечает игрока издали; виден
///   сквозь стены — `biolab-core`);
/// * микроцефалия — взгляд −4 блока, +1 брони (в маленькую голову не попасть);
/// * коротконогость — −6 % скорости, +0,15 к устойчивости, прыжок ниже;
/// * ходульные ноги — +6 % скорости, прыжок выше, −2 здоровья (хрупче);
/// * раздутый торс — +4 здоровья, −3 % скорости, чуть устойчивее;
/// * впалая грудь — −3 здоровья, +3 % скорости, +1 брони (узкая мишень);
/// * асимметрия — чисел нет: ходит боком (`tick`).
pub const GENE_ATTRS: [[f64; ATTRS]; 9] = [
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, -0.04, 1.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 4.0, 0.0],
    [0.0, 0.0, 0.0, 1.0, 0.0, -4.0, 0.0],
    [0.0, -0.06, 0.0, 0.0, 0.15, 0.0, -0.03],
    [-2.0, 0.06, 0.0, 0.0, 0.0, 0.0, 0.05],
    [4.0, -0.03, 0.0, 0.0, 0.05, 0.0, 0.0],
    [-3.0, 0.03, 0.0, 1.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
];

/// Действует ли ген на вид: гены рук — только у видов с руками, гены ног —
/// только у видов с ногами, остальные — у всех.
pub fn works_on(gene: usize, kind: &str) -> bool {
    match gene {
        0 | 1 => has_arms(kind),
        4 | 5 => has_legs(kind),
        _ => gene < GENES.len(),
    }
}

/// Прибавки генома пропорций для вида с базовым здоровьем `base_health`.
/// Здоровье масштабируется по виду, как у остальных генов
/// (`mutants::scaled_stats`: таблица — для зомби), и не срезает больше
/// четверти базы.
pub fn attributes(st: &Strengths, kind: &str, base_health: f64) -> [f64; ATTRS] {
    let mut total = [0.0; ATTRS];
    for (gene, strength) in st.iter().enumerate() {
        if *strength == 0 || !works_on(gene, kind) {
            continue;
        }
        for (sum, add) in total.iter_mut().zip(GENE_ATTRS[gene]) {
            *sum += add * f64::from(*strength);
        }
    }
    let base = if base_health > 0.0 { base_health } else { 20.0 };
    total[0] = (total[0] * (base / 20.0).clamp(0.25, 2.0)).max(-base / 4.0);
    total
}

/// Асимметрия: на сколько радиан за тик уводит в сторону шаг моба
/// (косая походка). 0 — гена нет.
pub fn drift(strength: u8) -> f64 {
    0.05 * f64::from(strength.min(3))
}

/// Повернуть горизонтальную скорость на угол `angle` (вправо, по часовой,
/// если смотреть сверху): скорость та же, направление — вбок.
pub fn turned(vx: f64, vz: f64, angle: f64) -> (f64, f64) {
    let (s, c) = angle.sin_cos();
    (vx * c - vz * s, vx * s + vz * c)
}

/// Силы генов пропорций по функции «сила гена по имени».
pub fn strengths(of: &dyn Fn(&str) -> u8) -> Strengths {
    GENES.map(|name| of(name).min(3))
}

// ---- Связь с игрой ----

/// План одной модели: что масштабировать и на сколько сдвигать. Считается
/// один раз на модель (по имени слоя): части и их длины от позы не зависят.
#[cfg(not(test))]
#[derive(Clone, Default)]
struct ModelPlan {
    root: String,
    parts: Vec<(String, Role, Side)>,
    /// Вертикальные ноги: имя, длина в пикселях.
    legs: Vec<(String, f32)>,
    /// Руки сбоку корпуса: имя, шарнир по `x`.
    arms: Vec<(String, f32)>,
    /// Половина ширины корпуса в пикселях; 0 — корпуса нет.
    body_half_width: f32,
    /// Руки — дети корпуса: тогда они и так идут за его масштабом.
    arms_in_body: bool,
    /// Прочие части внутри масштабируемых (рукав в руке, шляпа на голове):
    /// масштаб родителя они получают сами, свой — лишний.
    nested: Vec<String>,
}

#[cfg(not(test))]
fn plans() -> &'static std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<ModelPlan>>> {
    static PLANS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<ModelPlan>>>,
    > = std::sync::OnceLock::new();
    PLANS.get_or_init(Default::default)
}

#[cfg(not(test))]
fn walk_names<'a>(part: &'a crate::mobs::Part, out: &mut Vec<&'a str>) {
    out.push(part.name.as_str());
    for child in &part.children {
        walk_names(child, out);
    }
}

#[cfg(not(test))]
fn find<'a>(part: &'a crate::mobs::Part, name: &str) -> Option<&'a crate::mobs::Part> {
    if part.name == name {
        return Some(part);
    }
    part.children.iter().find_map(|c| find(c, name))
}

/// Габарит своих граней части по одной оси (`axis`), в пикселях; None —
/// граней нет.
#[cfg(not(test))]
fn extent(part: &crate::mobs::Part, axis: usize) -> Option<(f32, f32)> {
    let mut range: Option<(f32, f32)> = None;
    for poly in &part.polygons {
        for v in poly.vertices.iter() {
            let x = v[axis];
            range = Some(range.map_or((x, x), |(lo, hi)| (lo.min(x), hi.max(x))));
        }
    }
    range
}

#[cfg(not(test))]
fn plan_of(model: &crate::mobs::Part) -> ModelPlan {
    let mut names = Vec::new();
    walk_names(model, &mut names);
    let parts: Vec<(String, Role, Side)> = body_plan(&names)
        .into_iter()
        .map(|(n, r, s)| (n.to_string(), r, s))
        .collect();
    let body = parts.iter().find(|(_, r, _)| *r == Role::Body).and_then(|(n, _, _)| find(model, n));
    let body_half_width = body
        .and_then(|b| extent(b, 0))
        .map_or(0.0, |(lo, hi)| (hi - lo) / 2.0);
    let arms_in_body = body.is_some_and(|b| {
        parts
            .iter()
            .filter(|(_, r, _)| *r == Role::Arm)
            .any(|(n, _, _)| b.children.iter().any(|c| find(c, n).is_some()))
    });
    let mut legs = Vec::new();
    let mut arms = Vec::new();
    for (name, role, _) in &parts {
        let Some(part) = find(model, name) else { continue };
        match role {
            Role::Leg if upright(&part.pose) => {
                // Длина — до нижней грани самой ноги (ось y модели — вниз).
                if let Some((_, low)) = extent(part, 1)
                    && low > 0.5
                {
                    legs.push((name.clone(), low));
                }
            }
            Role::Arm => arms.push((name.clone(), part.pose[0])),
            _ => {}
        }
    }
    // Части внутри головы, корпуса, рук, ног и боковых частей.
    fn mark(part: &crate::mobs::Part, inside: bool, plan: &[(String, Role, Side)], out: &mut Vec<String>) {
        if inside {
            out.push(part.name.clone());
        }
        let scaled = plan
            .iter()
            .any(|(n, r, s)| *n == part.name && (*r != Role::Other || *s != Side::Middle));
        for child in &part.children {
            mark(child, inside || scaled, plan, out);
        }
    }
    let mut nested = Vec::new();
    mark(model, false, &parts, &mut nested);
    ModelPlan { root: model.name.clone(), parts, legs, arms, body_half_width, arms_in_body, nested }
}

/// Силы генов пропорций у моба или модели игрока, которую рисует окно.
/// Модель игрока — не моб: её гены — данные игрока (`biolab_players`), а
/// геном мобов по id к ней не относится (у своей модели id 0).
#[cfg(not(test))]
pub fn strengths_of(m: &crate::mobs::Mob) -> Strengths {
    if crate::biolab_players::is_player(m) {
        match crate::biolab_players::of_model(m.id) {
            Some(g) => strengths(&|name: &str| g.strength_of(name)),
            None => [0; 9],
        }
    } else {
        match crate::biolab_mutants::genome(m.id) {
            Some(g) => strengths(&|name: &str| g.strength_of(name)),
            None => [0; 9],
        }
    }
}

/// HEAD на `anim::setup`: поза частей по генам пропорций — масштаб частей,
/// плечи шире корпуса, вся модель — ступнями на землю.
#[cfg(not(test))]
pub fn shape(m: &crate::mobs::Mob, p: &mut crate::mobs::anim::Poses) {
    let st = strengths_of(m);
    if !any(&st) {
        return;
    }
    let (key, _) = crate::mobs::render::body(m);
    let plan = {
        let cached = plans().lock().ok().and_then(|t| t.get(&*key).cloned());
        match cached {
            Some(plan) => plan,
            None => {
                let model = m
                    .kind
                    .layer(&key)
                    .cloned()
                    .unwrap_or_else(|| m.kind.model.clone());
                let plan = std::sync::Arc::new(plan_of(&model));
                if let Ok(mut t) = plans().lock() {
                    t.insert(key.to_string(), plan.clone());
                }
                plan
            }
        }
    };
    let mut body_x = 1.0;
    let mut leg_scale: Vec<(String, f32)> = Vec::new();
    for (name, role, side) in &plan.parts {
        let k = part_scale(&st, *role, *side);
        if *role == Role::Body {
            body_x = k[0];
        }
        if *role == Role::Leg {
            leg_scale.push((name.clone(), k[1]));
        }
        if k == [1.0; 3] || !p.has(name) || (*role == Role::Other && plan.nested.contains(name)) {
            continue;
        }
        let q = p.get(name);
        q[6] *= k[0];
        q[7] *= k[1];
        q[8] *= k[2];
    }
    if body_x != 1.0 && !plan.arms_in_body && plan.body_half_width > 0.0 {
        for (name, pivot_x) in &plan.arms {
            let dx = shoulder_shift(plan.body_half_width, body_x, *pivot_x);
            if dx != 0.0 && p.has(name) {
                p.get(name)[0] += dx;
            }
        }
    }
    let legs: Vec<LegLength> = plan
        .legs
        .iter()
        .filter_map(|(name, length)| {
            let scale = leg_scale.iter().find(|(n, _)| n == name)?.1;
            Some(LegLength { length: *length, scale })
        })
        .collect();
    let dy = ground_shift(&legs);
    if dy.abs() > 0.01 && p.has(&plan.root) {
        p.get(&plan.root)[1] += dy;
    }
}

/// Сервер: прибавки к атрибутам по генам пропорций (`genes` — номер гена и
/// сила, как у `biolab_bridge::apply_genome`). Модификатор с тем же именем
/// заменяется; прибавка, которой больше нет, снимается. `fill` — долить
/// выросший максимум здоровья (новый мутант не должен быть израненным).
#[cfg(not(test))]
pub fn apply(m: &mut crate::mobs::Mob, genes: &[(usize, u8)], fill: bool) {
    use crate::mobs::finalize::{Attr, Op, add_modifier, has_modifier, modifier_amount, remove_modifier};
    let rows = [
        (Attr::MaxHealth, Op::AddValue),
        (Attr::MovementSpeed, Op::AddMultipliedBase),
        (Attr::AttackDamage, Op::AddValue),
        (Attr::Armor, Op::AddValue),
        (Attr::KnockbackResistance, Op::AddValue),
        (Attr::FollowRange, Op::AddValue),
        (Attr::JumpStrength, Op::AddValue),
    ];
    let genome = crate::biolab_mutants::genome_of(genes);
    let st = strengths(&|name: &str| genome.strength_of(name));
    if !any(&st)
        && !rows
            .iter()
            .zip(MODIFIERS)
            .any(|((attr, _), name)| has_modifier(m, *attr, name))
    {
        return;
    }
    let base_health = crate::mobs::finalize::base_value(m, Attr::MaxHealth);
    let kind = m.kind.id.to_string();
    let add = attributes(&st, &kind, base_health);
    let before = m.attrs.max_health;
    for (i, (attr, op)) in rows.into_iter().enumerate() {
        let name = MODIFIERS[i];
        if add[i] != 0.0 {
            if modifier_amount(m, attr, name) != Some(add[i]) {
                add_modifier(m, attr, name, add[i], op);
            }
        } else if has_modifier(m, attr, name) {
            remove_modifier(m, attr, name);
        }
    }
    let grown = m.attrs.max_health - before;
    if fill && grown > 0.0 {
        m.health = (m.health + grown).min(m.attrs.max_health);
    }
    m.health = m.health.min(m.attrs.max_health);
}

/// Сервер, каждый тик: асимметрия уводит шаг в сторону — моб идёт боком,
/// навигация его то и дело доворачивает (косая походка).
#[cfg(not(test))]
pub fn tick(m: &mut crate::mobs::Mob, genome: &crate::biolab_mutants::Genome) {
    let lopsided = genome.strength_of("asymmetry");
    if lopsided == 0 || !m.on_ground || !has_legs(&m.kind.id) {
        return;
    }
    let (vx, vz) = (m.velocity.x, m.velocity.z);
    if vx * vx + vz * vz < 1.0e-4 {
        return;
    }
    let (x, z) = turned(vx, vz, drift(lopsided));
    m.velocity.x = x;
    m.velocity.z = z;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn only(gene: &str, strength: u8) -> Strengths {
        let mut st = [0; 9];
        st[GENES.iter().position(|g| *g == gene).unwrap()] = strength;
        st
    }

    #[test]
    fn the_numbers_follow_gdd() {
        let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4);
        // Долгорукость: руки по длине ×1,3 / 1,6 / 2,0.
        for (s, y) in [(1, 1.3), (2, 1.6), (3, 2.0)] {
            assert!(close(part_scale(&only("long_arms", s), Role::Arm, Side::Right), [1.0, y, 1.0]));
        }
        assert!(close(part_scale(&only("heavy_arms", 1), Role::Arm, Side::Left), [1.4, 1.0, 1.4]));
        assert!(close(part_scale(&only("heavy_arms", 3), Role::Arm, Side::Left), [1.6, 1.0, 1.6]));
        assert!(close(part_scale(&only("macrocephaly", 3), Role::Head, Side::Middle), [1.8; 3]));
        assert!(close(part_scale(&only("microcephaly", 1), Role::Head, Side::Middle), [0.7; 3]));
        assert!(close(part_scale(&only("short_legs", 1), Role::Leg, Side::Right), [1.3, 0.6, 1.3]));
        assert!(close(part_scale(&only("stilt_legs", 2), Role::Leg, Side::Right), [1.0, 1.8, 1.0]));
        assert!(close(part_scale(&only("bloated_torso", 1), Role::Body, Side::Middle), [1.4, 1.0, 1.4]));
        assert!(close(part_scale(&only("sunken_chest", 1), Role::Body, Side::Middle), [0.75, 1.0, 0.75]));
        assert!(close(part_scale(&only("asymmetry", 1), Role::Arm, Side::Right), [1.3; 3]));
        assert!(close(part_scale(&only("asymmetry", 1), Role::Leg, Side::Left), [0.8; 3]));
        assert!(close(part_scale(&only("asymmetry", 3), Role::Other, Side::Middle), [1.0; 3]));
        // Ген не трогает чужие части.
        assert!(close(part_scale(&only("long_arms", 3), Role::Leg, Side::Right), [1.0; 3]));
        assert!(close(part_scale(&only("macrocephaly", 3), Role::Body, Side::Middle), [1.0; 3]));
        // Сильнее — дальше от нормы.
        for gene in GENES {
            for role in [Role::Head, Role::Body, Role::Arm, Role::Leg] {
                let dev = |s| {
                    part_scale(&only(gene, s), role, Side::Right).iter().map(|k| (k - 1.0).abs()).sum::<f32>()
                };
                assert!(dev(1) <= dev(2) + 1e-6 && dev(2) <= dev(3) + 1e-6, "{gene} {role:?}");
            }
        }
    }

    #[test]
    fn multipliers_combine_and_stay_in_limits() {
        // Противоположности гасят друг друга, а не спорят.
        let mut st = only("macrocephaly", 1);
        st[3] = 1;
        let head = part_scale(&st, Role::Head, Side::Middle);
        assert!((head[0] - 1.3 * 0.7).abs() < 1e-4);
        // Всё сразу на тройке — ни одна ось не вылезает за пределы.
        let all = [3u8; 9];
        for role in [Role::Head, Role::Body, Role::Arm, Role::Leg, Role::Other] {
            for side in [Side::Right, Side::Left, Side::Middle] {
                for k in part_scale(&all, role, side) {
                    assert!((LIMITS.0..=LIMITS.1).contains(&k), "{role:?} {side:?}: {k}");
                }
            }
        }
        // Длинные руки и асимметрия: 2,0 × 1,45 упирается в 2,5.
        let mut st = only("long_arms", 3);
        st[8] = 3;
        assert_eq!(part_scale(&st, Role::Arm, Side::Right)[1], 2.5);
        assert!(!any(&[0; 9]));
        assert!(any(&only("asymmetry", 1)));
    }

    #[test]
    fn body_plans_of_vanilla_shapes() {
        let role = |plan: &[(&str, Role, Side)], n: &str| plan.iter().find(|(m, _, _)| *m == n).map(|(_, r, s)| (*r, *s));
        // Зомби и игрок.
        let zombie = body_plan(&["root", "head", "hat", "body", "right_arm", "left_arm", "right_leg", "left_leg"]);
        assert_eq!(role(&zombie, "head"), Some((Role::Head, Side::Middle)));
        assert_eq!(role(&zombie, "body"), Some((Role::Body, Side::Middle)));
        assert_eq!(role(&zombie, "right_arm"), Some((Role::Arm, Side::Right)));
        assert_eq!(role(&zombie, "left_leg"), Some((Role::Leg, Side::Left)));
        assert_eq!(role(&zombie, "hat"), Some((Role::Other, Side::Middle)));
        assert_eq!(role(&zombie, "root"), None, "корень не масштабируется — его только сдвигают");
        // Корова: передние лапы — ноги, рук нет.
        let cow = body_plan(&["root", "head", "body", "right_hind_leg", "left_hind_leg", "right_front_leg", "left_front_leg"]);
        assert!(cow.iter().all(|(_, r, _)| *r != Role::Arm));
        assert_eq!(cow.iter().filter(|(_, r, _)| *r == Role::Leg).count(), 4);
        // Житель: одна часть «руки» посередине.
        let villager = body_plan(&["head", "nose", "body", "jacket", "arms", "right_leg", "left_leg"]);
        assert_eq!(role(&villager, "arms"), Some((Role::Arm, Side::Middle)));
        // Слизень: корпус — `cube`, ни рук, ни ног.
        let slime = body_plan(&["cube", "right_eye", "left_eye", "mouth"]);
        assert_eq!(role(&slime, "cube"), Some((Role::Body, Side::Middle)));
        assert_eq!(role(&slime, "right_eye"), Some((Role::Other, Side::Right)));
        // Повторы имён — одна часть (поза в таблице по имени одна).
        assert_eq!(body_plan(&["head", "head", "body"]).len(), 2);
    }

    #[test]
    fn feet_stay_on_the_ground() {
        // Зомби: ноги по 12 пикселей. Короткие (×0,6) — модель вниз на 4,8.
        let short = [LegLength { length: 12.0, scale: 0.6 }; 2];
        assert!((ground_shift(&short) - 4.8).abs() < 1e-4);
        // Ходули ×1,5 — вверх на 6.
        let stilts = [LegLength { length: 12.0, scale: 1.5 }; 2];
        assert!((ground_shift(&stilts) + 6.0).abs() < 1e-4);
        // Асимметрия: на земле стоит длинная нога, короткая висит.
        let lopsided = [LegLength { length: 12.0, scale: 1.3 }, LegLength { length: 12.0, scale: 0.8 }];
        assert!((ground_shift(&lopsided) + 3.6).abs() < 1e-4);
        assert_eq!(ground_shift(&[]), 0.0);
        assert_eq!(ground_shift(&[LegLength { length: 6.0, scale: 1.0 }]), 0.0);
        // Плечи: корпус ×1,4 — руки зомби (шарнир ±5) отходят на 1,6 наружу.
        assert!((shoulder_shift(4.0, 1.4, -5.0) + 1.6).abs() < 1e-4);
        assert!((shoulder_shift(4.0, 1.4, 5.0) - 1.6).abs() < 1e-4);
        assert_eq!(shoulder_shift(4.0, 1.4, 0.0), 0.0);
        // Ноги паука растут вбок — по ним не ставим.
        assert!(upright(&[0.0, 12.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]));
        assert!(!upright(&[0.0, 15.0, 0.0, 0.0, 0.4, -0.785, 1.0, 1.0, 1.0]));
    }

    #[test]
    fn genes_work_only_where_the_part_is() {
        assert!(has_arms("minecraft:zombie") && has_arms("player") && has_arms("biolab:infected"));
        assert!(!has_arms("minecraft:cow") && !has_arms("spider"));
        assert!(has_legs("cow") && !has_legs("minecraft:slime"));
        // Тяжёлые руки корове — ни урона, ни замедления.
        let heavy = only("heavy_arms", 3);
        assert_eq!(attributes(&heavy, "minecraft:cow", 10.0), [0.0; ATTRS]);
        let zombie = attributes(&heavy, "minecraft:zombie", 20.0);
        assert!((zombie[2] - 3.0).abs() < 1e-9 && zombie[1] < 0.0);
        // Ходули слизню — ничего, курице — да.
        assert_eq!(attributes(&only("stilt_legs", 2), "slime", 16.0), [0.0; ATTRS]);
        assert!(attributes(&only("stilt_legs", 2), "chicken", 4.0)[6] > 0.0);
        // Голова и корпус — у всех.
        assert!(attributes(&only("macrocephaly", 1), "slime", 16.0)[5] > 0.0);
        // Здоровье — по виду: курице раздутый торс прибавляет вчетверо меньше,
        // впалая грудь не срезает больше четверти базы.
        assert!((attributes(&only("bloated_torso", 1), "chicken", 4.0)[0] - 1.0).abs() < 1e-9);
        assert!(attributes(&only("sunken_chest", 3), "chicken", 4.0)[0] >= -1.0);
        assert_eq!(MODIFIERS.len(), ATTRS);
        for name in MODIFIERS {
            assert!(name.starts_with("biolab:proportion_"));
        }
    }

    #[test]
    fn lopsided_gait_turns_but_keeps_speed() {
        let (x, z) = turned(0.2, 0.0, drift(2));
        assert!(((x * x + z * z).sqrt() - 0.2).abs() < 1e-9);
        assert!(z > 0.0);
        assert_eq!(drift(0), 0.0);
        assert!(drift(3) > drift(1));
        assert!(drift(3) < 0.2, "боком, но не кругами");
    }
}
