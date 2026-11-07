#!/usr/bin/env python3
"""Спецодежда BioLab (GDD 4.25, M9): семь деталей двух ступеней.

Запуск из корня репозитория:
    python3 tools/mksuit.py
    python3 -I tools/check_item_models.py

Пишет для каждой детали:
  content/data/biolab/horizon/item/<id>.json — предмет на шаблоне кожаной
      брони (`minecraft:leather_*`): слот, прочность и слой на теле — шаблона
      (`equippable.asset_id` = `minecraft:leather`, цвет слоя — `dyed_color`
      стека, его ставит тик костюма). У химзащиты свой список модификаторов:
      броня шаблона и −2 % скорости за деталь, полный костюм — −8 %;
  content/assets/biolab/items/<id>.json, models/item/<id>.json,
      textures/item/<id>.png — плоская иконка 16×16 (одежду держат не в руке,
      объём ей ни к чему, как порошкам элементов, GDD 4.12);
  content/data/biolab/recipe/<id>.json — рецепт верстака.

Числа защиты — не здесь, а в `runtime.rs` (`SUIT_PIECES`).
"""
import importlib.util
import json
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("mkitem3d", os.path.join(HERE, "mkitem3d.py"))
m3 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(m3)

ROOT = os.path.dirname(HERE)
ASSETS = m3.ASSETS
DATA = os.path.join(ROOT, "content", "data", "biolab")

# Броня шаблона: (слот модификатора, id модификатора, броня).
ARMOR = {
    "helmet": ("head", "minecraft:armor.helmet", 1.0),
    "chestplate": ("chest", "minecraft:armor.chestplate", 3.0),
    "leggings": ("legs", "minecraft:armor.leggings", 2.0),
    "boots": ("feet", "minecraft:armor.boots", 1.0),
}
SPEED_PER_PIECE = -0.02

# Палитра: буква → цвет. «.» — прозрачно.
PALETTE = {
    "w": "#e6ebee", "W": "#ffffff", "g": "#b9c2c8", "G": "#8c979e",  # белая ткань
    "b": "#7fb6d8", "B": "#4f8cb5",                                      # голубое: воротник, бахилы
    "y": "#e0b420", "Y": "#f2d35a", "o": "#a9821a",                      # жёлтая химзащита
    "k": "#2b2f33", "K": "#4a5056",                                      # чёрное: фильтры, подошва
    "l": "#9fd8e8", "L": "#d6f2fa",                                      # стекло линз
    "s": "#8fd16a",                                                      # слизь-герметик на швах
}

SPRITES = {
    "lab_coat": [
        "................",
        "....bb....bb....",
        "...wwbb..bbww...",
        "..wwwwbbbbwwww..",
        ".wwwwwwbbwwwwww.",
        ".wwwwwwwGwwwwww.",
        ".wwgwwwwgwwwwgw.",
        ".wwg.wwwGwww.gw.",
        ".wwg.wwwgwww.gw.",
        "..gg.wwwGwww.gg.",
        "..gg.wwwgwww.gg.",
        ".....wwwGwww....",
        ".....wwwgwww....",
        ".....wwwGwww....",
        ".....gggggggg...",
        "................",
    ],
    "lab_pants": [
        "................",
        "....gggggggg....",
        "....wwwwwwww....",
        "....wwwwwwww....",
        "....wwwwwwww....",
        "....wwww.www....",
        "....www..www....",
        "....www..www....",
        "....www..www....",
        "....www..www....",
        "....www..www....",
        "....www..www....",
        "....www..www....",
        "....ggg..ggg....",
        "................",
        "................",
    ],
    "lab_shoes": [
        "................",
        "................",
        "................",
        "................",
        "................",
        "...bbb....bbb...",
        "...bLb....bLb...",
        "...bbb....bbb...",
        "...bbb....bbb...",
        "..bbbb...bbbb...",
        ".bbbbb..bbbbb...",
        ".bbbbb..bbbbb...",
        ".BBBBB..BBBBB...",
        "................",
        "................",
        "................",
    ],
    "hazmat_respirator": [
        "................",
        ".....yyyyyy.....",
        "....yYYYYYYy....",
        "...yYllllllYy...",
        "...yllLLlllly...",
        "...ylLLllllly...",
        "...yllllllllo...",
        "...oyyyyyyyyo...",
        "....oykkkkyo....",
        "..kk.ykKKky.kk..",
        ".kKKkykkkkykKKk.",
        ".kKKk.yyyy.kKKk.",
        "..kk..oooo..kk..",
        "................",
        "................",
        "................",
    ],
    "hazmat_suit": [
        "................",
        "....yy....yy....",
        "...yYyy..yyYy...",
        "..yYyyyyyyyyYy..",
        ".yyyyyysyyyyyyy.",
        ".yyyyyysyyyyyyy.",
        ".yyoyyysyyyyoyy.",
        ".yyo.yysyyy.oyy.",
        ".yyo.yysyyy.oyy.",
        ".kko.yysyyy.okk.",
        ".kk..yysyyy..kk.",
        ".....yysyyy.....",
        ".....yysyyy.....",
        ".....yysyyy.....",
        ".....oooooooo...",
        "................",
    ],
    "hazmat_pants": [
        "................",
        "....oooooooo....",
        "....yyyyyyyy....",
        "....yYyyyyyy....",
        "....yyyyyyyy....",
        "....yyys.yyy....",
        "....yyy..yyy....",
        "....yyy..yyy....",
        "....yyy..yyy....",
        "....yyy..yyy....",
        "....yyy..yyy....",
        "....yyy..yyy....",
        "....sss..sss....",
        "....yyy..yyy....",
        "................",
        "................",
    ],
    "hazmat_boots": [
        "................",
        "................",
        "................",
        "...yyy....yyy...",
        "...yYy....yYy...",
        "...yyy....yyy...",
        "...sss....sss...",
        "...kkk....kkk...",
        "...kkk....kkk...",
        "..kkkk...kkkk...",
        ".kKkkk..kKkkk...",
        ".kkkkk..kkkkk...",
        ".KKKKK..KKKKK...",
        "................",
        "................",
        "................",
    ],
}

