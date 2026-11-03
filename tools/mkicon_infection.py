#!/usr/bin/env python3
"""Иконка эффекта «Заражение» (18×18), как у остальных наших эффектов.

До ноября 2026 иконки эффектов мода подкладывал миксин в крейт рендера: он
дописывал в атлас псевдоним с тем ключом, который ищет движок. Эффекту без
своего файла доставалась пустая рамка, и `infection.png` никто не рисовал.
Теперь иконки берутся прямо из `assets/<мод>/textures/mob_effect/<имя>.png`,
а на отсутствующий файл движок рисует розово-чёрную клетку — поэтому своя
картинка нужна каждому эффекту.

Рисуем микроб: тёмно-зелёное тело с ободком, светлое ядро и жгутик.
"""

import os
import struct
import zlib

OUT = os.path.join(os.path.dirname(__file__), "..", "content", "assets", "biolab",
                   "textures", "mob_effect")
SIZE = 18

BODY = (74, 124, 58)
BODY_DARK = (46, 82, 38)
RIM = (126, 176, 86)
CORE = (176, 206, 120)
TAIL = (96, 142, 64)


def png(name, w, h, px):
    raw = b"".join(b"\x00" + b"".join(bytes(px(x, y)) for x in range(w)) for y in range(h))

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    with open(os.path.join(OUT, name), "wb") as f:
        f.write(data)


def pixel(x, y):
    # Тело — овал чуть выше середины, чтобы снизу поместился жгутик.
    cx, cy = 8.5, 7.5
    dx, dy = (x - cx) / 6.0, (y - cy) / 5.2
    d = dx * dx + dy * dy
    # Жгутик: волна вниз-вправо от нижнего края тела.
    tail = {(9, 12), (10, 13), (10, 14), (11, 15), (12, 15), (13, 14)}
    if (x, y) in tail:
        return (*TAIL, 255)
    if d > 1.0:
        return (0, 0, 0, 0)
    if d > 0.74:
        return (*RIM, 255)
    # Ядро и две гранулы — чтобы читалось как клетка, а не как капля.
    if (x - 7) ** 2 + (y - 6) ** 2 <= 2:
        return (*CORE, 255)
    if (x, y) in {(11, 9), (6, 10)}:
        return (*BODY_DARK, 255)
    return (*BODY, 255)


if __name__ == "__main__":
    os.makedirs(OUT, exist_ok=True)
    png("infection.png", SIZE, SIZE, pixel)
    print("готово: infection.png")
