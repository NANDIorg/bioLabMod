#!/usr/bin/env python3
"""Порча ландшафта BioLab (GDD 4.21, 4.27 Г, M10): три блока порчи и чистка.

Запуск из корня репозитория:
    python3 tools/mkblight.py
    python3 -I tools/check_models.py
    python3 -I tools/check_loot.py
    python3 -I tools/check_item_models.py

Пишет:
  * блоки `infected_grass`, `necrotic_soil` (шаблон `minecraft:coarse_dirt`:
    не тикает и трава на неё не ползёт), `spore_stalk` (шаблон стебля
    гриба, как секции стола 4.17) и `decon_station` (шаблон железного
    блока — без блок-сущности: запас хлора лежит в данных чанка) —
    `content/data/biolab/horizon/block/<id>.json`, blockstates, модели,
    текстуры 16×16, таблицы выпадения (каждый роняет себя);
  * предмет `sprayer` (шаблон бумаги, стопка 1; заряды — `custom_data`
    `biolab_spray`): плоская иконка, модель, рецепт верстака;
  * рецепты распылителя и станции.

Строки языка, вкладку и теги инструментов скрипт не трогает: они общие и
дописываются руками в конец.
"""
import importlib.util
import json
import os
import random
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("mkitem3d", os.path.join(HERE, "mkitem3d.py"))
m3 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(m3)

ROOT = os.path.dirname(HERE)
ASSETS = m3.ASSETS
DATA = os.path.join(ROOT, "content", "data", "biolab")


def dump(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=1)
        f.write("\n")


def noise(seed):
    rnd = random.Random(seed)
    return [[rnd.random() for _ in range(16)] for _ in range(16)]


def texture(kind, name, px):
    path = os.path.join(ASSETS, "textures", kind, f"{name}.png")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    m3.png(path, 16, 16, px)


C = m3.hexrgb

# ---- Текстуры блоков ------------------------------------------------------

def infected_top():
    n = noise(11)
    base, dark, spot = C("#7d7f4e"), C("#5c5a3a"), C("#8a5a8c")

    def px(x, y):
        v = n[y][x]
        c = m3.mix(base, dark, v * 0.8)
        if v > 0.93:
            c = spot
        elif v < 0.07:
            c = m3.shade(base, 18)
        return m3.rgba(c)
    texture("block", "infected_grass_top", px)


def infected_side():
    n = noise(12)
    dirt, dirt_dark = C("#79553a"), C("#5a3f2b")
    grass, grass_dark = C("#7d7f4e"), C("#5c5a3a")

    def px(x, y):
        v = n[y][x]
        edge = 3 + (1 if v > 0.6 else 0) + (1 if (x * 7) % 5 == 0 and v > 0.3 else 0)
        if y < edge:
            c = m3.mix(grass, grass_dark, v * 0.8)
            if v > 0.95:
                c = C("#8a5a8c")
        else:
            c = m3.mix(dirt, dirt_dark, v)
            if v > 0.94:
                c = C("#4a4038")
        return m3.rgba(c)
    texture("block", "infected_grass_side", px)


