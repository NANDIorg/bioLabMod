#!/usr/bin/env python3
"""Иконка эффекта «Паралич» (18×18), как у остальных наших эффектов (GDD 4.20, M5).

Иконку движок берёт из `assets/biolab/textures/mob_effect/paralysis.png`; без
файла рисует розово-чёрную клетку — поэтому картинка нужна (см. mkicon_infection.py).

Рисуем дротик кураре: тёмная игла с зелёной каплей яда на острие и оперение,
наискось, как у иконки ванильного отравления, — и цвет эффекта (землисто-зелёный
`0x5A6E3A` из `mob_effect/paralysis.json`) в капле.

Запуск из корня репозитория: python3 tools/mkicon_paralysis.py
"""

import os
import struct
import zlib

OUT = os.path.join(os.path.dirname(__file__), "..", "content", "assets", "biolab",
                   "textures", "mob_effect")

PALETTE = {
    ".": None,
    "k": (30, 34, 26),      # контур
    "s": (150, 160, 166),   # сталь иглы
    "l": (210, 218, 222),   # блик стали
    "g": (90, 110, 58),     # яд — цвет эффекта
    "y": (160, 190, 90),    # блик яда
    "f": (186, 64, 52),     # оперение
    "r": (120, 36, 30),     # тень оперения
}

# Дротик из правого верхнего угла в левый нижний: оперение сверху, капля внизу.
ART = [
    "..................",
    "............kkk...",
    "...........kfffk..",
    "..........kffrrk..",
    "..........kfrrk...",
    ".........kkrrk....",
    "........klsk......",
    ".......klsk.......",
    "......klsk........",
    ".....klsk.........",
    "....klsk..........",
    "...kgsk...........",
    "..kggk............",
    ".kgyggk...........",
    ".kggggk...........",
    "..kggk............",
    "...kk.............",
    "..................",
]


def png(name, rows):
    h, w = len(rows), len(rows[0])
    raw = b""
    for row in rows:
        raw += b"\x00"
        for ch in row:
            c = PALETTE[ch]
            raw += bytes((0, 0, 0, 0)) if c is None else bytes((*c, 255))

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    with open(os.path.join(OUT, name), "wb") as f:
        f.write(data)


if __name__ == "__main__":
    assert len(ART) == 18 and all(len(r) == 18 for r in ART), "иконка — 18×18"
    png("paralysis.png", ART)
    print("paralysis.png")
