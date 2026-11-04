#!/usr/bin/env python3
"""Свои звуки BioLab: синтез в WAV и перегонка в `.ogg` (GDD 4.15, «Звуки»).

Запуск из корня репозитория: `python3 tools/mksounds.py`. Пишет
`content/assets/biolab/sounds/*.ogg` и `content/assets/biolab/sounds.json`.

Почему синтез, а не записи: звуков нужно немного и коротких, а у записи
пришлось бы держать лицензию и исходник. Здесь исходник — этот файл: поменять
звук значит поправить функцию и перезапустить скрипт.

Воспроизводимость: шум берётся из `random.Random` с фиксированным зерном у
каждого звука (своё зерно — чтобы правка одного звука не сдвигала остальные),
а `ffmpeg` зовём с `bitexact`: иначе ogg-муксер ставит потоку случайный серийный
номер и пишет версию кодека, и каждый прогон давал бы новые байты. WAV —
промежуточный файл, живёт во временной папке и в репозиторий не попадает.

Громкость в файле — «полная»: тише или громче звук делает таблица в
`runtime.rs` (Sound.volume), как и с ванильными. Поэтому каждый звук
нормализуем к своему пику, а ровный гул — ниже щелчков: при равном пике гул
воспринимается намного громче короткого удара.

Субтитры — ключи `subtitles.biolab.*`, их текст лежит в `lang/ru_ru.json` и
`lang/en_us.json` (руками: у остальных строк перевода там тоже ручной текст).
"""

import json
import math
import os
import random
import shutil
import struct
import subprocess
import sys
import tempfile
import wave

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
ASSETS = os.path.join(ROOT, "content", "assets", "biolab")
SOUNDS_DIR = os.path.join(ASSETS, "sounds")
FFMPEG = "/opt/homebrew/bin/ffmpeg"

# Шипящим звукам (аэрозоль, скальпель) нужны верха выше 11 кГц — для них
# 44,1 кГц. Гулу и бульканью хватает 22,05 кГц: файл вдвое меньше.
HI = 44100
LO = 22050


# ---- Кирпичики ----

def env_ar(t, attack, release, length):
    """Огибающая «атака — плато — спад»: линейный подъём, косинусный спад.
    Косинус, а не линия, — иначе конец звука слышен как ступенька."""
    if t < attack:
        return t / attack
    if t > length - release:
        x = (length - t) / release
        return max(0.0, 0.5 - 0.5 * math.cos(math.pi * x))
    return 1.0


def decay(t, tau):
    """Экспоненциальное затухание удара."""
    return math.exp(-t / tau) if t >= 0 else 0.0


class OnePole:
    """Однополюсный фильтр нижних частот; `hp()` — верхних (вход минус НЧ)."""

    def __init__(self, fs, cutoff):
        self.fs = fs
        self.y = 0.0
        self.set(cutoff)

    def set(self, cutoff):
        self.a = 1.0 - math.exp(-2.0 * math.pi * cutoff / self.fs)

    def lp(self, x):
        self.y += self.a * (x - self.y)
        return self.y

    def hp(self, x):
        return x - self.lp(x)


class Band:
    """Полосовой биквад (RBJ, постоянное усиление пика). Центр можно
    двигать на ходу — так «едет» свист скальпеля и ветер раскрутки."""

    def __init__(self, fs, freq, q):
        self.fs = fs
        self.q = q
        self.x1 = self.x2 = self.y1 = self.y2 = 0.0
        self.set(freq)

    def set(self, freq):
        w = 2.0 * math.pi * min(freq, self.fs * 0.45) / self.fs
        alpha = math.sin(w) / (2.0 * self.q)
        a0 = 1.0 + alpha
        self.b0 = alpha / a0
        self.b2 = -alpha / a0
        self.a1 = -2.0 * math.cos(w) / a0
        self.a2 = (1.0 - alpha) / a0

    def __call__(self, x):
        y = self.b0 * x + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2
        self.x2, self.x1 = self.x1, x
        self.y2, self.y1 = self.y1, y
        return y


