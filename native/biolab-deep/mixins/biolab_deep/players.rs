// Гены игроков для отрисовки наростов (GDD 4.19, этап «игрок»).
//
// Гены игрока живут на сервере (`PlayerState.genes` в `biolab-core`), а рисует
// окно. С M11 сервер кладёт их в данные игрока `biolab:player_genes`
// (attachment на `Holder::Player(uuid)` с `sync` и `copy_on_death`), и они
// сами приходят в окно — тому игроку и всем, кто его видит. Своего пакета и
// канала больше нет.
//
// Отрисовка видит только `&Mob`, без `Game` и его данных, поэтому раз в тик
// окна бинарник (`view_sync.rs` → `biolab_bridge::show`) переписывает сюда
// пришедшее целиком:
// * свой игрок — по своему uuid; его модель (`player_model` окна, она же в
//   инвентаре) всегда моб с id 0 (`OWN_MODEL`), рука в кадре — тоже его;
// * другие игроки — модели в списке мобов окна, uuid которых окно знает
//   (`persist.uuids`). Так их видно в многопользовательской игре.
//
// Модуль открыт наружу (`visibility = "pub"`): его зовёт бинарник. Поэтому в
// открытых функциях — только открытые типы игры. Чистая часть — под тестами;
// функции, которым нужна игра, отгорожены `#[cfg(not(test))]`.
//
// Этот файл подключается как модуль `crate::biolab_players` крейта
// horizon-client (lib).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Id модели своего игрока: окно создаёт её `Mob::new(0, …)`
/// (`tick_player_model`), копия для инвентаря — тот же моб.
pub const OWN_MODEL: u32 = 0;
/// Вид модели игрока: и своя, и чужие (`AvatarRenderer`).
pub const PLAYER_KIND: &str = "minecraft:player";
/// Текстура наростов в общем атласе (как `biolab_growths::SPRITE`): её ищет
/// бинарник для руки в кадре.
pub const SPRITE: &str = "biolab:gui/mutations/growths";

/// Гены одного игрока: имя гена и сила 1–3.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerGenes {
    genes: Vec<(String, u8)>,
}

impl PlayerGenes {
    /// Нулевая сила — гена нет; больше 3 не бывает.
    pub fn new(genes: impl IntoIterator<Item = (String, u8)>) -> Self {
        let genes = genes
            .into_iter()
            .filter(|(_, s)| *s > 0)
            .map(|(name, s)| (name, s.min(3)))
            .collect();
        Self { genes }
    }

    /// Сила гена 0–3; 0 — гена нет.
    pub fn strength_of(&self, name: &str) -> u8 {
        self.genes
            .iter()
            .find(|(n, _)| n == name)
            .map_or(0, |(_, s)| *s)
    }

    pub fn is_empty(&self) -> bool {
        self.genes.is_empty()
    }
}

#[derive(Default)]
struct Store {
    own: Option<PlayerGenes>,
    others: HashMap<u32, PlayerGenes>,
}

/// Память окна: гены, пришедшие с сервера. Пишет тик окна, читает
/// отрисовка — в одиночной игре это разные потоки одного процесса.
fn store() -> &'static Mutex<Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE.get_or_init(Default::default)
}

/// Заменить гены целиком: свои и моделей других игроков по id сущности окна.
/// Снятый ген или ушедший игрок пропадают сами; игроки без генов не
/// записываются — их модели рисуются как есть.
pub fn replace_all(own: Vec<(String, u8)>, others: Vec<(u32, Vec<(String, u8)>)>) {
    let own = Some(PlayerGenes::new(own)).filter(|g| !g.is_empty());
    let others: HashMap<u32, PlayerGenes> = others
        .into_iter()
        .map(|(id, genes)| (id, PlayerGenes::new(genes)))
        .filter(|(_, genes)| !genes.is_empty())
        .collect();
    if let Ok(mut s) = store().lock() {
        s.own = own;
        s.others = others;
    }
}

/// Гены своего игрока: для руки в кадре.
pub fn own() -> Option<PlayerGenes> {
    store().lock().ok()?.own.clone()
}

/// Гены модели игрока с этим id: чужие — по id сущности окна, своя — id 0.
/// Отрисовка видит только моба, а не чей он, поэтому различаем по id. Чужой
/// игрок с id сущности 0 (сервер нумерует с нуля, но первым в мире обычно
/// появляется сам хозяин) перекрыл бы свои гены на своей модели — редкий
/// случай, на который не стоит заводить отдельный путь.
pub fn of_model(id: u32) -> Option<PlayerGenes> {
    let s = store().lock().ok()?;
    s.others
        .get(&id)
        .cloned()
        .or_else(|| (id == OWN_MODEL).then(|| s.own.clone()).flatten())
}

/// Модель игрока (своя или чужая), а не моб: геном мобов по id к ней не
/// относится. У своей модели id 0, а сервер нумерует мобов тоже с нуля, так
/// что без этой проверки мутант с id 0 красил бы и растягивал игрока.
#[cfg(not(test))]
pub fn is_player(m: &crate::mobs::Mob) -> bool {
    &*m.kind.id == PLAYER_KIND
}

/// Наросты на руке в кадре (`first_person_arm`, `arm_growths.rs`): когти,
/// перепонка, хитин, шипы и костяные наросты на той руке, которую рисует
/// игра. `arm` — часть руки игрока (как её рисует игра), `m` — её поза в
/// кадре, `uv` — место текстуры наростов в атласе.
#[cfg(not(test))]
#[allow(clippy::too_many_arguments)]
pub fn draw_arm(
    arm: &crate::mobs::Part,
    left: bool,
    m: glam::Mat4,
    light: u32,
    age: f32,
    uv: Option<[f32; 4]>,
    out: &mut Vec<crate::mesher::Vertex>,
) {
    let (Some(genes), Some(uv)) = (own(), uv) else {
        return;
    };
    let st = |name: &str| genes.strength_of(name);
    crate::biolab_growths::draw_first_person_arm(&st, arm, left, m, light, age, uv, out);
}

/// Наросты в кадре сами по себе (дописываются в буфер кадра рук,
/// `view_growths.rs`): третья рука у плеча и щупальца снизу кадра — их видно,
/// что бы ни было в руках.
/// `arms` — правая и левая руки игрока, `to_world` — кадр рук (вид с
/// покачиванием), `swing` — замах удара 0…1.
#[cfg(not(test))]
#[allow(clippy::too_many_arguments)]
pub fn draw_view(
    arms: [Option<&crate::mobs::Part>; 2],
    to_world: glam::Mat4,
    swing: f32,
    age: f32,
    light: u32,
    skin: Option<[f32; 4]>,
    growth: Option<[f32; 4]>,
    out: &mut Vec<crate::mesher::Vertex>,
) {
    let Some(genes) = own() else {
        return;
    };
    let st = |name: &str| genes.strength_of(name);
    crate::biolab_growths::draw_first_person_view(&st, arms, to_world, swing, age, light, skin, growth, out);
}
