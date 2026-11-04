// Данные мутанта на сущностях и игроках (attachments, M11): типы и доступ
// для хуков `biolab_core`.
//
// Геном моба, операционная (стол, срезы) и гены игрока — свои данные мода на
// носителе (`docs/native-mods.md` пина, «Server attachments»; API 317 —
// `has`, `get_existing`), а не метки-модификаторы `FollowRange`: данные
// сохраняются вместе с мобом и с `sync` сами едут на клиент.
//
// Модуль живёт в игре (`crate::game::biolab_core_data`): `Host`, через
// который читаются данные, есть только у `Game`. `biolab-deep` объявляет те
// же типы у себя (`crate::game::biolab_deep_data`) — ключ, тип значения и
// форма у обоих одни (контракт — в `runtime.rs`, раздел «Данные на
// сущностях»; его сверяет `biolab-deep/tests/modules.rs`). Здесь нет
// отметки «бросок был»: её ставит и читает только `biolab-deep`.
//
// На сервере носитель — живой моб уровня; у окна — копия того, что прислал
// сервер. Вне серверной игры и для ушедшего моба данные отвечают
// `MissingHolder` — здесь это «данных нет».
//
// Читать данные внутри обхода `game.mobs` нельзя (нужен `&mut Game`), поэтому
// хуки сначала собирают id, а потом спрашивают данные.

use crate::game::biolab_core as m;
use horizon_client::mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Host, Tag};
use horizon_world::nbt::JavaString;

fn encode_genes(text: &String) -> Result<Compound, AttachmentError> {
    let mut out = Compound::new();
    out.insert(JavaString::from(m::GENES_FIELD), Tag::String(JavaString::from(text.as_str())));
    Ok(out)
}

/// Строку приводим к виду таблицы: кто бы её ни записал, «изменилось ли» —
/// сравнение строк.
fn decode_genes(data: &Compound) -> Result<String, AttachmentError> {
    match data.get(&JavaString::from(m::GENES_FIELD)) {
        Some(Tag::String(text)) => Ok(m::genome_data(&m::genes_of_data(&text.to_string_lossy()))),
        _ => Err(AttachmentError::Decode),
    }
}

fn encode_surgery(bits: &u8) -> Result<Compound, AttachmentError> {
    let mut out = Compound::new();
    for (field, value) in m::surgery_fields(*bits) {
        out.insert(JavaString::from(field), Tag::Byte(value));
    }
    Ok(out)
}

fn decode_surgery(data: &Compound) -> Result<u8, AttachmentError> {
    Ok(m::surgery_from_fields(|field| match data.get(&JavaString::from(field)) {
        Some(Tag::Byte(value)) => Some(*value),
        _ => None,
    }))
}

/// Геном моба (`biolab:genome`).
pub static GENOME: AttachmentType<String> = AttachmentType {
    namespace: m::DATA_NAMESPACE,
    name: m::GENOME_DATA,
    default: String::new,
    codec: Some(AttachmentCodec {
        encode: encode_genes,
        decode: decode_genes,
    }),
    copy_on_death: false,
    sync: true,
};

/// Операционная моба (`biolab:surgery`): биты `FLAYED_BITS`, `TABLE_BIT`,
/// `ASLEEP_BIT`.
pub static SURGERY: AttachmentType<u8> = AttachmentType {
    namespace: m::DATA_NAMESPACE,
    name: m::SURGERY_DATA,
    default: || 0,
    codec: Some(AttachmentCodec {
        encode: encode_surgery,
        decode: decode_surgery,
    }),
    copy_on_death: false,
    sync: true,
};

/// Гены игрока (`biolab:player_genes`): копия `PlayerState.genes` для окна.
pub static PLAYER_GENES: AttachmentType<String> = AttachmentType {
    namespace: m::DATA_NAMESPACE,
    name: m::PLAYER_GENES_DATA,
    default: String::new,
    codec: Some(AttachmentCodec {
        encode: encode_genes,
        decode: decode_genes,
    }),
    copy_on_death: true,
    sync: true,
};

/// Гены моба по порядку таблицы; пусто — обычный моб (или моба нет).
pub fn genes(game: &mut crate::game::Game<'_>, id: u32) -> Vec<(&'static str, i32)> {
    let host: &mut dyn Host = game;
    match GENOME.get_existing(host, Holder::Entity(id)) {
        Ok(Some(text)) => m::genes_of_data(&text),
        _ => Vec::new(),
    }
}

/// Гены нескольких мобов разом: для обходов всех мобов, где данные внутри
/// цикла не прочитать. Мобы без генов в ответ не попадают.
pub fn genes_by_id(
    game: &mut crate::game::Game<'_>,
    ids: &[u32],
) -> std::collections::HashMap<u32, Vec<(&'static str, i32)>> {
    let mut out = std::collections::HashMap::new();
    for id in ids {
        let found = genes(game, *id);
        if !found.is_empty() {
            out.insert(*id, found);
        }
    }
    out
}

/// Записать геном моба; пустой — снять данные. Пишем, только если
/// изменилось: каждая запись уходит клиентам. Прибавки к атрибутам по новому
/// геному выставит проход мутантов `biolab-deep` (`mutant_tick.rs`). Ложь —
/// носителя нет.
pub fn set_genes(game: &mut crate::game::Game<'_>, id: u32, genes: &[(&'static str, i32)]) -> bool {
    let text = m::genome_data(genes);
    let host: &mut dyn Host = game;
    let holder = Holder::Entity(id);
    let now = match GENOME.get_existing(&mut *host, holder) {
        Ok(now) => now,
        Err(AttachmentError::MissingHolder) => return false,
        // Битое значение — перепишем.
        Err(_) => None,
    };
    if text.is_empty() {
        return now.is_none() || GENOME.remove(host, holder).is_ok();
    }
    if now.as_deref() == Some(text.as_str()) {
        return true;
    }
    GENOME.set(host, holder, text).is_ok()
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

/// Записать стол и срезы моба, сохранив «спит» (его ставит `biolab-deep`).
/// Всё по нулям — данные снимаются. Только если изменилось.
pub fn set_surgery(game: &mut crate::game::Game<'_>, id: u32, on_table: bool, flayed: u8) -> bool {
    let host: &mut dyn Host = game;
    let holder = Holder::Entity(id);
    let now = match SURGERY.get_existing(&mut *host, holder) {
        Ok(now) => now,
        Err(AttachmentError::MissingHolder) => return false,
        Err(_) => None,
    };
    let bits = m::surgery_bits(on_table, flayed, now.unwrap_or(0));
    if bits == 0 {
        return now.is_none() || SURGERY.remove(host, holder).is_ok();
    }
    if now == Some(bits) {
        return true;
    }
    SURGERY.set(host, holder, bits).is_ok()
}

/// Гены игрока — в его данные, для окна. Только если изменились.
pub fn set_player_genes(game: &mut crate::game::Game<'_>, uuid: u128, genes: &[(&'static str, i32)]) -> bool {
    let text = m::genome_data(genes);
    let host: &mut dyn Host = game;
    let holder = Holder::Player(uuid);
    let now = match PLAYER_GENES.get_existing(&mut *host, holder) {
        Ok(now) => now,
        Err(AttachmentError::MissingHolder) => return false,
        Err(_) => None,
    };
    if now.as_deref() == Some(text.as_str()) {
        return true;
    }
    // Пустая строка тоже записывается: так окно узнаёт, что наросты пора
    // снять, а после смерти игрока копия не воскресит снятый ген.
    PLAYER_GENES.set(host, holder, text).is_ok()
}