class Osc:
    """Синус с накоплением фазы: частоту можно менять плавно, без щелчков
    (у `sin(2πft)` при смене `f` фаза прыгает)."""

    def __init__(self, fs):
        self.fs = fs
        self.phase = 0.0

    def __call__(self, freq):
        self.phase += 2.0 * math.pi * freq / self.fs
        return math.sin(self.phase)


def finish(samples, fs, peak):
    """Пик к `peak`, 3 мс тишины в начале и спад 4 мс в конце.

    Начало не плавим: у щелчков (поршень, дозиметр, хлопок дротика) весь звук
    — первые миллисекунды, и плавный вход съедал бы его (так и вышло у
    дозиметра: −23 дБ вместо −2). Вместо этого звук начинается после короткой
    тишины — она же защищает удар от первого блока кодера. Гул и шум
    входят своей огибающей атаки."""
    top = max(abs(s) for s in samples) or 1.0
    k = peak / top
    edge = int(0.004 * fs)
    n = len(samples)
    out = [0.0] * int(0.003 * fs)
    for i, s in enumerate(samples):
        out.append(s * k * min(1.0, (n - 1 - i) / edge))
    return out


# ---- Машины ----

def rotor(fs, seed, length, freq_at, gain_at):
    """Ротор центрифуги: основной тон оборотов с гармониками, «вув» от
    дисбаланса (амплитуда качается с частотой вращения) и ветер в корпусе —
    полосовой шум, который поднимается вместе с оборотами."""
    rnd = random.Random(seed)
    n = int(length * fs)
    o1, o2, o3, wob = Osc(fs), Osc(fs), Osc(fs), Osc(fs)
    wind = Band(fs, 400, 1.2)
    out = []
    for i in range(n):
        t = i / fs
        f = freq_at(t)
        g = gain_at(t)
        tone = 0.55 * o1(f) + 0.3 * o2(2 * f) + 0.12 * o3(3.01 * f)
        # Дисбаланс: раз за оборот, но не быстрее 12 Гц — дальше это уже не
        # «вув», а дребезг.
        tone *= 0.85 + 0.15 * wob(min(f / 18.0, 12.0))
        wind.set(300 + 6 * f)
        air = wind(rnd.uniform(-1, 1)) * 0.35
        out.append(g * (tone + air * (0.3 + f / 400.0)))
    return out


def centrifuge_start():
    # Раскрутка: обороты выходят на рабочие по экспоненте, как у мотора.
    fs, length = LO, 1.6
    s = rotor(fs, 11, length,
              lambda t: 35 + 185 * (1 - math.exp(-t / 0.55)),
              lambda t: min(1.0, 0.15 + t / 0.9) * env_ar(t, 0.05, 0.25, length))
    return fs, finish(s, fs, 0.8)


def centrifuge_work():
    # Ровный рабочий гул; лёгкое плавание оборотов, чтобы не звучал как
    # генератор тона. Длинные края: звук повторяется раз в 4 с.
    fs, length = LO, 2.0
    s = rotor(fs, 12, length,
              lambda t: 220 * (1 + 0.004 * math.sin(2 * math.pi * 0.7 * t)),
              lambda t: env_ar(t, 0.35, 0.5, length))
    return fs, finish(s, fs, 0.6)


def centrifuge_done():
    # Выбег: обороты падают, а в конце — щелчок защёлки крышки.
    fs, length = LO, 1.4
    s = rotor(fs, 13, length,
              lambda t: 25 + 195 * math.exp(-t / 0.4),
              lambda t: decay(t, 0.55) * env_ar(t, 0.02, 0.2, length))
    rnd = random.Random(14)
    at = int(1.15 * fs)
    click = Band(fs, 2500, 3.0)
    for i in range(int(0.03 * fs)):
        s[at + i] += 0.9 * click(rnd.uniform(-1, 1)) * decay(i / fs, 0.005)
    return fs, finish(s, fs, 0.8)


