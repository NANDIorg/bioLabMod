#!/usr/bin/env python3
"""Свои растения BioLab (GDD 4.20): валериана, белладонна, мандрагора.

Запуск из корня репозитория:
    python3 tools/mkplants.py
    python3 -I tools/check_models.py && python3 -I tools/check_item_models.py

Пишет:
  content/assets/biolab/textures/block/<растение>_stage0..3.png — стадии роста;
  content/assets/biolab/models/block/<растение>_stage0..3.json  — модель ванильной
      культуры (`minecraft:block/crop`: четыре плоскости решёткой, как у свёклы);
  content/assets/biolab/blockstates/<растение>.json             — `age=0..3`;
  семена — плоские спрайты (`item/<id>`): горстка семян формы не имеет, как
      порошки элементов (GDD 4.12, «Остаются плоскими»);
  урожай — объёмные модели `item/3d/<форма>` (корни, корень-человечек, гроздь
      ягод): у них есть форма, как у стручка ГМ-урожая. Плоский спрайт
      `item/<id>` рисуется тоже — для частиц и на случай отката, как у
      остальных объёмных предметов.

Геометрию, PNG и пресеты положения в руке берём из `tools/mkitem3d.py`
(`box`, `recenter`, `upright`, `png`), чтобы урожай держался в руке так же,
как остальные предметы мода. Свой файл — чтобы не трогать общую таблицу
форм. Текстуры частей названы `plant_*`, чтобы не спорить с именами mkitem3d.
"""
import importlib.util
import json
import os
import random
import sys

# Без __pycache__ в tools/: mkitem3d подключаем как модуль.
sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("mkitem3d", os.path.join(HERE, "mkitem3d.py"))
m3 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(m3)

ASSETS = m3.ASSETS
BLOCK_TEX = os.path.join(ASSETS, "textures", "block")
ITEM_TEX = os.path.join(ASSETS, "textures", "item")
BLOCK_MODELS = os.path.join(ASSETS, "models", "block")
ITEM_MODELS = os.path.join(ASSETS, "models", "item")
BLOCKSTATES = os.path.join(ASSETS, "blockstates")

CLEAR = (0, 0, 0, 0)


def hexrgb(s):
    return m3.hexrgb(s)


# ---------------------------------------------------------------------------
# Холст 16×16
# ---------------------------------------------------------------------------


