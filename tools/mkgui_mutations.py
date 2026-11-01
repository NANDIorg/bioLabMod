#!/usr/bin/env python3
"""Текстура наростов мутантов (GDD 4.19): RGBA PNG без зависимостей.

Запуск из корня репозитория:
    python3 tools/mkgui_mutations.py

Пишет content/assets/biolab/textures/gui/mutations/growths.png (128×64).
Лежит в textures/gui: только оттуда текстуры мода попадают в общий атлас, из
которого рисуются мобы (ключ `biolab:gui/mutations/growths`).

Раскладка — блоки материалов 32×16 по строкам слева направо, в том же
порядке, что `Material` в native/biolab-deep/mixins/biolab_deep/growths.rs:
хитин, кость, рог, плоть, перепонка, глаз, жабры, кислота, свечение. Кубы
берут из блока развёртку от его угла, поэтому почти все материалы — сплошной
узор без рисунка по граням. Исключение — глаз: куб 3×3×1, глаз нарисован на
его переде (клетки 1…3, 1…3 от угла блока) и заде (5…7, 1…3).

Плоть и перепонку игра красит оттенком мутанта (умножением), поэтому они
светлые и малонасыщенные; кость, рог, глаза и кислота — своим цветом;
свечение рисуется полной яркостью.
"""
import os
import random
import struct
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "content", "assets", "biolab", "textures", "gui", "mutations")

W, H = 128, 64
BW, BH = 32, 16
ORDER = ["chitin", "bone", "horn", "flesh", "membrane", "eye", "gill", "acid", "glow"]


def clamp(v):
    return max(0, min(255, int(round(v))))


def shade(c, d):
    return tuple(clamp(v + d) for v in c[:3])


def mix(a, b, t):
    return tuple(clamp(a[i] + (b[i] - a[i]) * t) for i in range(3))


def write_png(path, img):
    raw = b"".join(b"\x00" + b"".join(bytes(p) for p in row) for row in img)

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    with open(path, "wb") as f:
        f.write(data)


class Block:
    """Блок материала: рисуем в своих координатах 0…31 × 0…15."""

    def __init__(self, img, index):
        self.img = img
        self.ox = (index % 4) * BW
        self.oy = (index // 4) * BH

    def put(self, x, y, c):
        if 0 <= x < BW and 0 <= y < BH:
            self.img[self.oy + y][self.ox + x] = (c[0], c[1], c[2], 255)

    def get(self, x, y):
        return self.img[self.oy + y][self.ox + x]

    def fill(self, c):
        for y in range(BH):
            for x in range(BW):
                self.put(x, y, c)

    def noise(self, amount, seed):
        rng = random.Random(seed)
        for y in range(BH):
            for x in range(BW):
                self.put(x, y, shade(self.get(x, y), rng.randint(-amount, amount)))


def chitin(b):
    """Пластины панциря: тёмно-оливковые полосы с бликом по верхнему краю."""
    base = (78, 84, 52)
    b.fill(base)
    for y in range(BH):
        band = y % 4
        for x in range(BW):
            c = base
            if band == 0:
                c = shade(base, 34)  # блик ребра пластины
            elif band == 3:
                c = shade(base, -26)  # тень под следующей пластиной
            b.put(x, y, c)
    b.noise(6, 1901)


def bone(b):
    """Кость: светлая, с продольными волокнами и порами."""
    b.fill((226, 216, 190))
    rng = random.Random(1902)
    for x in range(BW):
        d = rng.randint(-10, 6)
        for y in range(BH):
            b.put(x, y, shade(b.get(x, y), d))
    for _ in range(26):
        b.put(rng.randrange(BW), rng.randrange(BH), (176, 158, 126))
    b.noise(4, 1903)


def horn(b):
    """Рог и когти: кератин с кольцами, тёмный."""
    light, dark = (128, 110, 88), (58, 48, 40)
    for y in range(BH):
        for x in range(BW):
            c = mix(light, dark, (x % 8) / 10)
            if y % 3 == 0:
                c = shade(c, -18)  # кольца роста
            b.put(x, y, c)
    b.noise(5, 1904)


def flesh(b):
    """Плоть: светлая розово-серая, с тёмными прожилками (красится оттенком)."""
    b.fill((222, 194, 188))
    rng = random.Random(1905)
    for _ in range(6):
        x, y = rng.randrange(BW), rng.randrange(BH)
        for _ in range(9):
            b.put(x, y, (170, 108, 112))
            x += rng.choice((-1, 0, 1))
            y += rng.choice((-1, 0, 1))
    b.noise(8, 1906)


def membrane(b):
    """Перепонка: тонкая кожа с ветвящимися жилками."""
    b.fill((214, 196, 194))
    rng = random.Random(1907)
    for x0 in range(2, BW, 6):
        x = x0
        for y in range(BH):
            b.put(x, y, (156, 112, 118))
            if rng.random() < 0.4:
                x += rng.choice((-1, 1))
    b.noise(5, 1908)


def eye(b):
    """Глаз: веко вокруг, белок, цветная радужка и зрачок в центре 3×3."""
    b.fill((196, 150, 140))
    b.noise(6, 1909)
    for ox, iris in ((1, (214, 160, 36)), (5, (120, 190, 60))):
        for y in range(3):
            for x in range(3):
                corner = x != 1 and y != 1
                c = (238, 232, 214) if corner else iris
                b.put(ox + x, 1 + y, c)
        b.put(ox + 1, 2, (18, 10, 10))  # зрачок


def gill(b):
    """Жабры: тёмно-красные щели с розовыми краями."""
    for y in range(BH):
        for x in range(BW):
            b.put(x, y, (118, 30, 42) if x % 2 == 0 else (206, 132, 136))
    for x in range(BW):
        b.put(x, 0, (220, 160, 160))
    b.noise(5, 1910)


def acid(b):
    """Кислота: ядовито-зелёные пузыри с бликами."""
    b.fill((104, 196, 52))
    rng = random.Random(1911)
    for y in range(BH):
        for x in range(BW):
            if (x + y) % 5 == 0:
                b.put(x, y, (70, 150, 36))
    for _ in range(18):
        b.put(rng.randrange(BW), rng.randrange(BH), (214, 255, 160))
    b.noise(6, 1912)


def glow(b):
    """Свечение: светло-зелёно-голубые пятна, ярче к центру клеток."""
    for y in range(BH):
        for x in range(BW):
            edge = x % 2 == 0 or y % 2 == 0
            b.put(x, y, (150, 240, 190) if edge else (226, 255, 214))
    b.noise(4, 1913)


PAINTERS = {
    "chitin": chitin,
    "bone": bone,
    "horn": horn,
    "flesh": flesh,
    "membrane": membrane,
    "eye": eye,
    "gill": gill,
    "acid": acid,
    "glow": glow,
}


def main():
    img = [[(0, 0, 0, 0) for _ in range(W)] for _ in range(H)]
    for i, name in enumerate(ORDER):
        PAINTERS[name](Block(img, i))
    os.makedirs(OUT, exist_ok=True)
    write_png(os.path.join(OUT, "growths.png"), img)


if __name__ == "__main__":
    main()
