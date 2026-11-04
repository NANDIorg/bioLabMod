// Данные мутанта на сущностях и игроках (attachments, M11): типы и доступ.
//
// Раньше геном, срезы операционной и отметку «бросок был» хранили метками —
// модификаторами `FollowRange` с нулевой прибавкой. Horizon их при загрузке
// не читает, до окна они не доходят, и вокруг выросли костыли (`restore.rs`,
// общая память по id моба, свой пакет генов игрока). Данные на носителе
// (`docs/native-mods.md` пина, «Server attachments»; API 317 — `has`,
// `get_existing`) сохраняются вместе с мобом и с `sync` сами едут на клиент.
//
// Модуль живёт в бинарнике (`crate::game::biolab_deep_data`): `Host`, через
// который читаются данные, есть только у `Game`. `biolab-core` объявляет те же
// типы у себя (`crate::game::biolab_core_data`): ключ — пара «пространство
// имён, имя», а тип значения у обоих один и тот же (`String`, `u8`, `bool`) —
// иначе второй бандл получил бы `TypeMismatch` на уже прочитанном значении.
// Форма в сейве и в сети одна — кодек: `{genes: "…"}`, `{flayed, table,
// asleep}`, `{checked}`; чистая часть — в `runtime.rs`, её сверяют тесты.
//
// На сервере носитель — живой моб уровня; у окна — копия того, что прислал
// сервер (`set` там меняет только копию). Вне игры и для ушедшего моба
// данные отвечают `MissingHolder` — здесь это «данных нет».
//
// Внутренние поля `Game` (`persist.raw`, `mobs`) — не хук-поинты: при
// обновлении Horizon проверить.

use crate::game::biolab_deep as d;
use horizon_client::mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Host, Tag};
use horizon_world::nbt::JavaString;

fn text_field(data: &Compound, field: &str) -> Result<String, AttachmentError> {
    match data.get(&JavaString::from(field)) {
        Some(Tag::String(text)) => Ok(text.to_string_lossy()),
        _ => Err(AttachmentError::Decode),
    }
}

fn text_compound(field: &str, text: &str) -> Compound {
    let mut out = Compound::new();
    out.insert(JavaString::from(field), Tag::String(JavaString::from(text)));
    out
}

fn encode_genome(text: &String) -> Result<Compound, AttachmentError> {
    Ok(text_compound(d::GENES_FIELD, text))
}

/// Строку приводим к виду таблицы: так сравнение «изменилось ли» —
/// сравнение строк, кто бы её ни записал.
fn decode_genome(data: &Compound) -> Result<String, AttachmentError> {
    let text = text_field(data, d::GENES_FIELD)?;
    Ok(d::genome_text(&d::genome_from_text(&text)))
}

fn encode_surgery(bits: &u8) -> Result<Compound, AttachmentError> {
    let mut out = Compound::new();
    for (field, value) in d::surgery_fields(*bits) {
        out.insert(JavaString::from(field), Tag::Byte(value));
    }
    Ok(out)
}

fn decode_surgery(data: &Compound) -> Result<u8, AttachmentError> {
    Ok(d::surgery_from_fields(|field| match data.get(&JavaString::from(field)) {
        Some(Tag::Byte(value)) => Some(*value),
        _ => None,
    }))
}

fn encode_checked(checked: &bool) -> Result<Compound, AttachmentError> {
    let mut out = Compound::new();
    out.insert(JavaString::from(d::CHECKED_FIELD), Tag::Byte(i8::from(*checked)));
    Ok(out)
}

fn decode_checked(data: &Compound) -> Result<bool, AttachmentError> {
    match data.get(&JavaString::from(d::CHECKED_FIELD)) {
        Some(Tag::Byte(value)) => Ok(*value != 0),
        _ => Err(AttachmentError::Decode),
    }
}

/// Геном моба. Едет на клиент: наросты, оттенок и размер рисует окно.
pub static GENOME: AttachmentType<String> = AttachmentType {
    namespace: d::DATA_NAMESPACE,
    name: d::GENOME_DATA,
    default: String::new,
    codec: Some(AttachmentCodec {
        encode: encode_genome,
        decode: decode_genome,
    }),
    copy_on_death: false,
    sync: true,
};

/// Операционная: срезы, стол, сон (биты `runtime.rs`). Едет на клиент.
pub static SURGERY: AttachmentType<u8> = AttachmentType {
    namespace: d::DATA_NAMESPACE,
    name: d::SURGERY_DATA,
    default: || 0,
    codec: Some(AttachmentCodec {
        encode: encode_surgery,
        decode: decode_surgery,
    }),
    copy_on_death: false,
    sync: true,
};

