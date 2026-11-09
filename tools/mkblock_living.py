#!/usr/bin/env python3
"""Живые генераторы BioLab (M10, GDD 4.27 В): текстуры, модели, blockstates.

Запуск из корня репозитория:
    python3 tools/mkblock_living.py
    python3 tools/check_models.py

Блоки — на шаблоне печки (`facing`, `lit`), модель — ванильная
`minecraft:block/orientable` (куб: лицо, бока, верх), как у криобанка. Горящий
(`lit=true`, генератор работает) берёт модель `<блок>_on` со своим лицом.

Пишет (перезаписывает) в content/assets/biolab/:
- textures/block/<блок>_{front,front_on,side,top}.png — 16×16;
- models/block/<блок>.json и <блок>_on.json;
- blockstates/<блок>.json — `facing` поворачивает модель, `lit` выбирает _on.

Лица:
- микробный топливный элемент — окно с мутной бурой средой, два медных
  электрода, пузыри; горит — среда зеленеет, лампа бирюзовая;
- фотобиореактор — три стеклянные трубки с зелёной взвесью, верх стеклянный;
- глюкозный элемент — светлый корпус, окно с янтарным раствором, золотой
  электрод;
- электроцитная батарея — тёмный корпус, в окне столбик дисков-электроцитов,
  горит — голубая искра.
"""
import json
import os
import struct
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS = os.path.join(ROOT, "content", "assets", "biolab")
TEX = os.path.join(ASSETS, "textures", "block")
MODELS = os.path.join(ASSETS, "models", "block")
STATES = os.path.join(ASSETS, "blockstates")

EDGE = (22, 27, 31)
STEEL = (128, 140, 148)
STEEL_LIGHT = (176, 188, 196)
STEEL_DARK = (78, 88, 96)
GLASS = (14, 26, 30)
COPPER = (192, 108, 64)
COPPER_LIGHT = (232, 152, 100)
GOLD = (230, 190, 70)
TEAL = (63, 216, 196)
OFF = (42, 58, 52)


def clamp(v):
    return max(0, min(255, int(round(v))))


def shade(c, d):
    return tuple(clamp(v + d) for v in c[:3])


def write_png(name, img):
    h, w = len(img), len(img[0])
    raw = b"".join(b"\x00" + b"".join(bytes(p) for p in row) for row in img)

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    with open(os.path.join(TEX, name + ".png"), "wb") as f:
        f.write(data)


def canvas(base):
    return [[(*base, 255) for _ in range(16)] for _ in range(16)]


def put(img, x, y, c):
    if 0 <= x < 16 and 0 <= y < 16:
        img[y][x] = (*c[:3], 255)


def rect(img, x0, y0, x1, y1, c):
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            put(img, x, y, c)


def casing(body):
    """Стальной корпус: тёмный кант, фаска светлая сверху-слева, тёмная снизу-справа, заклёпки."""
    img = canvas(body)
    for i in range(16):
        put(img, i, 0, EDGE)
        put(img, i, 15, EDGE)
        put(img, 0, i, EDGE)
        put(img, 15, i, EDGE)
    for i in range(1, 15):
        put(img, i, 1, shade(body, 30))
        put(img, 1, i, shade(body, 30))
        put(img, i, 14, shade(body, -30))
        put(img, 14, i, shade(body, -30))
    for x, y in ((2, 2), (13, 2), (2, 13), (13, 13)):
        put(img, x, y, shade(body, -45))
    # Лёгкий шум листа, устойчивый от запуска к запуску.
    for y in range(2, 14):
        for x in range(2, 14):
            n = (x * 7 + y * 13 + x * y) % 9
            if n == 0:
                put(img, x, y, shade(body, -8))
            elif n == 4:
                put(img, x, y, shade(body, 6))
    return img


def window(img, x0, y0, x1, y1, fill):
    rect(img, x0 - 1, y0 - 1, x1 + 1, y1 + 1, EDGE)
    rect(img, x0, y0, x1, y1, fill)
    for x in range(x0, x1 + 1):
        put(img, x, y0, shade(fill, 35))


def lamp(img, on, colour=TEAL):
    rect(img, 11, 3, 12, 4, colour if on else OFF)


def top_of(body, vent=True):
    img = casing(body)
    if vent:
        for y in (5, 7, 9):
            rect(img, 4, y, 11, y, EDGE)
    return img