class Canvas:
    def __init__(self, seed):
        self.px = [[CLEAR] * 16 for _ in range(16)]
        self.rng = random.Random(seed)

    def put(self, x, y, c, shade=0):
        if 0 <= x < 16 and 0 <= y < 16:
            self.px[y][x] = m3.rgba(m3.shade(c, shade + self.rng.randint(-6, 6)))

    def stem(self, x, y0, y1, c, lean=0):
        """Стебель снизу (y0, ближе к земле) вверх до y1. `lean` — сдвиг по x
        каждые 4 пикселя: стебли не стоят по линейке."""
        for i, y in enumerate(range(y0, y1 - 1, -1)):
            self.put(x + (i // 4) * lean, y, c, -8 if i % 3 == 0 else 0)
        return x + ((y0 - y1) // 4) * lean

    def leaf(self, x, y, dx, c, length=3):
        """Лист от точки стебля в сторону dx (−1/1), чуть вверх."""
        for i in range(length):
            self.put(x + dx * (i + 1), y - (i + 1) // 2, c, 10 if i == 0 else 0)
            if i == 1:
                self.put(x + dx * (i + 1), y - (i + 1) // 2 + 1, c, -14)

    def save(self, path):
        m3.png(path, 16, 16, lambda x, y: self.px[y][x])


GREEN = hexrgb("#4f8f34")
GREEN_DARK = hexrgb("#336b25")
GREEN_PALE = hexrgb("#7dae4a")


def sprouts(c, xs, height):
    for x in xs:
        top = 15 - height - c.rng.randint(0, 1)
        c.stem(x, 15, top, GREEN)
        c.put(x - 1, top, GREEN_PALE)
        c.put(x + 1, top, GREEN_PALE)


# Валериана: перистые листья у земли, высокие стебли, к спелости — бледно-
# розовые зонтики соцветий.
def valerian(stage, c):
    if stage == 0:
        sprouts(c, [3, 8, 12], 3)
        return
    heights = [5, 8, 10][stage - 1]
    for x in (3, 7, 12):
        top = 15 - heights - c.rng.randint(0, 1)
        tip = c.stem(x, 15, top, GREEN, lean=c.rng.choice((0, 1)))
        for y in range(14, top + 1, -3):
            c.leaf(x, y, -1, GREEN_DARK, 2)
            c.leaf(x, y - 1, 1, GREEN, 2)
        if stage >= 2:
            bud = hexrgb("#d9c9b8") if stage == 2 else hexrgb("#f2d9e6")
            for dx in (-2, -1, 0, 1, 2):
                c.put(tip + dx, top - (1 if abs(dx) < 2 else 0), bud, 12 if dx == 0 else 0)
            if stage == 3:
                for dx in (-1, 0, 1):
                    c.put(tip + dx, top - 2, hexrgb("#e8b9d0"))


# Белладонна: кустик с широкими тёмными листьями, на третьей стадии —
# бурые колокольчики, на спелой — блестящие чёрные ягоды.
def belladonna(stage, c):
    if stage == 0:
        sprouts(c, [4, 9, 13], 2)
        return
    height = [5, 8, 11][stage - 1]
    for x in (3, 8, 12):
        top = 15 - height - c.rng.randint(0, 1)
        c.stem(x, 15, top, GREEN_DARK)
        for y in range(13, top, -3):
            for dx in (-1, 1):
                c.leaf(x, y, dx, GREEN if dx < 0 else GREEN_DARK, 3)
        if stage == 2:
            for y in (top + 2, top + 5):
                c.put(x + 1, y, hexrgb("#6d3a5c"))
                c.put(x + 1, y + 1, hexrgb("#8a4c74"))
        if stage == 3:
            for (bx, by) in ((x - 2, top + 3), (x + 2, top + 1), (x + 1, top + 6)):
                c.put(bx, by, hexrgb("#1c1424"))
                c.put(bx + 1, by, hexrgb("#2a1e36"))
                c.put(bx, by + 1, hexrgb("#2a1e36"))
                c.put(bx + 1, by + 1, hexrgb("#120c18"))
                c.px[by][bx] = m3.rgba(hexrgb("#6e5c86"))  # блик


# Мандрагора: розетка широких морщинистых листьев у самой земли, к спелости
# в центре — фиолетовые цветки, а из земли выглядывает «макушка» корня.
def mandrake(stage, c):
    if stage == 0:
        sprouts(c, [5, 10], 2)
        return
    spread = [3, 5, 7][stage - 1]
    rise = [3, 4, 5][stage - 1]
    # Листья — штрихи от середины наружу и вниз, широкие у основания, с
    # тёмной жилкой: розетка, а не сплошной холмик.
    for side in (-1, 1):
        for lift in range(0, rise, 2):
            for i in range(spread - lift // 2):
                x = (8 + i) if side > 0 else (7 - i)
                y = 15 - rise + lift + (i * (rise - lift)) // spread
                c.put(x, y, GREEN_DARK if i % 2 else GREEN)
                c.put(x, y + 1, GREEN_PALE if lift == 0 else GREEN, -6)
    for y in range(15 - rise - 1, 15):
        c.put(7, y, GREEN_DARK)
        c.put(8, y, GREEN)
    if stage >= 2:
        c.put(7, 15 - rise - 2, GREEN_PALE)
        c.put(8, 15 - rise - 2, GREEN_PALE)
    if stage == 3:
        for (x, y) in ((6, 15 - rise - 2), (9, 15 - rise - 2), (7, 15 - rise - 3), (8, 15 - rise - 3)):
            c.put(x, y, hexrgb("#7a4fa0"), 10)
        c.put(7, 15 - rise - 4, hexrgb("#a07ac4"))
        for x in (6, 7, 8, 9):
            c.put(x, 15, hexrgb("#a07850"), -6 if x in (6, 9) else 4)


CROPS = {"valerian": valerian, "belladonna": belladonna, "mandrake": mandrake}


# ---------------------------------------------------------------------------
# Плоские спрайты предметов
# ---------------------------------------------------------------------------


def seeds_sprite(colour, dark, tuft=None):
    """Горстка семян: пять-шесть зёрен по 2 пикселя, как у ванильных семян."""
    def draw(c):
        for (x, y) in ((5, 5), (9, 4), (4, 9), (8, 8), (11, 9), (7, 12)):
            c.put(x, y, colour, 10)
            c.put(x + 1, y, colour)
            c.put(x + 1, y + 1, dark)
            if tuft:
                c.put(x - 1, y - 1, tuft)
    return draw


def root_sprite(c):
    skin, dark = hexrgb("#c9a878"), hexrgb("#8f7048")
    for x in (5, 7, 9, 11):
        for y in range(5, 15 - abs(x - 8)):
            c.put(x + (y - 5) // 5 * (1 if x > 8 else -1), y, skin if y % 3 else dark)
    for x in range(5, 12):
        c.put(x, 4, dark)
    for (x, y) in ((7, 3), (8, 2), (9, 3), (8, 1)):
        c.put(x, y, GREEN)


def mandrake_sprite(c):
    skin, dark = hexrgb("#b08058"), hexrgb("#7a5234")
    for y in range(4, 11):
        for x in range(6, 10):
            c.put(x, y, skin if (x + y) % 4 else dark)
    for y in range(11, 15):
        c.put(6, y, skin)
        c.put(9, y, skin)
    for y in range(6, 10):
        c.put(5 - (y - 6) // 2, y, dark)
        c.put(10 + (y - 6) // 2, y, dark)
    for (x, y) in ((7, 3), (8, 3), (6, 2), (9, 2), (7, 1), (8, 0)):
        c.put(x, y, GREEN)


def berries_sprite(c):
    for (bx, by) in ((5, 7), (9, 6), (7, 10)):
        for dx in range(3):
            for dy in range(3):
                c.put(bx + dx, by + dy, hexrgb("#221830"), -10 if dx + dy > 2 else 0)
        c.px[by][bx] = m3.rgba(hexrgb("#7a6896"))
    for (x, y) in ((8, 3), (8, 4), (7, 5), (9, 5), (10, 4)):
        c.put(x, y, GREEN)


SPRITES = {
    "valerian_seeds": seeds_sprite(hexrgb("#c9b48a"), hexrgb("#8f7c58"), hexrgb("#ece4d6")),
    "belladonna_seeds": seeds_sprite(hexrgb("#7a5a3c"), hexrgb("#4a3424")),
    "mandrake_seeds": seeds_sprite(hexrgb("#3e3036"), hexrgb("#221a1e")),
    "valerian_root": root_sprite,
    "mandrake_root": mandrake_sprite,
    "belladonna_berries": berries_sprite,
}
FLAT = ["valerian_seeds", "belladonna_seeds", "mandrake_seeds"]

# ---------------------------------------------------------------------------
# Объёмный урожай
# ---------------------------------------------------------------------------


def t_root(x, y, n):
    return m3.rgba(m3.shade((201, 168, 120), (-22 if y % 4 == 0 else 0) + n * 3))


def t_mandrake(x, y, n):
    return m3.rgba(m3.shade((176, 128, 88), (-26 if (x + 2 * y) % 5 == 0 else 0) + n * 3))


def t_berry(x, y, n):
    return m3.rgba(m3.shade((34, 24, 48), (40 if x < 6 and y < 6 else 0) + n * 2))


def t_leaf(x, y, n):
    return m3.rgba(m3.shade(GREEN, (-16 if x == 8 else 0) + n * 3))


def t_stem(x, y, n):
    return m3.rgba(m3.shade(GREEN_DARK, n * 2))


TEXTURES = {
    "plant_root": t_root,
    "plant_mandrake": t_mandrake,
    "plant_berry": t_berry,
    "plant_leaf": t_leaf,
    "plant_stem": t_stem,
}

box = m3.box


def valerian_roots():
    """Пучок тонких корней под коронкой, сверху — срезанные черешки."""
    els = [box([5, 9, 6.5], [11, 11, 9.5], "plant_root")]
    for i, x in enumerate((5.25, 7, 8.75, 10)):
        tilt = ("z", (-22.5, 0, 0, 22.5)[i], [x + 0.375, 9, 8])
        els.append(box([x, 2 + (i % 2), 7.25], [x + 0.75, 9, 8], "plant_root", rot=tilt))
        els.append(box([x - 0.25, 3.5, 8.25], [x + 0.5, 9, 9], "plant_root", rot=tilt))
    for x in (6.5, 8.5):
        els.append(box([x, 11, 7.5], [x + 1, 14, 8.5], "plant_stem"))
    els.append(box([7.5, 11, 7.5], [8.5, 15, 8.5], "plant_stem"))
    return els


def mandrake_root():
    """Корень-человечек: туловище, две ноги, две руки-отростка, листья макушкой."""
    return [
        box([6, 6, 6.5], [10, 11, 9.5], "plant_mandrake"),
        box([6.5, 11, 7], [9.5, 12.5, 9], "plant_mandrake"),
        box([6.25, 1, 7.25], [7.75, 6, 8.75], "plant_mandrake", rot=("z", -22.5, [7, 6, 8])),
        box([8.25, 1, 7.25], [9.75, 6, 8.75], "plant_mandrake", rot=("z", 22.5, [9, 6, 8])),
        box([3.5, 8.5, 7.5], [6, 9.5, 8.5], "plant_mandrake", rot=("z", -22.5, [6, 9, 8])),
        box([10, 8.5, 7.5], [12.5, 9.5, 8.5], "plant_mandrake", rot=("z", 22.5, [10, 9, 8])),
        box([7.5, 12.5, 7.5], [8.5, 14, 8.5], "plant_stem"),
        box([5, 13.5, 7.75], [8, 14.25, 8.25], "plant_leaf", rot=("z", 22.5, [8, 13.5, 8])),
        box([8, 13.5, 7.75], [11, 14.25, 8.25], "plant_leaf", rot=("z", -22.5, [8, 13.5, 8])),
        box([7.75, 14, 6], [8.25, 16, 10], "plant_leaf", rot=("x", 22.5, [8, 14, 8])),
    ]


def berries():
    """Гроздь из трёх блестящих ягод на зелёных чашечках и черешке."""
    els = []
    for (x, y, z) in ((4.5, 3, 6.5), (9, 4, 7), (6.5, 7.5, 7.5)):
        els.append(box([x, y, z], [x + 3, y + 3, z + 3], "plant_berry"))
        els.append(box([x + 0.5, y + 3, z + 0.5], [x + 2.5, y + 3.5, z + 2.5], "plant_leaf"))
    els.append(box([7.5, 10.5, 8.5], [8.5, 14, 9.5], "plant_stem"))
    els.append(box([5.75, 6, 8.5], [6.5, 11, 9.25], "plant_stem", rot=("z", -22.5, [6, 11, 9])))
    els.append(box([10, 7, 8.5], [10.75, 11, 9.25], "plant_stem", rot=("z", 22.5, [10.5, 11, 9])))
    els.append(box([8.5, 12.5, 8.75], [12.5, 13.25, 9.25], "plant_leaf", rot=("z", 22.5, [8.5, 12.5, 9])))
    return els


# форма → (геометрия, положение в руке, текстура частиц)
FORMS = {
    "valerian_root": (valerian_roots, m3.upright(1.0), "plant_root"),
    "mandrake_root": (mandrake_root, m3.upright(1.0), "plant_mandrake"),
    "belladonna_berries": (berries, m3.upright(1.2), "plant_berry"),
}


# ---------------------------------------------------------------------------
# Запись
# ---------------------------------------------------------------------------


def dump(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    m3.dump(path, data)


def write_blocks():
    os.makedirs(BLOCK_TEX, exist_ok=True)
    for crop, draw in CROPS.items():
        variants = {}
        for stage in range(4):
            name = f"{crop}_stage{stage}"
            c = Canvas(name)
            draw(stage, c)
            c.save(os.path.join(BLOCK_TEX, name + ".png"))
            dump(os.path.join(BLOCK_MODELS, name + ".json"), {
                "parent": "minecraft:block/crop",
                "textures": {"crop": f"biolab:block/{name}"},
            })
            variants[f"age={stage}"] = {"model": f"biolab:block/{name}"}
        dump(os.path.join(BLOCKSTATES, crop + ".json"), {"variants": variants})


def write_items():
    os.makedirs(ITEM_TEX, exist_ok=True)
    for item, draw in SPRITES.items():
        c = Canvas(item)
        draw(c)
        c.save(os.path.join(ITEM_TEX, item + ".png"))
        dump(os.path.join(ITEM_MODELS, item + ".json"), {
            "parent": "minecraft:item/generated",
            "textures": {"layer0": f"biolab:item/{item}"},
        })
    for item in FLAT:
        dump(os.path.join(m3.ITEMS_DIR, item + ".json"),
             {"model": {"type": "minecraft:model", "model": f"biolab:item/{item}"}})

    os.makedirs(m3.TEX_DIR, exist_ok=True)
    for name, fn in TEXTURES.items():
        rng = random.Random(name)
        noise = [[rng.randint(-2, 2) for _ in range(16)] for _ in range(16)]
        m3.png(os.path.join(m3.TEX_DIR, name + ".png"), 16, 16,
               lambda x, y, fn=fn, noise=noise: fn(x, y, noise[y][x]))
    for name, (geometry, display, particle) in FORMS.items():
        elements = m3.recenter(geometry())
        used = sorted({face["texture"][1:] for e in elements for face in e["faces"].values()})
        textures = {v: "biolab:item/3d/" + v for v in used}
        textures["particle"] = "biolab:item/3d/" + particle
        dump(os.path.join(m3.MODEL_DIR, name + ".json"), {
            "textures": textures,
            "elements": elements,
            "gui_light": "side",
            "display": display,
        })
        # Объёмный везде, как остальные предметы мода (FLAT_CONTEXTS пуст).
        dump(os.path.join(m3.ITEMS_DIR, name + ".json"),
             {"model": {"type": "minecraft:model", "model": "biolab:item/3d/" + name}})


if __name__ == "__main__":
    write_blocks()
    write_items()
    print(f"культур: {len(CROPS)} × 4 стадии, плоских предметов: {len(FLAT)}, "
          f"объёмных: {len(FORMS)}, текстур частей: {len(TEXTURES)}")