/// «Бросок мутации уже был»: сохраняется, но клиенту не нужен.
pub static CHECKED: AttachmentType<bool> = AttachmentType {
    namespace: d::DATA_NAMESPACE,
    name: d::CHECKED_DATA,
    default: || false,
    codec: Some(AttachmentCodec {
        encode: encode_checked,
        decode: decode_checked,
    }),
    copy_on_death: false,
    sync: false,
};

/// Гены игрока: пишет `biolab-core` из состояния игрока, окно рисует
/// наросты. Переживают смерть — генотерапия навсегда (GDD 4.4).
pub static PLAYER_GENES: AttachmentType<String> = AttachmentType {
    namespace: d::DATA_NAMESPACE,
    name: d::PLAYER_GENES_DATA,
    default: String::new,
    codec: Some(AttachmentCodec {
        encode: encode_genome,
        decode: decode_genome,
    }),
    copy_on_death: true,
    sync: true,
};

/// Геном моба строкой данных; пусто — генома нет (или моба нет).
pub fn genome(game: &mut crate::game::Game<'_>, id: u32) -> String {
    let host: &mut dyn Host = game;
    GENOME
        .get_existing(host, Holder::Entity(id))
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Записать геном моба (строка `genome_text`); пустой — снять данные. Пишем,
/// только если изменилось: каждая запись уходит клиентам. Ложь — носителя
/// нет (окно, ушедший моб).
pub fn set_genome(game: &mut crate::game::Game<'_>, id: u32, text: &str) -> bool {
    let host: &mut dyn Host = game;
    let holder = Holder::Entity(id);
    let now = match GENOME.get_existing(&mut *host, holder) {
        Ok(now) => now,
        // Битое значение в сейве — перепишем; нет носителя — не выйдет.
        Err(AttachmentError::MissingHolder) => return false,
        Err(_) => None,
    };
    if text.is_empty() {
        return now.is_none() || GENOME.remove(host, holder).is_ok();
    }
    if now.as_deref() == Some(text) {
        return true;
    }
    GENOME.set(host, holder, text.to_owned()).is_ok()
}

/// Биты операционной моба; 0 — обычный моб.
pub fn surgery(game: &mut crate::game::Game<'_>, id: u32) -> u8 {
    let host: &mut dyn Host = game;
    SURGERY
        .get_existing(host, Holder::Entity(id))
        .ok()
        .flatten()
        .unwrap_or(0)
}

/// Записать биты операционной; 0 — снять данные. Только если изменилось.
pub fn set_surgery(game: &mut crate::game::Game<'_>, id: u32, bits: u8) -> bool {
    let host: &mut dyn Host = game;
    let holder = Holder::Entity(id);
    let now = match SURGERY.get_existing(&mut *host, holder) {
        Ok(now) => now,
        Err(AttachmentError::MissingHolder) => return false,
        Err(_) => None,
    };
    if bits == 0 {
        return now.is_none() || SURGERY.remove(host, holder).is_ok();
    }
    if now == Some(bits) {
        return true;
    }
    SURGERY.set(host, holder, bits).is_ok()
}

/// Был ли у моба бросок мутации. `has` — без создания значения по умолчанию.
pub fn checked(game: &mut crate::game::Game<'_>, id: u32) -> bool {
    let host: &mut dyn Host = game;
    CHECKED.has(host, Holder::Entity(id)).unwrap_or(false)
}

/// Отметить бросок мутации.
pub fn set_checked(game: &mut crate::game::Game<'_>, id: u32) -> bool {
    let host: &mut dyn Host = game;
    CHECKED.set(host, Holder::Entity(id), true).is_ok()
}

/// Гены игрока строкой данных; пусто — генов нет.
pub fn player_genes(game: &mut crate::game::Game<'_>, uuid: u128) -> String {
    let host: &mut dyn Host = game;
    PLAYER_GENES
        .get_existing(host, Holder::Player(uuid))
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Повторно разослать данные носителя тем, кто его видит сейчас (`sync_data`),
/// — только если они у него есть. Нужно потому, что сама игра шлёт клиенту
/// снимок данных, когда он начинает видеть носителя, лишь при загруженном
/// трейт-моде: `Hub::sync_attachments = !mod_api::mods().is_empty()`
/// (`game/lan.rs`), а без него `take_tracking` и `send_level_info`
/// (`lan/hub.rs`) снимков не ставят. Наши бандлы — миксины, `mods()` пуст,
/// поэтому без этого окно получало данные только в момент записи, а после
/// перезахода (загрузка из сейва, новое слежение) — никогда.
pub fn resync_entity(game: &mut crate::game::Game<'_>, id: u32) {
    let host: &mut dyn Host = game;
    let holder = Holder::Entity(id);
    if GENOME.has(&mut *host, holder).unwrap_or(false) {
        let _ = GENOME.sync_data(&mut *host, holder);
    }
    if SURGERY.has(&mut *host, holder).unwrap_or(false) {
        let _ = SURGERY.sync_data(&mut *host, holder);
    }
}

/// То же для игрока: его гены — ему и тем, кто его видит.
pub fn resync_player(game: &mut crate::game::Game<'_>, uuid: u128) {
    let host: &mut dyn Host = game;
    let holder = Holder::Player(uuid);
    if PLAYER_GENES.has(&mut *host, holder).unwrap_or(false) {
        let _ = PLAYER_GENES.sync_data(host, holder);
    }
}

/// Id модификатора: `id` (с 1.21) или `Name` (старая форма).
fn modifier_id(md: &Tag) -> Option<String> {
    d::MODIFIER_ID_KEYS.iter().find_map(|key| match md.get(key) {
        Some(Tag::String(id)) => Some(id.to_string_lossy()),
        _ => None,
    })
}

/// Id старых меток (`runtime::is_legacy_marker`) в сыром NBT моба: так их
/// хранил сейв прошлой версии, а Horizon при загрузке в живого моба их не
/// переносит. Атрибут, сумму и операцию не проверяем; ключи списков — и
/// новые (`attributes`/`modifiers`), и старые (`Attributes`/`Modifiers`): в
/// `/summon` пишут и так и так, а сырой NBT хранит призыв как есть.
fn legacy_ids(raw: &Tag) -> Vec<String> {
    let mut out = Vec::new();
    for attrs_key in d::ATTRIBUTES_KEYS {
        let Some(Tag::List(attrs)) = raw.get(attrs_key) else {
            continue;
        };
        for entry in attrs {
            for mods_key in d::MODIFIERS_KEYS {
                let Some(Tag::List(mods)) = entry.get(mods_key) else {
                    continue;
                };
                for md in mods {
                    if let Some(id) = modifier_id(md)
                        && d::is_legacy_marker(&id)
                    {
                        out.push(id);
                    }
                }
            }
        }
    }
    out
}

/// Вычеркнуть старые метки из сырого NBT: миграция одноразовая, а следующий
/// сейв пишет моба уже без них.
fn strip_legacy(raw: &mut Tag) {
    for attrs_key in d::ATTRIBUTES_KEYS {
        let Some(Tag::List(attrs)) = raw.get_mut(attrs_key) else {
            continue;
        };
        for entry in attrs.iter_mut() {
            for mods_key in d::MODIFIERS_KEYS {
                if let Some(Tag::List(mods)) = entry.get_mut(mods_key) {
                    mods.retain(|md| !modifier_id(md).is_some_and(|id| d::is_legacy_marker(&id)));
                }
            }
        }
    }
}

/// Миграция моба из прошлой версии: старые метки из сырого NBT (сейв или
/// `/summon` с NBT) — в данные, и метки снять. Данные важнее меток: что уже
/// записано данными, метки не перетирают. Метки вычёркиваются, только когда
/// всё записалось — не вышло (носителя ещё нет), попробуем на следующем
/// переборе. Истина — что-то перенесли.
pub fn migrate(game: &mut crate::game::Game<'_>, id: u32) -> bool {
    use horizon_client::mobs::finalize::{Attr, has_modifier, remove_modifier};
    let ids = match game.persist.raw.get(&id) {
        Some(raw) => legacy_ids(raw),
        None => return false,
    };
    let Some(old) = d::read_legacy(ids.iter().map(String::as_str)) else {
        return false;
    };
    let mut ok = true;
    if !old.genome.is_empty() && genome(game, id).is_empty() {
        ok &= set_genome(game, id, &old.genome);
    }
    if old.surgery != 0 && surgery(game, id) == 0 {
        ok &= set_surgery(game, id, old.surgery);
    }
    if old.checked && !checked(game, id) {
        ok &= set_checked(game, id);
    }
    if !ok {
        return false;
    }
    if let Some(raw) = game.persist.raw.get_mut(&id) {
        strip_legacy(raw);
    }
    // На живом мобе меток быть не должно (их не грузят), но если остались —
    // снимаем: иначе следующий сейв записал бы их снова.
    if let Some(m) = game.mobs.iter_mut().find(|m| m.id == id) {
        for marker in d::GENE_MARKERS
            .iter()
            .chain(d::FLAYED_MARKERS.iter())
            .chain([&d::TABLE_MARKER, &d::CHECKED_MARKER])
        {
            if has_modifier(m, Attr::FollowRange, marker) {
                remove_modifier(m, Attr::FollowRange, marker);
            }
        }
    }
    true
}

/// Здоровье моба из сырого NBT сейва: загрузка срезала его до базового
/// максимума, а прибавки генома ещё не стояли.
pub fn saved_health(game: &crate::game::Game<'_>, id: u32) -> Option<f32> {
    match game.persist.raw.get(&id)?.get("Health") {
        Some(Tag::Float(bits)) => Some(f32::from_bits(*bits)),
        _ => None,
    }
}