def fermenter_bubble():
    # Брожение: пузыри всплывают и лопаются. Пузырь в жидкости звучит
    # затухающим синусом, у которого частота растёт (Миннарт): так он и
    # «булькает», а не пищит. Под пузырями — тихий шум жидкости.
    fs, length = LO, 1.5
    rnd = random.Random(21)
    n = int(length * fs)
    s = [0.0] * n
    murmur = OnePole(fs, 350)
    for i in range(n):
        s[i] = 0.12 * murmur.lp(rnd.uniform(-1, 1)) * env_ar(i / fs, 0.2, 0.4, length)
    t = 0.05
    while t < length - 0.2:
        f0 = rnd.uniform(260, 850)
        tau = rnd.uniform(0.02, 0.05)
        amp = rnd.uniform(0.4, 1.0)
        osc = Osc(fs)
        start = int(t * fs)
        for i in range(int(6 * tau * fs)):
            u = i / fs
            if start + i < n:
                s[start + i] += amp * osc(f0 * (1 + 9 * u)) * decay(u, tau)
        # Пузыри идут кучками: то два подряд, то пауза.
        t += rnd.choice([0.06, 0.09, 0.14, 0.22, 0.3])
    return fs, finish(s, fs, 0.75)


def incubator_hum():
    # Инкубатор: тёплый трансформаторный гул на 100 Гц и обертонах. Самый
    # частый и самый «фоновый» звук мода — поэтому тихий пик и мягкий фильтр.
    # Обертоны до 400 Гц нужны, чтобы гул был слышен и в наушниках-затычках.
    fs, length = LO, 2.0
    rnd = random.Random(31)
    n = int(length * fs)
    o = [Osc(fs) for _ in range(4)]
    soft = OnePole(fs, 900)
    fan = OnePole(fs, 600)
    out = []
    for i in range(n):
        t = i / fs
        tone = 0.5 * o[0](100) + 0.3 * o[1](200) + 0.18 * o[2](300) + 0.1 * o[3](400.5)
        tone *= 1 + 0.08 * math.sin(2 * math.pi * 0.5 * t)
        air = 0.25 * fan.lp(rnd.uniform(-1, 1))
        out.append(soft.lp(tone + air) * env_ar(t, 0.45, 0.6, length))
    return fs, finish(out, fs, 0.45)


def machine_ready():
    # «Готово» лабораторного прибора: два коротких сигнала, второй выше.
    # Чистый синус с каплей второй гармоники — электронно, но не резко.
    fs, length = LO, 0.55
    n = int(length * fs)
    out = [0.0] * n
    for start, dur, f in [(0.0, 0.12, 1320.0), (0.17, 0.22, 1760.0)]:
        o1, o2 = Osc(fs), Osc(fs)
        for i in range(int(dur * fs)):
            u = i / fs
            g = env_ar(u, 0.006, 0.05, dur)
            out[int(start * fs) + i] += g * (0.85 * o1(f) + 0.15 * o2(2 * f))
    return fs, finish(out, fs, 0.7)


# ---- Инструменты и препараты ----

def syringe_inject():
    # Укол: щелчок поршня, короткий «пфф» раствора и тонкий писк резины.
    fs, length = HI, 0.35
    rnd = random.Random(41)
    n = int(length * fs)
    click = Band(fs, 4500, 2.0)
    hiss = Band(fs, 3200, 1.5)
    osc = Osc(fs)
    out = []
    for i in range(n):
        t = i / fs
        x = rnd.uniform(-1, 1)
        v = 0.9 * click(x) * decay(t, 0.003)
        # Фильтр зовём на каждом сэмпле, даже вне «пфф»: так он не стартует
        # с нуля посреди звука.
        h = hiss(x)
        if 0.02 <= t < 0.18:
            v += 0.6 * h * env_ar(t - 0.02, 0.02, 0.1, 0.16)
        if 0.05 <= t < 0.15:
            u = t - 0.05
            v += 0.15 * osc(2900 - 4000 * u) * env_ar(u, 0.01, 0.05, 0.1)
        out.append(v)
    return fs, finish(out, fs, 0.8)


