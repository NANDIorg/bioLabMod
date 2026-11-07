#!/usr/bin/env python3
"""3D-модели машин BioLab (GDD 4.15, «Силуэты»): текстуры, модели, blockstates.

Запуск из корня репозитория:
    python3 tools/mkblock_machines.py
    python3 tools/check_models.py

Пишет (перезаписывает) в content/assets/biolab/:
- textures/block/machine_*.png — общие материалы (сталь, медь, стекло, растворы,
  решётка, экран), <машина>_{side,top,front,accent,lamp,lamp_on}.png — свои;
- models/block/<машина>.json и <машина>_on.json (горящая, `lit=true`);
- blockstates/<машина>.json — `facing` поворачивает модель, `lit` выбирает _on;
- свечение (OptiFine-формат, Horizon его понимает): `<текстура>_e.png` рядом с
  текстурой — её пиксели рисуются полной яркостью. Светятся ядра горящих ламп,
  волна на экране и индикаторы (кисть `glow`). Индикатор работы горит только у
  горящей машины: её модель берёт копию `<машина>_<текстура>_on.png`;
- optifine/ctm/lab_bench_top/ — сшивка столешниц лабораторных столов в ряд;
- части многоблочных установок (GDD 4.18, `write_multiblock`): корпус, стекло,
  порты, колонна, секвенатор; у склеиваемых — модель `<блок>_formed` для `formed=true`,
  и ещё items/<блок>.json.
- биореактор (`write_bioreactor`): люк контроллера и общий вид передней грани
  куба 3×3×3 у собранного (`bioreactor_formed`, модель за границами блока).

Правила геометрии (блоки по шаблону печки — для игры сплошной куб, соседи не
рисуют прилегающие грани, поэтому дыры в модели показали бы пустоту):
- у каждой машины полное основание 16×16 и стенки/плита сзади; детали внутри
  0…16 по X и Z, выше 16 по Y — не больше чем на 4 пикселя;
- перед модели — север (z = 0), как у ванильной печки; из текстуры фронта грань
  берёт область u = 16 − x, v = 16 − y (так же считает ваниль без `uv`);
- `cullface` только у граней на границе блока: всегда у низа и у граней,
  закрывающих всю сторону 16×16. Частичные грани на границе рисуются всегда —
  иначе сквозь соседнюю машину был бы виден провал;
- светящиеся части (`glow`) в модели _on получают `light_emission` и яркую
  текстуру `<машина>_lamp_on`; элементы `lit_only` есть только в _on.
"""
import json
import math
import os
import random
import struct
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS = os.path.join(ROOT, "content", "assets", "biolab")
TEX = os.path.join(ASSETS, "textures", "block")
MODELS = os.path.join(ASSETS, "models", "block")
STATES = os.path.join(ASSETS, "blockstates")

# ---------------------------------------------------------------- палитра

EDGE = (22, 27, 31)
STEEL = (128, 140, 148)
STEEL_LIGHT = (176, 188, 196)
STEEL_DARK = (78, 88, 96)
FRAME = (62, 70, 78)
TEAL = (63, 216, 196)
TEAL_DIM = (30, 120, 110)
COPPER = (192, 108, 64)
COPPER_LIGHT = (232, 152, 100)
COPPER_DARK = (122, 64, 40)
HAZARD = (230, 190, 60)

# Цвет-акцент машины (тот же, что был на крышке куба) и цвет её лампы.
ACCENT = {
    "still": ((196, 112, 66), (255, 150, 50)),
    "incubator": ((210, 170, 90), (255, 200, 110)),
    "fermenter": ((110, 190, 120), (150, 255, 140)),
    "centrifuge": ((80, 140, 214), (120, 200, 255)),
    "synthesizer": ((140, 192, 226), (150, 255, 240)),
    "reactor": ((206, 96, 60), (255, 120, 40)),
    "press": ((220, 200, 110), (255, 230, 120)),
    "extractor": ((100, 186, 230), (140, 230, 255)),
    "cloning_vat": ((180, 140, 224), (220, 170, 255)),
    "spectrometer": ((200, 210, 228), (255, 255, 255)),
    "pen": ((230, 190, 60), (255, 220, 120)),
    "console": ((120, 210, 160), (120, 255, 170)),
}
FLUIDS = {
    "green": (96, 190, 70),
    "violet": (150, 90, 210),
    "cyan": (70, 200, 220),
    "amber": (220, 150, 40),
    "red": (200, 50, 50),
}


# ---------------------------------------------------------------- PNG

def stable(s):
    """Устойчивое зерно шума: hash() строк в Python меняется от запуска к запуску."""
    return zlib.crc32(s.encode())


def clamp(v):
    return max(0, min(255, int(round(v))))


def shade(c, d):
    return tuple(clamp(v + d) for v in c[:3])


def mix(a, b, t):
    return tuple(clamp(a[i] + (b[i] - a[i]) * t) for i in range(3))


def write_png(name, img, folder=TEX):
    h, w = len(img), len(img[0])
    raw = b"".join(b"\x00" + b"".join(bytes(p) for p in row) for row in img)

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    with open(os.path.join(folder, name + ".png"), "wb") as f:
        f.write(data)


# ---------------------------------------------------------------- кисти

def canvas(c, a=255):
    return [[(c[0], c[1], c[2], a) for _ in range(16)] for _ in range(16)]


def put(img, x, y, c, a=255):
    if 0 <= x < 16 and 0 <= y < 16:
        img[y][x] = (c[0], c[1], c[2], a)


def rect(img, x0, y0, x1, y1, c, a=255):
    for y in range(y0, y1):
        for x in range(x0, x1):
            put(img, x, y, c, a)


def grain(img, amp, seed, x0=0, y0=0, x1=16, y1=16):
    """Шум ±amp поверх того, что уже нарисовано (сохраняет прозрачность)."""
    rng = random.Random(seed)
    for y in range(y0, y1):
        for x in range(x0, x1):
            r, g, b, a = img[y][x]
            d = rng.randint(-amp, amp)
            img[y][x] = (clamp(r + d), clamp(g + d), clamp(b + d), a)


