#!/usr/bin/env python3
"""Проверка генерации мира BioLab (без зависимостей): фичи, размещения и
модификаторы биомов.

Запуск из корня репозитория:
    python3 tools/check_worldgen.py [--worldgen <папка 26.3 кэша Horizon>]

Зачем отдельно. Модификатор биома (`data/biolab/neoforge/biome_modifier/`)
Horizon применяет, когда собирает корень контента (`mod_content.rs`,
`place_in_biomes`; разбор — `mod_content_biomes.rs`), и **битый модификатор
читаемого вида останавливает сборку целиком: игра стартует без всех
контентных модов** («content mods not loaded: ...» в логе). Битый — это
нечитаемый JSON, нет `type`, у `add_features` нет `step`, `biomes` или
`features`, незнакомый шаг генерации. Ссылка на незнакомый биом, тег или
фичу и тег внутри списка — мягче: модификатор пропускается с предупреждением
в `horizon-mod-content-warnings.txt`, и заросли молча не растут. Фича или
размещение, которые генератор не разобрал, тоже молча пропускаются
(`unsupported_features`). Поэтому всё это здесь — ошибки.

Что проверяет:
  * каждый JSON в worldgen/ и neoforge/biome_modifier/ читается;
  * модификатор: вид из читаемых (`add_features`, `remove_features`, `none`),
    шаг — из одиннадцати шагов генерации (`STEPS`), `biomes` и `features` —
    id, `#тег` или список id (без тегов), объекты `any`/`or`/`and`/`not`;
    биомы и теги биомов есть у ванили, фичи — у мода или у ванили;
  * размещение: `feature` есть, модификаторы размещения — известные движку,
    предикаты блоков ссылаются на существующие блоки и теги;
  * фича: вид из поддержанных здесь (`block_column`, `simple_block`), у
    `block_column` все обязательные поля; состояние блока — существующий
    блок с допустимыми свойствами (у блока мода — свойства его шаблона);
  * `simple_block` с культурой — ошибка: генерация спросит правило выживания
    культуры (пашня под ней) и не поставит ничего; для зарослей — `block_column`;
  * размещение мода без модификатора и фича без размещения — предупреждение.

Ванильные биомы, теги, фичи и блоки берутся из кэша версии Horizon
(`versions/*/.local/horizon-client/26.3`, пин `31f8349` в приоритете). Без
кэша ссылки на ваниль не проверяются (предупреждение), остальное — да.

Код выхода 1, если есть ошибки.
"""
import glob
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "content", "data")
NS = "biolab"
MOD = os.path.join(DATA, NS)

# `horizon-world/src/worldgen/structure/start.rs`, STEPS.
STEPS = [
    "raw_generation", "lakes", "local_modifications", "underground_structures",
    "surface_structures", "strongholds", "underground_ores", "underground_decoration",
    "fluid_springs", "vegetal_decoration", "top_layer_modification",
]
# Модификаторы размещения, которые разбирает движок (`worldgen/placement.rs`).
PLACEMENTS = {
    "minecraft:biome", "minecraft:block_predicate_filter", "minecraft:environment_scan",
    "minecraft:heightmap", "minecraft:surface_relative_threshold_filter",
    "minecraft:surface_water_depth_filter", "minecraft:noise_based_count",
    "minecraft:noise_threshold_count", "minecraft:count_on_every_layer", "minecraft:count",
    "minecraft:in_square", "minecraft:cuboid", "minecraft:offset", "minecraft:height_range",
    "minecraft:rarity_filter", "minecraft:random_chance", "minecraft:fixed_placement",
    "minecraft:randomly_selected",
}
# Предикаты блоков (`worldgen/block_predicate.rs`).
PREDICATES = {
    "minecraft:would_survive", "minecraft:has_sturdy_face", "minecraft:replaceable",
    "minecraft:unobstructed", "minecraft:matching_biomes", "minecraft:matching_fluids",
    "minecraft:matching_blocks", "minecraft:matching_block_tag", "minecraft:height_range",
    "minecraft:volume_match", "minecraft:true", "minecraft:not", "minecraft:any_of",
    "minecraft:all_of", "minecraft:inside_world_bounds", "minecraft:solid",
}
HEIGHTMAPS = {"WORLD_SURFACE_WG", "WORLD_SURFACE", "OCEAN_FLOOR_WG", "OCEAN_FLOOR",
              "MOTION_BLOCKING", "MOTION_BLOCKING_NO_LEAVES"}
