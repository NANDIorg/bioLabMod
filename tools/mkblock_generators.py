#!/usr/bin/env python3
"""Генераторы тиров 3–5 (M10, GDD 4.27 Б2): текстуры, модели, blockstates.

Запуск из корня репозитория:
    python3 tools/mkblock_generators.py
    python3 tools/check_models.py

Блоки на шаблоне печки, модель — куб `minecraft:block/orientable` (как у
криобанка): перед с лампой работы, бока и верх. Горящий (`lit=true`) —
модель `<блок>_on` с горящей лампой на переде. Кисти и PNG — из
`mkblock_machines.py` (тот же стиль: сталь, медь, бирюзовая метка).

Пишет (перезаписывает) в content/assets/biolab/:
- textures/block/<блок>_{front,front_on,side,top}.png;
- models/block/<блок>.json, <блок>_on.json;
- blockstates/<блок>.json — `facing` поворачивает, `lit` выбирает _on.
"""
import importlib.util
import os
import sys

sys.dont_write_bytecode = True  # кисти грузим из соседнего файла — без __pycache__
HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("mkblock_machines", os.path.join(HERE, "mkblock_machines.py"))
mk = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mk)

canvas, put, rect, grain, bevel, outline, plate = mk.canvas, mk.put, mk.rect, mk.grain, mk.bevel, mk.outline, mk.plate
shade, mix, stable, write_png = mk.shade, mk.mix, mk.stable, mk.write_png
EDGE, STEEL, STEEL_DARK, STEEL_LIGHT, FRAME = mk.EDGE, mk.STEEL, mk.STEEL_DARK, mk.STEEL_LIGHT, mk.FRAME
COPPER, COPPER_DARK, COPPER_LIGHT, HAZARD, TEAL = mk.COPPER, mk.COPPER_DARK, mk.COPPER_LIGHT, mk.HAZARD, mk.TEAL

CELL = (34, 58, 112)       # кремниевый элемент
CELL_LIGHT = (70, 110, 176)
OBSIDIAN = (34, 26, 46)
MAGMA = (240, 120, 30)
RTG_GREEN = (124, 240, 60)


def lamp(img, x0, y0, x1, y1, c, on):
    """Лампа работы: погасшая — тусклая, горящая — яркая с белым ядром."""
    if on:
        rect(img, x0, y0, x1, y1, c)
        rect(img, x0 + 1, y0, x1 - 1, y1, mix(c, (255, 255, 255), 0.5))
    else:
        rect(img, x0, y0, x1, y1, mix(c, (20, 24, 28), 0.7))
    outline(img, x0 - 1, y0 - 1, x1 + 1, y1 + 1, EDGE)


def label(img):
    """Бирюзовая метка BioLab в нижнем углу передней грани."""
    put(img, 2, 13, TEAL)
    put(img, 3, 13, shade(TEAL, -60))


# ---------------------------------------------------------------- солнечная панель

def solar(on):
    top = canvas(CELL)
    grain(top, 6, stable("solar_top") & 0xFFF)
    for i in (0, 5, 10, 15):  # серебряная сетка проводников
        rect(top, i, 0, i + 1, 16, STEEL_LIGHT)
        rect(top, 0, i, 16, i + 1, STEEL_LIGHT)
    for x, y in ((2, 2), (7, 7), (12, 12), (12, 2), (2, 12)):  # блики
        put(top, x, y, CELL_LIGHT)
        put(top, x + 1, y, CELL_LIGHT)
    side = plate(STEEL, stable("solar_side") & 0xFFF)
    rect(side, 0, 0, 16, 3, CELL)
    rect(side, 0, 3, 16, 4, STEEL_LIGHT)
    front = [row[:] for row in side]
    rect(front, 4, 7, 12, 11, STEEL_DARK)  # инвертор
    outline(front, 4, 7, 12, 11, EDGE)
    lamp(front, 6, 8, 10, 10, (255, 210, 80), on)
    label(front)
    return front, side, top


# ---------------------------------------------------------------- двигатель