def scalpel_cut():
    # Разрез: быстрый «вжик» — полосовой шум, центр которого съезжает вниз,
    # и под ним глухой влажный отклик ткани.
    fs, length = HI, 0.3
    rnd = random.Random(51)
    n = int(length * fs)
    swish = Band(fs, 7000, 4.0)
    flesh = OnePole(fs, 400)
    out = []
    for i in range(n):
        t = i / fs
        swish.set(7000 - 15000 * min(t, 0.3))
        x = rnd.uniform(-1, 1)
        v = 1.2 * swish(x) * env_ar(t, 0.012, 0.12, 0.2) * (t < 0.2)
        v += 0.5 * flesh.lp(x) * decay(t - 0.04, 0.04) * (t >= 0.04)
        out.append(v)
    return fs, finish(out, fs, 0.8)


def spray_use():
    # Аэрозоль: «пшш» — верхний шум. Первые 30 мс клапан дрожит, дальше ровно.
    fs, length = HI, 0.55
    rnd = random.Random(61)
    n = int(length * fs)
    hp1, hp2 = OnePole(fs, 2500), OnePole(fs, 2500)
    top = OnePole(fs, 12000)
    out = []
    for i in range(n):
        t = i / fs
        x = top.lp(hp2.hp(hp1.hp(rnd.uniform(-1, 1))))
        flutter = 0.6 + 0.4 * math.sin(2 * math.pi * 90 * t) if t < 0.03 else 1.0
        out.append(x * flutter * env_ar(t, 0.01, 0.18, length))
    return fs, finish(out, fs, 0.7)


def dart_shoot():
    # Пневматический выстрел дротика: хлопок воздуха, свист пролёта (тон
    # падает — дротик удаляется) и шипение остатка воздуха в стволе.
    fs, length = HI, 0.4
    rnd = random.Random(71)
    n = int(length * fs)
    pop = OnePole(fs, 1500)
    tail = Band(fs, 5000, 1.0)
    osc = Osc(fs)
    out = []
    for i in range(n):
        t = i / fs
        x = rnd.uniform(-1, 1)
        v = 1.3 * pop.lp(x) * decay(t, 0.012)
        if t < 0.14:
            v += 0.35 * osc(1900 - 9000 * t) * env_ar(t, 0.005, 0.08, 0.14)
        v += 0.25 * tail(x) * decay(t - 0.02, 0.08) * (t >= 0.02)
        out.append(v)
    return fs, finish(out, fs, 0.8)


def bandage_apply():
    # Перевязка: два-три оборота бинта — шорох ткани. Каждый оборот —
    # «горб» полосового шума с зернистой амплитудой (волокна цепляются).
    fs, length = LO, 0.8
    rnd = random.Random(81)
    n = int(length * fs)
    cloth = Band(fs, 2200, 0.8)
    grain = OnePole(fs, 60)
    strokes = [(0.0, 0.26, 1.0), (0.3, 0.22, 0.8), (0.55, 0.22, 0.65)]
    out = []
    for i in range(n):
        t = i / fs
        g = 0.0
        for start, dur, amp in strokes:
            u = t - start
            if 0 <= u < dur:
                g += amp * math.sin(math.pi * u / dur) ** 2
        rough = 0.5 + 2.5 * abs(grain.lp(rnd.uniform(-1, 1)))
        out.append(cloth(rnd.uniform(-1, 1)) * g * rough)
    return fs, finish(out, fs, 0.7)


