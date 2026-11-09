#!/usr/bin/env python3
"""Энергосеть BioLab (M10, GDD 4.27 А): кабели, повторитель, накопители, батарея.

Запуск из корня репозитория:
    python3 -I -B tools/mkpower.py   (-B: без tools/__pycache__ от импорта)
    python3 -I tools/check_models.py

Пишет (перезаписывает) в content/assets/biolab/:
- кабели и повторитель (`copper_cable`, `power_cable`, `cable_repeater`) —
  блоки на шаблоне `minecraft:chorus_plant` (`connects_to`, `docs/mods.md`
  пина): платформа сама ставит свойства сторон north…down по соседям из
  тега `#biolab:cable_connects`. Модели: `<id>_core` (рисуется всегда) и
  `<id>_side` (рука к северу; blockstate-multipart поворачивает её к каждой
  соединённой стороне, как у ванильного хоруса), `<id>_inventory` —
  прямой отрезок для предмета;
- накопители (`leyden_jar`, `lead_battery`, `capacitor_bank`) — блоки на
  шаблоне `minecraft:iron_block` со своей моделью из элементов (правила
  машин из `check_models.py`: низ закрыт, детали в 0…16);
- батарея-предмет `battery` — плоская иконка.
Генератор детерминирован: повторный прогон не меняет файлы.
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mkblock_machines as mb  # noqa: E402  (кисти, PNG, элементы моделей)

ASSETS = mb.ASSETS
TEX = mb.TEX
MODELS = mb.MODELS
STATES = mb.STATES
ITEM_TEX = os.path.join(ASSETS, "textures", "item")
ITEM_MODELS = os.path.join(ASSETS, "models", "item")
ITEMS = os.path.join(ASSETS, "items")

RUBBER = (44, 40, 38)
RUBBER_RED = (150, 44, 34)
BRAID = (150, 156, 160)
GLASS = (176, 206, 196)
LEAD = (78, 82, 88)
CAN_BLUE = (58, 92, 170)


# ---------------------------------------------------------------- текстуры

def cable_tex(base, stripe, seed):
    """Изоляция с продольной полосой меди (или оплётки) и кольцами."""
    img = mb.canvas(base)
    mb.grain(img, 4, seed)
    for y in range(16):
        mb.put(img, 7, y, stripe)
        mb.put(img, 8, y, mb.shade(stripe, -30))
    for x in range(16):
        if x % 5 == 0:
            for y in range(16):
                img[y][x] = mb.shade(img[y][x], -18) + (255,)
    return img


def repeater_tex():
    img = mb.plate(mb.STEEL, mb.stable("cable_repeater") & 0xFFF)
    mb.outline(img, 3, 3, 13, 13, mb.FRAME)
    mb.rect(img, 4, 4, 12, 12, mb.STEEL_DARK)
    for x in range(5, 11):  # медная катушка
        mb.put(img, x, 6, mb.COPPER_LIGHT if x % 2 else mb.COPPER)
        mb.put(img, x, 9, mb.COPPER_LIGHT if x % 2 else mb.COPPER)
    mb.glow(img, 7, 7, mb.TEAL)
    mb.glow(img, 8, 8, mb.TEAL)
    return img


def jar_glass():
    img = mb.canvas(GLASS)
    mb.grain(img, 3, 11)
    for y in range(16):
        mb.put(img, 3, y, mb.shade(GLASS, 30))
        mb.put(img, 12, y, mb.shade(GLASS, -24))
    for y in range(9, 16):  # медная фольга снизу
        for x in range(16):
            mb.put(img, x, y, mb.COPPER if (x + y) % 5 else mb.COPPER_LIGHT)
    mb.rect(img, 0, 8, 16, 9, mb.COPPER_DARK)
    return img


def jar_top():
    img = mb.canvas((120, 90, 60))  # пробка
    mb.grain(img, 5, 12)
    mb.outline(img, 0, 0, 16, 16, (80, 58, 40))
    return img


def brass():
    img = mb.canvas((214, 176, 84))
    mb.grain(img, 6, 13)
    mb.put(img, 5, 5, (255, 230, 150))
    return img


def lead_side():
    img = mb.canvas(LEAD)
    mb.grain(img, 4, 14)
    for x in range(1, 16, 3):  # рёбра банок
        for y in range(16):
            mb.put(img, x, y, mb.shade(LEAD, -22))
    mb.rect(img, 3, 5, 13, 10, (226, 200, 70))  # табличка
    mb.outline(img, 3, 5, 13, 10, mb.EDGE)
    for x in range(5, 11):
        mb.put(img, x, 7, mb.EDGE if x % 2 else (226, 200, 70))
    mb.rect(img, 0, 0, 16, 1, mb.shade(LEAD, 20))
    return img


def lead_top():
    img = mb.canvas(mb.shade(LEAD, 10))
    mb.grain(img, 3, 15)
    for x, y in ((3, 3), (7, 3), (11, 3), (3, 11), (7, 11), (11, 11)):
        mb.put(img, x, y, mb.EDGE)  # пробки банок
    return img


def terminal(colour):
    img = mb.canvas(colour)
    mb.grain(img, 5, sum(colour))
    mb.bevel(img, 0, 0, 16, 16, 30, -40)
    return img


def can_side():
    img = mb.canvas(CAN_BLUE)
    for y in range(16):
        mb.rect(img, 0, y, 16, y + 1, mb.shade(CAN_BLUE, 10 - abs(8 - y) * 2))
    for x in range(16):
        mb.put(img, x, 2, mb.shade(CAN_BLUE, 40))
    mb.rect(img, 6, 6, 10, 12, (230, 232, 236))  # белая метка «−»
    mb.rect(img, 7, 6, 9, 12, mb.shade(CAN_BLUE, -30))
    return img


def can_top():
    img = mb.canvas((196, 200, 206))
    mb.grain(img, 3, 16)
    for i in range(16):  # насечка «крест»
        mb.put(img, i, i, (140, 146, 152))
        mb.put(img, 15 - i, i, (140, 146, 152))
    return img


def battery_icon():
    """Батарея-предмет: медный цилиндр с полосой заряда и плюсом."""
    img = mb.canvas((0, 0, 0), 0)
    body = (70, 74, 80)
    mb.rect(img, 4, 3, 12, 15, body)
    mb.outline(img, 4, 3, 12, 15, mb.EDGE)
    mb.rect(img, 6, 1, 10, 3, mb.COPPER_LIGHT)
    mb.outline(img, 6, 1, 10, 3, mb.EDGE)
    for y in range(5, 13):
        for x in range(5, 11):
            mb.put(img, x, y, mb.COPPER if x < 7 else mb.shade(body, 10))
    mb.rect(img, 8, 6, 9, 9, mb.TEAL)   # «+»
    mb.rect(img, 7, 7, 10, 8, mb.TEAL)
    mb.rect(img, 7, 11, 10, 12, (230, 232, 236))  # «−»
    return img


# ---------------------------------------------------------------- модели

def dump(path, model):
    mb.dump_model(path, model)


def box(f, t, tex, **over):
    return mb.element_json(mb.E(f, t, tex, **over), False)


def cable_models(name, half, tex, arm=None):
    """Ядро толщиной 2×half и рука к северу. `arm` — своя толщина руки
    (повторитель: толстое ядро, рука как у медного кабеля)."""
    lo, hi = 8 - half, 8 + half
    a = arm if arm is not None else half
    alo, ahi = 8 - a, 8 + a
    textures = {"cable": f"biolab:block/{tex}", "particle": f"biolab:block/{tex}"}
    dump(os.path.join(MODELS, f"{name}_core.json"), {
        "parent": "minecraft:block/block",
        "ambientocclusion": False,
        "textures": textures,
        "elements": [box([lo, lo, lo], [hi, hi, hi], "cable")],
    })
    side_tex = dict(textures)
    if arm is not None:
        side_tex["cable"] = "biolab:block/copper_cable"
    dump(os.path.join(MODELS, f"{name}_side.json"), {
        "ambientocclusion": False,
        "textures": side_tex,
        "elements": [box([alo, alo, 0], [ahi, ahi, lo], "cable",
                         faces=("down", "up", "north", "west", "east"))],
    })
    inventory = [box([alo, alo, 0], [ahi, ahi, 16], "cable")]
    if arm is not None:
        inventory = [box([alo, alo, 0], [ahi, ahi, lo], "arm"),
                     box([lo, lo, lo], [hi, hi, hi], "cable"),
                     box([alo, alo, hi], [ahi, ahi, 16], "arm")]
        textures = dict(textures, arm="biolab:block/copper_cable")
    dump(os.path.join(MODELS, f"{name}_inventory.json"), {
        "parent": "minecraft:block/block",
        "textures": textures,
        "elements": inventory,
    })
    parts = [{"apply": {"model": f"biolab:block/{name}_core"}}]
    for side, rot in (("north", {}), ("east", {"y": 90}), ("south", {"y": 180}), ("west", {"y": 270}),
                      ("up", {"x": 270}), ("down", {"x": 90})):
        parts.append({"when": {side: "true"}, "apply": dict({"model": f"biolab:block/{name}_side"}, **rot)})
    with open(os.path.join(STATES, name + ".json"), "w") as f:
        f.write('{ "multipart": [\n')
        f.write(",\n".join("  " + json.dumps(p) for p in parts))
        f.write("\n] }\n")
    item(name, f"biolab:block/{name}_inventory")


def machine_model(name, elements, textures):
    textures = dict(textures)
    textures.setdefault("particle", next(iter(textures.values())))
    dump(os.path.join(MODELS, name + ".json"), {
        "parent": "minecraft:block/block",
        "textures": textures,
        "elements": elements,
    })
    with open(os.path.join(STATES, name + ".json"), "w") as f:
        f.write('{ "variants": { "": { "model": "biolab:block/%s" } } }\n' % name)
    item(name, f"biolab:block/{name}")


def item(name, model):
    with open(os.path.join(ITEMS, name + ".json"), "w") as f:
        f.write('{ "model": { "type": "minecraft:model", "model": "%s" } }\n' % model)


def leyden_jar():
    frame = "biolab:block/machine_frame"
    els = [
        box([0, 0, 0], [16, 1, 16], "base"),                       # подставка
        box([4, 1, 5], [12, 11, 11], "glass"),                     # банка
        box([5, 1, 4], [11, 11, 5], "glass"),
        box([5, 1, 11], [11, 11, 12], "glass"),
        box([5, 11, 5], [11, 12, 11], "cork"),                     # пробка
        box([7.5, 12, 7.5], [8.5, 15, 8.5], "rod"),                # стержень
        box([7, 15, 7], [9, 16, 9], "ball"),                       # шарик
    ]
    machine_model("leyden_jar", els, {"base": frame, "glass": "biolab:block/leyden_jar_glass",
                                      "cork": "biolab:block/leyden_jar_top", "rod": "biolab:block/machine_steel",
                                      "ball": "biolab:block/leyden_jar_ball",
                                      "particle": "biolab:block/leyden_jar_glass"})


def lead_battery():
    els = [
        box([0, 0, 0], [16, 1, 16], "case"),
        box([1, 1, 2], [15, 11, 14], "case", up="top"),
        box([3, 11, 6], [5, 13, 8], "plus"),     # клеммы
        box([11, 11, 6], [13, 13, 8], "minus"),
        box([2, 1, 1], [14, 10, 2], "case"),      # стенки-рёбра
        box([2, 1, 14], [14, 10, 15], "case"),
    ]
    machine_model("lead_battery", els, {"case": "biolab:block/lead_battery_side",
                                        "top": "biolab:block/lead_battery_top",
                                        "plus": "biolab:block/lead_battery_plus",
                                        "minus": "biolab:block/lead_battery_minus"})


def capacitor_bank():
    els = [box([0, 0, 0], [16, 2, 16], "frame")]
    for x0 in (1, 9):
        for z0 in (1, 9):
            els += [mb.element_json(e, False) for e in
                    mb.cyl(x0, x0 + 6, z0, z0 + 6, 2, 14, "can", up="cap")]
    els.append(box([2, 14, 3], [14, 15, 5], "bus"))     # медные шины
    els.append(box([2, 14, 11], [14, 15, 13], "bus"))
    machine_model("capacitor_bank", els, {"frame": "biolab:block/machine_frame",
                                          "can": "biolab:block/capacitor_bank_can",
                                          "cap": "biolab:block/capacitor_bank_top",
                                          "bus": "biolab:block/machine_copper",
                                          "particle": "biolab:block/capacitor_bank_can"})


def main():
    os.makedirs(TEX, exist_ok=True)
    mb.write_png("copper_cable", cable_tex(RUBBER, mb.COPPER_LIGHT, 21))
    mb.write_png("power_cable", cable_tex(RUBBER_RED, BRAID, 22))
    mb.write_glowing("cable_repeater", repeater_tex())
    mb.write_png("leyden_jar_glass", jar_glass())
    mb.write_png("leyden_jar_top", jar_top())
    mb.write_png("leyden_jar_ball", brass())
    mb.write_png("lead_battery_side", lead_side())
    mb.write_png("lead_battery_top", lead_top())
    mb.write_png("lead_battery_plus", terminal((200, 60, 50)))
    mb.write_png("lead_battery_minus", terminal((50, 52, 58)))
    mb.write_png("capacitor_bank_can", can_side())
    mb.write_png("capacitor_bank_top", can_top())

    cable_models("copper_cable", 2, "copper_cable")
    cable_models("power_cable", 3, "power_cable")
    cable_models("cable_repeater", 5, "cable_repeater", arm=2)
    leyden_jar()
    lead_battery()
    capacitor_bank()

    mb.write_png("battery", battery_icon(), ITEM_TEX)
    dump(os.path.join(ITEM_MODELS, "battery.json"), {
        "parent": "minecraft:item/generated",
        "textures": {"layer0": "biolab:item/battery"},
    })
    item("battery", "biolab:item/battery")
    print("энергосеть: 3 проводника, 3 накопителя, батарея")


if __name__ == "__main__":
    main()