# id → (часть шаблона, химзащита ли, рецепт верстака).
PIECES = {
    "lab_coat": ("chestplate", False, (["W W", "WGW", "WWW"], {"W": "minecraft:white_wool", "G": "minecraft:glass_pane"})),
    "lab_pants": ("leggings", False, (["WWW", "W W", "W W"], {"W": "minecraft:white_wool"})),
    "lab_shoes": ("boots", False, (["W W", "B B"], {"W": "minecraft:white_wool", "B": "minecraft:light_blue_wool"})),
    "hazmat_respirator": ("helmet", True, (["SGS", "IFI"], {
        "S": "minecraft:slime_ball", "G": "minecraft:glass_pane", "I": "minecraft:iron_ingot",
        "F": "biolab:filter_cartridge"})),
    "hazmat_suit": ("chestplate", True, ([" S ", "YCY", " S "], {
        "S": "minecraft:slime_ball", "Y": "minecraft:yellow_dye", "C": "biolab:lab_coat"})),
    "hazmat_pants": ("leggings", True, ([" S ", "YCY", " S "], {
        "S": "minecraft:slime_ball", "Y": "minecraft:yellow_dye", "C": "biolab:lab_pants"})),
    "hazmat_boots": ("boots", True, ([" S ", "KCK"], {
        "S": "minecraft:slime_ball", "K": "minecraft:black_dye", "C": "biolab:lab_shoes"})),
}


def dump(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=1)
        f.write("\n")


def sprite(name, rows):
    assert len(rows) == 16 and all(len(r) == 16 for r in rows), name
    colours = {k: m3.hexrgb(v) for k, v in PALETTE.items()}

    def px(x, y):
        c = rows[y][x]
        return m3.CLEAR if c == "." else m3.rgba(colours[c])

    m3.png(os.path.join(ASSETS, "textures", "item", f"{name}.png"), 16, 16, px)


def item_file(name, part, hazmat):
    item = {"template": f"minecraft:leather_{part}", "max_stack_size": 1}
    if hazmat:
        slot, armor_id, armor = ARMOR[part]
        item["components"] = {
            "minecraft:attribute_modifiers": [
                {"type": "minecraft:armor", "amount": armor, "id": armor_id, "operation": "add_value", "slot": slot},
                {"type": "minecraft:armor_toughness", "amount": 0.0, "id": armor_id, "operation": "add_value", "slot": slot},
                {"type": "minecraft:movement_speed", "amount": SPEED_PER_PIECE, "id": f"biolab:{name}_weight",
                 "operation": "add_multiplied_base", "slot": slot},
            ]
        }
    return item


def main():
    for name, (part, hazmat, (pattern, key)) in PIECES.items():
        sprite(name, SPRITES[name])
        dump(os.path.join(DATA, "horizon", "item", f"{name}.json"), item_file(name, part, hazmat))
        dump(os.path.join(ASSETS, "items", f"{name}.json"),
             {"model": {"type": "minecraft:model", "model": f"biolab:item/{name}"}})
        dump(os.path.join(ASSETS, "models", "item", f"{name}.json"),
             {"parent": "minecraft:item/generated", "textures": {"layer0": f"biolab:item/{name}"}})
        dump(os.path.join(DATA, "recipe", f"{name}.json"), {
            "type": "minecraft:crafting_shaped",
            "category": "equipment",
            "pattern": pattern,
            "key": key,
            "result": {"id": f"biolab:{name}", "count": 1},
        })
    print(f"спецодежда: {len(PIECES)} деталей")


if __name__ == "__main__":
    main()