def gene_therapy_use():
    # Генотерапия: восходящее мерцающее арпеджио (ля мажор) из расстроенных
    # пар синусов и мягкий подъём шума — «что-то переписывается внутри».
    # Хвост — две линии задержки с обратной связью, вместо реверберации.
    fs, length = LO, 1.6
    rnd = random.Random(91)
    n = int(length * fs)
    notes = [(0.0, 440.0), (0.12, 554.4), (0.24, 659.3), (0.36, 880.0)]
    oscs = [(Osc(fs), Osc(fs)) for _ in notes]
    swell = Band(fs, 800, 2.0)
    dry = []
    for i in range(n):
        t = i / fs
        v = 0.0
        for (start, f), (a, b) in zip(notes, oscs):
            u = t - start
            if u >= 0:
                vib = 1 + 0.004 * math.sin(2 * math.pi * 5.5 * u)
                v += 0.25 * (a(f * vib) + b(f * 1.006 * vib)) * decay(u, 0.45) * min(1.0, u / 0.015)
        swell.set(600 + 2400 * min(t / 0.9, 1.0))
        v += 0.15 * swell(rnd.uniform(-1, 1)) * env_ar(t, 0.6, 0.5, 1.1) * (t < 1.1)
        dry.append(v)
    out = list(dry)
    for delay, fb in [(int(0.071 * fs), 0.35), (int(0.113 * fs), 0.3)]:
        for i in range(delay, n):
            out[i] += fb * out[i - delay]
    for i in range(n):
        out[i] *= env_ar(i / fs, 0.0001, 0.35, length)
    return fs, finish(out, fs, 0.7)


def dosimeter_click():
    # Щелчок счётчика Гейгера (на будущее — дозиметр, M9): импульс в пару
    # миллисекунд со звоном корпуса ~4 кГц. Темп щелчков задаёт код, высоту —
    # таблица; файл один и короткий, его играют много раз подряд.
    fs, length = HI, 0.05
    rnd = random.Random(101)
    n = int(length * fs)
    ring = Band(fs, 4200, 6.0)
    body = Band(fs, 1300, 2.0)
    out = []
    for i in range(n):
        t = i / fs
        x = rnd.uniform(-1, 1) * decay(t, 0.0008)
        out.append(1.5 * ring(x) + 0.8 * body(x))
    for i in range(n):
        out[i] *= decay(i / fs, 0.008)
    return fs, finish(out, fs, 0.8)


# ---- Таблица ----
#
# Событие (без `biolab:`) → (файл в `sounds/`, функция). Событие называем по
# ванильному образцу (`block.<блок>.<что>`, `item.<предмет>.<что>`), субтитр —
# `subtitles.biolab.<событие>`. Какое событие где играет — таблица звуков в
# `native/biolab-core/mixins/biolab_core/runtime.rs` («Звуки»); её тест
# сверяет, что всё сыгранное объявлено здесь и что файлы на месте.
SOUNDS = [
    ("block.centrifuge.start", "centrifuge_start", centrifuge_start),
    ("block.centrifuge.work", "centrifuge_work", centrifuge_work),
    ("block.centrifuge.done", "centrifuge_done", centrifuge_done),
    ("block.fermenter.bubble", "fermenter_bubble", fermenter_bubble),
    ("block.incubator.hum", "incubator_hum", incubator_hum),
    ("block.machine.ready", "machine_ready", machine_ready),
    ("item.syringe.inject", "syringe_inject", syringe_inject),
    ("item.scalpel.cut", "scalpel_cut", scalpel_cut),
    ("item.spray.use", "spray_use", spray_use),
    ("item.dart_gun.shoot", "dart_shoot", dart_shoot),
    ("item.bandage.apply", "bandage_apply", bandage_apply),
    ("item.gene_therapy.use", "gene_therapy_use", gene_therapy_use),
    ("item.dosimeter.click", "dosimeter_click", dosimeter_click),
]