DIRECTIONS = {"down", "up", "north", "south", "west", "east"}
# Фичи, которые здесь проверяются по полям. Остальные виды — ошибка: добавить
# вид сюда вместе с его проверкой.
FEATURES = {"minecraft:block_column", "minecraft:simple_block"}
# Шаблоны с правилом выживания культуры: на траве `simple_block` их не ставит.
CROP_TEMPLATES = {"minecraft:beetroots", "minecraft:wheat", "minecraft:carrots", "minecraft:potatoes"}

errors = []
warnings = []


def err(where, msg):
    errors.append(f"{where}: {msg}")


def warn(where, msg):
    warnings.append(f"{where}: {msg}")


def rel(path):
    return os.path.relpath(path, ROOT)


def load(path):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError) as e:
        err(rel(path), f"не читается: {e}")
        return None


def full(i):
    return i if ":" in i else "minecraft:" + i


class Vanilla:
    """Ванильные реестры из кэша Horizon; None — проверить нельзя."""

    def __init__(self, base):
        self.base = base
        self.biomes = self.biome_tags = self.placed = self.features = None
        self.blocks = self.block_tags = None
        if not base:
            return
        wg = os.path.join(base, "worldgen")
        bundle = lambda name: set(json.load(open(os.path.join(wg, name), encoding="utf-8")))
        self.biomes = bundle("biomes.json")
        self.biome_tags = bundle("biome_tags.json")
        self.placed = bundle("placed_features.json")
        self.features = bundle("features.json")
        self.block_tags = bundle("block_tags.json")
        self.blocks = set(json.load(open(os.path.join(base, "facts.json"), encoding="utf-8"))["defaults"])

    def blockstate_properties(self, block):
        """Свойства ванильного блока по его blockstate: имя → значения."""
        if not self.base:
            return None
        path = os.path.join(self.base, "assets", "minecraft", "blockstates", block.split(":", 1)[1] + ".json")
        if not os.path.isfile(path):
            return None
        doc = json.load(open(path, encoding="utf-8"))
        props = {}
        for key in doc.get("variants", {}):
            for pair in filter(None, key.split(",")):
                k, _, v = pair.partition("=")
                props.setdefault(k, set()).add(v)
        return props


def find_base(argv):
    if "--worldgen" in argv:
        return argv[argv.index("--worldgen") + 1]
    pattern = os.path.expanduser(
        "~/Library/Application Support/horizon-launcher/versions/*/.local/horizon-client/26.3")
    found = [p for p in sorted(glob.glob(pattern))
             if os.path.isfile(os.path.join(p, "worldgen", "biomes.json"))
             and os.path.isfile(os.path.join(p, "facts.json"))]
    pinned = [p for p in found if "31f8349" in p]
    return (pinned or found or [None])[-1]


def mod_files(kind):
    return {f"{NS}:{os.path.relpath(p, os.path.join(MOD, *kind.split('/')))[:-5]}": p
            for p in sorted(glob.glob(os.path.join(MOD, *kind.split("/"), "**", "*.json"), recursive=True))}


def mod_block(block):
    path = os.path.join(MOD, "horizon", "block", block.split(":", 1)[1] + ".json")
    return load(path) if os.path.isfile(path) else None


def check_holders(where, value, what, ids, tags, mod_ids=()):
    """`biomes`/`features` модификатора: id, #тег, список id или объект NeoForge."""
    if isinstance(value, str):
        if value.startswith("#"):
            tag = full(value[1:])
            if tags is not None and tag not in tags:
                err(where, f"тега {what} #{tag} нет — модификатор пропустят")
            return
        i = full(value)
        if i.startswith(NS + ":"):
            if i not in mod_ids:
                err(where, f"{what} {i} у мода нет")
        elif ids is not None and i not in ids:
            err(where, f"{what} {i} у ванили нет — модификатор пропустят")
        return
    if isinstance(value, list):
        if not value:
            err(where, f"пустой список {what}")
        for v in value:
            if not isinstance(v, str) or v.startswith("#"):
                err(where, f"в списке {what} только id, без тегов: {v!r} — модификатор пропустят")
            else:
                check_holders(where, v, what, ids, tags, mod_ids)
        return
    if isinstance(value, dict):
        kind = str(value.get("type", "")).removeprefix("neoforge:")
        if kind == "any":
            return
        if kind in ("or", "and"):
            for v in value.get("values") or err(where, f"{kind}: нет values") or []:
                check_holders(where, v, what, ids, tags, mod_ids)
            return
        if kind == "not":
            if "value" not in value:
                err(where, "not: нет value")
            else:
                check_holders(where, value["value"], what, ids, tags, mod_ids)
            return
        err(where, f"вид набора {kind!r} движок не читает — модификатор пропустят")
        return
    err(where, f"{what}: не id, не тег и не список")


