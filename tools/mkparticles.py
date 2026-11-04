#!/usr/bin/env python3
"""Свои частицы BioLab (GDD 3.9, «Частицы»): спрайты 8×8 и спрайт-сеты.

Запуск из корня репозитория:
    python3 tools/mkparticles.py

Пишет:
  content/assets/biolab/textures/particle/<имя>/<n>.png — кадры, RGBA без PIL;
  content/assets/biolab/particles/<имя>.json — `{"textures": ["biolab:<имя>/<n>", ...]}`.

Имена — те же, что в `native/biolab-core/mixins/biolab_core/particles.rs` (`ALL`);
что у каждой частицы из кода есть JSON и все его PNG, следит `tests/particles.rs`.

Почему так рисуем. Тип частицы мода в нашем пине (API 103) — ванильный
`SimpleParticleType`: билборд без оттенка, один случайный кадр из набора на всю
жизнь, без анимации по возрасту. Поэтому кадры — не фазы анимации, а варианты
(разные размер и форма), а цвет — в самой текстуре. Альфа только 0 или 255:
у ванильных частиц вырез по альфе, полупрозрачный пиксель дал бы рваный край.
Имена кадров — цифры: суффиксы `_n`, `_s`, `_e` атлас Horizon считает
картами нормалей, бликов и свечения.

Рисуем во всю клетку 8×8. Квад частицы мода — 0,2–0,4 блока (ванильный
`quadSize` 0,1–0,2 в обе стороны), и первые кадры в 2–4 пикселя выходили
точками в 0,05–0,1 блока: в игре их было не разглядеть.
"""
import json
import os
import struct
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS = os.path.join(ROOT, "content", "assets", "biolab")
SIZE = 8

