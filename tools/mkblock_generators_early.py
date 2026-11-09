#!/usr/bin/env python3
"""Генераторы тиров 0–2 BioLab (M10, GDD 4.27 Б1): текстуры, модели, blockstates,
определения предметов.

Запуск из корня репозитория:
    python3 tools/mkblock_generators_early.py
    python3 tools/check_models.py
    python3 tools/check_item_models.py

Пишет (перезаписывает) в content/assets/biolab/:
- textures/block/gen_*.png — свои текстуры генераторов (общие материалы —
  machine_* из mkblock_machines.py, их не трогаем);
- models/block/<генератор>.json и <генератор>_on.json (работает: `lit=true`,
  у пластины — `powered=true`);
- blockstates/<генератор>.json: у генераторов на шаблоне печки `facing`
  поворачивает модель, `lit` выбирает _on; у пластины — `powered`; у динамо
  одно состояние;
- items/<генератор>.json, а для пластин столба — textures/item/<id>.png,
  models/item/<id>.json, items/<id>.json.

Правила геометрии — как у машин (см. mkblock_machines.py): полное основание,
детали внутри 0…16 по X и Z, не выше 20 по Y, у граней `uv`. Кисти, PNG и
сборка элементов берутся оттуда же.
"""
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mkblock_machines as mk  # noqa: E402

E = mk.E
NO_D = mk.NO_D
ASSETS = mk.ASSETS
ITEM_TEX = os.path.join(ASSETS, "textures", "item")
ITEM_MODELS = os.path.join(ASSETS, "models", "item")
ITEMS = os.path.join(ASSETS, "items")

SHARED = {"steel": "machine_steel", "frame": "machine_frame", "copper": "machine_copper",
          "glass": "machine_glass", "vent": "machine_vent"}

WOOD = (138, 100, 58)
WOOD_DARK = (92, 64, 36)
ZINC = (168, 176, 182)
FELT = (150, 120, 92)
CANVAS = (226, 222, 210)
FIRE = (255, 150, 40)
SPARK = (170, 220, 255)


# ---------------------------------------------------------------- текстуры

def lamp(c, on):
    img = mk.canvas(mk.EDGE)
    mk.rect(img, 1, 1, 15, 15, c if on else mk.dim(c))
    if on:
        mk.rect(img, 5, 5, 11, 11, mk.shade(c, 40))
    mk.grain(img, 3, mk.stable(f"gen_lamp{c}{on}"))
    return img


def disc():
    """Стеклянный диск машины Уимсхёрста с медными секторами."""
    img = mk.canvas((150, 200, 214))
    mk.grain(img, 4, mk.stable("gen_disc"))
    for k in range(8):
        a = k * math.pi / 4
        for r in range(3, 7):
            x, y = int(round(7.5 + r * math.cos(a))), int(round(7.5 + r * math.sin(a)))
            mk.put(img, x, y, mk.COPPER_LIGHT)
    mk.rect(img, 6, 6, 10, 10, mk.STEEL_DARK)
    mk.outline(img, 0, 0, 16, 16, mk.FRAME)
    return img


def piezo(on):
    img = mk.plate(mk.STEEL_DARK, mk.stable("gen_piezo"), rivets=True)
    # Кристаллы кварца сеткой, горят при нажатии.
    for x in (4, 8, 12):
        for y in (4, 8, 12):
            c = SPARK if on else (196, 200, 210)
            mk.rect(img, x - 1, y - 1, x + 1, y + 1, c)
    return img


def pile_side():
    """Столб: чередуются цинк, медь и войлок, пропитанный кислотой."""
    img = mk.canvas(ZINC)
    for y in range(16):
        c = (ZINC, mk.COPPER, FELT)[y % 3]
        mk.rect(img, 0, y, 16, y + 1, c)
    mk.grain(img, 5, mk.stable("gen_pile"))
    return img


def thermo(c, sign):
    img = mk.plate(c, mk.stable(f"gen_thermo{sign}"))
    mk.rect(img, 6, 7, 10, 9, mk.EDGE)
    if sign == "+":
        mk.rect(img, 7, 6, 9, 10, mk.EDGE)
    return img


def thermo_front(on):
    img = mk.plate(mk.STEEL, mk.stable("gen_thermo_front"))
    mk.dial(img, 6, 6, angle=30 if on else -60)
    return img


def boiler_front(on):
    img = mk.plate(mk.STEEL_DARK, mk.stable("gen_boiler_front"))
    mk.rect(img, 3, 4, 13, 13, mk.EDGE)
    mk.rect(img, 4, 5, 12, 12, FIRE if on else (40, 30, 26))
    if on:
        mk.rect(img, 6, 7, 10, 11, (255, 220, 120))
    for x in range(4, 12, 2):
        mk.put(img, x, 4, mk.STEEL_LIGHT)
    return img