def engine(on):
    side = plate(STEEL_DARK, stable("engine_side") & 0xFFF)
    for x in range(2, 14, 3):  # медные рёбра охлаждения
        rect(side, x, 3, x + 2, 13, COPPER)
        rect(side, x + 1, 3, x + 2, 13, COPPER_DARK)
    for x in range(0, 16):
        put(side, x, 14, HAZARD if x % 4 < 2 else EDGE)
    top = plate(STEEL, stable("engine_top") & 0xFFF)
    rect(top, 9, 9, 14, 14, EDGE)  # выхлопная труба
    outline(top, 9, 9, 14, 14, STEEL_LIGHT)
    rect(top, 2, 2, 7, 7, COPPER)
    outline(top, 2, 2, 7, 7, COPPER_DARK)
    front = plate(STEEL, stable("engine_front") & 0xFFF)
    rect(front, 2, 2, 14, 8, EDGE)  # решётка радиатора
    for y in (3, 5, 7):
        rect(front, 3, y, 13, y + 1, STEEL_DARK)
    lamp(front, 6, 10, 10, 12, (255, 150, 40), on)
    label(front)
    return front, side, top


# ---------------------------------------------------------------- геотермальный съёмник

def geothermal(on):
    side = canvas(OBSIDIAN)
    grain(side, 5, stable("geo_side") & 0xFFF)
    bevel(side, 0, 0, 16, 16)
    for x in (3, 12):  # медные трубы теплообменника
        rect(side, x, 0, x + 2, 16, COPPER)
        rect(side, x + 1, 0, x + 2, 16, COPPER_DARK)
    rect(side, 0, 14, 16, 16, shade(MAGMA, -120))  # снизу жар
    top = plate(STEEL, stable("geo_top") & 0xFFF)
    rect(top, 5, 5, 11, 11, COPPER)
    outline(top, 5, 5, 11, 11, COPPER_DARK)
    front = [row[:] for row in side]
    rect(front, 5, 3, 11, 11, EDGE)  # смотровое окно в камеру
    for y in range(4, 10, 2):
        rect(front, 6, y, 10, y + 1, MAGMA if on else shade(MAGMA, -140))
    lamp(front, 6, 12, 10, 13, (255, 120, 40), on)
    return front, side, top


# ---------------------------------------------------------------- изотопная батарея

def rtg(on):
    side = canvas(FRAME)
    grain(side, 4, stable("rtg_side") & 0xFFF)
    for x in range(1, 16, 3):  # радиатор: рёбра по всей высоте
        rect(side, x, 0, x + 1, 16, STEEL_LIGHT)
        rect(side, x + 1, 0, x + 2, 16, STEEL_DARK)
    top = plate(STEEL_DARK, stable("rtg_top") & 0xFFF)
    rect(top, 4, 4, 12, 12, HAZARD)
    for x, y in ((7, 7), (8, 7), (7, 8), (8, 8), (5, 5), (10, 5), (7, 10), (8, 10), (5, 6), (10, 6)):
        put(top, x, y, EDGE)  # знак радиации
    front = plate(STEEL, stable("rtg_front") & 0xFFF)
    rect(front, 3, 2, 13, 10, HAZARD)
    outline(front, 3, 2, 13, 10, EDGE)
    for x, y in ((7, 5), (8, 5), (7, 6), (8, 6), (5, 3), (10, 3), (5, 4), (10, 4), (7, 8), (8, 8)):
        put(front, x, y, EDGE)
    lamp(front, 6, 12, 10, 14, RTG_GREEN, on)
    label(front)
    return front, side, top


GENERATORS = {"solar_panel": solar, "engine_generator": engine, "geothermal_tap": geothermal, "rtg": rtg}


def main():
    for name, paint in GENERATORS.items():
        front, side, top = paint(False)
        front_on, _, _ = paint(True)
        write_png(f"{name}_front", front)
        write_png(f"{name}_front_on", front_on)
        write_png(f"{name}_side", side)
        write_png(f"{name}_top", top)
        tex = {"front": f"{name}_front", "side": f"{name}_side", "top": f"{name}_top",
               "particle": f"{name}_side"}
        mk.write_model(name, "minecraft:block/orientable", tex)
        mk.write_model(f"{name}_on", "minecraft:block/orientable", {**tex, "front": f"{name}_front_on"})
        mk.facing_states(name, f"{name}_on")
        print(f"{name}: текстуры, модели, состояния")


if __name__ == "__main__":
    main()
