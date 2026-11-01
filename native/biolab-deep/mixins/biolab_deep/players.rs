// Гены игроков для отрисовки наростов (GDD 4.19, этап «игрок»).
//
// Гены игрока живут на сервере (`PlayerState.genes` в `biolab-core`), а рисует
// окно. У мобов геном видно по меткам-модификаторам или из общей памяти
// процесса по id моба, у игрока ни того ни другого: меток на нём нет, а
// модель своего игрока — отдельный моб окна с id 0, к серверу отношения не
// имеющий. Поэтому гены приходят пакетом сервер → клиент: `biolab-core` раз в
// секунду шлёт его каналом `biolab_core:genes` (`genes_packet`), бинарник
// `biolab-deep` ловит его перехватом `native_net_from_server` (`genes_net.rs`)
// и отдаёт сюда вместе со списком «id сущности → uuid», который знает только
// окно (`persist.uuids`).
//
// Здесь гены раскладываются по моделям:
// * свой игрок — первая запись пакета; его модель (`player_model` окна, она
//   же в инвентаре) всегда моб с id 0 (`OWN_MODEL`), рука в кадре — тоже его;
// * другие игроки — модели в списке мобов окна, найденные по uuid. Так их
//   видно в многопользовательской игре (выделенный сервер, LAN): у каждого
//   клиента свой пакет, в нём гены всех игроков мира с генами.
//
// Модуль открыт наружу (`visibility = "pub"`): его зовёт бинарник. Поэтому в
// открытых функциях — только открытые типы игры. Чистая часть (разбор пакета,
// раскладка по моделям) — под тестами; функции, которым нужна игра,
// отгорожены `#[cfg(not(test))]`.
//
// Этот файл подключается как модуль `crate::biolab_players` крейта
// horizon-client (lib).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Канал пакета генов: тот же, что `NET_GENES` в `biolab-core`.
pub const CHANNEL: &str = "biolab_core:genes";
/// Версия формата пакета (первый байт), как `GENES_PACKET_VERSION` в core.
pub const VERSION: u8 = 1;
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

/// Разбор пакета генов (формат — `genes_packet` в `biolab-core`): uuid
/// игрока и его гены, получатель первым. `None` — чужая версия или пакет
/// обрезан: тогда старое состояние лучше не трогать.
pub fn decode(data: &[u8]) -> Option<Vec<(u128, PlayerGenes)>> {
    let (&version, mut rest) = data.split_first()?;
    if version != VERSION {
        return None;
    }
    let mut out = Vec::new();
    while !rest.is_empty() {
        let uuid = u128::from_be_bytes(rest.get(..16)?.try_into().ok()?);
        let count = *rest.get(16)?;
        rest = &rest[17..];
        let mut genes = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            let len = usize::from(*rest.first()?);
            let name = std::str::from_utf8(rest.get(1..1 + len)?).ok()?;
            let strength = *rest.get(1 + len)?;
            genes.push((name.to_owned(), strength));
            rest = &rest[2 + len..];
        }
        out.push((uuid, PlayerGenes::new(genes)));
    }
    Some(out)
}

/// Кому какие гены: свой игрок (первая запись) и модели других игроков —
/// id сущностей окна, чей uuid есть в пакете. `entities` — пары (id
/// сущности, uuid) из памяти окна; у одного uuid может оказаться несколько id
/// (сущность пересоздана), гены получают все. Игроки без генов не
/// записываются: их модели рисуются как есть.
pub fn assign(
    players: Vec<(u128, PlayerGenes)>,
    entities: &[(u32, u128)],
) -> (Option<PlayerGenes>, HashMap<u32, PlayerGenes>) {
    let mut players = players.into_iter();
    let own = players.next().map(|(_, genes)| genes);
    let mut others = HashMap::new();
    for (uuid, genes) in players {
        if genes.is_empty() {
            continue;
        }
        for (id, _) in entities.iter().filter(|(_, u)| *u == uuid) {
            others.insert(*id, genes.clone());
        }
    }
    (own, others)
}

#[derive(Default)]
struct Store {
    own: Option<PlayerGenes>,
    others: HashMap<u32, PlayerGenes>,
}

/// Память окна: разобранный последний пакет. Пишет перехват сети, читает
/// отрисовка — в одиночной игре это разные потоки одного процесса.
fn store() -> &'static Mutex<Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE.get_or_init(Default::default)
}

/// Пакет пришёл: запомнить гены, заменив прежние целиком (снятый ген или
/// ушедший игрок пропадают сами). Ложь — пакет не разобран.
pub fn receive(data: &[u8], entities: &[(u32, u128)]) -> bool {
    let Some(players) = decode(data) else {
        return false;
    };
    let (own, others) = assign(players, entities);
    if let Ok(mut s) = store().lock() {
        s.own = own.filter(|g| !g.is_empty());
        s.others = others;
    }
    true
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