def drum():
    img = mk.plate(mk.COPPER, mk.stable("gen_drum"), amp=5)
    for y in (3, 12):
        mk.rect(img, 0, y, 16, y + 1, mk.COPPER_DARK)
    return img


def coil():
    img = mk.canvas(mk.COPPER)
    for y in range(0, 16, 2):
        mk.rect(img, 0, y, 16, y + 1, mk.COPPER_LIGHT)
    mk.grain(img, 4, mk.stable("gen_coil"))
    return img


def wood():
    img = mk.canvas(WOOD)
    for y in (0, 5, 10, 15):
        mk.rect(img, 0, y, 16, y + 1, WOOD_DARK)
    mk.grain(img, 6, mk.stable("gen_wood"))
    return img


def canvas_tex():
    img = mk.canvas(CANVAS)
    mk.grain(img, 5, mk.stable("gen_canvas"))
    mk.outline(img, 0, 0, 16, 16, (180, 172, 156))
    return img


def plate_item(c, dark, light, mark):
    """Пластина в инвентаре: косой прямоугольник с бликом и клеймом."""
    img = [[(0, 0, 0, 0) for _ in range(16)] for _ in range(16)]
    for y in range(3, 13):
        shift = (12 - y) // 3
        for x in range(2 + shift, 12 + shift):
            img[y][x] = (*c, 255)
    for y in range(3, 13):
        shift = (12 - y) // 3
        img[y][2 + shift] = (*light, 255)
        img[y][11 + shift] = (*dark, 255)
    for x in range(2, 15):
        if img[12][x][3]:
            img[12][x] = (*dark, 255)
        if img[3][x][3]:
            img[3][x] = (*light, 255)
    for (x, y) in mark:
        img[y][x] = (*dark, 255)
    return img


def textures():
    out = {
        "gen_lamp": lamp((90, 230, 120), False),
        "gen_lamp_on": lamp((90, 230, 120), True),
        "gen_disc": disc(),
        "gen_piezo": piezo(False),
        "gen_piezo_on": piezo(True),
        "gen_pile": pile_side(),
        "gen_thermo_copper": thermo(mk.COPPER, "+"),
        "gen_thermo_iron": thermo(mk.STEEL_LIGHT, "-"),
        "gen_thermo_front": thermo_front(False),
        "gen_thermo_front_on": thermo_front(True),
        "gen_boiler_front": boiler_front(False),
        "gen_boiler_front_on": boiler_front(True),
        "gen_drum": drum(),
        "gen_coil": coil(),
        "gen_wood": wood(),
        "gen_canvas": canvas_tex(),
    }
    for name, img in out.items():
        mk.write_png(name, img)
    os.makedirs(ITEM_TEX, exist_ok=True)
    mk.write_png("zinc_plate", plate_item(ZINC, (110, 118, 124), (220, 226, 230), [(7, 7), (8, 7), (8, 8), (7, 9), (8, 9)]),
                 folder=ITEM_TEX)
    mk.write_png("copper_plate", plate_item(mk.COPPER, mk.COPPER_DARK, mk.COPPER_LIGHT, [(7, 7), (8, 7), (7, 8), (7, 9), (8, 9)]),
                 folder=ITEM_TEX)


# ---------------------------------------------------------------- модели

def electrophore():
    return [
        E([0, 0, 0], [16, 2, 16], "frame", up="steel"),
        E([2, 2, 2], [4, 15, 4], "steel", faces=NO_D),
        E([12, 2, 2], [14, 15, 4], "steel", faces=NO_D),
        E([3, 3, 6], [13, 15, 7], "disc", faces=NO_D),
        E([3, 3, 9], [13, 15, 10], "disc", faces=NO_D),
        E([7, 8, 4], [9, 10, 14], "steel", faces=NO_D),
        *mk.cyl(2, 6, 11, 15, 2, 9, "glass", faces=NO_D),
        E([10, 2, 11], [14, 9, 15], "copper", faces=NO_D),
        E([14, 8, 8], [16, 10, 9], "steel", faces=NO_D),
        E([15, 8, 9], [16, 13, 10], "frame", faces=NO_D),
        E([6, 15, 7], [10, 16, 9], "lamp", faces=NO_D, glow=10),
    ]


def piezo_plate(pressed):
    h = 3 if pressed else 4
    return [
        E([0, 0, 0], [16, 2, 16], "frame", up="steel"),
        E([1, 2, 1], [15, h, 15], "top", faces=NO_D),
    ]


