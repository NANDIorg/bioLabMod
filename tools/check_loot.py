#!/usr/bin/env python3
"""Проверка выпадения блоков BioLab (без зависимостей).

Запуск из корня репозитория:
    python3 tools/check_loot.py [--facts <папка mod-content из кэша Horizon>]

Что проверяет:
  * у каждого блока с предметом (`"item": true` в horizon/block) есть
    таблица `loot_table/blocks/<имя>.json`, и она роняет сам этот блок;
  * у блоков без предмета (лужи) таблицы нет: ронять им нечего;
  * исключение — культуры (шаблон `minecraft:beetroots`, GDD 4.20): своего
    предмета нет, а таблица есть и роняет семена `<блок>_seeds`;
  * таблицы написаны в формате 26.3, который читает движок Horizon
    (`horizon-items/src/items.rs`, `LootContext::condition`): условие —
    строка или объект с ключом `type`. Старый вид `{"condition": "..."}`
    без `type` движок считает незнакомым условием и отвечает «нет» —
    из-за этого операционный стол ничего не ронял. Модификаторы — ключ
    `modifier`, а не `functions`;
  * предмет в записи таблицы — ванильный или наш (блок с предметом или
    `horizon/item`);
  * если шаблон блока требует инструмент (`requires_tool`), блок стоит в
    ванильном теге `data/minecraft/tags/block/mineable/*.json` мода: без
    этого ни один инструмент не считается подходящим и лут не катится
    вовсе (`game.rs`, `break_block_as_player`: `harvest`).

`requires_tool` берётся из свойств блока, иначе — из таблицы шаблонов ниже.
Если найден кэш Horizon (`gameplay-facts.json` и `facts.json`; путь по
умолчанию — `~/Library/Application Support/horizon/.horizon-cache/mod-content`),
значения сверяются с тем, что движок реально выставил блоку.

Код выхода 1, если есть ошибки.
"""
import glob
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "content", "data")
NS = "biolab"
BLOCKS = os.path.join(DATA, NS, "horizon", "block")
ITEMS = os.path.join(DATA, NS, "horizon", "item")
LOOT = os.path.join(DATA, NS, "loot_table")
TOOL_TAGS = os.path.join(DATA, "minecraft", "tags", "block", "mineable")
DEFAULT_FACTS = os.path.expanduser(
    "~/Library/Application Support/horizon/.horizon-cache/mod-content")

# `requires_tool` ванильных шаблонов (`requiresCorrectToolForDrops`), снято с
# gameplay-facts.json пина 31f8349. Новый шаблон — добавить сюда.
TEMPLATE_REQUIRES_TOOL = {
    "minecraft:furnace": True,
    "minecraft:snow": True,
    "minecraft:crafting_table": False,
    "minecraft:mushroom_stem": False,
    "minecraft:beetroots": False,
    # Части многоблочных установок (GDD 4.18).
    "minecraft:iron_block": True,
    "minecraft:glass": False,
    # Блоки порчи ландшафта (GDD 4.21, M10).
    "minecraft:coarse_dirt": False,
}

# Шаблоны культур (GDD 4.20): у блока нет своего предмета — семена отдельный
# предмет `<блок>_seeds`, — но таблица у него есть и роняет семена.
CROP_TEMPLATES = {"minecraft:beetroots"}

# Виды условий и записей, которые знает движок Horizon (items.rs).
CONDITIONS = {
    "minecraft:survives_explosion", "minecraft:tool/can_silk_touch",
    "minecraft:tool/can_shear", "minecraft:inverted", "minecraft:any_of",
    "minecraft:all_of", "minecraft:random_chance", "minecraft:table_bonus",
    "minecraft:match_block", "minecraft:block_state_property",
    "minecraft:match_tool", "minecraft:killed_by_player",
    "minecraft:random_chance_with_enchanted_bonus", "minecraft:location_check",
}
ENTRIES = {
    "minecraft:item", "minecraft:alternatives", "minecraft:group",
    "minecraft:sequence", "minecraft:dynamic", "minecraft:empty",
    "minecraft:tag", "minecraft:loot_table",
}

errors = []
warnings = []


def err(where, msg):
    errors.append(f"{where}: {msg}")


def warn(where, msg):
    warnings.append(f"{where}: {msg}")