def check_modifier(path, vanilla, placed_ids):
    where = rel(path)
    doc = load(path)
    if doc is None:
        err(where, "битый модификатор валит загрузку ВСЕХ контентных модов")
        return []
    kind = doc.get("type")
    if not isinstance(kind, str):
        err(where, "нет \"type\": загрузка всех контентных модов остановится")
        return []
    short = kind.removeprefix("neoforge:").removeprefix("forge:")
    if short == "none":
        return []
    if short not in ("add_features", "remove_features"):
        warn(where, f"вид {kind} Horizon пропускает: читаются только add_features, remove_features, none")
        return []
    for key in ("biomes", "features"):
        if key not in doc:
            err(where, f"нет \"{key}\": загрузка всех контентных модов остановится")
    if short == "add_features":
        step = doc.get("step")
        if step not in STEPS:
            err(where, f"шаг {step!r} незнаком (нужен один из {', '.join(STEPS)}): "
                       "загрузка всех контентных модов остановится")
    else:
        steps = doc.get("steps", STEPS)
        for s in steps if isinstance(steps, list) else [steps]:
            if s not in STEPS:
                err(where, f"шаг {s!r} незнаком: загрузка всех контентных модов остановится")
    if "biomes" in doc:
        check_holders(where, doc["biomes"], "биома", vanilla.biomes, vanilla.biome_tags)
    if "features" in doc:
        check_holders(where, doc["features"], "размещённой фичи", vanilla.placed, None, placed_ids)
    feats = doc.get("features")
    return [feats] if isinstance(feats, str) else [f for f in feats or [] if isinstance(f, str)]


def check_predicate(where, p, vanilla):
    if not isinstance(p, dict) or "type" not in p:
        err(where, f"предикат без type: {p!r}")
        return
    kind = full(p["type"])
    if kind not in PREDICATES:
        err(where, f"предикат {kind} движок не знает")
    if kind in ("minecraft:all_of", "minecraft:any_of"):
        for i, q in enumerate(p.get("predicates", [])):
            check_predicate(f"{where}.predicates[{i}]", q, vanilla)
    if kind == "minecraft:not":
        check_predicate(f"{where}.predicate", p.get("predicate"), vanilla)
    if kind == "minecraft:matching_blocks":
        blocks = p.get("blocks")
        for b in [blocks] if isinstance(blocks, str) else blocks or []:
            if b.startswith("#"):
                if vanilla.block_tags is not None and full(b[1:]) not in vanilla.block_tags:
                    err(where, f"тега блоков {b} нет")
            elif not block_exists(full(b), vanilla):
                err(where, f"блока {b} нет")
    if kind == "minecraft:matching_block_tag":
        tag = full(p.get("tag", ""))
        if vanilla.block_tags is not None and tag not in vanilla.block_tags:
            err(where, f"тега блоков {tag} нет")


def block_exists(block, vanilla):
    if block.startswith(NS + ":"):
        return mod_block(block) is not None
    return vanilla.blocks is None or block in vanilla.blocks


def check_state(where, state, vanilla):
    """Состояние блока: сокращённая форма `{id, properties}` (как в ванильных
    features.json 26.3) или поставщик `simple_state_provider`."""
    if isinstance(state, dict) and state.get("type") == "minecraft:simple_state_provider":
        state = state.get("state", {})
        block, props = state.get("Name"), state.get("Properties", {})
    elif isinstance(state, dict) and "id" in state:
        block, props = state["id"], state.get("properties", {})
    elif isinstance(state, str):
        block, props = state, {}
    else:
        err(where, f"поставщик состояния не понят: {state!r}")
        return None
    block = full(block)
    decl = mod_block(block) if block.startswith(NS + ":") else None
    if block.startswith(NS + ":") and decl is None:
        err(where, f"блока {block} у мода нет")
        return None
    if not block.startswith(NS + ":") and not block_exists(block, vanilla):
        err(where, f"блока {block} нет")
        return None
    template = decl.get("template") if decl else block
    known = vanilla.blockstate_properties(template)
    if decl:
        for k, spec in decl.get("state_properties", {}).items():
            known = dict(known or {})
            known[k] = set(spec.get("values", ["false", "true"]))
    if known is not None:
        for k, v in props.items():
            if k not in known:
                err(where, f"{block}: у шаблона {template} нет свойства {k}")
            elif str(v) not in known[k]:
                err(where, f"{block}: {k}={v} — допустимы {sorted(known[k])}")
    return template