def voltaic_pile():
    return [
        E([0, 0, 0], [16, 2, 16], "frame", up="steel"),
        E([4, 2, 4], [12, 14, 12], "pile", faces=NO_D, up="copper"),
        E([3, 14, 3], [13, 15, 13], "steel", faces=NO_D),
        E([2, 2, 2], [3, 15, 3], "glass", faces=NO_D),
        E([13, 2, 2], [14, 15, 3], "glass", faces=NO_D),
        E([2, 2, 13], [3, 15, 14], "glass", faces=NO_D),
        E([13, 2, 13], [14, 15, 14], "glass", faces=NO_D),
        E([5, 15, 7], [6, 17, 8], "copper", faces=NO_D),
        E([10, 15, 7], [11, 17, 8], "steel", faces=NO_D),
        E([6, 2, 0.5], [10, 4, 1.5], "lamp", faces=("up", "north", "west", "east"), glow=10),
    ]


def thermocouple():
    return [
        E([0, 0, 0], [8, 16, 16], "thermo_copper", north="front_l"),
        E([8, 0, 0], [16, 16, 16], "thermo_iron", north="front_r"),
    ]


def steam_boiler():
    return [
        E([0, 0, 0], [16, 6, 16], "frame", north="front", up="steel"),
        *mk.cyl(1, 15, 1, 15, 6, 15, "drum", up="drum"),
        E([2, 15, 2], [14, 16, 14], "steel", faces=NO_D),
        E([6, 16, 6], [10, 16.5, 10], "steel", faces=("up", "north", "south", "west", "east")),
        E([12, 9, 0], [14, 13, 1], "lamp", faces=("up", "down", "north", "west", "east"), glow=10),
    ]


def dynamo():
    return [
        E([0, 0, 0], [16, 2, 16], "frame", up="steel"),
        E([1, 2, 3], [15, 4, 13], "steel", faces=NO_D),
        *mk.cyl(3, 13, 3, 13, 4, 14, "coil", faces=NO_D, up="steel"),
        E([0, 8, 7], [3, 10, 9], "steel", faces=NO_D),
        E([13, 8, 7], [16, 10, 9], "steel", faces=NO_D),
    ]


def water_wheel():
    o = (8, 8, 8)
    return [
        E([0, 0, 0], [16, 1, 16], "frame", up="steel"),
        E([0, 1, 13], [16, 16, 16], "frame", faces=NO_D),
        E([7, 7, 2], [9, 9, 13], "steel", faces=NO_D),
        E([6, 6, 4], [10, 10, 7], "steel", faces=NO_D),
        E([7, 1.5, 5], [9, 14.5, 6], "wood", faces=NO_D),
        E([1.5, 7, 5], [14.5, 9, 6], "wood", faces=NO_D),
        E([7, 1.5, 5], [9, 14.5, 6], "wood", faces=NO_D, rot=("z", 45, o)),
        E([7, 1.5, 5], [9, 14.5, 6], "wood", faces=NO_D, rot=("z", -45, o)),
        E([2, 14, 2], [14, 16, 9], "wood", faces=NO_D),
        E([2, 1, 2], [14, 2, 9], "wood", faces=NO_D),
        E([0, 2, 2], [2, 14, 9], "wood", faces=NO_D),
        E([14, 2, 2], [16, 14, 9], "wood", faces=NO_D),
        E([11, 12, 12], [13, 14, 13], "lamp", faces=("up", "north", "west", "east"), glow=8),
    ]


def wind_turbine():
    o = (8, 9, 3)
    return [
        E([0, 0, 0], [16, 1, 16], "frame", up="steel"),
        E([6, 1, 6], [10, 6, 10], "steel", faces=NO_D),
        E([5, 6, 4], [11, 12, 14], "steel", faces=NO_D, north="frame"),
        E([6.5, 7.5, 2], [9.5, 10.5, 4], "frame", faces=NO_D),
        E([7, 2, 2.5], [9, 16, 3.5], "canvas", faces=NO_D, rot=("z", 45, o)),
        E([7, 2, 2.5], [9, 16, 3.5], "canvas", faces=NO_D, rot=("z", -45, o)),
        E([7.5, 10, 13], [8.5, 15, 16], "canvas", faces=NO_D),
        E([7.5, 12, 8], [8.5, 13, 9], "lamp", faces=NO_D, glow=12),
    ]


OWN_TEX = {
    "lamp": "gen_lamp", "disc": "gen_disc", "pile": "gen_pile",
    "thermo_copper": "gen_thermo_copper", "thermo_iron": "gen_thermo_iron",
    "drum": "gen_drum", "coil": "gen_coil", "wood": "gen_wood", "canvas": "gen_canvas",
}
# Что меняется в работающей модели.
LIT_TEX = {"lamp": "gen_lamp_on"}

