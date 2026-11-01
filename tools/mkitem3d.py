#!/usr/bin/env python3
"""Объёмные модели предметов BioLab (GDD 4.12): текстуры, модели, определения.

Запуск из корня репозитория:
    python3 tools/mkitem3d.py
    python3 tools/check_item_models.py

Пишет:
  content/assets/biolab/textures/item/3d/*.png — текстуры частей 16×16;
  content/assets/biolab/models/item/3d/*.json  — геометрия форм (и варианты
                                                 с другой текстурой);
  content/assets/biolab/items/<id>.json        — определения предметов.

Как устроено. У предмета две модели: плоская `biolab:item/<id>` (спрайт,
не меняется) и объёмная `biolab:item/3d/<форма>`. Определение предмета
выбирает между ними по `minecraft:display_context`, как ванильная подзорная
труба: в инвентаре, на земле, в рамке и на полке — плоская иконка, в руке —
объёмная. Исключение — GUI_3D: GDD 4.12 прямо просит для колбы, шприца и
тюбика объём и в инвентаре, у них плоская только на земле, в рамке и на полке.

Варианты одной формы различаются цветом содержимого: грани, которые красятся,
помечены `tintindex: 0`, а цвет задаёт определение предмета
(`minecraft:constant`). Цвета взяты из плоских текстур предметов. Текстура
содержимого (`liquid`, `paint`, `medium`) — почти белая, чтобы тон
совпадал с иконкой.

UV у граней «мировые»: грань берёт тексел по своей координате x (или z) и
высоте 16 − y, с какой стороны на неё ни смотри. Поэтому рисунок на
противоположных гранях совпадает, а вырезы (лезвие, оперение) не зеркалятся.

Подстройка в руке — пресеты UPRIGHT (стоячие вещи: колба, банка, микроскоп)
и PEN (вытянутые вдоль Y, остриём вверх: шприц, дротик, скальпель). Меняется
пресет — меняются все формы на нём. Левая рука не задаётся: игра отражает
правую.
"""
import json
import math
import os
import random
import struct
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS = os.path.join(ROOT, "content", "assets", "biolab")
TEX_DIR = os.path.join(ASSETS, "textures", "item", "3d")
MODEL_DIR = os.path.join(ASSETS, "models", "item", "3d")
ITEMS_DIR = os.path.join(ASSETS, "items")

# ---------------------------------------------------------------------------
# PNG
# ---------------------------------------------------------------------------


def png(path, w, h, px):
    raw = b"".join(b"\x00" + b"".join(bytes(px(x, y)) for x in range(w)) for y in range(h))

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    with open(path, "wb") as f:
        f.write(data)


def clamp(v):
    return max(0, min(255, int(round(v))))


def hexrgb(s):
    s = s.lstrip("#")
    return tuple(int(s[i:i + 2], 16) for i in (0, 2, 4))


def shade(c, d):
    return tuple(clamp(v + d) for v in c[:3])


def mix(a, b, t):
    return tuple(clamp(a[i] + (b[i] - a[i]) * t) for i in range(3))


def rgba(c, a=255):
    return (clamp(c[0]), clamp(c[1]), clamp(c[2]), clamp(a))


CLEAR = (0, 0, 0, 0)

# ---------------------------------------------------------------------------
# Текстуры частей. Каждая — функция (x, y, шум) → RGBA.
# ---------------------------------------------------------------------------


def cylinder(x):
    """Подсветка «цилиндра» по столбцу: блик слева, тень справа."""
    return {4: 14, 5: 22, 6: 10, 11: -10, 12: -16, 13: -10}.get(x, 0)


def t_glass(x, y, n):
    if x in (5, 6):
        return rgba((236, 248, 252), 150)
    if x == 12:
        return rgba((190, 220, 232), 120)
    return rgba((196, 226, 238), 82 + n)


def t_glass_rim(x, y, n):
    return rgba(shade((206, 228, 236), cylinder(x) + n), 190)


def t_liquid(x, y, n):
    # Почти белая: умножается на цвет вещества (tint).
    v = 246 + cylinder(x) // 2 + n
    return rgba((v, v, v))


def t_paint(x, y, n):
    v = 244 + cylinder(x) // 2 + n
    if y in (6, 10):
        v -= 14
    return rgba((v, v, v))