def necrotic():
    n = noise(13)
    base, dark, pale = C("#4a3f39"), C("#2f2824"), C("#8c8274")
    cracks = {(x, (5 + x // 2 + (x * x) % 3) % 16) for x in range(16)} | {((3 + y // 3) % 16, y) for y in range(8, 16)}

    def px(x, y):
        v = n[y][x]
        c = m3.mix(base, dark, v * 0.7)
        if (x, y) in cracks:
            c = C("#1c1714")
        elif v > 0.95:
            c = pale
        return m3.rgba(c)
    texture("block", "necrotic_soil", px)


def stalk_stem():
    n = noise(14)
    base, dark = C("#b9a8bf"), C("#8a7690")

    def px(x, y):
        v = n[y][x]
        stripe = (x % 4 == 1)
        c = m3.mix(base, dark, 0.5 if stripe else v * 0.4)
        return m3.rgba(c)
    texture("block", "spore_stalk_stem", px)


def stalk_cap():
    n = noise(15)
    base, dark, dot = C("#7a4a86"), C("#53305e"), C("#d9c2e0")

    def px(x, y):
        v = n[y][x]
        c = m3.mix(base, dark, v * 0.6)
        if (x * 5 + y * 3) % 11 == 0 and v > 0.3:
            c = dot
        return m3.rgba(c)
    texture("block", "spore_stalk_cap", px)


def station_side():
    n = noise(16)
    steel, steel_dark = C("#9aa3a8"), C("#6f777c")

    def px(x, y):
        v = n[y][x]
        c = m3.mix(steel, steel_dark, v * 0.35)
        if x in (0, 15) or y in (0, 15):
            c = C("#5a6166")
        if 11 <= y <= 13 and 1 <= x <= 14:
            c = C("#e0b420") if ((x + y) // 2) % 2 == 0 else C("#2b2f33")
        if 4 <= x <= 8 and 3 <= y <= 7:
            c = C("#2b2f33") if x in (4, 8) or y in (3, 7) else C("#c9e8d0")
            if (x, y) == (6, 5) or (x, y) == (7, 4):
                c = C("#c0392b")
        if 10 <= x <= 12 and 3 <= y <= 7:
            c = C("#4a5056") if y % 2 else C("#3a3f44")
        return m3.rgba(c)
    texture("block", "decon_station_side", px)


def station_top():
    n = noise(17)
    steel, steel_dark = C("#9aa3a8"), C("#6f777c")

    def px(x, y):
        v = n[y][x]
        c = m3.mix(steel, steel_dark, v * 0.35)
        d = (x - 7.5) ** 2 + (y - 7.5) ** 2
        if 20 <= d <= 34:
            c = C("#e0b420")
        elif d < 6:
            c = C("#2b2f33")
        elif d < 20 and (x + y) % 2 == 0:
            c = C("#4a5056")
        if x in (0, 15) or y in (0, 15):
            c = C("#5a6166")
        return m3.rgba(c)
    texture("block", "decon_station_top", px)


def station_bottom():
    n = noise(18)

    def px(x, y):
        return m3.rgba(m3.mix(C("#5a6166"), C("#3a3f44"), n[y][x] * 0.5))
    texture("block", "decon_station_bottom", px)


# ---- Иконка распылителя ---------------------------------------------------

SPRAYER = [
    "................",
    "................",
    "..........kkk...",
    ".........kKKk...",
    "....yyyy.kKk.ss.",
    "...yYYYyyk....s.",
    "...yYyyyy.......",
    "...yYyyyo.......",
    "...yYyyyo.......",
    "...yyyyyo.......",
    "....oooo........",
    "....k..k........",
    "....kk.k........",
    ".....k.kk.......",
    ".....kkk........",
    "................",
]
PALETTE = {
    "y": "#e0b420", "Y": "#f2d35a", "o": "#a9821a",
    "k": "#2b2f33", "K": "#6f777c", "s": "#c9e8d0",
}


def sprayer_icon():
    colours = {k: C(v) for k, v in PALETTE.items()}

    def px(x, y):
        ch = SPRAYER[y][x]
        return m3.CLEAR if ch == "." else m3.rgba(colours[ch])
    texture("item", "sprayer", px)


# ---- Данные ---------------------------------------------------------------

BLOCKS = {
    "infected_grass": {"template": "minecraft:coarse_dirt",
                       "properties": {"hardness": 0.6, "explosion_resistance": 0.6, "sounds": "minecraft:grass_block"}},
    "necrotic_soil": {"template": "minecraft:coarse_dirt",
                      "properties": {"hardness": 0.7, "explosion_resistance": 0.7, "sounds": "minecraft:gravel"}},
    "spore_stalk": {"template": "minecraft:mushroom_stem",
                    "properties": {"hardness": 0.4, "explosion_resistance": 0.4, "sounds": "minecraft:nether_wart_block"}},
    "decon_station": {"template": "minecraft:iron_block",
                      "properties": {"hardness": 3.0, "explosion_resistance": 6.0, "requires_tool": True,
                                     "sounds": "minecraft:iron_block"}},
}

MODELS = {
    "infected_grass": {"parent": "minecraft:block/cube_bottom_top", "textures": {
        "top": "biolab:block/infected_grass_top", "side": "biolab:block/infected_grass_side",
        "bottom": "minecraft:block/dirt", "particle": "biolab:block/infected_grass_side"}},
    "necrotic_soil": {"parent": "minecraft:block/cube_all", "textures": {
        "all": "biolab:block/necrotic_soil", "particle": "biolab:block/necrotic_soil"}},
    "decon_station": {"parent": "minecraft:block/cube_bottom_top", "textures": {
        "top": "biolab:block/decon_station_top", "side": "biolab:block/decon_station_side",
        "bottom": "biolab:block/decon_station_bottom", "particle": "biolab:block/decon_station_side"}},
}


def box(frm, to, tex, faces=("north", "south", "east", "west", "up", "down")):
    return {"from": frm, "to": to, "faces": {f: {"texture": tex} for f in faces}}


SPORE_STALK_MODEL = {
    "parent": "minecraft:block/block",
    "textures": {"stem": "biolab:block/spore_stalk_stem", "cap": "biolab:block/spore_stalk_cap",
                 "particle": "biolab:block/spore_stalk_cap"},
    "elements": [
        box([6, 0, 6], [10, 11, 10], "#stem", ("north", "south", "east", "west", "down")),
        box([3, 11, 3], [13, 15, 13], "#cap"),
        box([5, 15, 5], [11, 16, 11], "#cap", ("north", "south", "east", "west", "up")),
    ],
}

RECIPES = {
    "sprayer": ([" I ", "IBI", " S "], {
        "I": "minecraft:iron_ingot", "B": "minecraft:bucket", "S": "minecraft:stick"}),
    "decon_station": (["ISI", "CFC", "III"], {
        "I": "minecraft:iron_ingot", "S": "biolab:sprayer", "C": "biolab:casing",
        "F": "biolab:filter_cartridge"}),
}


def loot(name):
    return {
        "type": "minecraft:block",
        "pools": [{
            "condition": {"type": "minecraft:survives_explosion"},
            "entries": [{"type": "minecraft:item", "name": f"biolab:{name}"}],
            "rolls": 1,
        }],
        "random_sequence": f"biolab:blocks/{name}",
    }


def main():
    for make in (infected_top, infected_side, necrotic, stalk_stem, stalk_cap,
                 station_side, station_top, station_bottom, sprayer_icon):
        make()
    for name, block in BLOCKS.items():
        dump(os.path.join(DATA, "horizon", "block", f"{name}.json"), dict(block, item=True))
        dump(os.path.join(DATA, "loot_table", "blocks", f"{name}.json"), loot(name))
        dump(os.path.join(ASSETS, "items", f"{name}.json"),
             {"model": {"type": "minecraft:model", "model": f"biolab:block/{name}"}})
        if name == "spore_stalk":
            dump(os.path.join(ASSETS, "models", "block", f"{name}.json"), SPORE_STALK_MODEL)
            dump(os.path.join(ASSETS, "blockstates", f"{name}.json"),
                 {"multipart": [{"apply": {"model": f"biolab:block/{name}"}}]})
        else:
            dump(os.path.join(ASSETS, "models", "block", f"{name}.json"), MODELS[name])
            dump(os.path.join(ASSETS, "blockstates", f"{name}.json"),
                 {"variants": {"": {"model": f"biolab:block/{name}"}}})
    dump(os.path.join(DATA, "horizon", "item", "sprayer.json"),
         {"template": "minecraft:paper", "max_stack_size": 1})
    dump(os.path.join(ASSETS, "items", "sprayer.json"),
         {"model": {"type": "minecraft:model", "model": "biolab:item/sprayer"}})
    dump(os.path.join(ASSETS, "models", "item", "sprayer.json"),
         {"parent": "minecraft:item/generated", "textures": {"layer0": "biolab:item/sprayer"}})
    for name, (pattern, key) in RECIPES.items():
        dump(os.path.join(DATA, "recipe", f"{name}.json"), {
            "type": "minecraft:crafting_shaped",
            "category": "misc",
            "pattern": pattern,
            "key": key,
            "result": {"id": f"biolab:{name}", "count": 1},
        })
    print(f"порча: {len(BLOCKS)} блока, распылитель, {len(RECIPES)} рецепта")


if __name__ == "__main__":
    main()