# Генераторы на шаблоне печки: имя → (сборка, частица, свои текстуры по ключу).
FURNACE = {
    "electrophore": (electrophore, "machine_frame", {}),
    "voltaic_pile": (voltaic_pile, "gen_pile", {}),
    "thermocouple": (thermocouple, "gen_thermo_copper",
                     {"front_l": "gen_thermo_front", "front_r": "gen_thermo_front"}),
    "steam_boiler": (steam_boiler, "gen_drum", {"front": "gen_boiler_front"}),
    "water_wheel": (water_wheel, "gen_wood", {}),
    "wind_turbine": (wind_turbine, "machine_steel", {}),
}
# Работающая модель: своя текстура взамен (фронт котла горит, стрелка термопары).
FURNACE_ON = {
    "thermocouple": {"front_l": "gen_thermo_front_on", "front_r": "gen_thermo_front_on"},
    "steam_boiler": {"front": "gen_boiler_front_on"},
}


def tex_map(elements, particle, own):
    keys = mk.used_keys(elements)
    tex = {"particle": "biolab:block/" + particle}
    for k in sorted(keys):
        name = own.get(k) or OWN_TEX.get(k) or SHARED.get(k)
        if not name:
            raise SystemExit(f"неизвестная текстура #{k}")
        tex[k] = "biolab:block/" + name
    return tex


def write_model(name, elements, tex, lit):
    mk.dump_model(os.path.join(mk.MODELS, name + ".json"), {
        "parent": "minecraft:block/block",
        "textures": tex,
        "elements": [mk.element_json(e, lit) for e in elements],
    })


def write_states(name, variants):
    with open(os.path.join(mk.STATES, name + ".json"), "w") as f:
        f.write('{ "variants": {\n')
        f.write(",\n".join(f'  "{k}": {json.dumps(v)}' for k, v in variants.items()))
        f.write(" } }\n")


def write_item(name, model):
    with open(os.path.join(ITEMS, name + ".json"), "w") as f:
        f.write(json.dumps({"model": {"type": "minecraft:model", "model": model}}, indent=1) + "\n")


def furnace_generator(name):
    build, particle, own = FURNACE[name]
    elements = build()
    tex = tex_map(elements, particle, own)
    write_model(name, elements, tex, False)
    on = dict(tex)
    for k, v in {**LIT_TEX, **FURNACE_ON.get(name, {})}.items():
        if k in on:
            on[k] = "biolab:block/" + v
    write_model(name + "_on", elements, on, True)
    variants = {}
    for facing, y in (("north", 0), ("east", 90), ("south", 180), ("west", 270)):
        for lit in ("false", "true"):
            v = {"model": f"biolab:block/{name}{'_on' if lit == 'true' else ''}"}
            if y:
                v["y"] = y
            variants[f"facing={facing},lit={lit}"] = v
    write_states(name, variants)
    write_item(name, f"biolab:block/{name}")
    return len(elements)


def main():
    textures()
    for name in FURNACE:
        n = furnace_generator(name)
        print(f"{name:14s} элементов {n:2d}")
    # Нажимная пластина: шаблон iron_block со своим `powered`.
    for pressed in (False, True):
        els = piezo_plate(pressed)
        tex = tex_map(els, "machine_frame", {"top": "gen_piezo_on" if pressed else "gen_piezo"})
        write_model("piezo_plate" + ("_on" if pressed else ""), els, tex, pressed)
    write_states("piezo_plate", {"powered=false": {"model": "biolab:block/piezo_plate"},
                                 "powered=true": {"model": "biolab:block/piezo_plate_on"}})
    write_item("piezo_plate", "biolab:block/piezo_plate")
    # Динамо: шаблон iron_block, одно состояние.
    els = dynamo()
    write_model("dynamo", els, tex_map(els, "gen_coil", {}), False)
    write_states("dynamo", {"": {"model": "biolab:block/dynamo"}})
    write_item("dynamo", "biolab:block/dynamo")
    # Пластины столба — плоские предметы.
    os.makedirs(ITEM_MODELS, exist_ok=True)
    for name in ("zinc_plate", "copper_plate"):
        with open(os.path.join(ITEM_MODELS, name + ".json"), "w") as f:
            f.write('{\n  "parent": "minecraft:item/generated",\n'
                    f'  "textures": {{"layer0": "biolab:item/{name}"}}\n}}\n')
        write_item(name, f"biolab:item/{name}")
    print("пластина, динамо, пластины столба")


if __name__ == "__main__":
    main()