def write_wav(path, fs, samples, channels):
    """16-битный WAV. Стерео для встроенного кодера пишем сами, дублируя
    канал: `-ac 2` в ffmpeg разводит моно по каналам с −3 дБ, и звук
    становился тише задуманного."""
    with wave.open(path, "wb") as w:
        w.setnchannels(channels)
        w.setsampwidth(2)
        w.setframerate(fs)
        w.writeframes(b"".join(
            struct.pack("<h", int(max(-1.0, min(1.0, s)) * 32767)) * channels
            for s in samples))


def encoder():
    """Чем кодировать в Vorbis. Лучший — `libvorbis` в ffmpeg (моно, q 4).
    Но ffmpeg из Homebrew собран без него (на 04.11.2026), поэтому дальше —
    `oggenc` из vorbis-tools, а последний вариант — встроенный кодер ffmpeg
    `vorbis`. Встроенный умеет только стерео: пишем два одинаковых канала.
    Позиционности это не ломает — Horizon сводит любой файл в моно перед
    панорамой (`horizon-client-render/src/audio.rs`, `decode`: сумма каналов
    пополам), — но файл чуть больше, а ванильная Java-игра стерео не
    позиционирует; для неё нужен libvorbis или oggenc."""
    out = subprocess.run([FFMPEG, "-hide_banner", "-encoders"],
                         capture_output=True, text=True).stdout
    if " libvorbis " in out:
        return "libvorbis"
    if shutil.which("oggenc"):
        return "oggenc"
    if " vorbis " in out:
        return "vorbis"
    sys.exit("нечем кодировать Vorbis: нужен ffmpeg с libvorbis или oggenc")


def to_ogg(wav, ogg, how):
    if how == "oggenc":
        # Серийный номер потока задаём сами: по умолчанию он случайный.
        cmd = ["oggenc", "-Q", "-q", "4", "-s", "1", "--discard-comments", "-o", ogg, wav]
    else:
        # `bitexact` у выхода (после `-i`; до него флаг относится ко входу):
        # ogg-муксер берёт серийный номер не случайно, а версия
        # кодека не попадает в теги — прогон за прогоном одни и те же байты.
        cmd = [FFMPEG, "-y", "-loglevel", "error", "-i", wav, "-fflags", "+bitexact",
               "-map_metadata", "-1", "-flags:a", "+bitexact", "-q:a", "4"]
        if how == "libvorbis":
            cmd += ["-c:a", "libvorbis"]
        else:
            cmd += ["-c:a", "vorbis", "-strict", "experimental"]
        cmd.append(ogg)
    subprocess.run(cmd, check=True)


def main():
    if not os.path.exists(FFMPEG):
        sys.exit(f"нет {FFMPEG}: поставьте ffmpeg (brew install ffmpeg)")
    how = encoder()
    print(f"кодер: {how}")
    os.makedirs(SOUNDS_DIR, exist_ok=True)
    table = {}
    with tempfile.TemporaryDirectory(prefix="biolab-sounds-") as tmp:
        for event, name, make in SOUNDS:
            fs, samples = make()
            wav = os.path.join(tmp, name + ".wav")
            ogg = os.path.join(SOUNDS_DIR, name + ".ogg")
            write_wav(wav, fs, samples, 2 if how == "vorbis" else 1)
            to_ogg(wav, ogg, how)
            size = os.path.getsize(ogg)
            print(f"{name}.ogg  {len(samples) / fs:.2f} с  {fs} Гц  {size / 1024:.1f} КБ")
            table[event] = {
                "subtitle": f"subtitles.biolab.{event}",
                "sounds": [f"biolab:{name}"],
            }
    with open(os.path.join(ASSETS, "sounds.json"), "w", encoding="utf-8") as f:
        json.dump(table, f, indent=2, ensure_ascii=False)
        f.write("\n")


if __name__ == "__main__":
    main()