def read(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def rel(path):
    return os.path.relpath(path, ROOT)


def check_condition(where, c):
    if isinstance(c, str):
        kind, body = c, {}
    elif isinstance(c, dict):
        if "type" not in c:
            # Ровно баг стола: `{"condition": "minecraft:survives_explosion"}`.
            err(where, f"условие без \"type\" — движок его отвергнет: {json.dumps(c)}")
            return
        kind, body = c["type"], c
    elif isinstance(c, list):
        for i, part in enumerate(c):
            check_condition(f"{where}[{i}]", part)
        return
    else:
        err(where, f"условие не строка и не объект: {c!r}")
        return
    if kind.startswith("minecraft:") and kind not in CONDITIONS:
        err(where, f"незнакомое движку условие {kind}")
    if "term" in body:
        check_condition(f"{where}.term", body["term"])
    for i, t in enumerate(body.get("terms", [])):
        check_condition(f"{where}.terms[{i}]", t)


def check_conditions(where, node):
    if "condition" in node:
        check_condition(f"{where}.condition", node["condition"])
    conds = node.get("conditions")
    if conds is not None:
        if not isinstance(conds, list):
            err(where, "\"conditions\" должен быть списком")
        else:
            for i, c in enumerate(conds):
                check_condition(f"{where}.conditions[{i}]", c)
    if "functions" in node:
        err(where, "\"functions\" движок 26.3 не читает — модификаторы пишутся в \"modifier\"")


def entry_items(where, e, known_items, out):
    """Предметы, которые запись может уронить."""
    check_conditions(where, e)
    kind = e.get("type")
    if kind not in ENTRIES:
        err(where, f"незнакомый вид записи {kind!r}")
        return
    if kind == "minecraft:item":
        name = e.get("name", "")
        if not name.startswith("minecraft:") and name not in known_items:
            err(where, f"предмета {name!r} нет ни у ванили, ни у мода")
        out.add(name)
    for i, child in enumerate(e.get("children", [])):
        entry_items(f"{where}.children[{i}]", child, known_items, out)


def check_table(path, known_items):
    where = rel(path)
    try:
        table = read(path)
    except (OSError, ValueError) as e:
        err(where, f"не читается: {e}")
        return set()
    check_conditions(where, table)
    pools = table.get("pools")
    if not isinstance(pools, list) or not pools:
        err(where, "нет \"pools\": таблица ничего не уронит")
        return set()
    dropped = set()
    for p, pool in enumerate(pools):
        pw = f"{where} pools[{p}]"
        check_conditions(pw, pool)
        if "rolls" not in pool:
            err(pw, "нет \"rolls\"")
        for i, e in enumerate(pool.get("entries", [])):
            entry_items(f"{pw}.entries[{i}]", e, known_items, dropped)
    return dropped


def tagged_blocks():
    """Блок → в каких тегах mineable/* мода он стоит."""
    tags = {}
    for path in sorted(glob.glob(os.path.join(TOOL_TAGS, "*.json"))):
        tag = os.path.splitext(os.path.basename(path))[0]
        for v in read(path).get("values", []):
            v = v["id"] if isinstance(v, dict) else v
            tags.setdefault(v, []).append(tag)
    return tags


def engine_requires_tool(facts_dir):
    """`requires_tool` по умолчанию у наших блоков из кэша Horizon, если он есть."""
    gameplay = os.path.join(facts_dir, "gameplay-facts.json")
    facts = os.path.join(facts_dir, "facts.json")
    if not (os.path.isfile(gameplay) and os.path.isfile(facts)):
        return None
    defaults = read(facts).get("defaults", {})
    states = read(gameplay).get("states", [])
    out = {}
    for name, sid in defaults.items():
        if name.startswith(NS + ":") and 0 <= sid < len(states):
            out[name] = bool(states[sid].get("requires_tool"))
    return out


def main(argv):
    facts_dir = DEFAULT_FACTS
    if "--facts" in argv:
        facts_dir = argv[argv.index("--facts") + 1]

    blocks = {}
    for path in sorted(glob.glob(os.path.join(BLOCKS, "*.json"))):
        name = f"{NS}:{os.path.splitext(os.path.basename(path))[0]}"
        blocks[name] = read(path)
    known_items = {n for n, b in blocks.items() if b.get("item", True)}
    known_items |= {f"{NS}:{os.path.splitext(os.path.basename(p))[0]}"
                    for p in glob.glob(os.path.join(ITEMS, "*.json"))}

    tables = {}
    for path in sorted(glob.glob(os.path.join(LOOT, "**", "*.json"), recursive=True)):
        tables[os.path.relpath(path, LOOT)] = check_table(path, known_items)

    tags = tagged_blocks()
    engine = engine_requires_tool(facts_dir)
    drops = []
    for name, decl in blocks.items():
        short = name.split(":", 1)[1]
        where = f"horizon/block/{short}.json"
        table = f"blocks/{short}.json"
        has_item = decl.get("item", True)
        template = decl.get("template", "")
        if template in CROP_TEMPLATES:
            seeds = f"{name}_seeds"
            if has_item:
                err(where, "у культуры не должно быть своего предмета: семена — отдельный предмет")
            elif table not in tables:
                err(where, f"культура без таблицы loot_table/{table} — ничего не уронит")
            elif seeds not in tables[table]:
                err(where, f"loot_table/{table} не роняет семена {seeds}")
            else:
                drops.append(short)
            continue
        if has_item:
            if table not in tables:
                err(where, f"блок с предметом, а таблицы loot_table/{table} нет — ломается в никуда")
            elif name not in tables[table]:
                err(where, f"loot_table/{table} не роняет сам блок {name}")
            else:
                drops.append(short)
        elif table in tables:
            err(where, f"у блока без предмета есть loot_table/{table}")

        props = decl.get("properties", {})
        if "requires_tool" in props:
            requires = bool(props["requires_tool"])
        elif template in TEMPLATE_REQUIRES_TOOL:
            requires = TEMPLATE_REQUIRES_TOOL[template]
        else:
            err(where, f"шаблон {template} не знаю: допишите его в TEMPLATE_REQUIRES_TOOL")
            continue
        if engine is not None and name in engine and engine[name] != requires:
            err(where, f"requires_tool: ждём {requires}, а движок выставил {engine[name]}")
        if requires and has_item and name not in tags:
            err(where, "шаблон требует инструмент, а блока нет в data/minecraft/tags/block/mineable/*: "
                       "лут не выпадет ни с чем")

    for tagged in tags:
        if tagged.startswith(NS + ":") and tagged not in blocks:
            err("tags/block/mineable", f"{tagged} — такого блока нет")

    for w in warnings:
        print("предупреждение:", w)
    for e in errors:
        print("ОШИБКА:", e)
    source = "кэш Horizon" if engine is not None else "таблица шаблонов (кэш не найден)"
    print(f"Блоков: {len(blocks)}, выпадают: {len(drops)}, таблиц: {len(tables)}; "
          f"requires_tool — {source}.")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