def t_medium(x, y, n):
    v = 240 + n * 2
    return rgba((v, v, v))


def t_colonies(x, y, n):
    rng = random.Random(7)
    spots = [(rng.uniform(3, 13), rng.uniform(3, 13), rng.uniform(0.7, 1.6)) for _ in range(9)]
    v = 240 + n
    for sx, sy, r in spots:
        d = math.hypot(x + 0.5 - sx, y + 0.5 - sy)
        if d < r:
            v = 150 + int(d * 20)
            break
    return rgba((v, v, v))


def t_cork(x, y, n):
    base = (139, 98, 60) if (x + y) % 5 else (112, 78, 46)
    return rgba(shade(base, cylinder(x) // 2 + n * 3))


def t_steel(x, y, n):
    return rgba(shade((168, 182, 194), cylinder(x) + n * 2))


def t_steel_dark(x, y, n):
    return rgba(shade((104, 112, 122), cylinder(x) // 2 + n * 2))


def t_rubber(x, y, n):
    return rgba(shade((54, 58, 64), n * 2))


def t_needle(x, y, n):
    return rgba(shade((226, 232, 238), cylinder(x) // 2))


def t_can(x, y, n):
    return rgba(shade((196, 200, 208), cylinder(x) + n))


def t_cap_dark(x, y, n):
    return rgba(shade((90, 96, 104), cylinder(x) // 2 + n * 2))


def t_tube(x, y, n):
    return rgba(shade((228, 230, 234), cylinder(x) // 2 + n))


def t_tube_crimp(x, y, n):
    return rgba(shade((210, 212, 218), -14 if y % 2 else 0))


def t_pill_shell(x, y, n):
    return rgba(shade((238, 238, 232), 10 if y == 7 else (-12 if y == 9 else 0)))


def t_lid(x, y, n):
    return rgba(shade((150, 156, 164), (8 if y % 2 else -8) + cylinder(x) // 2))


def t_cotton(x, y, n):
    return rgba(shade((244, 242, 232), n * 3))


def t_stick(x, y, n):
    return rgba(shade((232, 226, 206), cylinder(x) // 2))


def t_device(x, y, n):
    return rgba(shade((60, 70, 88), n * 2 + (6 if x in (4, 5) else 0)))


def t_screen(x, y, n):
    if y in (7, 9):
        return rgba((70, 150, 104))
    wave = 7 + round(1.5 * math.sin(x * 1.3))
    if y == wave:
        return rgba((196, 255, 214))
    return rgba((120, 220, 160))


def t_button(x, y, n):
    return rgba((206, 84, 76))


def t_scope_body(x, y, n):
    return rgba(shade((178, 184, 194), cylinder(x) + n))


def t_scope_dark(x, y, n):
    return rgba(shade((86, 90, 98), n * 2))


def t_lens(x, y, n):
    return rgba(shade((150, 220, 240), 20 if (x + y) % 4 == 0 else 0))


def t_scalpel_grip(x, y, n):
    return rgba(shade((150, 160, 172), -16 if y % 2 == 0 and y > 8 else 6))


def t_scalpel_blade(x, y, n):
    # Лезвие на гранях [7, 9]–[9.75, 15.5]: u = x, v = 16 − y.
    # Обух слева прямой, режущая кромка справа скругляется к острию.
    if not (7 <= x <= 9 and 0 <= y <= 6):
        return CLEAR
    t = (7.0 - (y + 0.5)) / 6.5  # 0 у пятки лезвия, 1 у острия
    width = 2.75 * (1 - t * t) + 0.35
    edge = x + 0.5 - 7
    if edge > width:
        return CLEAR
    if edge > width - 0.9:
        return rgba((236, 242, 248))
    return rgba((186, 196, 208))


def t_fin(x, y, n):
    # Оперение дротика на гранях [4.5, 0]–[11.5, 6]: u = x или z, v = 16 − y.
    if y < 10:
        return CLEAR
    half = 0.5 + (y + 0.5 - 10) * 0.55
    if abs(x + 0.5 - 8) > half:
        return CLEAR
    return rgba(shade((228, 228, 216), -18 if x in (7, 8) else n * 2))


def t_gauze(x, y, n):
    return rgba(shade((238, 234, 226), -6 if (x % 3 == 0 or y % 3 == 0) else n))


def t_gauze_end(x, y, n):
    dx, dy = x + 0.5 - 8, y + 0.5 - 8
    if abs(dx) < 1 and abs(dy) < 2.5 or abs(dy) < 1 and abs(dx) < 2.5:
        return rgba((196, 90, 90))
    r = math.hypot(dx, dy)
    return rgba(shade((238, 234, 226), -18 if int(r) % 2 else 0))


def t_tape_red(x, y, n):
    return rgba(shade((196, 90, 90), n * 2))


def t_dna(flip):
    light, dark, gold = (120, 200, 240), (70, 140, 190), (230, 200, 120)

    def px(x, y, n):
        u, v = x + 0.5, y + 0.5
        a, b = (dark, light) if flip else (light, dark)
        if u < 5 or v < 5:
            return rgba(a)
        if u > 11 or v > 11:
            return rgba(b)
        return rgba(shade(gold, 8 if y == 7 else 0))

    return px


def t_blueprint(x, y, n):
    # Чертёж на свисающем листе: u = x 1.5..14.5, v = 16 − y 9..15.
    line = (210, 228, 244)
    if y in (9, 15) and 2 <= x <= 13 or x in (2, 13) and 9 <= y <= 15:
        return rgba(line)
    if (y in (11, 13) and 4 <= x <= 8) or (x in (4, 8) and 11 <= y <= 13):
        return rgba(line)
    if y == 12 and 9 <= x <= 11 or x == 11 and 10 <= y <= 14:
        return rgba(shade(line, -30))
    return rgba(shade((46, 86, 148), n * 2))


def t_blueprint_roll(x, y, n):
    if y in (7, 9):
        return rgba((88, 128, 190))
    return rgba(shade((46, 86, 148), n * 2 + cylinder(y) // 2))


def t_blueprint_end(x, y, n):
    r = math.hypot(x + 0.5 - 8, y + 0.5 - 8)
    return rgba((210, 228, 244) if int(r * 1.4) % 2 else (46, 86, 148))


def t_bucket(x, y, n):
    if y in (7, 8):
        return rgba((104, 110, 118))
    return rgba(shade((150, 156, 164), cylinder(x) + n * 2))


def t_bucket_dark(x, y, n):
    return rgba(shade((104, 110, 118), n * 2))


def t_pod(x, y, n):
    return rgba(shade((128, 194, 86), (18 if x in (6, 7) else 0) + (-14 if x == 10 else 0) + n * 2))


def t_seed(x, y, n):
    return rgba(shade((232, 214, 140), n * 3))


def t_stem(x, y, n):
    return rgba(shade((100, 140, 60), n * 2))


TEXTURES = {
    "glass": t_glass,
    "glass_rim": t_glass_rim,
    "liquid": t_liquid,
    "paint": t_paint,
    "medium": t_medium,
    "colonies": t_colonies,
    "cork": t_cork,
    "steel": t_steel,
    "steel_dark": t_steel_dark,
    "rubber": t_rubber,
    "needle": t_needle,
    "can": t_can,
    "cap_dark": t_cap_dark,
    "tube": t_tube,
    "tube_crimp": t_tube_crimp,
    "pill_shell": t_pill_shell,
    "lid": t_lid,
    "cotton": t_cotton,
    "stick": t_stick,
    "device": t_device,
    "screen": t_screen,
    "button": t_button,
    "scope_body": t_scope_body,
    "scope_dark": t_scope_dark,
    "lens": t_lens,
    "scalpel_grip": t_scalpel_grip,
    "scalpel_blade": t_scalpel_blade,
    "fin": t_fin,
    "gauze": t_gauze,
    "gauze_end": t_gauze_end,
    "tape_red": t_tape_red,
    "dna_rung": t_dna(False),
    "dna_rung_flip": t_dna(True),
    "blueprint": t_blueprint,
    "blueprint_roll": t_blueprint_roll,
    "blueprint_end": t_blueprint_end,
    "bucket": t_bucket,
    "bucket_dark": t_bucket_dark,
    "pod": t_pod,
    "seed": t_seed,
    "stem": t_stem,
}


def write_textures():
    os.makedirs(TEX_DIR, exist_ok=True)
    for name, fn in TEXTURES.items():
        rng = random.Random(name)
        noise = [[rng.randint(-2, 2) for _ in range(16)] for _ in range(16)]
        png(os.path.join(TEX_DIR, name + ".png"), 16, 16,
            lambda x, y, fn=fn, noise=noise: fn(x, y, noise[y][x]))


# ---------------------------------------------------------------------------
# Геометрия
# ---------------------------------------------------------------------------

DIRS = ("north", "south", "west", "east", "up", "down")
SIDES = ("north", "south", "west", "east")


def world_uv(face, fr, to):
    """UV по мировым координатам (см. шапку): тексел = (x или z, 16 − y)."""
    x1, y1, z1 = fr
    x2, y2, z2 = to
    v1, v2 = 16 - y2, 16 - y1
    return {
        "south": [x1, v1, x2, v2],
        "north": [x2, v1, x1, v2],
        "west": [z1, v1, z2, v2],
        "east": [z2, v1, z1, v2],
        "up": [x1, z1, x2, z2],
        "down": [x1, z2, x2, z1],
    }[face]


def box(fr, to, tex, faces=DIRS, tint=False, rot=None, face_tex=None, uv_box=None, uv=None):
    """Элемент модели. tex — имя переменной текстуры; face_tex переопределяет
    текстуру отдельных граней; uv_box — (fr, to), по которым считать UV, если
    они не совпадают с настоящим положением (ступени ДНК); uv — явные UV
    отдельных граней."""
    ufr, uto = uv_box or (fr, to)
    el = {"from": list(fr), "to": list(to)}
    if rot:
        axis, angle, origin = rot
        el["rotation"] = {"angle": angle, "axis": axis, "origin": list(origin)}
    el["faces"] = {}
    for f in faces:
        face_uv = (uv or {}).get(f) or world_uv(f, ufr, uto)
        face = {"uv": [round(c, 3) for c in face_uv],
                "texture": "#" + (face_tex or {}).get(f, tex)}
        if tint:
            face["tintindex"] = 0
        el["faces"][f] = face
    return el


def recenter(elements):
    """Сдвигает форму так, чтобы её габарит был по центру куба 0..16:
    тогда точка хвата в руке и центр в инвентаре — посередине предмета.
    UV при этом не меняются: они посчитаны по исходным координатам."""
    lo = [min(min(e["from"][i], e["to"][i]) for e in elements) for i in range(3)]
    hi = [max(max(e["from"][i], e["to"][i]) for e in elements) for i in range(3)]
    shift = [round((8 - (lo[i] + hi[i]) / 2) * 4) / 4 for i in range(3)]
    for e in elements:
        e["from"] = [round(e["from"][i] + shift[i], 3) for i in range(3)]
        e["to"] = [round(e["to"][i] + shift[i], 3) for i in range(3)]
        if "rotation" in e:
            o = e["rotation"]["origin"]
            e["rotation"]["origin"] = [round(o[i] + shift[i], 3) for i in range(3)]
    return elements


def flask():
    return [
        box([4.5, 1.5, 4.5], [11.5, 4.5, 11.5], "liquid", tint=True),
        box([4, 1, 4], [12, 6, 12], "glass"),
        box([5, 0, 5], [11, 7, 11], "glass"),
        box([6.75, 11.5, 6.75], [9.25, 14, 9.25], "cork"),
        box([6.5, 7, 6.5], [9.5, 11, 9.5], "glass"),
        box([6, 11, 6], [10, 12, 10], "glass_rim"),
    ]


def syringe():
    return [
        box([6.6, 4.1, 6.6], [9.4, 4.8, 9.4], "rubber"),
        box([6.5, 4.8, 6.5], [9.5, 10.5, 9.5], "liquid", tint=True),
        box([7.5, 1, 7.5], [8.5, 4, 8.5], "steel_dark"),
        box([6.5, 0, 6.5], [9.5, 1, 9.5], "steel_dark"),
        box([4.5, 3.5, 7], [11.5, 4.5, 9], "steel"),
        box([7, 11, 7], [9, 12, 9], "steel"),
        box([7.75, 12, 7.75], [8.25, 16, 8.25], "needle"),
        box([6, 4, 6], [10, 11, 10], "glass"),
    ]


def dart():
    return [
        box([7.75, 13, 7.75], [8.25, 16, 8.25], "needle"),
        box([7, 7, 7], [9, 12, 9], "liquid", tint=True),
        box([6.75, 12, 6.75], [9.25, 13, 9.25], "steel"),
        box([6.75, 6, 6.75], [9.25, 7, 9.25], "steel"),
        box([7.5, 1, 7.5], [8.5, 6, 8.5], "steel_dark"),
        box([4.5, 0, 8], [11.5, 6, 8], "fin", faces=("north", "south")),
        box([8, 0, 4.5], [8, 6, 11.5], "fin", faces=("west", "east")),
    ]


def spray():
    return [
        box([5, 0, 5], [11, 10, 11], "can"),
        box([4.9, 3, 4.9], [11.1, 7.5, 11.1], "paint", faces=SIDES, tint=True),
        box([5.5, 10, 5.5], [10.5, 11, 10.5], "can"),
        box([7, 11, 7], [9, 13, 9], "cap_dark"),
        box([9, 12, 7.5], [10, 12.75, 8.5], "cap_dark"),
    ]


def tube():
    return [
        box([5.5, 4, 6.5], [10.5, 12, 9.5], "tube"),
        box([5.4, 6, 6.4], [10.6, 10, 9.6], "paint", faces=SIDES, tint=True),
        box([6, 2.5, 7], [10, 4, 9], "tube"),
        box([5, 0.5, 7.6], [11, 2.5, 8.4], "tube_crimp"),
        box([6.5, 12, 7], [9.5, 13, 9], "tube"),
        box([7, 13, 7], [9, 15.5, 9], "cap_dark"),
    ]


def pill():
    return [
        box([4, 6.5, 6.5], [8, 9.5, 9.5], "pill_shell"),
        box([3, 7, 7], [4, 9, 9], "pill_shell"),
        box([8, 6.5, 6.5], [12, 9.5, 9.5], "paint", tint=True),
        box([12, 7, 7], [13, 9, 9], "paint", tint=True),
    ]


def jar():
    return [
        box([5, 0.5, 5], [11, 7, 11], "liquid", tint=True),
        box([4, 10, 4], [12, 12, 12], "lid"),
        box([4.5, 0, 4.5], [11.5, 10, 11.5], "glass"),
    ]


def petri():
    return [
        box([3, 0.5, 3], [13, 1.5, 13], "medium", tint=True),
        box([2.5, 0, 2.5], [13.5, 2, 13.5], "glass", faces=("north", "south", "west", "east", "down")),
        box([2, 1.25, 2], [14, 2.75, 14], "glass"),
    ]


def swab():
    return [
        box([7.5, 0, 7.5], [8.5, 12, 8.5], "stick"),
        box([7.25, 11.25, 7.25], [8.75, 12, 8.75], "cotton"),
        box([6.75, 12, 6.75], [9.25, 15, 9.25], "cotton"),
        box([7.25, 15, 7.25], [8.75, 15.75, 8.75], "cotton"),
    ]


def analyzer():
    return [
        box([4.5, 0, 6.5], [11.5, 11, 9.5], "device"),
        box([5.5, 5.5, 9.5], [10.5, 10, 9.75], "screen", faces=("south", "west", "east", "up", "down")),
        box([5.75, 2, 9.5], [7.25, 3, 9.8], "button"),
        box([8.75, 2, 9.5], [10.25, 3, 9.8], "steel"),
        box([6.5, 11, 7.5], [7.5, 14, 8.5], "steel_dark"),
        box([6.25, 14, 7.25], [7.75, 15, 8.75], "lens"),
    ]


def microscope():
    tilt = ("z", -22.5, [7, 9, 8])
    return [
        box([3.5, 0, 4.5], [12.5, 1.5, 11.5], "scope_dark"),
        box([9.5, 1.5, 6.5], [11.5, 9.5, 9.5], "scope_body"),
        box([7.5, 9.5, 7], [10.5, 11.5, 9], "scope_body"),
        box([4, 5, 5], [10, 6, 11], "scope_dark"),
        box([6, 1.5, 7], [8, 2.5, 9], "lens"),
        box([11.5, 6.5, 7.25], [12.5, 8, 8.75], "scope_dark"),
        box([5.75, 7, 7], [7.75, 14, 9], "scope_body", rot=tilt),
        box([6, 14, 7.25], [7.5, 16, 8.75], "scope_dark", rot=tilt),
        box([6, 6, 7.5], [7, 7.5, 8.5], "steel", rot=tilt),
    ]


def scalpel():
    return [
        box([7.25, 0, 7.5], [8.75, 8, 8.5], "scalpel_grip"),
        box([7, 8, 7.25], [9, 9, 8.75], "steel_dark"),
        box([7, 9, 7.875], [9.75, 15.5, 8.125], "scalpel_blade", faces=("north", "south")),
    ]


def bandage():
    ends = {"west": "gauze_end", "east": "gauze_end"}
    return [
        box([4.5, 4, 5], [11.5, 12, 11], "gauze", face_tex=ends),
        box([4.5, 5, 4], [11.5, 11, 12], "gauze", face_tex=ends),
        box([7.5, 3.9, 4.9], [8.5, 12.1, 11.1], "tape_red", faces=("north", "south", "up", "down")),
        box([5.5, 2, 10.75], [10.5, 5, 11.25], "gauze"),
    ]


def dna():
    """Семь ступеней, каждая повёрнута на 45° от предыдущей: концы ступеней
    (голубой и синий) складываются в две закрученные нити. Повороты
    ограничены ±45°, поэтому 90° — это та же ступень вдоль Z, а 180° — та же
    ступень с переставленными цветами (dna_rung_flip)."""
    els = []
    for k in range(7):
        a = k * 45
        y = 1 + k * 2.25
        m = a % 180
        tex = "dna_rung_flip" if a % 360 >= 180 else "dna_rung"
        if m < 90:
            fr, to = [3.5, y, 7.25], [12.5, y + 1.5, 8.75]
            ufr, uto = [3.5, 7.25, 7.25], [12.5, 8.75, 8.75]
            ends = {"west": [3.5, 7.25, 5, 8.75], "east": [11, 7.25, 12.5, 8.75]}
            angle = -m
        else:
            fr, to = [7.25, y, 3.5], [8.75, y + 1.5, 12.5]
            ufr, uto = [7.25, 7.25, 3.5], [8.75, 8.75, 12.5]
            ends = {"north": [7.25, 3.5, 8.75, 5], "south": [7.25, 11, 8.75, 12.5]}
            angle = -(m - 90)
        rot = ("y", angle, [8, y + 0.75, 8]) if angle else None
        els.append(box(fr, to, tex, rot=rot, uv_box=(ufr, uto), uv=ends))
    return els


def blueprint():
    """Тонкий длинный свиток, под ним свисает развёрнутый край с чертежом."""
    ends = {"west": "blueprint_end", "east": "blueprint_end"}
    return [
        box([1, 6.5, 7], [15, 9.5, 9], "blueprint_roll", face_tex=ends),
        box([1, 7, 6.5], [15, 9, 9.5], "blueprint_roll", face_tex=ends),
        box([1.5, 1, 9], [14.5, 7, 9.25], "blueprint"),
    ]


def bucket():
    return [
        box([4.75, 1, 4.75], [11.25, 7.5, 11.25], "liquid", tint=True, faces=("up",)),
        box([4.5, 0, 4.5], [11.5, 1, 11.5], "bucket"),
        box([4, 0, 4], [12, 9, 4.75], "bucket"),
        box([4, 0, 11.25], [12, 9, 12], "bucket"),
        box([4, 0, 4.75], [4.75, 9, 11.25], "bucket"),
        box([11.25, 0, 4.75], [12, 9, 11.25], "bucket"),
        box([5.5, 12.75, 7.75], [10.5, 13.5, 8.25], "bucket_dark"),
        box([3.9, 9, 7.75], [4.6, 13.5, 8.25], "bucket_dark", rot=("z", -22.5, [4.25, 9, 8])),
        box([11.4, 9, 7.75], [12.1, 13.5, 8.25], "bucket_dark", rot=("z", 22.5, [11.75, 9, 8])),
    ]


def pod():
    tilt = ("z", -22.5, [8, 8, 8])
    return [
        box([6, 3, 6.5], [10, 13, 9.5], "pod", rot=tilt),
        box([6.5, 1.5, 7], [9.5, 14.5, 9], "pod", rot=tilt),
        box([7.5, 14.5, 7.5], [8.5, 16, 8.5], "stem", rot=tilt),
        box([7, 4.5, 9.5], [8.5, 6, 10], "seed", rot=tilt),
        box([7.5, 7.5, 9.5], [9, 9, 10], "seed", rot=tilt),
        box([7, 10.5, 9.5], [8.5, 12, 10], "seed", rot=tilt),
    ]


# ---------------------------------------------------------------------------
# Положение в руке
# ---------------------------------------------------------------------------


def tr(rotation, translation, scale):
    return {"rotation": rotation, "translation": translation, "scale": [round(scale, 3)] * 3}


def upright(s):
    """Стоячая вещь: в руке держится вертикально, как блок (ванильный
    block.json: 75/45 от третьего лица, 45 от первого)."""
    return {
        "thirdperson_righthand": tr([75, 45, 0], [0, 2.5, 0], 0.45 * s),
        "firstperson_righthand": tr([0, 45, 0], [0, 3.5, 0], 0.32 * s),
        "gui": tr([20, -35, 0], [0, 0, 0], 0.95 * s),
        "ground": tr([0, 0, 0], [0, 3, 0], 0.35 * s),
        "fixed": tr([0, 0, 0], [0, 0, 0], 0.75 * s),
    }


def pen(s):
    """Вытянутая вещь вдоль Y остриём вверх, держится как ручка. Углы —
    ванильные handheld.json ([0,-90,55] и [0,-90,25]), пересчитанные с
    диагонального спрайта на вертикальную модель: минус 45° по Z."""
    return {
        "thirdperson_righthand": tr([0, -90, 10], [0, 4, 0.5], 0.75 * s),
        "firstperson_righthand": tr([0, -90, -20], [1.13, 4.2, 1.13], 0.4 * s),
        "gui": tr([25, 0, -45], [0, 0, 0], 1.3 * s),
        "ground": tr([0, 0, 0], [0, 2, 0], 0.4 * s),
        "fixed": tr([0, 0, -45], [0, 0, 0], 0.9 * s),
    }


# форма → (геометрия, положение в руке, текстура частиц)
FORMS = {
    "flask": (flask, upright(1.0), "glass_rim"),
    "syringe": (syringe, pen(1.0), "steel"),
    "dart": (dart, pen(1.0), "steel"),
    "spray": (spray, upright(1.0), "can"),
    "tube": (tube, pen(1.05), "tube"),
    "pill": (pill, upright(1.4), "pill_shell"),
    "jar": (jar, upright(1.1), "lid"),
    "petri": (petri, upright(1.1), "glass_rim"),
    "swab": (swab, pen(1.05), "cotton"),
    "analyzer": (analyzer, upright(1.0), "device"),
    "microscope": (microscope, upright(1.0), "scope_body"),
    "scalpel": (scalpel, pen(1.0), "steel"),
    "bandage": (bandage, upright(1.4), "gauze"),
    "dna": (dna, upright(1.0), "dna_rung"),
    "blueprint": (blueprint, upright(1.0), "blueprint_roll"),
    "bucket": (bucket, upright(1.05), "bucket"),
    "pod": (pod, upright(1.0), "pod"),
}

# Варианты формы с другой текстурой (не только цветом): модель-наследник.
# имя → (форма, {переменная: текстура})
VARIANTS = {
    "petri_colonies": ("petri", {"medium": "colonies"}),
}

# предмет → (модель формы или варианта, цвет содержимого или None)
ITEMS = {
    "flask": ("flask", "#468cdc"),
    "syringe": ("syringe", "#d6e2ea"),
    "syringe_mutagen": ("syringe", "#d678e6"),
    "syringe_penicillin": ("syringe", "#e2e2b4"),
    "syringe_sedative": ("syringe", "#967ac8"),
    "syringe_stimulant": ("syringe", "#c48c4a"),
    "syringe_vaccine": ("syringe", "#c4a0e6"),
    "syringe_venom": ("syringe", "#68c45c"),
    "dart_sedative": ("dart", "#967ac8"),
    "dart_venom": ("dart", "#68c45c"),
    "spray_analgesic": ("spray", "#d6e8f0"),
    "spray_stimulant": ("spray", "#c48c4a"),
    "ointment_analgesic": ("tube", "#d6e8f0"),
    "pill_analgesic": ("pill", "#d6e8f0"),
    "pill_antidote": ("pill", "#404046"),
    "pill_penicillin": ("pill", "#e2e2b4"),
    "pill_stimulant": ("pill", "#c48c4a"),
    "extract_caffeine": ("jar", "#7a5636"),
    "extract_penicillin": ("jar", "#e2e2b4"),
    "extract_salicylate": ("jar", "#d6e8f0"),
    "extract_sedative": ("jar", "#967ac8"),
    "extract_venom": ("jar", "#68c45c"),
    "nutrient_broth": ("jar", "#c4d68c"),
    "mutagen": ("jar", "#c460d6"),
    "agar": ("petri", "#e2d696"),
    "culture": ("petri_colonies", "#c4d696"),
    "culture_weakened": ("petri_colonies", "#c4a0e6"),
    "swab": ("swab", None),
    "analyzer": ("analyzer", None),
    "microscope": ("microscope", None),
    "scalpel": ("scalpel", None),
    "bandage": ("bandage", None),
    "dna_fragment": ("dna", None),
    "blueprint": ("blueprint", None),
    "acid_puddle_bucket": ("bucket", "#7cc83c"),
    "mutagen_bucket": ("bucket", "#c460d6"),
    "slime_bucket": ("bucket", "#96ba78"),
    "sludge_bucket": ("bucket", "#60583c"),
    "gm_crop": ("pod", None),
}

# Объёмные и в инвентаре: GDD 4.12 называет колбу, шприц и тюбик мази вещами,
# которые «разглядывают в инвентаре, и плоская иконка им не идёт».
GUI_3D = {"flask", "syringe", "syringe_mutagen", "syringe_penicillin", "syringe_sedative",
          "syringe_stimulant", "syringe_vaccine", "syringe_venom", "ointment_analgesic"}

FLAT_CONTEXTS = ["gui", "ground", "fixed", "on_shelf"]


def dump(path, data):
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
        f.write("\n")


def write_models():
    os.makedirs(MODEL_DIR, exist_ok=True)
    for name, (geometry, display, particle) in FORMS.items():
        elements = recenter(geometry())
        used = sorted({face["texture"][1:] for e in elements for face in e["faces"].values()})
        textures = {v: "biolab:item/3d/" + v for v in used}
        textures["particle"] = "biolab:item/3d/" + particle
        dump(os.path.join(MODEL_DIR, name + ".json"), {
            "textures": textures,
            "elements": elements,
            "gui_light": "side",
            "display": display,
        })
    for name, (form, overrides) in VARIANTS.items():
        dump(os.path.join(MODEL_DIR, name + ".json"), {
            "parent": "biolab:item/3d/" + form,
            "textures": {k: "biolab:item/3d/" + v for k, v in overrides.items()},
        })


def write_items():
    for item, (model, colour) in ITEMS.items():
        solid = {"type": "minecraft:model", "model": "biolab:item/3d/" + model}
        if colour:
            r, g, b = hexrgb(colour)
            solid["tints"] = [{"type": "minecraft:constant", "value": (r << 16) | (g << 8) | b}]
        when = [c for c in FLAT_CONTEXTS if not (item in GUI_3D and c == "gui")]
        dump(os.path.join(ITEMS_DIR, item + ".json"), {
            "model": {
                "type": "minecraft:select",
                "cases": [{
                    "model": {"type": "minecraft:model", "model": "biolab:item/" + item},
                    "when": when,
                }],
                "fallback": solid,
                "property": "minecraft:display_context",
            }
        })


if __name__ == "__main__":
    write_textures()
    write_models()
    write_items()
    print(f"текстур: {len(TEXTURES)}, форм: {len(FORMS)}, вариантов: {len(VARIANTS)}, "
          f"предметов: {len(ITEMS)} (объёмные и в инвентаре: {len(GUI_3D)})")