def check_feature(path, vanilla):
    where = rel(path)
    doc = load(path)
    if doc is None:
        return
    kind = full(str(doc.get("type", "")))
    if kind not in FEATURES:
        err(where, f"вид фичи {kind} здесь не проверяется: добавьте его в FEATURES с проверкой полей")
        return
    if kind == "minecraft:simple_block":
        template = check_state(f"{where}.to_place", doc.get("to_place"), vanilla)
        if template in CROP_TEMPLATES:
            err(where, f"simple_block проверит выживание культуры ({template}: пашня под ней) "
                       "и не поставит её на траву — нужен block_column")
        return
    for key in ("layers", "direction", "allowed_placement", "prioritize_tip"):
        if key not in doc:
            err(where, f"block_column без обязательного {key}: генератор фичу пропустит")
    if doc.get("direction") not in DIRECTIONS:
        err(where, f"направление {doc.get('direction')!r} незнакомо")
    if not isinstance(doc.get("prioritize_tip"), bool):
        err(where, "prioritize_tip — true или false")
    check_predicate(f"{where}.allowed_placement", doc.get("allowed_placement"), vanilla)
    layers = doc.get("layers")
    if not isinstance(layers, list) or not layers:
        err(where, "layers — непустой список")
        return
    for i, layer in enumerate(layers):
        h = layer.get("height")
        if not (isinstance(h, int) and h >= 0) and not (isinstance(h, dict) and "type" in h):
            err(f"{where}.layers[{i}]", f"высота {h!r}: целое ≥ 0 или поставщик")
        check_state(f"{where}.layers[{i}].provider", layer.get("provider"), vanilla)


def check_placed(path, vanilla, feature_ids):
    where = rel(path)
    doc = load(path)
    if doc is None:
        return None
    feature = doc.get("feature")
    if isinstance(feature, str):
        f = full(feature)
        if f.startswith(NS + ":"):
            if f not in feature_ids:
                err(where, f"фичи {f} у мода нет")
        elif vanilla.features is not None and f not in vanilla.features:
            err(where, f"фичи {f} у ванили нет")
    elif not isinstance(feature, dict):
        err(where, "нет \"feature\"")
    placement = doc.get("placement")
    if not isinstance(placement, list):
        err(where, "placement — список модификаторов")
        return feature
    for i, m in enumerate(placement):
        at = f"{where}.placement[{i}]"
        kind = full(str(m.get("type", ""))) if isinstance(m, dict) else ""
        if kind not in PLACEMENTS:
            err(at, f"модификатор размещения {kind!r} движок не знает: размещение пропустят")
            continue
        if kind == "minecraft:heightmap" and m.get("heightmap") not in HEIGHTMAPS:
            err(at, f"карта высот {m.get('heightmap')!r} незнакома")
        if kind == "minecraft:rarity_filter" and not (isinstance(m.get("chance"), int) and m["chance"] > 0):
            err(at, "rarity_filter: chance — целое > 0")
        if kind == "minecraft:block_predicate_filter":
            check_predicate(f"{at}.predicate", m.get("predicate"), vanilla)
    return feature


def main(argv):
    base = find_base(argv)
    vanilla = Vanilla(base)
    if base is None:
        warn("кэш Horizon", "не найден: ссылки на ванильные биомы, теги и блоки не проверены")

    features = mod_files("worldgen/configured_feature")
    placed = mod_files("worldgen/placed_feature")
    for path in features.values():
        check_feature(path, vanilla)
    used_features = set()
    for path in placed.values():
        f = check_placed(path, vanilla, features)
        if isinstance(f, str):
            used_features.add(full(f))
    used_placed = set()
    modifiers = []
    for sub in ("neoforge", "forge"):
        modifiers += sorted(glob.glob(os.path.join(MOD, sub, "biome_modifier", "**", "*.json"), recursive=True))
    for path in modifiers:
        used_placed.update(full(f) for f in check_modifier(path, vanilla, placed))
    # Модификатор в чужом месте (не biome_modifier/) Horizon не увидит вовсе.
    for path in glob.glob(os.path.join(MOD, "**", "*.json"), recursive=True):
        r = rel(path)
        if "biome_modifier" in r and path not in modifiers:
            err(r, "модификатор лежит не в data/<ns>/neoforge/biome_modifier/: Horizon его не прочтёт")

    for f in sorted(set(features) - used_features):
        warn(f, "фича без размещения")
    for p in sorted(set(placed) - used_placed):
        warn(p, "размещение не стоит ни в одном модификаторе биомов — не генерируется")

    for w in warnings:
        print("предупреждение:", w)
    for e in errors:
        print("ОШИБКА:", e)
    print(f"фич: {len(features)}, размещений: {len(placed)}, модификаторов биомов: {len(modifiers)}; "
          f"ваниль: {base or 'не найдена'}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