# Имя частицы → (палитра, кадры). Кадр — 8 строк по 8 символов, «.» — пусто.
PARTICLES = {
    # Споры заражения: зелёные, как иконка «Заражения».
    "spore": (
        {"d": (52, 88, 28), "g": (120, 176, 52), "l": (200, 230, 110)},
        [
            ["...dd...",
             "..dggd..",
             ".dgllgd.",
             "dglllggd",
             "dggllggd",
             ".dgggdd.",
             "..dggd..",
             "...dd..."],
            ["........",
             "..d..d..",
             ".dggggd.",
             "..glgg..",
             "..gggg..",
             ".dggggd.",
             "..d..d..",
             "........"],
            [".gl.....",
             ".gg..gl.",
             ".....gg.",
             "..gl....",
             "..gg..gl",
             "......gg",
             "gl......",
             "gg......"],
        ],
    ),
    # Споры мутагена: фиолетовые искры, цвет эффекта «Мутагенез».
    "mutagen_spore": (
        {"d": (92, 30, 108), "v": (196, 82, 200), "l": (250, 180, 246)},
        [
            ["....l...",
             "....v...",
             "...vlv..",
             ".vvlllvv",
             "...vlv..",
             "....v...",
             "....l...",
             "........"],
            ["...dd...",
             "..dvvd..",
             ".dvllvd.",
             "dvllllvd",
             ".dvllvd.",
             "..dvvd..",
             "...dd...",
             "........"],
            [".l....l.",
             "lv....vl",
             "........",
             "...vl...",
             "...lv...",
             "........",
             "lv....vl",
             ".l....l."],
        ],
    ),
    # Кислотный дым: жёлто-зелёные клубы.
    "acid_fume": (
        {"d": (112, 140, 36), "a": (174, 210, 60), "l": (224, 244, 136)},
        [
            ["...aa...",
             "..aalaa.",
             ".aallla.",
             "aallllaa",
             "aalllaad",
             ".aaaaad.",
             "..addd..",
             "........"],
            ["........",
             "..aaa...",
             ".aalaa..",
             ".aalllaa",
             "aaallaad",
             ".aaaadd.",
             "...dd...",
             "........"],
            [".....aa.",
             "....ala.",
             "...aaa..",
             "..aal...",
             "..aa....",
             ".aal....",
             ".add....",
             "..d....."],
        ],
    ),
    # Пена брожения: светлые пузырьки с тёмным ободком.
    "foam": (
        {"o": (150, 112, 52), "f": (236, 210, 150), "w": (255, 250, 226)},
        [
            ["..oooo..",
             ".owwffo.",
             "owwffffo",
             "offffffo",
             "offffffo",
             "offffffo",
             ".offffo.",
             "..oooo.."],
            ["ooo.....",
             "owo.ooo.",
             "ooo.owo.",
             "....ooo.",
             ".ooo....",
             ".owo..oo",
             ".ooo..oo",
             "........"],
            ["........",
             "..ooo...",
             ".owffo..",
             ".offfo..",
             ".offfo..",
             "..ooo...",
             "........",
             "........"],
        ],
    ),
    # Пузыри раствора клонирующего чана: бирюзовые кольца, цвет экрана чана.
    "bubble": (
        {"t": (63, 216, 196), "w": (214, 255, 248)},
        [
            ["..tttt..",
             ".tw...t.",
             "tw.....t",
             "t......t",
             "t......t",
             "t......t",
             ".t....t.",
             "..tttt.."],
            ["........",
             "..ttt...",
             ".tw..t..",
             ".t...t..",
             ".t...t..",
             "..ttt...",
             "........",
             "........"],
            [".ttt....",
             "tw.t....",
             "t..t....",
             ".tt.....",
             "....ttt.",
             "...tw..t",
             "...t...t",
             "....ttt."],
        ],
    ),
    # Пар инкубатора: белые клочья.
    "steam": (
        {"s": (196, 204, 210), "m": (232, 238, 242), "w": (255, 255, 255)},
        [
            ["..mmm...",
             ".mwwwm..",
             "mwwwwwm.",
             "mwwwwwwm",
             "smwwwwms",
             ".smmmms.",
             "..ssss..",
             "........"],
            ["........",
             ".mm..mm.",
             "mwwmmwwm",
             "mwwwwwwm",
             "smwwwwms",
             ".ssmmss.",
             "........",
             "........"],
            ["....mm..",
             "...mwm..",
             "...mm...",
             "..mm....",
             "..mwm...",
             "...mm...",
             "..ms....",
             "..s....."],
        ],
    ),
    # «Сон»: буква Z сиреневым, цвет эффекта «Седация», с тенью снизу.
    "sleep": (
        {"k": (80, 60, 132), "z": (176, 150, 232), "l": (232, 222, 255)},
        [
            [".lzzzzz.",
             ".zzzzzzk",
             "....zzk.",
             "...zzk..",
             "..zzk...",
             ".zzk....",
             ".zzzzzz.",
             "..kkkkkk"],
            ["........",
             "..lzzz..",
             "....zk..",
             "...zk...",
             "..zk....",
             "..zzzz..",
             "...kkk..",
             "........"],
        ],
    ),
    # Кровь: тёмно-красные капли и брызги.
    "blood": (
        {"d": (84, 6, 8), "r": (158, 16, 20), "h": (222, 70, 64)},
        [
            ["...r....",
             "...r....",
             "..rrr...",
             "..rhrr..",
             ".rhhrr..",
             ".rrrrr..",
             "..rrd...",
             "...dd..."],
            ["........",
             "....r...",
             "...rr...",
             "...hrr..",
             "..rhrr..",
             "..rrrd..",
             "...dd...",
             "........"],
            ["r....r..",
             ".r..rr..",
             "..rrr...",
             ".rhhrrr.",
             "rrhrrr..",
             "..rrd.r.",
             ".r..d...",
             "r.....r."],
        ],
    ),
}


def png(path, rows, palette):
    def px(x, y):
        c = rows[y][x]
        return (0, 0, 0, 0) if c == "." else (*palette[c], 255)

    raw = b"".join(b"\x00" + b"".join(bytes(px(x, y)) for x in range(SIZE)) for y in range(SIZE))

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    with open(path, "wb") as f:
        f.write(data)


def check(name, palette, frames):
    for n, rows in enumerate(frames):
        assert len(rows) == SIZE, f"{name}/{n}: {len(rows)} строк"
        for row in rows:
            assert len(row) == SIZE, f"{name}/{n}: строка {row!r}"
            for c in row:
                assert c == "." or c in palette, f"{name}/{n}: нет цвета {c!r}"
        assert any(c != "." for row in rows for c in row), f"{name}/{n}: пустой кадр"


if __name__ == "__main__":
    os.makedirs(os.path.join(ASSETS, "particles"), exist_ok=True)
    for name, (palette, frames) in PARTICLES.items():
        check(name, palette, frames)
        folder = os.path.join(ASSETS, "textures", "particle", name)
        os.makedirs(folder, exist_ok=True)
        for n, rows in enumerate(frames):
            png(os.path.join(folder, f"{n}.png"), rows, palette)
        spec = {"textures": [f"biolab:{name}/{n}" for n in range(len(frames))]}
        with open(os.path.join(ASSETS, "particles", f"{name}.json"), "w") as f:
            json.dump(spec, f, ensure_ascii=False, indent=1)
            f.write("\n")
        print(f"готово: {name} ({len(frames)} кадр.)")
