#!/usr/bin/env python3
"""GUI-текстуры экранов машин BioLab (GDD 4.16): RGBA PNG без зависимостей.

Запуск из корня репозитория:
    python3 tools/mkgui_machine.py

Пишет в content/assets/biolab/textures/gui/machine/. Цвета те же, что у
запасных заливок в коде (константы SCREEN_* в runtime.rs), поэтому экран без
текстур выглядит так же, только без фактуры. Размеры спрайтов зашиты в
screen_draw.rs: панели 176×166 и 176×133, слоты 18×18, барабан 4 кадра по
20×20, клон 14×30 — меняя их здесь, меняйте и там.
"""
import math
import os
import random
import struct
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "content", "assets", "biolab", "textures", "gui", "machine")

EDGE = (16, 22, 25)
STEEL = (91, 102, 110)
STEEL_LIGHT = (154, 168, 177)
STEEL_DARK = (52, 61, 67)
TEAL = (63, 216, 196)
TEAL_DIM = (30, 106, 98)
SLOT = (44, 53, 59)
OUT_SLOT = (29, 58, 56)


def png(name, w, h, px):
    raw = b"".join(b"\x00" + b"".join(bytes(px(x, y)) for x in range(w)) for y in range(h))

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    with open(os.path.join(OUT, name), "wb") as f:
        f.write(data)


def clamp(v):
    return max(0, min(255, int(round(v))))


def shade(c, d):
    return tuple(clamp(v + d) for v in c)


def mix(a, b, t):
    return tuple(clamp(a[i] + (b[i] - a[i]) * t) for i in range(3))


def rgba(c, a=255):
    return (c[0], c[1], c[2], a)


def panel(name, w, h, inventory_y):
    """Стальная панель: шлифовка вдоль, фаска, бирюзовая подсветка под
    заголовком и по краю, тёмная плита под инвентарём, заклёпки."""
    rng = random.Random(w * 1000 + h)
    rows = [rng.randint(-4, 4) for _ in range(h)]
    grain = [[rng.randint(-2, 2) for _ in range(w)] for _ in range(h)]
    plate_top = inventory_y - 13
    rivets = {(3, 3), (w - 5, 3), (3, h - 5), (w - 5, h - 5)}

    def px(x, y):
        if x == 0 or y == 0 or x == w - 1 or y == h - 1:
            return rgba(EDGE)
        for rx, ry in rivets:
            if rx <= x < rx + 2 and ry <= y < ry + 2:
                return rgba(STEEL_LIGHT if (x, y) == (rx, ry) else STEEL_DARK)
        if y == 1 or x == 1:
            return rgba(STEEL_LIGHT)
        if y == h - 2 or x == w - 2:
            return rgba(STEEL_DARK)
        # Подсветка: тонкая бирюзовая линия на два пикселя внутри края.
        if (x == 2 or x == w - 3) and 2 <= y <= h - 3 or (y == 2 or y == h - 3) and 2 <= x <= w - 3:
            return rgba(mix(STEEL, TEAL, 0.28))
        # Полоса под заголовком: яркий отрезок слева, дальше приглушённая.
        if y == 15 and 4 <= x < w - 4:
            return rgba(TEAL if x < 28 else TEAL_DIM)
        if y == 16 and 4 <= x < w - 4:
            return rgba(mix(STEEL, TEAL, 0.18))
        base = STEEL
        if plate_top <= y < h - 4 and 4 <= x < w - 4:
            if y == plate_top:
                return rgba(TEAL_DIM)
            base = shade(STEEL, -12)
        return rgba(shade(base, rows[y] + grain[y][x]))

    png(name, w, h, px)


def slot(name, inner, glow):
    def px(x, y):
        if x == 17 or y == 17:
            return rgba(STEEL_LIGHT)
        if x == 0 or y == 0:
            return rgba(EDGE)
        if glow and y == 16:
            return rgba(TEAL)
        if glow and (x, y) in ((1, 1), (16, 1)):
            return rgba(TEAL_DIM)
        return rgba(shade(inner, (8 - y) // 3))

    png(name, 18, 18, px)


def drum():
    """Барабан центрифуги, четыре кадра поворота. Три окна симметричны через
    120°, кадры идут по 30°: последний переходит в первый без скачка."""

    def px(x, y):
        frame, fx = divmod(x, 20)
        dx, dy = fx - 9.5, y - 9.5
        r = math.hypot(dx, dy)
        if r > 9.6:
            return (0, 0, 0, 0)
        if r > 8.6:
            return rgba(EDGE)
        if r > 7.2:
            return rgba(STEEL_LIGHT if dy < 0 else STEEL)
        if r < 2.2:
            return rgba(TEAL)
        if r < 3.0:
            return rgba(EDGE)
        angle = math.atan2(dy, dx) - math.radians(frame * 30)
        for k in range(3):
            a = angle - k * 2 * math.pi / 3
            hx, hy = 5.0 * math.cos(a), 5.0 * math.sin(a)
            if math.hypot(hx - 5.0, hy) < 1.6:
                return rgba(TEAL_DIM)
        return rgba(shade(STEEL_DARK, int(-dy // 3)))

    png("drum.png", 80, 20, px)


def clone():
    """Силуэт клона 14×30 по тем же частям, что `clone_parts` в runtime.rs."""
    parts = [
        (3, 0, 8, 8),     # голова
        (3, 8, 8, 12),    # тело
        (0, 8, 3, 11),    # руки
        (11, 8, 3, 11),
        (3, 20, 3, 10),   # ноги
        (8, 20, 3, 10),
    ]
    body = (168, 224, 184)

    def inside(x, y):
        return any(px <= x < px + w and py <= y < py + h for px, py, w, h in parts)

    def px(x, y):
        if not inside(x, y):
            return (0, 0, 0, 0)
        # Контур темнее, правый бок в тени, глаза на голове.
        edge = not all(inside(x + dx, y + dy) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
        if (x, y) in ((5, 3), (8, 3)):
            return rgba((30, 74, 54))
        if edge:
            return rgba(shade(body, -60))
        return rgba(shade(body, -18 if x >= 8 else 0))

    png("clone.png", 14, 30, px)


def main():
    os.makedirs(OUT, exist_ok=True)
    panel("panel.png", 176, 166, 84)
    panel("panel_short.png", 176, 133, 51)
    slot("slot.png", SLOT, False)
    slot("slot_out.png", OUT_SLOT, True)
    drum()
    clone()


if __name__ == "__main__":
    main()