# ---------------------------------------------------------------- генераторы

def microbial(on):
    img = casing(STEEL)
    murk = (96, 128, 60) if on else (110, 92, 52)
    window(img, 3, 6, 12, 12, murk)
    for x in (5, 10):
        rect(img, x, 5, x, 12, COPPER)
        put(img, x, 5, COPPER_LIGHT)
    for x, y in ((7, 11), (8, 9), (7, 8), (11, 10)):
        put(img, x, y, shade(murk, 45 if on else 25))
    lamp(img, on)
    return img


def photo(on):
    img = casing(STEEL_LIGHT)
    algae = (80, 200, 90) if on else (60, 140, 70)
    for x in (3, 7, 11):
        rect(img, x - 1, 3, x + 2, 13, EDGE)
        rect(img, x, 4, x + 1, 12, algae)
        put(img, x, 4, shade(algae, 50))
        put(img, x + 1, 9, shade(algae, -30))
    return img


def photo_top():
    img = canvas(GLASS)
    for i in range(16):
        put(img, i, 0, STEEL)
        put(img, i, 15, STEEL)
        put(img, 0, i, STEEL)
        put(img, 15, i, STEEL)
    for y in range(1, 15):
        for x in range(1, 15):
            if (x + y) % 6 == 0:
                put(img, x, y, (40, 90, 70))
    return img


def glucose(on):
    img = casing((206, 210, 214))
    syrup = (236, 180, 70) if on else (190, 140, 60)
    window(img, 3, 6, 12, 12, syrup)
    rect(img, 7, 5, 8, 12, GOLD)
    put(img, 7, 5, (255, 230, 140))
    lamp(img, on, (255, 210, 110))
    return img


def electrocyte(on):
    body = (60, 66, 74)
    img = casing(body)
    window(img, 5, 3, 10, 13, (20, 22, 30))
    for i, y in enumerate(range(4, 13, 2)):
        disc = (220, 150, 170) if i % 2 == 0 else (200, 130, 160)
        rect(img, 6, y, 9, y, disc)
        put(img, 6, y + 1, (60, 40, 60))
        put(img, 9, y + 1, (60, 40, 60))
    if on:
        for x, y in ((11, 6), (12, 5), (12, 8), (11, 10)):
            put(img, x, y, (150, 220, 255))
    rect(img, 12, 12, 13, 13, (120, 200, 255) if on else OFF)
    return img


BLOCKS = {
    "microbial_cell": (microbial, lambda: casing(STEEL), lambda: top_of(STEEL)),
    "photobioreactor": (photo, lambda: casing(STEEL_LIGHT), photo_top),
    "glucose_cell": (glucose, lambda: casing((206, 210, 214)), lambda: top_of((206, 210, 214))),
    "electrocyte_battery": (electrocyte, lambda: casing((60, 66, 74)), lambda: top_of((60, 66, 74), vent=False)),
}


def model(name, front):
    return {
        "parent": "minecraft:block/orientable",
        "textures": {
            "front": f"biolab:block/{front}",
            "side": f"biolab:block/{name}_side",
            "top": f"biolab:block/{name}_top",
            "particle": f"biolab:block/{name}_side",
        },
    }


def blockstate(name):
    variants = {}
    for facing, y in (("north", 0), ("east", 90), ("south", 180), ("west", 270)):
        for lit in ("false", "true"):
            v = {"model": f"biolab:block/{name}" + ("_on" if lit == "true" else "")}
            if y:
                v["y"] = y
            variants[f"facing={facing},lit={lit}"] = v
    return {"variants": variants}


def dump(path, data):
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=1)
        f.write("\n")


def main():
    for name, (front, side, top) in BLOCKS.items():
        write_png(f"{name}_front", front(False))
        write_png(f"{name}_front_on", front(True))
        write_png(f"{name}_side", side())
        write_png(f"{name}_top", top())
        dump(os.path.join(MODELS, f"{name}.json"), model(name, f"{name}_front"))
        dump(os.path.join(MODELS, f"{name}_on.json"), model(name, f"{name}_front_on"))
        dump(os.path.join(STATES, f"{name}.json"), blockstate(name))
    print("живые генераторы:", ", ".join(BLOCKS))


if __name__ == "__main__":
    main()