def bevel(img, x0, y0, x1, y1, light=24, dark=-34):
    """Фаска: верх и левый край светлее, низ и правый темнее."""
    for x in range(x0, x1):
        img[y0][x] = shade(img[y0][x], light) + (img[y0][x][3],)
        img[y1 - 1][x] = shade(img[y1 - 1][x], dark) + (img[y1 - 1][x][3],)
    for y in range(y0, y1):
        img[y][x0] = shade(img[y][x0], light // 2) + (img[y][x0][3],)
        img[y][x1 - 1] = shade(img[y][x1 - 1], dark) + (img[y][x1 - 1][3],)


def outline(img, x0, y0, x1, y1, c):
    for x in range(x0, x1):
        put(img, x, y0, c)
        put(img, x, y1 - 1, c)
    for y in range(y0, y1):
        put(img, x0, y, c)
        put(img, x1 - 1, y, c)


def rivet(img, x, y):
    put(img, x, y, STEEL_LIGHT)
    put(img, x + 1, y + 1, STEEL_DARK)


def plate(c, seed, amp=4, rivets=True):
    img = canvas(c)
    grain(img, amp, seed)
    bevel(img, 0, 0, 16, 16)
    if rivets:
        for x, y in ((2, 2), (12, 2), (2, 12), (12, 12)):
            rivet(img, x, y)
    return img


def dial(img, cx, cy, face=(226, 230, 220), needle=(200, 40, 40), angle=-40):
    """Круглая шкала 4×4 с ободом и стрелкой; (cx, cy) — левый верхний угол."""
    for dy in range(4):
        for dx in range(4):
            corner = dx in (0, 3) and dy in (0, 3)
            put(img, cx + dx, cy + dy, EDGE if corner else face)
    outline(img, cx - 1, cy - 1, cx + 5, cy + 5, STEEL_DARK)
    for x, y in ((cx - 1, cy - 1), (cx + 4, cy - 1), (cx - 1, cy + 4), (cx + 4, cy + 4)):
        put(img, x, y, FRAME)
    a = math.radians(angle)
    put(img, cx + 1, cy + 2, needle)
    put(img, cx + 1 + round(math.cos(a)), cy + 2 + round(math.sin(a)), needle)


# ---------------------------------------------------------------- свечение
#
# Horizon понимает OptiFine-свечение блоков: `<текстура>_e.png` рядом с
# текстурой рисуется поверх неё полной яркостью (непрозрачные пиксели, cutout).
# `light_emission` элемента светит гранью целиком и не ярче своего уровня, а
# индикатор — это один-два пикселя на корпусе, их так не зажечь. Поэтому
# индикаторы отмечаются кистью `glow`, а файлы `_e` пишет `write_glowing`.
#
# Индикатор питания горит всегда. Индикатор работы (`lit=True`) в обычной
# текстуре тусклый, а горит в её копии `<текстура>_on`, которую берёт модель
# горящей машины. Копия обязана отличаться от оригинала хоть пикселем: блоки с
# одинаковыми пикселями делят слой атласа, и тогда свечения нет ни у одного.

GLOW = {}  # id(картинки) -> (картинка, [(x, y, яркий цвет, только в работе)])
# В лампах и на экране светится всё, что ярче порога (по самому яркому каналу):
# рамка, решётка и тёмный фон остаются тёмными.
BRIGHT = 210


def dim(c):
    """Погасший индикатор работы."""
    return mix(c, (20, 24, 28), 0.6)


def glow(img, x, y, c, lit=False):
    put(img, x, y, dim(c) if lit else c)
    GLOW.setdefault(id(img), (img, []))[1].append((x, y, c, lit))


def led(img, x, y, c, lit=False):
    glow(img, x, y, c, lit)
    put(img, x + 1, y, shade(c, -50))


def bright_pixels(img, limit=BRIGHT):
    return [(x, y) for y in range(16) for x in range(16)
            if img[y][x][3] == 255 and max(img[y][x][:3]) >= limit]


def emissive(img, pts):
    """Слой `_e`: пиксели `pts` из картинки, остальное прозрачное."""
    out = canvas((0, 0, 0), 0)
    for x, y in pts:
        out[y][x] = img[y][x]
    return out


def write_e(name, img, pts, folder=TEX):
    """`<name>_e.png` со светящимися пикселями; без них — убрать старый файл,
    иначе после правки росписи текстура светилась бы по прежней разметке."""
    path = os.path.join(folder, name + "_e.png")
    if pts:
        write_png(name + "_e", emissive(img, pts), folder)
    elif os.path.exists(path):
        os.remove(path)


def write_glowing(name, img):
    """Текстура с индикаторами `glow`: сама, `_e` и, если есть индикаторы
    работы, `_on` с `_on_e`. Возвращает True, если `_on` есть."""
    marks = GLOW.get(id(img), (img, []))[1]
    write_png(name, img)
    always = [(x, y) for x, y, _, lit in marks if not lit]
    write_e(name, img, always)
    lit = [(x, y, c) for x, y, c, on in marks if on]
    on_path = os.path.join(TEX, name + "_on.png")
    if not lit:
        if os.path.exists(on_path):
            os.remove(on_path)
        write_e(name + "_on", img, [])
        return False
    on = [row[:] for row in img]
    for x, y, c in lit:
        put(on, x, y, c)
    write_png(name + "_on", on)
    write_e(name + "_on", on, always + [(x, y) for x, y, _ in lit])
    return True


def vents(img, x0, y0, x1, y1):
    for y in range(y0, y1):
        for x in range(x0, x1):
            put(img, x, y, EDGE if (y - y0) % 2 == 0 else shade(STEEL_DARK, -8))


def hazard(img, x0, y0, x1, y1):
    for y in range(y0, y1):
        for x in range(x0, x1):
            put(img, x, y, HAZARD if (x + y) % 4 < 2 else EDGE)


def label(img, x0, y0, x1, y1, acc):
    """Табличка цвета машины с бирюзовой меткой BioLab слева."""
    rect(img, x0, y0, x1, y1, acc)
    grain(img, 3, x0 * 31 + y0, x0, y0, x1, y1)
    bevel(img, x0, y0, x1, y1, 30, -40)
    put(img, x0 + 1, y0 + (y1 - y0) // 2, TEAL)
    for x in range(x0 + 3, x1 - 2):
        if x % 2:
            put(img, x, y0 + (y1 - y0) // 2, shade(acc, -70))


# ---------------------------------------------------------------- общие материалы

def shared_textures():
    write_png("machine_steel", plate(STEEL, 1))
    write_png("machine_frame", plate(FRAME, 2, amp=3, rivets=False))

    img = canvas(COPPER)
    grain(img, 5, 3)
    for y in range(16):
        if y % 4 == 0:
            rect(img, 0, y, 16, y + 1, COPPER_LIGHT)
        elif y % 4 == 3:
            rect(img, 0, y, 16, y + 1, COPPER_DARK)
    for y in range(16):  # блик вдоль трубы
        put(img, 5, y, mix(img[y][5][:3], COPPER_LIGHT, 0.6))
    write_png("machine_copper", img)

    # Стекло: полупрозрачное, край плотнее, два косых блика.
    img = canvas((200, 236, 240), 40)
    for y in range(16):
        for x in range(16):
            if x in (0, 15) or y in (0, 15):
                put(img, x, y, (214, 240, 244), 140)
            elif x + y in (9, 21):
                put(img, x, y, (255, 255, 255), 110)
    write_png("machine_glass", img)

    for name, c in FLUIDS.items():
        if name == "violet":
            continue  # раствор чана рисует paint_cloning_vat (с зародышем)
        img = canvas(c)
        for y in range(16):
            rect(img, 0, y, 16, y + 1, shade(c, 14 - y * 2))
        grain(img, 6, stable(name) & 0xFFFF)
        rng = random.Random(len(name) * 7)
        for _ in range(7):  # пузырьки
            x, y = rng.randrange(1, 15), rng.randrange(1, 15)
            put(img, x, y, mix(c, (255, 255, 255), 0.55))
        write_png("machine_fluid_" + name, img)

    img = canvas(STEEL_DARK)
    vents(img, 0, 0, 16, 16)
    bevel(img, 0, 0, 16, 16)
    write_png("machine_vent", img)

    # Решётка вольера: прутья через 4 пикселя, поперечины сверху, снизу, по центру.
    img = canvas((0, 0, 0), 0)
    for y in range(16):
        for x in range(16):
            if x % 4 in (1, 2) or y in (0, 15, 7, 8):
                light = x % 4 == 1 or y in (0, 7)
                put(img, x, y, STEEL_LIGHT if light else STEEL_DARK)
    write_png("machine_bars", img)

    # Экран: выключен — тёмный с развёрткой, включён — бирюзовая волна и строки.
    off = canvas((16, 30, 30))
    for y in range(0, 16, 2):
        rect(off, 0, y, 16, y + 1, (20, 38, 38))
    put(off, 2, 2, (60, 90, 90))
    write_png("machine_screen", off)
    on = canvas((12, 40, 38))
    for y in range(0, 16, 2):
        rect(on, 0, y, 16, y + 1, (16, 52, 48))
    for x in range(16):
        y = 6 + round(2.5 * math.sin(x * 0.8))
        put(on, x, y, TEAL)
        put(on, x, y + 1, TEAL_DIM)
    for y, n in ((11, 9), (13, 6)):
        for x in range(2, 2 + n):
            put(on, x, y, (140, 255, 220) if x % 3 else TEAL_DIM)
    put(on, 13, 2, (255, 220, 120))
    write_png("machine_screen_on", on)
    # Включённый экран: волна и строки светятся, фон и развёртка — нет.
    write_e("machine_screen_on", on, bright_pixels(on))

    # Пробирки экстрактора (и стола): столбцы по 2 пикселя, снизу раствор, сверху стекло.
    img = canvas((0, 0, 0), 0)
    for i, c in enumerate([FLUIDS[k] for k in ("red", "green", "violet", "amber", "cyan")]):
        for y in range(16):
            for dx in range(2):
                x = i * 2 + dx
                if y >= 11:
                    put(img, x, y, shade(c, 25 if dx == 0 else -10))
                elif y == 10:
                    put(img, x, y, mix(c, (255, 255, 255), 0.4))
                else:
                    put(img, x, y, (220, 244, 248), 110 if dx else 170)
    write_png("machine_tubes", img)


# ---------------------------------------------------------------- свои текстуры

def side_tex(m, acc):
    img = plate(STEEL, stable(m) & 0xFFF)
    rect(img, 1, 5, 15, 7, acc)
    rect(img, 1, 7, 15, 8, shade(acc, -70))
    vents(img, 4, 10, 12, 14)
    return img


def top_tex(m, acc):
    img = plate(STEEL_DARK, (stable(m) >> 3) & 0xFFF, amp=3)
    for x, y in ((0, 0), (14, 0), (0, 14), (14, 14)):
        rect(img, x, y, x + 2, y + 2, acc)
    return img


def accent_tex(m, acc):
    img = canvas(acc)
    grain(img, 5, (stable(m) >> 5) & 0xFFF)
    for y in (0, 15):
        rect(img, 0, y, 16, y + 1, shade(acc, -55))
    for x in range(1, 16, 4):
        put(img, x, 1, shade(acc, 45))
        put(img, x, 14, shade(acc, -80))
    for y in range(1, 15):  # блик цилиндра
        put(img, 4, y, shade(img[y][4], 34))
        put(img, 5, y, shade(img[y][5], 18))
    return img


def lamp_tex(c, on):
    if on:
        img = canvas(c)
        rect(img, 4, 4, 12, 12, mix(c, (255, 255, 255), 0.45))
        rect(img, 6, 6, 10, 10, (255, 255, 255))
        grain(img, 4, 7)
        outline(img, 0, 0, 16, 16, mix(c, (0, 0, 0), 0.2))
    else:
        dim = mix(c, (20, 24, 28), 0.72)
        img = canvas(dim)
        grain(img, 3, 8)
        outline(img, 0, 0, 16, 16, shade(dim, -20))
        for i in range(2, 6):
            put(img, i, 2, mix(dim, (255, 255, 255), 0.35))
    return img


def machine_textures(m, paint, keep):
    """Рисует свои текстуры машины и пишет только те, на которые ссылается модель."""
    acc, lamp = ACCENT[m]
    tex = {
        "side": side_tex(m, acc),
        "top": top_tex(m, acc),
        "front": plate(STEEL, (stable(m) >> 7) & 0xFFF),
        "accent": accent_tex(m, acc),
        "lamp": lamp_tex(lamp, False),
        "lamp_on": lamp_tex(lamp, True),
    }
    GLOW.clear()
    paint(tex, acc, lamp)
    with_on = set()
    for k, img in tex.items():
        if k not in keep:
            continue
        if k == "lamp_on":
            # Горящая лампа: ядро и спираль — полной яркостью, поверх
            # `light_emission` элемента, который светит лишь своим уровнем.
            write_png(f"{m}_{k}", img)
            write_e(f"{m}_{k}", img, bright_pixels(img))
        elif k != "lamp" and write_glowing(f"{m}_{k}", img):
            with_on.add(k)
        elif k == "lamp":
            write_png(f"{m}_{k}", img)
    return with_on


# Росписи: координаты в текстуре. Грань элемента [x1..x2, y1..y2] спереди
# берёт u = 16 − x2…16 − x1, v = 16 − y2…16 − y1.

def paint_still(t, acc, lamp):
    f = t["front"]  # основание: v 11..16, топка u 5..11 закрыта люком
    rect(f, 0, 11, 16, 16, FRAME)
    grain(f, 3, 11, 0, 11, 16, 16)
    rect(f, 0, 11, 16, 12, TEAL_DIM)
    rivet(f, 1, 13)
    rivet(f, 13, 13)
    s = t["side"]  # задняя плита: медная окантовка, термометр
    rect(s, 0, 0, 16, 2, COPPER)
    rect(s, 0, 1, 16, 2, COPPER_DARK)
    rect(s, 2, 3, 3, 10, (226, 230, 220))
    rect(s, 2, 6, 3, 10, (200, 40, 40))
    # Люк топки: решётка поверх углей.
    for name, glow in (("lamp", False), ("lamp_on", True)):
        img = canvas((255, 170, 60) if glow else (70, 30, 20))
        grain(img, 6 if glow else 3, 9)
        for x in range(16):
            for y in range(16):
                if x % 3 == 0 or y in (0, 15):
                    put(img, x, y, (40, 36, 34))
        t[name] = img
    t["accent"] = accent_tex("still", COPPER)


def paint_incubator(t, acc, lamp):
    f = t["front"]
    rect(f, 2, 13, 14, 16, STEEL_DARK)  # пульт под дверцей
    rect(f, 3, 14, 7, 15, (40, 24, 12))  # табло температуры
    for x in (3, 5):  # цифры табло горят, пока идёт инкубация
        glow(f, x, 14, (255, 190, 90), lit=True)
    led(f, 8, 14, TEAL)  # питание
    led(f, 11, 14, lamp, lit=True)  # работа
    label(f, 3, 0, 13, 3, acc)  # табличка над дверцей
    for y in (4, 11):  # петли справа (x 0..2 — правая стенка, u 14..16)
        rect(f, 14, y, 15, y + 2, STEEL_DARK)


def paint_fermenter(t, acc, lamp):
    f = t["front"]
    rect(f, 0, 13, 16, 16, FRAME)
    rect(f, 0, 13, 16, 14, TEAL_DIM)
    label(f, 5, 14, 11, 16, acc)
    s = t["side"]  # бак: обручи цвета машины
    s2 = plate(STEEL, 41, rivets=False)
    for y in (3, 4, 11, 12):
        rect(s2, 0, y, 16, y + 1, acc if y in (3, 11) else shade(acc, -60))
    for x in range(1, 16, 3):
        put(s2, x, 8, STEEL_LIGHT)
    t["side"] = s2
    tp = t["top"]  # круглый люк по центру, u/v 5..11
    rect(tp, 5, 5, 11, 11, STEEL)
    outline(tp, 5, 5, 11, 11, EDGE)
    for x, y in ((5, 5), (10, 5), (5, 10), (10, 10)):
        put(tp, x, y, STEEL_DARK)
    for name, glow in (("lamp", False), ("lamp_on", True)):
        c = FLUIDS["green"]
        img = canvas(shade(c, 40) if glow else mix(c, (10, 20, 10), 0.55))
        grain(img, 5, 4)
        rng = random.Random(5)
        for _ in range(10):
            put(img, rng.randrange(16), rng.randrange(16), (230, 255, 220) if glow else shade(c, -10))
        outline(img, 0, 0, 16, 16, STEEL_DARK)
        t[name] = img


def paint_centrifuge(t, acc, lamp):
    f = t["front"]  # корпус: v 8..16
    rect(f, 0, 8, 16, 9, acc)
    dial(f, 3, 10, angle=-120)
    vents(f, 9, 11, 14, 15)
    led(f, 3, 15, TEAL)  # питание
    tp = t["top"]  # крышка барабана с лучами (центр 8,8)
    for y in range(3, 13):
        for x in range(3, 13):
            dx, dy = x - 7.5, y - 7.5
            spoke = abs(dx) < 0.6 or abs(dy) < 0.6 or abs(abs(dx) - abs(dy)) < 0.6
            put(tp, x, y, STEEL_LIGHT if spoke else STEEL)
    outline(tp, 3, 3, 13, 13, EDGE)


def paint_synthesizer(t, acc, lamp):
    f = t["front"]
    rect(f, 0, 13, 16, 16, STEEL_DARK)  # основание: v 13..16
    rect(f, 0, 13, 16, 14, TEAL_DIM)
    for x in (3, 6, 9):  # питание и два индикатора работы
        led(f, x, 14, TEAL if x == 3 else acc, lit=x != 3)
    for x in (12, 13):  # красная кнопка «идёт синтез»
        glow(f, x, 14, (230, 80, 60), lit=True)
    label(f, 4, 1, 12, 3, acc)  # крышка камеры: v 1..3
    for u in (0, 14):  # стойки камеры: v 3..13
        rect(f, u, 3, u + 2, 13, FRAME)
        glow(f, u + 1 if u == 0 else u, 4, TEAL)  # огни на стойках камеры
    s = t["side"]
    rect(s, 3, 6, 13, 11, (40, 60, 66))  # иллюминатор сбоку
    outline(s, 3, 6, 13, 11, EDGE)
    put(s, 4, 7, (120, 170, 180))


def paint_reactor(t, acc, lamp):
    f = t["front"]
    rect(f, 0, 13, 16, 16, FRAME)
    rect(f, 0, 13, 16, 14, (150, 60, 30))
    # Манометр: грань элемента [6..10, 7..11] → u 6..10, v 5..9.
    rect(f, 6, 5, 10, 9, (230, 230, 220))
    for x, y in ((6, 5), (9, 5), (6, 8), (9, 8)):
        put(f, x, y, STEEL_DARK)
    put(f, 7, 7, (200, 30, 30))
    put(f, 8, 6, (200, 30, 30))
    put(f, 7, 6, EDGE)
    put(f, 8, 8, (60, 160, 60))
    for name, glow in (("lamp", False), ("lamp_on", True)):
        img = canvas((255, 110, 30) if glow else (60, 26, 18))
        grain(img, 6 if glow else 3, 12)
        for y in range(16):
            for x in range(16):
                if y % 3 == 0:
                    put(img, x, y, (255, 220, 120) if glow else (34, 30, 28))
        t[name] = img


def paint_press(t, acc, lamp):
    f = t["front"]
    hazard(f, 0, 12, 16, 16)  # основание
    label(f, 3, 0, 13, 3, acc)  # траверса: v 0..3
    for u in (0, 14):  # щёки: v 0..12
        rect(f, u, 0, u + 2, 12, FRAME)
    a = t["accent"]  # плита: жёлто-чёрная кромка спереди (v 6..8)
    hazard(a, 0, 6, 16, 8)


def paint_extractor(t, acc, lamp):
    f = t["front"]
    rect(f, 0, 13, 16, 16, FRAME)
    rect(f, 0, 13, 16, 14, TEAL_DIM)
    led(f, 2, 14, TEAL)  # питание
    label(f, 6, 14, 14, 16, acc)
    a = t["accent"]  # колонна: тёмные кольца
    for y in (3, 8, 13):
        rect(a, 0, y, 16, y + 1, shade(acc, -80))


def paint_cloning_vat(t, acc, lamp):
    f = t["front"]
    rect(f, 0, 13, 16, 16, STEEL_DARK)
    for x in (2, 5):  # питание
        led(f, x, 14, TEAL)
    label(f, 8, 13, 14, 16, acc)
    label(f, 1, 0, 15, 3, acc)  # крышка: v 0..3
    # Раствор с зародышем: силуэт по центру грани (u/v 6..10).
    for name, glow in (("lamp", False), ("lamp_on", True)):
        c = mix(FLUIDS["violet"], acc, 0.4)
        c = shade(c, 30) if glow else mix(c, (20, 10, 30), 0.45)
        img = canvas(c)
        for y in range(16):
            rect(img, 0, y, 16, y + 1, shade(c, 10 - y))
        grain(img, 5, 21)
        body = shade(c, -70)
        rect(img, 7, 6, 10, 9, body)   # голова
        rect(img, 6, 9, 10, 13, body)  # тело
        put(img, 5, 10, body)
        put(img, 10, 10, body)
        rng = random.Random(3)
        for _ in range(8):
            put(img, rng.randrange(16), rng.randrange(16), mix(c, (255, 255, 255), 0.5))
        t[name] = img


def paint_spectrometer(t, acc, lamp):
    f = t["front"]
    # Шкала у задней стенки: грань [0..16, 6..14] → v 2..10.
    rect(f, 0, 2, 16, 10, (30, 32, 40))
    rainbow = [(200, 40, 40), (230, 120, 40), (230, 210, 60), (80, 190, 80),
               (60, 160, 220), (70, 80, 200), (150, 70, 200)]
    for x in range(1, 15):
        c = rainbow[min(6, (x - 1) * 7 // 14)]
        for y in range(5, 8):  # спектр на шкале виден, пока идёт анализ
            glow(f, x, y, c, lit=True)
        if x % 3 == 1:
            put(f, x, 4, STEEL_LIGHT)
    rect(f, 0, 2, 16, 3, STEEL_DARK)
    rect(f, 0, 9, 16, 10, STEEL_DARK)
    # Основание: v 10..16 — ручки и щель.
    rect(f, 0, 10, 16, 16, STEEL)
    grain(f, 3, 5, 0, 10, 16, 16)
    rect(f, 0, 10, 16, 11, TEAL_DIM)
    rect(f, 5, 12, 11, 14, EDGE)
    for x in range(6, 10):  # зелёная щель «анализ»
        glow(f, x, 13, (80, 190, 80), lit=True)
    for x in (2, 13):
        rect(f, x, 12, x + 2, 14, STEEL_DARK)
        put(f, x, 12, STEEL_LIGHT)
    # Призма: стекло с радугой по краю.
    img = canvas((220, 236, 255), 110)
    for y in range(16):
        put(img, 0, y, (255, 255, 255), 200)
        put(img, 15, y, rainbow[y * 7 // 16], 210)
        put(img, 14, y, rainbow[y * 7 // 16], 120)
    t["accent"] = img


def paint_pen(t, acc, lamp):
    f = t["front"]
    hazard(f, 0, 0, 16, 2)  # крыша спереди
    t["side"] = plate(STEEL_DARK, 77)  # задняя стенка с кормушкой
    s = t["side"]
    rect(s, 2, 10, 14, 12, FRAME)
    label(s, 5, 3, 11, 5, acc)
    tp = t["top"]  # пол вольера — солома
    straw = canvas((190, 160, 80))
    grain(straw, 12, 31)
    rng = random.Random(9)
    for _ in range(26):
        x, y = rng.randrange(15), rng.randrange(16)
        put(straw, x, y, (226, 200, 110))
        put(straw, x + 1, y, (150, 120, 60))
    t["floor"] = straw
    hazard(tp, 0, 0, 16, 1)


def paint_console(t, acc, lamp):
    f = t["front"]
    # Корпус монитора: грань [0..16, 8..16] → v 0..8; рамка вокруг экрана.
    rect(f, 0, 0, 16, 8, FRAME)
    grain(f, 3, 6, 0, 0, 16, 8)
    rect(f, 1, 0, 15, 1, acc)
    # Тумба: v 7..16 — ящики.
    rect(f, 0, 8, 16, 16, STEEL)
    grain(f, 3, 7, 0, 8, 16, 16)
    for y0 in (9, 13):
        outline(f, 2, y0, 14, y0 + 3, STEEL_DARK)
        rect(f, 6, y0 + 1, 10, y0 + 2, STEEL_LIGHT)
    tp = t["top"]  # наклонная панель: кнопки и клавиши
    rect(tp, 1, 2, 15, 10, (52, 60, 66))
    for x in range(2, 14, 2):
        put(tp, x, 7, STEEL_LIGHT)
        put(tp, x, 8, STEEL_LIGHT)
    for x in (3, 7, 11):  # кнопки пульта подсвечены, пока консоль работает
        glow(tp, x, 4, acc, lit=True)


def lab_bench_textures():
    s = plate(STEEL, 90)  # тумба: ящик и дверца
    rect(s, 1, 5, 15, 16, STEEL)
    grain(s, 3, 91, 1, 5, 15, 16)
    outline(s, 2, 6, 14, 9, STEEL_DARK)
    rect(s, 6, 7, 10, 8, STEEL_LIGHT)
    outline(s, 2, 10, 8, 16, STEEL_DARK)
    outline(s, 8, 10, 14, 16, STEEL_DARK)
    put(s, 6, 13, STEEL_LIGHT)
    put(s, 9, 13, STEEL_LIGHT)
    rect(s, 1, 5, 15, 6, TEAL_DIM)
    write_png("lab_bench", s)
    write_png("lab_bench_top", bench_top("lrtb"))
    bench_top_ctm()


def bench_top(edges, corners=False):
    """Столешница: тёмный пластик с бирюзовой кромкой по краям `edges`
    (l, r, t, b — лево, право, верх, низ текстуры); `corners` — только угловые
    пиксели кромки (внутренний угол стыка)."""
    acc = (124, 230, 110)
    tp = canvas((44, 50, 56))
    grain(tp, 3, 92)
    for e, (x0, y0, x1, y1) in {"l": (0, 0, 1, 16), "r": (15, 0, 16, 16),
                                "t": (0, 0, 16, 1), "b": (0, 15, 16, 16)}.items():
        if e in edges:
            rect(tp, x0, y0, x1, y1, TEAL_DIM)
    if corners:
        for x, y in ((0, 0), (15, 0), (0, 15), (15, 15)):
            put(tp, x, y, TEAL_DIM)
    for x, y in ((4, 9), (5, 9), (11, 4)):
        put(tp, x, y, shade(acc, -90))
    return tp


CTM = os.path.join(ASSETS, "optifine", "ctm")


def bench_top_ctm():
    """Сшивка столешниц (OptiFine CTM, `ctm_compact`): столы в ряд — одна
    столешница, кромка только по внешнему краю. Пять плиток в порядке шаблона
    OptiFine: 0 — кромка кругом (как сама текстура), 1 — без кромки, 2 — кромка
    слева и справа (соседи сверху и снизу), 3 — сверху и снизу, 4 — только
    углы. Движок режет грань на четверти и каждой берёт свою плитку.

    Соединяемся по блоку (`connect=block`), а не по плитке: соседняя плитка
    ищется по первой грани модели, смотрящей вверх, а у стола это низ рамы, не
    столешница. `matchTiles`, а не `matchBlocks`: иначе плитки легли бы на все
    верхние грани стола — колбы и штатив тоже."""
    folder = os.path.join(CTM, "lab_bench_top")
    os.makedirs(folder, exist_ok=True)
    tiles = [bench_top("lrtb"), bench_top(""), bench_top("lr"), bench_top("tb"),
             bench_top("", corners=True)]
    for i, img in enumerate(tiles):
        write_png(str(i), img, folder)
    with open(os.path.join(folder, "lab_bench_top.properties"), "w") as f:
        f.write("# Пишет tools/mkblock_machines.py: столешницы столов в ряд сшиваются.\n"
                "matchTiles=biolab:block/lab_bench_top\n"
                "method=ctm_compact\n"
                "tiles=0-4\n"
                "connect=block\n"
                "faces=top\n")


# Операционная (GDD 4.17): стол и кость для срезанных конечностей.
SURGICAL = (96, 168, 140)  # хирургическая зелень полотна
STRAP = (92, 58, 36)  # кожа ремней
BONE = (232, 222, 196)


def operating_table_textures():
    # Тумба под столешницей: бирюзовая полоса, решётка, табличка с крестом.
    s = plate(STEEL, 120)
    rect(s, 1, 2, 15, 3, TEAL_DIM)
    vents(s, 3, 5, 13, 9)
    label(s, 4, 11, 12, 14, SURGICAL)
    for x, y in ((7, 11), (8, 11), (6, 12), (7, 12), (8, 12), (9, 12), (7, 13), (8, 13)):
        put(s, x, y, (230, 60, 60))
    write_png("operating_table_side", s)
    # Полотно: зелёная ткань с прострочкой по краю.
    pad = canvas(SURGICAL)
    grain(pad, 4, 121)
    for y in range(0, 16, 3):
        rect(pad, 0, y, 16, y + 1, shade(SURGICAL, 8))
    outline(pad, 0, 0, 16, 16, shade(SURGICAL, -40))
    for i in range(1, 15, 2):
        for x, y in ((i, 1), (i, 14), (1, i), (14, i)):
            put(pad, x, y, shade(SURGICAL, 36))
    write_png("operating_table_pad", pad)
    # Ремень: кожа с прострочкой, пряжка рисуется отдельным элементом.
    st = canvas(STRAP)
    grain(st, 5, 122)
    for y in (0, 15):
        rect(st, 0, y, 16, y + 1, shade(STRAP, -30))
    for x in range(0, 16, 2):
        put(st, x, 2, shade(STRAP, 30))
        put(st, x, 13, shade(STRAP, 30))
    write_png("operating_table_strap", st)
    # Абажур лампы и её стекло (светится всегда: лампа над изголовьем).
    shade_img = accent_tex("operating_table", (214, 222, 226))
    write_png("operating_table_accent", shade_img)
    lamp = lamp_tex((255, 246, 214), True)
    write_png("operating_table_lamp", lamp)
    write_e("operating_table_lamp", lamp, bright_pixels(lamp))


def bone_texture():
    """Кость срезанной конечности (рисует проход костей в `biolab_deep`).
    Лежит в textures/gui: только оттуда текстуры мода попадают в общий
    атлас, из которого рисуются мобы (ключ `biolab:gui/surgery/bone`).
    Конечность берёт из неё произвольный кусок, поэтому без рисунка —
    светлая кость с волокнами вдоль и тёмными порами."""
    img = canvas(BONE)
    rng = random.Random(4170)
    for x in range(16):
        d = rng.randint(-10, 8)
        for y in range(16):
            img[y][x] = shade(img[y][x], d) + (255,)
    grain(img, 4, 4171)
    for _ in range(9):
        x, y = rng.randrange(16), rng.randrange(16)
        put(img, x, y, (178, 160, 128))
    for _ in range(5):  # прожилки — розоватые, кость свежая
        x, y = rng.randrange(16), rng.randrange(15)
        put(img, x, y, (214, 168, 150))
        put(img, x, y + 1, (214, 168, 150))
    folder = os.path.join(ASSETS, "textures", "gui", "surgery")
    os.makedirs(folder, exist_ok=True)
    write_png("bone", img, folder)


PAINTERS = {
    "still": paint_still,
    "incubator": paint_incubator,
    "fermenter": paint_fermenter,
    "centrifuge": paint_centrifuge,
    "synthesizer": paint_synthesizer,
    "reactor": paint_reactor,
    "press": paint_press,
    "extractor": paint_extractor,
    "cloning_vat": paint_cloning_vat,
    "spectrometer": paint_spectrometer,
    "pen": paint_pen,
    "console": paint_console,
}


# ---------------------------------------------------------------- модели

FACES = ("down", "up", "north", "south", "west", "east")


def auto_uv(face, f, t):
    """UV, как ваниль считает без явного `uv`, со сдвигом в 0..16 для частей выше 16."""
    x1, y1, z1 = f
    x2, y2, z2 = t
    uv = {
        "down": [x1, 16 - z2, x2, 16 - z1],
        "up": [x1, z1, x2, z2],
        "north": [16 - x2, 16 - y2, 16 - x1, 16 - y1],
        "south": [x1, 16 - y2, x2, 16 - y1],
        "west": [z1, 16 - y2, z2, 16 - y1],
        "east": [16 - z2, 16 - y2, 16 - z1, 16 - y1],
    }[face]
    for a, b in ((0, 2), (1, 3)):
        if uv[a] < 0:
            uv[a], uv[b] = 0, uv[b] - uv[a]
        if uv[b] > 16:
            uv[a], uv[b] = uv[a] - (uv[b] - 16), 16
    return [round(v, 4) for v in uv]


def on_boundary(face, f, t):
    return {"down": f[1] == 0, "up": t[1] == 16, "north": f[2] == 0,
            "south": t[2] == 16, "west": f[0] == 0, "east": t[0] == 16}[face]


def full_side(face, f, t):
    span = {"down": (0, 2), "up": (0, 2), "north": (0, 1), "south": (0, 1),
            "west": (2, 1), "east": (2, 1)}[face]
    return all(f[i] == 0 and t[i] == 16 for i in span)


def num(v):
    return int(v) if float(v).is_integer() else v


def E(f, t, tex, faces=FACES, glow=False, lit_only=False, rot=None, uv=None, **over):
    """Элемент: `tex` — ключ текстуры для всех граней, `over` — по граням
    (north="front", up="top"…), `rot` = (ось, угол, центр), `uv` — {грань: uv}."""
    return dict(f=f, t=t, tex=tex, faces=faces, glow=glow, lit_only=lit_only, rot=rot,
                uv=uv or {}, over=over)


def element_json(e, lit):
    faces = {}
    for face in FACES:
        if face not in e["faces"]:
            continue
        key = e["over"].get(face, e["tex"])
        d = {"uv": e["uv"].get(face) or auto_uv(face, e["f"], e["t"]), "texture": "#" + key}
        if on_boundary(face, e["f"], e["t"]) and (face == "down" or full_side(face, e["f"], e["t"])) \
                and e["rot"] is None:
            d["cullface"] = face
        faces[face] = d
    out = {"from": [num(v) for v in e["f"]], "to": [num(v) for v in e["t"]]}
    if e["rot"]:
        axis, angle, origin = e["rot"]
        out["rotation"] = {"origin": [num(v) for v in origin], "axis": axis, "angle": angle}
    if lit and e["glow"]:
        out["light_emission"] = 15 if e["glow"] is True else e["glow"]
    out["faces"] = faces
    return out


def dump_model(path, model):
    """JSON с одним элементом на строку — короче и читается глазами."""
    lines = ["{"]
    keys = [k for k in model if k != "elements"]
    for k in keys:
        lines.append(f'  "{k}": {json.dumps(model[k], ensure_ascii=False)},')
    if "elements" in model:
        lines.append('  "elements": [')
        els = [json.dumps(el, ensure_ascii=False) for el in model["elements"]]
        lines.append(",\n".join("    " + s for s in els))
        lines.append("  ]")
    else:
        lines[-1] = lines[-1].rstrip(",")
    lines.append("}")
    with open(path, "w") as f:
        f.write("\n".join(lines) + "\n")


def cyl(x0, x1, z0, z1, y0, y1, tex, **kw):
    """«Цилиндр» без перекрытия граней: средняя плита и две боковые полосы."""
    return [E([x0 + 1, y0, z0], [x1 - 1, y1, z1], tex, **kw),
            E([x0, y0, z0 + 1], [x0 + 1, y1, z1 - 1], tex, **kw),
            E([x1 - 1, y0, z0 + 1], [x1, y1, z1 - 1], tex, **kw)]


NO_S = ("down", "up", "north", "west", "east")  # задняя грань упирается в корпус
NO_D = ("up", "north", "south", "west", "east")  # стоит на поверхности
PANE_Z = ("north", "south")
PANE_X = ("west", "east")


def still():
    return [
        E([0, 0, 0], [16, 1, 16], "frame"),
        E([0, 1, 1], [16, 5, 16], "frame", north="front", up="top", faces=NO_D),
        E([5, 1, 0], [11, 4, 1], "lamp", faces=("up", "north", "west", "east"), glow=13),
        *cyl(2, 12, 2, 12, 5, 12, "accent"),
        E([4, 12, 4], [10, 14, 10], "accent", faces=NO_D),
        E([6, 14, 6], [8, 17, 8], "copper", faces=NO_D),
        E([8, 15, 6], [15, 17, 8], "copper", faces=("down", "up", "north", "south", "east")),
        E([13, 5, 6], [15, 15, 8], "copper", faces=NO_D),
        E([12, 7, 4], [16, 8, 10], "copper"),
        E([12, 9.5, 4], [16, 10.5, 10], "copper"),
        E([12, 12, 4], [16, 13, 10], "copper"),
        E([13, 10, 2], [14, 11, 6], "copper", faces=("down", "up", "north", "west", "east")),
        E([12, 5, 1], [15, 8, 4], "glass", faces=NO_D),
        E([12.5, 5, 1.5], [14.5, 7, 3.5], "fluid_cyan", faces=NO_D),
        E([13, 8, 2], [14, 10, 3], "glass", faces=("north", "south", "west", "east")),
        E([0, 5, 15], [16, 16, 16], "side", faces=NO_D, up="top"),
    ]


def incubator():
    return [
        E([14, 0, 0], [16, 16, 16], "side", north="front", up="top", west="frame"),
        E([0, 0, 0], [2, 16, 16], "side", north="front", up="top", east="frame"),
        E([2, 0, 0], [14, 3, 16], "side", north="front", up="frame", faces=("down", "up", "north", "south")),
        E([2, 13, 0], [14, 16, 16], "side", north="front", up="top", down="frame",
          faces=("down", "up", "north", "south")),
        E([2, 3, 15], [14, 13, 16], "frame", faces=("north", "south"), south="side"),
        E([2, 3, 1], [14, 3.5, 15], "lamp", faces=("up", "north"), glow=12),
        E([2, 7.5, 1], [14, 8, 15], "vent", faces=("up", "down", "north")),
        E([4, 8, 4], [8, 9, 8], "fluid_amber", faces=NO_D),
        E([9, 8, 7], [12, 9, 10], "fluid_amber", faces=NO_D),
        E([8, 3.5, 4], [12, 4.5, 8], "fluid_amber", faces=NO_D),
        E([2, 3, 0.5], [14, 13, 1], "glass", faces=PANE_Z),
        E([3, 5, 0], [4, 10, 0.5], "steel", faces=("up", "down", "north", "west", "east")),
        E([10, 16, 10], [13, 18, 13], "steel", faces=NO_D, up="top"),
    ]


def fermenter():
    return [
        E([0, 0, 0], [16, 3, 16], "frame", north="front", up="top"),
        *cyl(2, 14, 2, 14, 3, 14, "side", up="top"),
        E([4, 14, 4], [12, 15, 12], "steel", faces=NO_D, up="top"),
        E([5, 15, 5], [11, 16, 11], "steel", faces=NO_D, up="top"),
        E([6, 16, 7.5], [10, 17, 8.5], "frame", faces=NO_D),
        E([7.5, 16, 6], [8.5, 17, 10], "frame", faces=NO_D),
        E([6, 5, 1], [10, 12, 2], "lamp", faces=NO_S, glow=10),
        E([14, 5, 7], [16, 7, 9], "copper", faces=("up", "down", "north", "south")),
        E([15, 4, 6], [16, 8, 10], "accent"),
    ]


def centrifuge():
    return [
        E([0, 0, 0], [16, 8, 16], "side", north="front", up="top"),
        *cyl(2, 14, 2, 14, 8, 12, "accent", faces=NO_D, up="top"),
        E([3.5, 12, 3.5], [12.5, 13, 12.5], "steel", faces=NO_D, up="top",
          rot=("y", 22.5, [8, 12, 8])),
        E([7, 13, 7], [9, 14, 9], "lamp", faces=NO_D, glow=12),
        E([4, 8, 14], [12, 13, 16], "side", faces=NO_D, up="top"),
        E([12, 8, 0.5], [13.5, 9, 2], "lamp", faces=NO_D, glow=10),
        E([2.5, 8, 0.5], [4, 9, 2], "lamp", faces=NO_D, glow=10),
        E([7, 11, 1], [9, 12, 2], "frame", faces=("up", "down", "north", "west", "east")),
    ]


def synthesizer():
    return [
        E([0, 0, 0], [16, 3, 16], "side", north="front", up="frame"),
        E([14, 3, 0], [16, 13, 16], "side", north="front", west="frame", faces=("north", "south", "west", "east")),
        E([0, 3, 0], [2, 13, 16], "side", north="front", east="frame", faces=("north", "south", "west", "east")),
        E([2, 3, 14], [14, 13, 16], "frame", faces=("north", "south"), south="side"),
        E([0, 13, 0], [16, 15, 16], "side", north="front", up="top", down="frame"),
        E([2, 3, 0.5], [14, 13, 1.5], "glass", faces=PANE_Z),
        E([6, 3, 4], [10, 5, 8], "steel", faces=NO_D),
        E([7, 5, 5], [9, 7, 7], "lamp", faces=NO_D, glow=True),
        E([10, 8, 5], [11.5, 13, 7], "steel", faces=("down", "north", "south", "west", "east"),
          rot=("z", -22.5, [10.75, 13, 6])),
        E([4.5, 8, 5], [6, 13, 7], "steel", faces=("down", "north", "south", "west", "east"),
          rot=("z", 22.5, [5.25, 13, 6])),
        E([8.2, 7.6, 5.25], [9.6, 8.4, 6.75], "copper"),
        E([6.4, 7.6, 5.25], [7.8, 8.4, 6.75], "copper"),
        E([5, 15, 5], [11, 16.5, 11], "accent", faces=NO_D),
        E([11.5, 15, 11.5], [13.5, 18, 13.5], "copper", faces=NO_D),
        E([3, 15, 1], [5, 16, 3], "lamp", faces=NO_D, glow=10),
    ]


def reactor():
    return [
        E([0, 0, 0], [16, 0.5, 16], "frame"),
        E([0, 0.5, 1], [16, 3, 16], "frame", north="front", up="top", faces=NO_D),
        E([4, 0.5, 0], [12, 2.5, 1], "lamp", faces=("up", "north", "west", "east"), glow=13),
        *cyl(3, 13, 3, 15, 3, 14, "accent"),
        E([5, 14, 5], [11, 15, 13], "accent", faces=NO_D, up="top"),
        E([6, 7, 2], [10, 11, 3], "steel", faces=NO_S, north="front"),
        E([0, 3, 7], [2, 15, 9], "copper", faces=NO_D),
        E([0, 15, 7], [6, 17, 9], "copper", faces=("down", "up", "north", "south", "west")),
        E([13, 3, 1], [15, 12, 3], "copper", faces=NO_D),
        E([12, 10, 2], [13, 12, 4], "copper", faces=("up", "down", "north", "south")),
        E([12.5, 6, 0.5], [15.5, 9, 1], "accent", faces=NO_S),
    ]


def press():
    return [
        E([0, 0, 0], [16, 4, 16], "frame", north="front", up="top"),
        E([14, 4, 0], [16, 16, 14], "side", north="front", up="top", faces=NO_D),
        E([0, 4, 0], [2, 16, 14], "side", north="front", up="top", faces=NO_D),
        E([2, 13, 2], [14, 16, 14], "side", north="front", up="top", faces=("down", "up", "north", "south")),
        E([6, 10, 6], [10, 13, 10], "steel", faces=("down", "north", "south", "west", "east")),
        E([3, 8, 3], [13, 10, 13], "accent"),
        E([4, 4, 4], [12, 5, 12], "steel", faces=NO_D),
        E([6, 5, 6], [10, 6, 10], "copper", faces=NO_D),
        E([0, 4, 14], [16, 13, 16], "side", faces=NO_D, up="top"),
        E([7, 11, 10], [9, 12, 14], "copper", faces=("up", "down", "west", "east")),
        E([11, 14, 1], [13, 15, 2], "lamp", faces=NO_S, glow=10),
    ]


def tube(x, z, h, col, y=5):
    """Пробирка 2×2 высотой h: раствор из столбца `col` атласа machine_tubes."""
    side = [col * 2, 16 - h, col * 2 + 2, 16]
    return E([x, y, z], [x + 2, y + h, z + 2], "tubes", faces=NO_D,
             uv={"up": [col * 2, 0, col * 2 + 2, 2], "north": side, "south": side,
                 "west": side, "east": side})


def extractor():
    return [
        E([0, 0, 0], [16, 3, 16], "frame", north="front", up="top"),
        *cyl(5, 11, 8, 14, 3, 15, "accent"),
        E([6.5, 6, 7], [9.5, 12, 8], "lamp", faces=NO_S, glow=11),
        E([6, 15, 9], [10, 17, 13], "steel", faces=NO_D, up="top"),
        E([1, 3, 1], [15, 5, 6], "steel", faces=NO_D),
        tube(12, 2.5, 5, 0),
        tube(9, 2.5, 4, 1),
        tube(6, 2.5, 5, 2),
        tube(3, 2.5, 3, 3),
        E([7, 11, 4], [9, 12, 8], "steel", faces=("up", "down", "north", "west", "east")),
        E([7.5, 10, 3.5], [8.5, 11, 4.5], "copper", faces=("down", "north", "south", "west", "east")),
        E([0, 3, 14], [16, 11, 16], "side", faces=NO_D, up="top"),
    ]


def cloning_vat():
    posts = [E([x, 3, z], [x + 1.5, 13, z + 1.5], "frame", faces=("north", "south", "west", "east"))
             for x, z in ((0, 0), (14.5, 0), (0, 14.5), (14.5, 14.5))]
    return [
        E([0, 0, 0], [16, 3, 16], "side", north="front", up="steel"),
        E([0, 13, 0], [16, 16, 16], "side", north="front", up="top", down="steel"),
        *posts,
        E([0.5, 3, 0.5], [15.5, 13, 15.5], "glass", faces=("north", "south", "west", "east")),
        E([1.5, 3, 1.5], [14.5, 12, 14.5], "lamp", faces=("up", "north", "south", "west", "east"), glow=10),
        E([5, 16, 5], [11, 17, 11], "steel", faces=NO_D, up="top"),
        E([7, 17, 7], [9, 18.5, 9], "copper", faces=NO_D),
    ]


def spectrometer():
    beams = [E([7.75, 8, 8], [8.25, 8.5, 12.5], "fluid_" + c, faces=("up", "down", "west", "east"),
               glow=True, lit_only=True, rot=("y", a, [8, 8.25, 8]))
             for c, a in (("red", -22.5), ("green", 0), ("cyan", 22.5))]
    return [
        E([0, 0, 0], [16, 6, 16], "side", north="front", up="top"),
        E([11, 6, 6], [15, 10, 10], "side", faces=NO_D, up="top"),
        E([10, 7, 7], [11, 9, 9], "lamp", faces=("up", "down", "north", "south", "west"), glow=True),
        E([5, 6, 5], [11, 6.5, 11], "frame", faces=NO_D),
        E([6, 6.5, 6], [10, 11, 10], "accent", faces=NO_D, rot=("y", 45, [8, 8, 8])),
        E([1, 7, 6], [5, 9, 8], "frame", faces=NO_D),
        E([2, 6, 6.5], [3, 7, 7.5], "frame", faces=("north", "south", "west", "east")),
        E([0, 6, 13], [16, 14, 16], "side", faces=NO_D, north="front", up="top"),
        E([9.5, 8, 7.75], [10, 8.5, 8.25], "lamp", faces=("up", "down", "north", "south"),
          glow=True, lit_only=True),
        *beams,
    ]


def pen():
    posts = [E([x, 2, z], [x + 2, 14, z + 2], "frame", faces=("north", "south", "west", "east"))
             for x, z in ((0, 0), (14, 0), (0, 14), (14, 14))]
    return [
        E([0, 0, 0], [16, 2, 16], "frame", up="floor"),
        E([0, 14, 0], [16, 16, 16], "side", north="front", up="top", down="steel"),
        *posts,
        E([2, 2, 0.5], [14, 14, 1.5], "bars", faces=PANE_Z),
        E([14.5, 2, 2], [15.5, 14, 14], "bars", faces=PANE_X),
        E([0.5, 2, 2], [1.5, 14, 14], "bars", faces=PANE_X),
        E([2, 2, 14], [14, 14, 16], "side", faces=("north", "south")),
        E([3, 2, 11], [13, 4, 14], "copper", faces=NO_D, up="fluid_amber"),
        E([6, 13, 6], [10, 14, 10], "lamp", faces=("down", "north", "south", "west", "east"), glow=12),
    ]


def console():
    tilt = ("x", -22.5, [8, 12, 10.5])
    lamps = [E([x, 12, 4], [x + 2, 12.5, 6], "lamp", faces=("up", "north", "west", "east"),
               glow=10, rot=tilt) for x in (3, 7, 11)]
    return [
        E([0, 0, 0], [16, 9, 16], "side", north="front", up="steel"),
        E([1, 8, 1.5], [15, 12, 10.5], "frame", faces=("up", "west", "east"), up="top", rot=tilt),
        *lamps,
        E([0, 8, 10.5], [16, 16, 16], "side", north="front", up="top", faces=NO_D),
        E([2, 9.5, 10], [14, 15, 10.5], "screen", faces=("north",), glow=11),
        E([13, 16, 12], [14, 18, 13], "frame", faces=NO_D),
        E([12.5, 18, 11.5], [14.5, 19, 13.5], "lamp", faces=FACES, glow=8),
    ]


def lab_bench():
    return [
        E([0, 0, 0], [16, 1, 16], "frame"),
        E([1, 1, 1], [15, 11, 15], "side", faces=("north", "south", "west", "east")),
        E([0, 11, 0], [16, 13, 16], "steel", up="top"),
        E([2, 13, 2], [6, 16, 6], "glass", faces=NO_D),
        E([2.5, 13, 2.5], [5.5, 15, 5.5], "fluid_green", faces=NO_D),
        E([3.5, 16, 3.5], [4.5, 18, 4.5], "glass", faces=("north", "south", "west", "east")),
        E([8, 13, 9], [13, 13.5, 13], "frame", faces=NO_D),
        E([11, 13.5, 11], [12, 20, 12], "frame", faces=NO_D),
        E([8, 17, 10.5], [11, 18, 11.5], "frame", faces=("up", "down", "north", "south", "west")),
        E([7, 13.5, 9.5], [10, 16.5, 12.5], "glass", faces=NO_D),
        E([7.5, 13.5, 10], [9.5, 15, 12], "fluid_amber", faces=NO_D),
        E([8, 16.5, 10.5], [9, 18, 11.5], "glass", faces=("north", "south", "west", "east")),
        E([9, 13, 3], [14, 14, 6], "frame", faces=NO_D),
        tube(10, 3.5, 3.5, 4, y=14),
        tube(12, 3.5, 2.5, 0, y=14),
    ]


# Операционный стол из секций (GDD 4.17). Каждая функция — часть модели в
# «северной» ориентации: изголовье и сторона, о которой речь, — север (z = 0).
# Blockstate (multipart) поворачивает части по `y` к нужной стороне.
#
# Шаблон блока — `minecraft:mushroom_stem`: свойства north/east/south/west/
# up/down, `false` — за стороной соседняя секция, `true` — сторона открыта.
# Рама и полотно идут сплошь через весь ряд: на соединённой стороне их
# достраивает до границы блока полоса `join`. Борт (`rail`) — по открытым
# сторонам, ножка — в углу, где открыты обе стороны (на концах ряда), уголок
# борта (`cap`) — где открыта хотя бы одна, чтобы борт шёл без щелей от секции
# к секции. Ремни — на каждой секции, поперёк ряда.
#
# Лампа над изголовьем — на одном торце: изголовье однозначно задаёт сам
# стол. Пациента кладут головой к секции с меньшей координатой
# (`TableRow::yaw` в biolab_core): у ряда вдоль X — к западному концу, вдоль
# Z — к северному; одиночная секция считается рядом вдоль X. Так лампа и
# голова всегда на одном конце, и выбирать «оба торца или ни одного» не нужно.


def operating_table_core():
    """Основа каждой секции: дно, тумба, рама и полотно с отступом 1 от краёв.

    Столешница — на высоте полного блока (y = 16): моба кладут на верхнюю
    грань, отрисовка поворачивает его так, чтобы он лёг ровно на неё. Для
    игры секция — сплошной куб, поэтому под рамой тумба: сквозь открытые бока
    был бы виден провал на месте граней соседних блоков."""
    return [
        E([0, 0, 0], [16, 1, 16], "frame"),
        E([1, 1, 1], [15, 12, 15], "side", faces=("north", "south", "west", "east")),
        E([0, 12, 0], [16, 14, 16], "frame", up="steel"),
        E([1, 14, 1], [15, 16, 15], "pad", faces=NO_D),
    ]


def operating_table_rail():
    """Открытая сторона: стальной борт по краю рамы, ниже полотна. Торцов
    у борта нет: их закрывают уголки (`cap`), которые есть всегда, когда есть
    борт."""
    return [E([1, 14, 0], [15, 15, 1], "steel", faces=("up", "north"))]


def operating_table_join():
    """Соединённая сторона: тумба и полотно до границы блока, чтобы у соседней
    секции они продолжались без щели."""
    return [
        E([1, 1, 0], [15, 12, 1], "side", faces=("west", "east")),
        E([1, 14, 0], [15, 16, 1], "pad", faces=("up", "west", "east")),
    ]


def operating_table_cap():
    """Северо-западный уголок борта: замыкает борт на углу секции."""
    return [E([0, 14, 0], [1, 15, 1], "steel", faces=("up", "north", "west"))]


def operating_table_leg():
    """Северо-западная ножка: только на конце ряда, где открыты обе стороны."""
    return [E([0, 1, 0], [2, 12, 2], "steel", faces=("north", "south", "west", "east"))]


def operating_table_straps():
    """Два ремня поперёк стола (вдоль X — для ряда вдоль Z) с пряжками; через
    полотно и вниз по бокам."""
    out = []
    for z in (4.5, 10.5):
        out.append(E([0.5, 13.5, z], [15.5, 16.5, z + 1.5], "strap", faces=NO_D))
        out.append(E([10.5, 16.5, z - 0.25], [12.5, 16.75, z + 1.75], "steel", faces=NO_D))
    return out


def operating_table_head():
    """Изголовье (север): подушка и лампа на кронштейне. Светится всегда.
    Лампа выше 16 — не больше чем на 4."""
    return [
        E([3, 16, 1.5], [13, 17, 4], "pad", faces=NO_D),
        E([14, 14, 0.5], [15.5, 20, 2], "steel", faces=NO_D),
        E([8, 19, 0.75], [14, 20, 1.75], "steel", faces=("up", "down", "north", "south", "west")),
        E([7.5, 17.5, 0], [12.5, 19, 4], "accent", faces=("up", "north", "south", "west", "east")),
        E([8, 17.25, 0.5], [12, 17.5, 3.5], "lamp", faces=("down", "north", "south", "west", "east"),
          glow=True),
    ]


TABLE_PARTS = {
    "core": operating_table_core,
    "rail": operating_table_rail,
    "join": operating_table_join,
    "cap": operating_table_cap,
    "leg": operating_table_leg,
    "straps": operating_table_straps,
    "head": operating_table_head,
}
# Поворот части blockstate'ом: сторона модели «север» уходит на эту сторону.
TABLE_TURN = {"north": 0, "east": 90, "south": 180, "west": 270}
# Сторона «слева» от данной (её угол с ней — угол частей `cap` и `leg`): для
# севера — запад; поворот на 90° переводит запад в север, север в восток.
TABLE_LEFT = {"north": "west", "east": "north", "south": "east", "west": "south"}
TURN_FACE = {"north": "east", "east": "south", "south": "west", "west": "north",
             "up": "up", "down": "down"}


def turn_y(e, times):
    """Элемент, повёрнутый по часовой (сверху) на 90° `times` раз — как
    поворот `y` в blockstate: север уходит на восток."""
    for _ in range(times % 4):
        (x1, y1, z1), (x2, y2, z2) = e["f"], e["t"]
        e = dict(e, f=[16 - z2, y1, x1], t=[16 - z1, y2, x2],
                 faces=tuple(TURN_FACE[f] for f in e["faces"]),
                 over={TURN_FACE[k]: v for k, v in e["over"].items()},
                 uv={TURN_FACE[k]: v for k, v in e["uv"].items()})
    return e


def operating_table():
    """Одиночная секция целиком — иконка в инвентаре: основа, борта и уголки
    со всех сторон, четыре ножки, ремни поперёк, изголовье на севере."""
    out = list(operating_table_core())
    for deg in TABLE_TURN.values():
        for part in (operating_table_rail, operating_table_cap, operating_table_leg):
            out += [turn_y(e, deg // 90) for e in part()]
    return out + operating_table_straps() + operating_table_head()


def table_multipart():
    """Части по сторонам: условия на свойства гриба (`"true"` — открыта)."""
    def apply(name, deg=0):
        a = {"model": f"biolab:block/operating_table_{name}"}
        if deg:
            a["y"] = deg
        return a

    all_open = {"north": "true", "south": "true", "east": "true", "west": "true"}
    parts = [{"apply": apply("core")}]
    for side, deg in TABLE_TURN.items():
        left = TABLE_LEFT[side]
        parts.append({"when": {side: "true"}, "apply": apply("rail", deg)})
        parts.append({"when": {side: "false"}, "apply": apply("join", deg)})
        parts.append({"when": {"OR": [{side: "true"}, {left: "true"}]}, "apply": apply("cap", deg)})
        parts.append({"when": {side: "true", left: "true"}, "apply": apply("leg", deg)})
    # Ремни поперёк ряда: базовые идут вдоль X — это ряд вдоль Z; ряду вдоль X
    # и одиночной секции (она — ряд вдоль X) их поворачиваем на 90°. Секция,
    # соединённая по обеим осям (крест), считается рядом вдоль X — как в
    # `table_row` при равенстве.
    parts.append({"when": {"OR": [
        {"north": "false", "east": "true", "west": "true"},
        {"south": "false", "east": "true", "west": "true"},
    ]}, "apply": apply("straps")})
    parts.append({"when": {"OR": [{"east": "false"}, {"west": "false"}, dict(all_open)]},
                  "apply": apply("straps", 90)})
    # Изголовье — на конце ряда с меньшей координатой: северный конец ряда
    # вдоль Z, западный конец ряда вдоль X (и одиночная секция).
    parts.append({"when": {"north": "true", "south": "false"}, "apply": apply("head")})
    parts.append({"when": {"OR": [{"west": "true", "east": "false"}, dict(all_open)]},
                  "apply": apply("head", 270)})
    return parts


MACHINES = {
    "still": still, "incubator": incubator, "fermenter": fermenter, "centrifuge": centrifuge,
    "synthesizer": synthesizer, "reactor": reactor, "press": press, "extractor": extractor,
    "cloning_vat": cloning_vat, "spectrometer": spectrometer, "pen": pen, "console": console,
}

SHARED = {"steel": "machine_steel", "frame": "machine_frame", "copper": "machine_copper",
          "glass": "machine_glass", "vent": "machine_vent", "bars": "machine_bars",
          "screen": "machine_screen", "tubes": "machine_tubes",
          **{"fluid_" + k: "machine_fluid_" + k for k in FLUIDS if k != "violet"}}
OWN = ("side", "top", "front", "accent", "lamp")


def used_keys(elements):
    keys = set()
    for e in elements:
        for face in e["faces"]:
            keys.add(e["over"].get(face, e["tex"]))
    return keys


def texture_map(name, keys, own=OWN, extra=None):
    tex = {"particle": f"biolab:block/{name}_side"}
    for k in sorted(keys):
        if k in SHARED:
            tex[k] = "biolab:block/" + SHARED[k]
        elif k in own:
            tex[k] = f"biolab:block/{name}_{k}"
        elif extra and k in extra:
            tex[k] = extra[k]
        else:
            raise SystemExit(f"{name}: неизвестная текстура #{k}")
    return tex


def write_machine(name, build, with_on=()):
    """`with_on` — свои текстуры с индикаторами работы: горящая модель берёт их `_on`."""
    elements = build()
    base = [e for e in elements if not e["lit_only"]]
    extra = {"floor": f"biolab:block/{name}_floor"}
    tex = texture_map(name, used_keys(elements), extra=extra)
    dump_model(os.path.join(MODELS, name + ".json"), {
        "parent": "minecraft:block/block",
        "textures": {k: v for k, v in tex.items() if k in used_keys(base) or k == "particle"},
        "elements": [element_json(e, False) for e in base],
    })
    lit_tex = {k: v for k, v in tex.items() if k not in used_keys(base)}
    if "lamp" in tex:
        lit_tex["lamp"] = f"biolab:block/{name}_lamp_on"
    if "screen" in tex:
        lit_tex["screen"] = "biolab:block/machine_screen_on"
    for k in sorted(with_on):
        if k in tex:
            lit_tex[k] = f"biolab:block/{name}_{k}_on"
    dump_model(os.path.join(MODELS, name + "_on.json"), {
        "parent": f"biolab:block/{name}",
        "textures": lit_tex,
        "elements": [element_json(e, True) for e in elements],
    })
    variants = {}
    for facing, y in (("north", 0), ("east", 90), ("south", 180), ("west", 270)):
        for lit in ("false", "true"):
            v = {"model": f"biolab:block/{name}{'_on' if lit == 'true' else ''}"}
            if y:
                v["y"] = y
            variants[f"facing={facing},lit={lit}"] = v
    with open(os.path.join(STATES, name + ".json"), "w") as f:
        f.write('{ "variants": {\n')
        f.write(",\n".join(f'  "{k}": {json.dumps(v)}' for k, v in variants.items()))
        f.write(" } }\n")
    return len(base), len(elements)


def main():
    os.makedirs(TEX, exist_ok=True)
    shared_textures()
    lab_bench_textures()
    for name, build in MACHINES.items():
        keys = used_keys(build()) | {"side"}
        if "lamp" in keys:
            keys.add("lamp_on")
        with_on = machine_textures(name, PAINTERS[name], keys)
        n, n_on = write_machine(name, build, with_on)
        print(f"{name:13s} элементов {n:2d} (горящая {n_on:2d})")
    elements = lab_bench()
    tex = texture_map("lab_bench", used_keys(elements), own=(),
                      extra={"side": "biolab:block/lab_bench", "top": "biolab:block/lab_bench_top"})
    tex["particle"] = "biolab:block/lab_bench"
    dump_model(os.path.join(MODELS, "lab_bench.json"), {
        "parent": "minecraft:block/block", "textures": tex,
        "elements": [element_json(e, False) for e in elements]})
    print(f"{'lab_bench':13s} элементов {len(elements):2d}")
    write_operating_table()
    bone_texture()


def write_operating_table():
    """Операционный стол: шаблон — `mushroom_stem`, секции соединяются в ряд
    (см. комментарий к частям выше). Иконка — одиночная секция."""
    operating_table_textures()
    own = {k: f"biolab:block/operating_table_{k}" for k in ("side", "pad", "strap", "accent", "lamp")}
    models = {"operating_table": operating_table()}
    for name, build in TABLE_PARTS.items():
        models[f"operating_table_{name}"] = build()
    for name, elements in models.items():
        tex = texture_map("operating_table", used_keys(elements), own=(), extra=own)
        tex["particle"] = "biolab:block/operating_table_side"
        dump_model(os.path.join(MODELS, name + ".json"), {
            "parent": "minecraft:block/block", "textures": tex,
            "elements": [element_json(e, True) for e in elements]})
    with open(os.path.join(STATES, "operating_table.json"), "w") as f:
        rows = [json.dumps(p, ensure_ascii=False) for p in table_multipart()]
        f.write('{ "multipart": [\n' + ",\n".join("  " + r for r in rows) + "\n] }\n")
    with open(os.path.join(ASSETS, "items", "operating_table.json"), "w") as f:
        f.write('{ "model": { "type": "minecraft:model", "model": "biolab:block/operating_table" } }\n')
    print(f"{'operating_table':13s} элементов {len(models['operating_table']):2d} "
          f"(частей секции {len(TABLE_PARTS)})")


# ---------------------------------------------------------------- установки (GDD 4.18)
#
# Многоблочные установки: общие части (корпус, стекло, порты) и колонна.
# Блоки — сплошные кубы на ванильных родителях (`cube_all`, `cube_column`,
# `cube_bottom_top`, `orientable`): установка — это стена из блоков, детали
# снаружи куба выпирали бы из стены. Вид задают текстуры.
#
# «Склеенный» вид (`formed=true`, его ставит контроллер собранной установки):
# у корпуса и стекла пропадает рамка по краю — соседние блоки сливаются в одну
# стену; у тарелки и конденсатора пропадают фланцы на стыках — башня идёт
# одной трубой, а у тарелки появляется отвод фракции. Склеенная текстура не
# должна рисовать ничего у края, иначе шов останется (`seamless` ниже).
#
# Контроллер колонны рисует только себя: модель может выйти за блок на один
# блок в каждую сторону (`elements` от −16 до 32), а башня до 7 блоков —
# общий вид колонны всё равно собирают её секции. Рисунок контроллера на всю
# грань установки — дело кубических установок (биореактор, секвенатор).

CASING = (70, 80, 88)  # корпус: темнее машин, чтобы машины на его фоне читались


def casing_tex(seed, formed):
    """Плита корпуса. Несклеенная — фаска, тёмный шов по краю и заклёпки;
    склеенная — тот же металл без края (тянется через соседей) и бирюзовая
    жила посередине: установка под током."""
    img = canvas(CASING)
    grain(img, 4, seed)
    if formed:
        for y in (7, 8):
            for x in range(16):
                put(img, x, y, shade(CASING, -14))
        for x in range(1, 16, 4):
            put(img, x, 7, TEAL_DIM)
        return img
    bevel(img, 0, 0, 16, 16)
    outline(img, 0, 0, 16, 16, EDGE)
    for x, y in ((2, 2), (12, 2), (2, 12), (12, 12)):
        rivet(img, x, y)
    rect(img, 5, 7, 11, 9, shade(CASING, -14))
    put(img, 5, 7, TEAL_DIM)
    return img


def glass_tex(formed):
    """Стекло установки (шаблон стекла — вырезанные пиксели, полупрозрачности
    нет): несклеенное — стальная рамка и блики, склеенное — одни блики, рамки
    нет, стекло тянется сплошь."""
    img = canvas((0, 0, 0), 0)
    for y in range(16):
        for x in range(16):
            if x + y in (6, 7, 19):
                put(img, x, y, (226, 246, 250), 200)
    if not formed:
        outline(img, 0, 0, 16, 16, STEEL_DARK)
        outline(img, 1, 1, 15, 15, STEEL)
        for x, y in ((1, 1), (14, 1), (1, 14), (14, 14)):
            put(img, x, y, STEEL_LIGHT)
    return img


def port_front(kind):
    """Лицо порта: люк с горловиной и стрелкой — бирюзовая внутрь (вход),
    янтарная наружу (выход)."""
    img = casing_tex(stable("port_" + kind) & 0xFFF, False)
    c = TEAL if kind == "input" else HAZARD
    rect(img, 3, 3, 13, 13, STEEL_DARK)
    outline(img, 3, 3, 13, 13, EDGE)
    rect(img, 5, 5, 11, 11, (24, 30, 34))
    bevel(img, 3, 3, 13, 13, 20, -30)
    # Стрелка: вход — остриём вниз, в горловину; выход — остриём вверх.
    rows = [(7, 9), (7, 9), (5, 11), (6, 10), (7, 9)]
    if kind != "input":
        rows.reverse()
    for i, (a, b) in enumerate(rows):
        for x in range(a, b):
            put(img, x, 5 + i + (kind != "input"), c)
    for x in range(4, 12, 2):  # полоса-маркировка под люком
        put(img, x, 14, c)
        put(img, x + 1, 14, EDGE)
    return img


def column_textures():
    """Кипятильник, тарелка и конденсатор колонны."""
    # Кипятильник: топка с окном внизу лица, манометр, медный котёл по бокам.
    front = plate(STEEL, stable("column_front") & 0xFFF)
    dial(front, 10, 2)
    rect(front, 2, 2, 7, 5, COPPER)
    outline(front, 2, 2, 7, 5, COPPER_DARK)
    rect(front, 3, 8, 13, 14, EDGE)
    outline(front, 2, 7, 14, 15, STEEL_DARK)
    lit = [row[:] for row in front]
    for x in range(4, 12, 2):  # колосники
        put(front, x, 12, (60, 40, 30))
        put(front, x + 1, 13, (90, 50, 30))
    rng = random.Random(stable("column_fire"))
    for y in range(9, 14):
        for x in range(3, 13):
            heat = (y - 8) / 5
            c = mix((255, 120, 30), (255, 230, 140), rng.random() * (1 - heat))
            put(lit, x, y, c if rng.random() < 0.35 + 0.6 * heat else (200, 70, 20))
    write_png("column_front", front)
    write_png("column_front_on", lit)
    write_e("column_front_on", lit, bright_pixels(lit, 200))
    side = canvas(COPPER)
    grain(side, 5, stable("column_side") & 0xFFF)
    bevel(side, 0, 0, 16, 16)
    for y in (3, 12):  # обручи котла
        rect(side, 0, y, 16, y + 1, STEEL_DARK)
        rect(side, 0, y + 1, 16, y + 2, STEEL)
    for x in (2, 6, 10, 14):
        put(side, x, 3, STEEL_LIGHT)
    write_png("column_side", side)
    top = plate(STEEL_DARK, stable("column_top") & 0xFFF, amp=3)
    rect(top, 4, 4, 12, 12, COPPER_DARK)
    rect(top, 5, 5, 11, 11, (34, 26, 22))
    outline(top, 4, 4, 12, 12, EDGE)
    write_png("column_top", top)

    # Тарелка: стальная обечайка, смотровое окно с уровнем флегмы. Без
    # склейки — фланцы сверху и снизу (секция сама по себе); склеенная —
    # гладкая труба и медный отвод фракции справа.
    def tray(formed):
        img = canvas(STEEL)
        grain(img, 4, stable("column_tray") & 0xFFF)
        for x in (4, 11):  # продольные блики обечайки — через стык
            for y in range(16):
                put(img, x, y, shade(img[y][x], 18))
        rect(img, 5, 5, 11, 11, EDGE)
        rect(img, 6, 6, 10, 10, (200, 236, 240))
        rect(img, 6, 8, 10, 10, (190, 150, 70))
        put(img, 6, 6, (255, 255, 255))
        if formed:
            rect(img, 12, 7, 16, 9, COPPER)
            rect(img, 12, 9, 16, 10, COPPER_DARK)
            put(img, 12, 7, COPPER_LIGHT)
        else:
            for y in (0, 15):
                rect(img, 0, y, 16, y + 1, STEEL_DARK)
            for y in (1, 14):
                rect(img, 0, y, 16, y + 1, STEEL_LIGHT)
            for x in (2, 7, 12):
                put(img, x, 1, STEEL_DARK)
                put(img, x, 14, STEEL_DARK)
        return img

    write_png("column_tray_side", tray(False))
    write_png("column_tray_side_formed", tray(True))
    end = plate(STEEL_DARK, stable("column_tray_end") & 0xFFF, amp=3, rivets=False)
    outline(end, 1, 1, 15, 15, STEEL)
    for x, y in ((4, 4), (10, 4), (4, 10), (10, 10), (7, 7)):  # колпачки тарелки
        rect(end, x, y, x + 2, y + 2, COPPER)
        put(end, x, y, COPPER_LIGHT)
    write_png("column_tray_end", end)

    # Конденсатор: медный змеевик под кожухом, решётка охлаждения. Без склейки
    # — фланец снизу; склеенный — кожух уходит вниз в тарелку без шва.
    def condenser(formed):
        img = plate(STEEL_DARK, stable("column_condenser") & 0xFFF, amp=3, rivets=False)
        if formed:  # без фаски по низу и бокам: шов только сверху, где колонна кончается
            for y in range(16):
                for x in range(16):
                    img[y][x] = shade(STEEL_DARK, 0) + (255,)
            grain(img, 3, stable("column_condenser") & 0xFFF)
            rect(img, 0, 0, 16, 1, STEEL_LIGHT)
        for i, y in enumerate(range(3, 13, 3)):
            rect(img, 2, y, 14, y + 2, COPPER)
            rect(img, 2, y + 1, 14, y + 2, COPPER_DARK)
            put(img, 2 if i % 2 else 13, y + 2, COPPER)
        if not formed:
            rect(img, 0, 14, 16, 16, STEEL)
            rect(img, 0, 15, 16, 16, EDGE)
        return img

    write_png("column_condenser_side", condenser(False))
    write_png("column_condenser_side_formed", condenser(True))
    top = plate(STEEL, stable("column_condenser_top") & 0xFFF)
    vents(top, 3, 3, 13, 13)
    write_png("column_condenser_top", top)
    write_png("column_condenser_bottom", end)


# Нуклеотиды на экранах секвенатора: A, C, G, T.
BASES = ((90, 220, 120), (80, 150, 240), (240, 200, 70), (230, 80, 80))


def sequencer_textures():
    """Генный секвенатор (GDD 4.18, п. 3): консоль, стойка, монитор."""
    # Консоль: сверху экран с прочтением (у работающей — яркий и светится),
    # ниже клавиатура; бока — корпус с решёткой, крыша — плита.
    front = plate(STEEL_DARK, stable("sequencer_front") & 0xFFF, amp=3)
    rect(front, 2, 2, 14, 8, EDGE)
    rect(front, 3, 3, 13, 7, (14, 26, 30))
    for y in range(10, 14, 2):  # клавиши
        for x in range(2, 14, 2):
            put(front, x, y, STEEL_LIGHT)
            put(front, x, y + 1, STEEL)
    rect(front, 6, 14, 10, 15, TEAL_DIM)
    lit = [row[:] for row in front]
    rng = random.Random(stable("sequencer_read"))
    for x in range(3, 13):  # хроматограмма: пик на каждом основании
        c = BASES[rng.randrange(4)]
        top = 3 + rng.randrange(3)
        for y in range(top, 7):
            put(lit, x, y, c if y == top else shade(c, -70))
            if y == top:
                put(front, x, y, dim(c))
    write_png("sequencer_front", front)
    write_png("sequencer_front_on", lit)
    write_e("sequencer_front_on", lit, bright_pixels(lit, 200))
    side = casing_tex(stable("sequencer_side") & 0xFFF, False)
    vents(side, 4, 4, 12, 12)
    write_png("sequencer_side", side)
    top = plate(STEEL, stable("sequencer_top") & 0xFFF)
    rect(top, 4, 4, 12, 12, STEEL_DARK)
    outline(top, 4, 4, 12, 12, EDGE)
    write_png("sequencer_top", top)

    # Стойка: шкаф с рядами кассет. Несклеенная — рама по краю, лампы
    # погашены; склеенная — без рамы (шкафы сливаются в ряд), лампы горят.
    def rack(formed):
        img = canvas((44, 52, 58))
        grain(img, 3, stable("sequencer_rack") & 0xFFF)
        for y in range(2, 15, 3):
            rect(img, 0 if formed else 2, y, 16 if formed else 14, y + 2, (30, 36, 40))
            rect(img, 0 if formed else 2, y, 16 if formed else 14, y + 1, (58, 68, 74))
            for i, x in enumerate((4, 8, 12)):
                c = BASES[(y + i) % 4]
                put(img, x, y, c if formed else dim(c))
        if not formed:
            outline(img, 0, 0, 16, 16, EDGE)
            outline(img, 1, 1, 15, 15, STEEL_DARK)
        return img

    rack_on = rack(True)
    write_png("sequencer_rack_side", rack(False))
    write_png("sequencer_rack_side_formed", rack_on)
    write_e("sequencer_rack_side_formed", rack_on,
            [(x, y) for y in range(16) for x in range(16) if rack_on[y][x][:3] in BASES])
    end = plate(STEEL_DARK, stable("sequencer_rack_end") & 0xFFF, amp=3, rivets=False)
    vents(end, 3, 3, 13, 13)
    write_png("sequencer_rack_end", end)

    # Монитор: несклеенный — тёмный экран в раме; склеенный — рамы нет,
    # по всему экрану бегут прочтения цветными основаниями (светятся).
    def monitor(formed):
        img = canvas((14, 26, 30))
        if formed:
            rng = random.Random(stable("sequencer_monitor"))
            for y in range(1, 16, 2):
                for x in range(16):
                    if rng.random() < 0.7:
                        put(img, x, y, BASES[rng.randrange(4)])
            return img
        outline(img, 0, 0, 16, 16, STEEL_DARK)
        outline(img, 1, 1, 15, 15, EDGE)
        for x in range(3, 13, 3):
            put(img, x, 7, TEAL_DIM)
        return img

    screen = monitor(True)
    write_png("sequencer_monitor_side", monitor(False))
    write_png("sequencer_monitor_side_formed", screen)
    write_e("sequencer_monitor_side_formed", screen,
            [(x, y) for y in range(16) for x in range(16) if screen[y][x][:3] in BASES])
    cap = plate(STEEL, stable("sequencer_monitor_end") & 0xFFF, rivets=False)
    write_png("sequencer_monitor_end", cap)
    return rack_on, screen


def seamless(img):
    """Склеенная текстура не рисует шов по краю: каждый крайний пиксель —
    тот же материал, что внутри. Проверка грубая — нет ли у края рамки:
    крайние ряды не темнее середины больше чем на порог."""
    def lum(p):
        return sum(p[:3]) / 3 if p[3] else None

    inner = [lum(img[y][x]) for y in range(4, 12) for x in range(4, 12)]
    inner = [v for v in inner if v is not None]
    if not inner:
        return True
    mid = sum(inner) / len(inner)
    edge = [lum(img[y][x]) for y in (0, 15) for x in range(16)] + \
           [lum(img[y][x]) for x in (0, 15) for y in range(16)]
    edge = [v for v in edge if v is not None]
    return not edge or sum(edge) / len(edge) > mid - 30


def write_model(name, parent, textures):
    dump_model(os.path.join(MODELS, name + ".json"), {
        "parent": parent,
        "textures": {k: f"biolab:block/{v}" for k, v in textures.items()},
    })


def write_states(name, variants):
    with open(os.path.join(STATES, name + ".json"), "w") as f:
        f.write('{ "variants": {\n')
        f.write(",\n".join(f'  "{k}": {json.dumps(v)}' for k, v in variants.items()))
        f.write(" } }\n")


def write_item(name):
    with open(os.path.join(ASSETS, "items", name + ".json"), "w") as f:
        f.write('{ "model": { "type": "minecraft:model", "model": "biolab:block/%s" } }\n' % name)


def formed_states(name):
    """`formed` — свойство блока (`state_properties`), ставит контроллер."""
    write_states(name, {"formed=false": {"model": f"biolab:block/{name}"},
                        "formed=true": {"model": f"biolab:block/{name}_formed"}})


def facing_states(name, lit_model=None):
    """Шаблон печи: `facing` поворачивает модель, `lit` — горящая (если есть)."""
    variants = {}
    for facing, y in (("north", 0), ("east", 90), ("south", 180), ("west", 270)):
        for lit in ("false", "true"):
            model = lit_model if lit == "true" and lit_model else name
            v = {"model": f"biolab:block/{model}"}
            if y:
                v["y"] = y
            variants[f"facing={facing},lit={lit}"] = v
    write_states(name, variants)


def write_multiblock():
    casing = casing_tex(stable("casing") & 0xFFF, False)
    casing_formed = casing_tex(stable("casing") & 0xFFF, True)
    glass, glass_formed = glass_tex(False), glass_tex(True)
    for img, what in ((casing_formed, "корпус"), (glass_formed, "стекло")):
        if not seamless(img):
            raise SystemExit(f"склеенное {what}: шов по краю")
    write_png("casing", casing)
    write_png("casing_formed", casing_formed)
    write_png("casing_glass", glass)
    write_png("casing_glass_formed", glass_formed)
    for name in ("casing", "casing_glass"):
        write_model(name, "minecraft:block/cube_all", {"all": name, "particle": name})
        write_model(name + "_formed", "minecraft:block/cube_all",
                    {"all": name + "_formed", "particle": name})
        formed_states(name)
        write_item(name)

    for kind in ("input", "output"):
        name = f"{kind}_port"
        write_png(f"{name}_front", port_front(kind))
        write_model(name, "minecraft:block/orientable",
                    {"front": f"{name}_front", "side": "casing", "top": "casing", "particle": "casing"})
        facing_states(name)
        write_item(name)

    column_textures()
    write_model("column", "minecraft:block/orientable",
                {"front": "column_front", "side": "column_side", "top": "column_top", "particle": "column_side"})
    write_model("column_on", "minecraft:block/orientable",
                {"front": "column_front_on", "side": "column_side", "top": "column_top", "particle": "column_side"})
    facing_states("column", "column_on")
    write_item("column")
    for formed in (False, True):
        suffix = "_formed" if formed else ""
        write_model("column_tray" + suffix, "minecraft:block/cube_column",
                    {"side": "column_tray_side" + suffix, "end": "column_tray_end",
                     "particle": "column_tray_side"})
        write_model("column_condenser" + suffix, "minecraft:block/cube_bottom_top",
                    {"side": "column_condenser_side" + suffix, "top": "column_condenser_top",
                     "bottom": "column_condenser_bottom", "particle": "column_condenser_side"})
    for name in ("column_tray", "column_condenser"):
        formed_states(name)
        write_item(name)
    print("установки: корпус, стекло, порты, колонна (кипятильник, тарелка, конденсатор)")

    # Генный секвенатор: консоль на шаблоне печи, стойка и монитор склеиваются.
    rack_on, screen = sequencer_textures()
    for img, what in ((rack_on, "стойка"), (screen, "монитор")):
        if not seamless(img):
            raise SystemExit(f"склеенная {what} секвенатора: шов по краю")
    write_model("sequencer", "minecraft:block/orientable",
                {"front": "sequencer_front", "side": "sequencer_side", "top": "sequencer_top",
                 "particle": "sequencer_side"})
    write_model("sequencer_on", "minecraft:block/orientable",
                {"front": "sequencer_front_on", "side": "sequencer_side", "top": "sequencer_top",
                 "particle": "sequencer_side"})
    facing_states("sequencer", "sequencer_on")
    write_item("sequencer")
    for name in ("sequencer_rack", "sequencer_monitor"):
        for formed in (False, True):
            suffix = "_formed" if formed else ""
            write_model(name + suffix, "minecraft:block/cube_column",
                        {"side": f"{name}_side{suffix}", "end": f"{name}_end", "particle": f"{name}_side"})
        formed_states(name)
        write_item(name)
    print("секвенатор: консоль, стойка, монитор")


# ---------------------------------------------------------------- биореактор (GDD 4.18, п. 2)
#
# Контроллер биореактора — в середине передней стенки куба 3×3×3. Пока куб
# не собран, это обычный блок на шаблоне печи (`orientable`): люк со смотровым
# окном, за которым среда. Собранный (`formed=true`, своё свойство блока,
# ставит модуль `biolab_bioreactor`) рисует общий вид всей передней стенки:
# модель выходит за блок на целый блок в каждую сторону (`elements` от −16 до
# 32) — стальная рама по краю грани 3×3, трубы от рамы к люку и сам люк
# выступом. Внутренние клетки стенки модель не закрывает: порты и стекло
# там видны. Рисунок рамы — одна текстура 48×48 на всю грань
# (`bioreactor_panel`, пиксель текстуры = пиксель модели), каждая часть берёт
# из неё свой кусок (`panel_uv`). `tools/check_models.py` пускает за
# границы блока только этот блок (`OVERSIZE`).

MEDIUM = (96, 190, 70)  # среда за стеклом люка


def big_canvas(w, h, c):
    return [[(c[0], c[1], c[2], 255) for _ in range(w)] for _ in range(h)]


def big_put(img, x, y, c, a=255):
    if 0 <= y < len(img) and 0 <= x < len(img[0]):
        img[y][x] = (c[0], c[1], c[2], a)


def porthole(img, cx, cy, r, lit, seed, put_fn=put):
    """Круглое окно люка: обод, стекло со средой (тёмнее книзу), пузыри.
    Горящий (`lit`) — среда ярче и бурлит."""
    rng = random.Random(seed)
    for y in range(cy - r - 1, cy + r + 2):
        for x in range(cx - r - 1, cx + r + 2):
            d = math.hypot(x + 0.5 - cx, y + 0.5 - cy)
            if d <= r - 0.6:
                depth = (y - (cy - r)) / (2 * r)
                c = shade(MEDIUM, 30 if lit else 0)
                c = mix(c, shade(MEDIUM, -60), depth * 0.6)
                put_fn(img, x, y, c)
            elif d <= r + 0.4:
                put_fn(img, x, y, STEEL_LIGHT if y < cy else STEEL_DARK)
            elif d <= r + 1.2:
                put_fn(img, x, y, EDGE)
    # Блик на стекле и пузыри.
    put_fn(img, cx - r + 2, cy - r + 2, (220, 250, 220))
    for _ in range(6 if lit else 2):
        bx = cx + rng.randint(-r + 2, r - 2)
        by = cy + rng.randint(-r + 2, r - 2)
        if math.hypot(bx + 0.5 - cx, by + 0.5 - cy) < r - 1.5:
            put_fn(img, bx, by, (210, 250, 170) if lit else (150, 220, 120))


def bioreactor_textures():
    """Люк контроллера (несобранный), бока, верх и рисунок всей грани."""
    for lit in (False, True):
        front = plate(CASING, stable("bioreactor_front") & 0xFFF)
        porthole(front, 8, 7, 5, lit, stable("bioreactor_bubbles") + lit)
        for x in range(3, 13):  # бирюзовая маркировка под люком
            put(front, x, 14, TEAL if x % 3 else EDGE)
        write_png("bioreactor_front" + ("_on" if lit else ""), front)
    side = casing_tex(stable("bioreactor_side") & 0xFFF, False)
    rect(side, 0, 9, 16, 11, COPPER)  # труба среды вдоль бака
    rect(side, 0, 10, 16, 11, COPPER_DARK)
    for x in (3, 12):
        rect(side, x, 8, x + 1, 12, STEEL_DARK)  # хомуты
    write_png("bioreactor_side", side)
    top = plate(CASING, stable("bioreactor_top") & 0xFFF)
    rect(top, 4, 4, 12, 12, STEEL_DARK)
    vents(top, 5, 5, 11, 11)
    outline(top, 4, 4, 12, 12, EDGE)
    write_png("bioreactor_top", top)

    # Вся передняя грань 48×48: рама 4 пикселя, трубы, люк в середине.
    for lit in (False, True):
        img = big_canvas(48, 48, CASING)
        rng = random.Random(stable("bioreactor_panel"))
        for y in range(48):
            for x in range(48):
                d = rng.randint(-4, 4)
                r, g, b, a = img[y][x]
                img[y][x] = (clamp(r + d), clamp(g + d), clamp(b + d), a)
        # Рама: тёмная сталь с фаской, по низу — предупредительная полоса.
        for y in range(48):
            for x in range(48):
                if min(x, y, 47 - x, 47 - y) < 4:
                    c = STEEL_DARK
                    if y < 1 or x < 1:
                        c = STEEL
                    elif y > 46 or x > 46:
                        c = EDGE
                    elif min(x, y, 47 - x, 47 - y) == 3:
                        c = EDGE
                    big_put(img, x, y, c)
        for x in range(4, 44):
            big_put(img, x, 45, HAZARD if (x // 2) % 2 else EDGE)
            big_put(img, x, 46, HAZARD if ((x + 1) // 2) % 2 else EDGE)
        for x in range(18, 30):  # табличка установки сверху: бирюзовая жила
            big_put(img, x, 1, TEAL_DIM)
            big_put(img, x, 2, TEAL if x % 3 else TEAL_DIM)
        for x, y in ((1, 1), (45, 1), (1, 45), (45, 45), (22, 45), (1, 22), (45, 22)):
            big_put(img, x, y, STEEL_LIGHT)
            big_put(img, x + 1, y + 1, EDGE)
        # Трубы от рамы к люку на высоте модели y 7…9 (строки 23–24).
        for x in list(range(4, 16)) + list(range(32, 44)):
            big_put(img, x, 23, COPPER_LIGHT if x % 4 else COPPER)
            big_put(img, x, 24, COPPER_DARK)
        # Люк — середина 16×16 (строки и столбцы 16–31).
        for y in range(16, 32):
            for x in range(16, 32):
                big_put(img, x, y, shade(CASING, 6))
        for x in range(16, 32):
            big_put(img, x, 16, shade(CASING, 30))
            big_put(img, x, 31, EDGE)
        for y in range(16, 32):
            big_put(img, 16, y, shade(CASING, 18))
            big_put(img, 31, y, EDGE)
        porthole(img, 24, 24, 5, lit, stable("bioreactor_bubbles") + lit, big_put)
        for x, y in ((18, 18), (29, 18), (18, 29), (29, 29)):
            big_put(img, x, y, STEEL_LIGHT)
            big_put(img, x + 1, y + 1 if y + 1 < 31 else y, STEEL_DARK)
        write_png("bioreactor_panel" + ("_on" if lit else ""), img)


def panel_uv(x0, x1, y0, y1):
    """Кусок текстуры грани 48×48 под прямоугольник модели x0…x1, y0…y1 на
    северной грани: ваниль кладёт u = 16 − x, v = 16 − y, здесь всё сдвинуто
    на блок (грань от −16 до 32) и сжато втрое (48 пикселей — 16 единиц uv)."""
    return [round((32 - x1) / 3, 4), round((32 - y1) / 3, 4),
            round((32 - x0) / 3, 4), round((32 - y0) / 3, 4)]


def slab_faces(x0, x1, y0, y1):
    """Грани выступа перед северной гранью: лицо — свой кусок рисунка, кромки
    (верх, низ, бока) — тонкие полоски того же куска."""
    u0, v0, u1, v1 = panel_uv(x0, x1, y0, y1)
    thin_v = [u0, v0, u1, round(min(v0 + 1 / 3, 16), 4)]
    thin_u = [u0, v0, round(min(u0 + 1 / 3, 16), 4), v1]
    return {
        "down": {"uv": thin_v, "texture": "#panel"},
        "up": {"uv": thin_v, "texture": "#panel"},
        "north": {"uv": [u0, v0, u1, v1], "texture": "#panel"},
        "west": {"uv": thin_u, "texture": "#panel"},
        "east": {"uv": thin_u, "texture": "#panel"},
    }


def bioreactor_formed_elements():
    full = [0, 0, 16, 16]
    core = {"from": [0, 0, 0], "to": [16, 16, 16], "faces": {
        "down": {"uv": full, "texture": "#casing", "cullface": "down"},
        "up": {"uv": full, "texture": "#casing", "cullface": "up"},
        "north": {"uv": panel_uv(0, 16, 0, 16), "texture": "#panel", "cullface": "north"},
        "south": {"uv": full, "texture": "#casing", "cullface": "south"},
        "west": {"uv": full, "texture": "#casing", "cullface": "west"},
        "east": {"uv": full, "texture": "#casing", "cullface": "east"},
    }}
    out = [core]
    # Рама по краю грани 3×3, на полпикселя перед стенкой.
    for x0, x1, y0, y1 in ((-16, 32, 28, 32), (-16, 32, -16, -12), (-16, -12, -12, 28), (28, 32, -12, 28)):
        out.append({"from": [x0, y0, -0.5], "to": [x1, y1, 0], "faces": slab_faces(x0, x1, y0, y1)})
    # Трубы от рамы к люку и выступ люка.
    for x0, x1, y0, y1, z in ((-12, 0, 7, 9, -1), (16, 28, 7, 9, -1), (2, 14, 2, 14, -0.5)):
        out.append({"from": [x0, y0, z], "to": [x1, y1, 0], "faces": slab_faces(x0, x1, y0, y1)})
    return out


def write_bioreactor():
    bioreactor_textures()
    for lit in (False, True):
        suffix = "_on" if lit else ""
        write_model("bioreactor" + suffix, "minecraft:block/orientable",
                    {"front": "bioreactor_front" + suffix, "side": "bioreactor_side",
                     "top": "bioreactor_top", "particle": "bioreactor_side"})
        dump_model(os.path.join(MODELS, f"bioreactor_formed{suffix}.json"), {
            "parent": "minecraft:block/block",
            "textures": {"panel": f"biolab:block/bioreactor_panel{suffix}",
                         "casing": "biolab:block/casing_formed",
                         "particle": "biolab:block/bioreactor_side"},
            "elements": bioreactor_formed_elements()})
    variants = {}
    for facing, y in (("north", 0), ("east", 90), ("south", 180), ("west", 270)):
        for formed in ("false", "true"):
            for lit in ("false", "true"):
                model = "bioreactor" + ("_formed" if formed == "true" else "") + ("_on" if lit == "true" else "")
                v = {"model": f"biolab:block/{model}"}
                if y:
                    v["y"] = y
                variants[f"facing={facing},formed={formed},lit={lit}"] = v
    write_states("bioreactor", variants)
    write_item("bioreactor")
    print(f"биореактор: люк, общий вид грани 3×3 ({len(bioreactor_formed_elements())} элементов)")


# ---------------------------------------------------------------- карантинная камера (GDD 4.18 п. 4)
#
# Чистая панель — белая эмаль на стальном листе, без рамки установки: из неё
# строят комнату, а не машину. Фильтр — та же панель с решёткой вытяжки и
# лампой «камера держит» (горит при `lit=true`). Патрон фильтра — предмет:
# стальной стакан, шерсть и полоса угля.

ENAMEL = (222, 230, 230)


def panel_tex(seed):
    img = canvas(ENAMEL)
    grain(img, 3, seed)
    bevel(img, 0, 0, 16, 16, 12, -22)
    outline(img, 0, 0, 16, 16, shade(ENAMEL, -40))
    for x, y in ((1, 1), (14, 1), (1, 14), (14, 14)):  # винты
        put(img, x, y, STEEL)
    for x in range(3, 13):  # бирюзовая полоса «чистой зоны»
        put(img, x, 12, TEAL_DIM)
    return img


def quarantine_front(lit):
    img = panel_tex(stable("quarantine_front") & 0xFFF)
    rect(img, 2, 2, 14, 10, STEEL_DARK)
    outline(img, 2, 2, 14, 10, EDGE)
    vents(img, 3, 3, 13, 9)
    for x in range(4, 12, 3):  # белые волокна фильтра за решёткой
        put(img, x, 4, (236, 236, 228))
        put(img, x + 1, 6, (236, 236, 228))
    lamp = (90, 240, 150) if lit else (40, 80, 64)
    outline(img, 6, 10, 10, 14, EDGE)
    rect(img, 7, 11, 9, 13, lamp)
    return img


def cartridge_tex():
    img = canvas((0, 0, 0), 0)
    wool = (232, 232, 224)
    for y in range(3, 14):
        for x in range(4, 12):
            put(img, x, y, shade(wool, -12 if x in (4, 11) else 0))
    rect(img, 4, 7, 12, 10, (40, 40, 44))  # уголь
    for x in (5, 8, 10):
        put(img, x, 8, (70, 70, 76))
    for y in (2, 3, 13, 14):  # стальные крышки
        for x in range(4, 12):
            put(img, x, y, STEEL if y in (2, 14) else STEEL_LIGHT)
    rect(img, 7, 0, 9, 2, STEEL_DARK)  # горловина
    return img


def write_quarantine():
    write_png("clean_panel", panel_tex(stable("clean_panel") & 0xFFF))
    write_model("clean_panel", "minecraft:block/cube_all", {"all": "clean_panel", "particle": "clean_panel"})
    write_states("clean_panel", {"": {"model": "biolab:block/clean_panel"}})
    write_item("clean_panel")

    write_png("quarantine_front", quarantine_front(False))
    on = quarantine_front(True)
    write_png("quarantine_front_on", on)
    write_e("quarantine_front_on", on, [(x, y) for x in (7, 8) for y in (11, 12)])
    for model, front in (("quarantine", "quarantine_front"), ("quarantine_on", "quarantine_front_on")):
        write_model(model, "minecraft:block/orientable",
                    {"front": front, "side": "clean_panel", "top": "clean_panel", "particle": "clean_panel"})
    facing_states("quarantine", "quarantine_on")
    write_item("quarantine")

    write_png("filter_cartridge", cartridge_tex(), os.path.join(ASSETS, "textures", "item"))
    dump_model(os.path.join(ASSETS, "models", "item", "filter_cartridge.json"), {
        "parent": "minecraft:item/generated",
        "textures": {"layer0": "biolab:item/filter_cartridge"},
    })
    with open(os.path.join(ASSETS, "items", "filter_cartridge.json"), "w") as f:
        f.write('{ "model": { "type": "minecraft:model", "model": "biolab:item/filter_cartridge" } }\n')
    print("карантинная камера: фильтр, чистая панель, патрон")


# ---------------------------------------------------------------- криобанк (GDD 4.23 п. 9)
#
# Криобанк — белый лабораторный холодильник: сталь с инеем по кромке, спереди
# дверца с окошком изморози и голубое табло температуры. Термобокс — предмет:
# белый пенопластовый ящик с голубой крышкой и ручкой.

FROST = (196, 232, 244)
ICE_BLUE = (110, 190, 230)


def cryo_side(seed):
    img = canvas((214, 222, 226))
    grain(img, 3, seed)
    bevel(img, 0, 0, 16, 16, 12, -26)
    outline(img, 0, 0, 16, 16, shade((214, 222, 226), -48))
    for x in range(1, 15):  # иней по верхней кромке
        if (x * 7 + seed) % 3:
            put(img, x, 1, FROST)
    for x, y in ((1, 1), (14, 1), (1, 14), (14, 14)):
        put(img, x, y, STEEL)
    return img


def cryo_front():
    img = cryo_side(stable("cryobank_front") & 0xFFF)
    rect(img, 2, 2, 14, 15, (200, 210, 216))
    outline(img, 2, 2, 14, 15, EDGE)
    rect(img, 4, 4, 12, 9, (150, 196, 220))  # окошко с изморозью
    outline(img, 4, 4, 12, 9, STEEL_DARK)
    for x, y in ((5, 5), (7, 6), (9, 5), (10, 7), (6, 7)):
        put(img, x, y, FROST)
    rect(img, 5, 10, 11, 13, (18, 30, 40))  # табло
    for x in (6, 7, 9):
        put(img, x, 11, ICE_BLUE)
    put(img, 12, 11, STEEL_LIGHT)  # ручка
    put(img, 12, 12, STEEL_LIGHT)
    return img


def thermobox_tex():
    img = canvas((0, 0, 0), 0)
    foam = (236, 240, 236)
    rect(img, 2, 6, 14, 15, foam)
    outline(img, 2, 6, 14, 15, shade(foam, -60))
    rect(img, 1, 4, 15, 7, ICE_BLUE)  # крышка
    outline(img, 1, 4, 15, 7, shade(ICE_BLUE, -60))
    rect(img, 6, 2, 10, 3, STEEL_DARK)  # ручка
    put(img, 6, 3, STEEL_DARK)
    put(img, 9, 3, STEEL_DARK)
    for x in (5, 8, 11):  # снежинки на боку
        put(img, x, 10, FROST)
        put(img, x - 1, 11, FROST)
        put(img, x + 1, 11, FROST)
        put(img, x, 12, FROST)
    return img


def write_cryobank():
    write_png("cryobank_side", cryo_side(stable("cryobank_side") & 0xFFF))
    write_png("cryobank_front", cryo_front())
    write_model("cryobank", "minecraft:block/orientable",
                {"front": "cryobank_front", "side": "cryobank_side", "top": "cryobank_side",
                 "particle": "cryobank_side"})
    facing_states("cryobank")
    write_item("cryobank")

    write_png("thermobox", thermobox_tex(), os.path.join(ASSETS, "textures", "item"))
    dump_model(os.path.join(ASSETS, "models", "item", "thermobox.json"), {
        "parent": "minecraft:item/generated",
        "textures": {"layer0": "biolab:item/thermobox"},
    })
    with open(os.path.join(ASSETS, "items", "thermobox.json"), "w") as f:
        f.write('{ "model": { "type": "minecraft:model", "model": "biolab:item/thermobox" } }\n')
    print("криобанк и термобокс")


if __name__ == "__main__":
    main()
    write_multiblock()
    write_bioreactor()
    write_quarantine()
    write_cryobank()
