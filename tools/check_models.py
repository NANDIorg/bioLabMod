#!/usr/bin/env python3
"""Проверка ресурсов BioLab: blockstates, модели, предметы, текстуры.

Запуск из любого места:
    python3 tools/check_models.py [--vanilla <assets>] [-v]

Что проверяет:
- все JSON в content/assets/*/{blockstates,models,items} читаются;
- blockstates: у каждого состояния блока есть вариант (свойства — из ванильного
  blockstate шаблона блока, content/data/<ns>/horizon/block/<id>.json), ссылки на
  модели существуют, поворот `x`/`y` кратен 90; у multipart условия `when`
  (с `OR`/`AND`, `a|b`, `!a`) — только по свойствам шаблона, и у каждого
  состояния рисуется хоть одна часть (секции стола на шаблоне `mushroom_stem`);
- модели: родители находятся, у каждой `#текстуры` грани есть значение, файлы
  текстур есть (PNG, сторона кратна 16); элементы: `from`/`to` в −16…32 и
  from ≤ to, поворот по одной оси с углом из −45, −22.5, 0, 22.5, 45, `uv` в 0…16,
  `cullface` только у грани, лежащей на этой границе блока, `light_emission` 0…15;
- машины (блоки на шаблонах-кубах `furnace`, `iron_block`, у которых своя
  модель из элементов): у граней есть `uv`; детали по X и Z внутри 0…16 с учётом
  поворота; не выше 20 по Y; низ закрыт целиком (грани `down` на y = 0 покрывают
  16×16); есть `display` для gui/ground/fixed/рук; 6–20 элементов (предупреждение);
- предметы: ссылки на модели в items/*.json существуют, у каждого блока с
  `"item": true` есть items/<id>.json.

Ссылки на `minecraft:` сверяются с ванильными ресурсами, если они найдены
(`--vanilla`, переменная BIOLAB_VANILLA или ресурсы лаунчера Horizon); иначе
принимаются на веру. Код выхода 1 — есть ошибки.
"""
import argparse
import glob
import itertools
import json
import math
import os
import struct
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CONTENT = os.path.join(ROOT, "content")
ANGLES = (-45, -22.5, 0, 22.5, 45)
DIRS = ("down", "up", "north", "south", "west", "east")
DISPLAY = ("gui", "ground", "fixed", "thirdperson_righthand", "firstperson_righthand")
CUBE_TEMPLATES = ("minecraft:furnace", "minecraft:iron_block")
MAX_Y = 20
# Свойства шаблонов, если ванильных ресурсов под рукой нет.
KNOWN_STATES = {
    "minecraft:furnace": {"facing": ["east", "north", "south", "west"], "lit": ["false", "true"]},
    "minecraft:iron_block": {},
    "minecraft:snow": {"layers": [str(i) for i in range(1, 9)]},
    # Секции операционного стола: стороны гриба, `false` — соседняя секция.
    "minecraft:mushroom_stem": {side: ["false", "true"] for side in DIRS},
}

errors = []
warnings = []


def err(where, msg):
    errors.append(f"{where}: {msg}")


def warn(where, msg):
    warnings.append(f"{where}: {msg}")


def rel(path):
    return os.path.relpath(path, ROOT)


def load(path):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError) as e:
        err(rel(path), f"не читается: {e}")
        return None


def split(ref, default="minecraft"):
    ns, _, path = ref.partition(":")
    return (ns, path) if path else (default, ns)


def find_vanilla(arg):
    for cand in [arg, os.environ.get("BIOLAB_VANILLA")]:
        if cand:
            return cand if os.path.isdir(os.path.join(cand, "minecraft")) else None
    pattern = os.path.expanduser(
        "~/Library/Application Support/horizon-launcher/versions/*/.local/horizon-client/26.3/assets")
    for cand in sorted(glob.glob(pattern)):
        if os.path.isfile(os.path.join(cand, "minecraft", "models", "block", "block.json")):
            return cand
    return None


class Assets:
    def __init__(self, vanilla):
        self.vanilla = vanilla
        self.cache = {}

    def path(self, ns, kind, path, ext):
        if ns == "minecraft":
            if not self.vanilla:
                return None
            return os.path.join(self.vanilla, ns, kind, path + ext)
        return os.path.join(CONTENT, "assets", ns, kind, path + ext)

    def exists(self, ns, kind, path, ext):
        """True/False; None — ванильных ресурсов нет, проверить нельзя."""
        p = self.path(ns, kind, path, ext)
        return None if p is None else os.path.isfile(p)

    def model(self, ref):
        ns, path = split(ref)
        key = (ns, path)
        if key not in self.cache:
            p = self.path(ns, "models", path, ".json")
            self.cache[key] = load(p) if p and os.path.isfile(p) else None
        return self.cache[key]

    def chain(self, ref, where):
        """Модель и её родители по порядку; None на месте ненайденного звена."""
        out, seen = [], set()
        while ref:
            if ref in seen:
                err(where, f"цикл родителей на {ref}")
                break
            seen.add(ref)
            ns, path = split(ref)
            if ns == "minecraft" and path.startswith("builtin/"):
                break  # встроенные «модели» игры (builtin/generated, builtin/entity)
            m = self.model(ref)
            if m is None:
                if self.exists(ns, "models", path, ".json") is not None:
                    err(where, f"нет модели {ref}")
                out.append(None)
                break
            out.append(m)
            ref = m.get("parent")
        return out


def png_size(path):
    with open(path, "rb") as f:
        head = f.read(24)
    if head[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    return struct.unpack(">II", head[16:24])


def check_texture(assets, ref, where, checked):
    if ref in checked:
        return
    checked.add(ref)
    ns, path = split(ref)
    p = assets.path(ns, "textures", path, ".png")
    if p is None:
        return
    if not os.path.isfile(p):
        err(where, f"нет текстуры {ref} ({rel(p) if ns != 'minecraft' else p})")
        return
    size = png_size(p)
    if size is None:
        err(where, f"{ref}: не PNG")
    elif size[0] % 16 or size[1] % size[0]:
        err(where, f"{ref}: размер {size[0]}×{size[1]}, нужен 16×16 (или кадры анимации)")


def resolve_texture(chain, key, where):
    seen = set()
    while key.startswith("#"):
        key = key[1:]
        if key in seen:
            err(where, f"цикл текстур на #{key}")
            return None
        seen.add(key)
        value = None
        for m in chain:
            if m is not None and key in m.get("textures", {}):
                value = m["textures"][key]
                break
        if value is None:
            # Ванильный родитель, которого не видно, мог задать слот сам.
            if any(m is None for m in chain):
                return None
            err(where, f"текстура #{key} не задана")
            return None
        key = value if isinstance(value, str) else value.get("sprite", "")
    return key


def num(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def vec(v, n=3):
    return isinstance(v, list) and len(v) == n and all(num(x) for x in v)


def on_plane(face, f, t):
    return {"down": f[1] == 0, "up": t[1] == 16, "north": f[2] == 0,
            "south": t[2] == 16, "west": f[0] == 0, "east": t[0] == 16}[face]


def rotate(p, rot):
    axis, angle, o = rot["axis"], math.radians(rot["angle"]), rot["origin"]
    s, c = math.sin(angle), math.cos(angle)
    x, y, z = p[0] - o[0], p[1] - o[1], p[2] - o[2]
    if axis == "x":
        y, z = c * y - s * z, s * y + c * z
    elif axis == "y":
        x, z = c * x + s * z, -s * x + c * z
    else:
        x, y = c * x - s * y, s * x + c * y
    return x + o[0], y + o[1], z + o[2]


def check_element(e, where, machine):
    if not isinstance(e, dict):
        err(where, "элемент не объект")
        return None
    f, t = e.get("from"), e.get("to")
    if not (vec(f) and vec(t)):
        err(where, "нет from/to из трёх чисел")
        return None
    for v in f + t:
        if not -16 <= v <= 32:
            err(where, f"координата {v} вне −16…32")
    if any(a > b for a, b in zip(f, t)):
        err(where, f"from {f} больше to {t}")
    rot = e.get("rotation")
    if rot is not None:
        if not isinstance(rot, dict) or set(rot) - {"origin", "axis", "angle", "rescale"}:
            err(where, f"поворот: только origin/axis/angle/rescale, есть {rot}")
            rot = None
        elif rot.get("axis") not in ("x", "y", "z") or rot.get("angle") not in ANGLES \
                or not vec(rot.get("origin")):
            err(where, f"поворот: ось x/y/z, угол из {ANGLES}, origin из трёх чисел — есть {rot}")
            rot = None
    le = e.get("light_emission", 0)
    if not isinstance(le, int) or isinstance(le, bool) or not 0 <= le <= 15:
        err(where, f"light_emission {le} не целое 0…15")
    faces = e.get("faces")
    if not isinstance(faces, dict) or not faces:
        err(where, "нет граней")
        return None
    for name, face in faces.items():
        w = f"{where} грань {name}"
        if name not in DIRS:
            err(w, "неизвестное направление")
            continue
        if not isinstance(face, dict) or not isinstance(face.get("texture"), str):
            err(w, "нет texture")
            continue
        uv = face.get("uv")
        if uv is None:
            if machine:
                err(w, "нет uv")
        elif not vec(uv, 4) or not all(0 <= x <= 16 for x in uv):
            err(w, f"uv {uv} не 4 числа в 0…16")
        cull = face.get("cullface")
        if cull is not None:
            if cull not in DIRS:
                err(w, f"cullface {cull}: нет такого направления")
            elif cull != name or not on_plane(name, f, t) or rot is not None:
                err(w, f"cullface {cull} у грани не на границе блока")
        if face.get("rotation", 0) % 90:
            err(w, "поворот текстуры не кратен 90")
    if machine:
        corners = [(x, y, z) for x in (f[0], t[0]) for y in (f[1], t[1]) for z in (f[2], t[2])]
        if rot is not None:
            corners = [rotate(p, rot) for p in corners]
        xs = [p[0] for p in corners]
        ys = [p[1] for p in corners]
        zs = [p[2] for p in corners]
        if min(xs) < -0.01 or max(xs) > 16.01 or min(zs) < -0.01 or max(zs) > 16.01:
            err(where, f"выходит за 0…16 по X/Z: x {min(xs):.2f}…{max(xs):.2f}, z {min(zs):.2f}…{max(zs):.2f}")
        if min(ys) < -0.01 or max(ys) > MAX_Y + 0.01:
            err(where, f"по Y {min(ys):.2f}…{max(ys):.2f}, допустимо 0…{MAX_Y}")
    return f, t, faces, rot


def check_model(assets, ref, where, machine, checked_tex, checked_models):
    # Модель машины уже проверена строго — предмет с ней же не проверяем повторно.
    if (ref, machine) in checked_models or (ref, True) in checked_models:
        return
    checked_models.add((ref, machine))
    chain = assets.chain(ref, where)
    if not chain or chain[0] is None:
        return
    w = f"{where} → {ref}"
    elements = next((m["elements"] for m in chain if m is not None and "elements" in m), None)
    for m in chain:
        if m is None:
            continue
        for k, v in m.get("textures", {}).items():
            tex = v if isinstance(v, str) else v.get("sprite", "") if isinstance(v, dict) else ""
            if not tex:
                err(w, f"текстура {k}: пустое значение")
            elif not tex.startswith("#"):
                check_texture(assets, tex, w, checked_tex)
    if elements is None:
        if machine:
            err(w, "у машины нет своих elements")
        return
    if machine and not 6 <= len(elements) <= 20:
        warn(w, f"{len(elements)} элементов, советуется 6–20")
    bottom = set()
    for i, e in enumerate(elements):
        res = check_element(e, f"{w} элемент {i}", machine)
        if res is None:
            continue
        f, t, faces, rot = res
        for name, face in faces.items():
            if isinstance(face, dict) and isinstance(face.get("texture"), str):
                tex = resolve_texture(chain, face["texture"], f"{w} элемент {i}")
                if tex:
                    check_texture(assets, tex, w, checked_tex)
        if machine and rot is None and f[1] == 0 and "down" in faces:
            for x in range(max(0, int(f[0] * 2)), min(32, int(math.ceil(t[0] * 2)))):
                for z in range(max(0, int(f[2] * 2)), min(32, int(math.ceil(t[2] * 2)))):
                    if f[0] <= x / 2 and (x + 1) / 2 <= t[0] and f[2] <= z / 2 and (z + 1) / 2 <= t[2]:
                        bottom.add((x, z))
    if machine:
        holes = 32 * 32 - len(bottom)
        if holes:
            err(w, f"низ закрыт не целиком: не хватает {holes / 4:g} пикс²")
        display = {}
        known = False
        for m in reversed(chain):
            if m is None:
                known = True  # ванильный родитель без ресурсов: не проверить
                continue
            display.update(m.get("display", {}))
        missing = [k for k in DISPLAY if k not in display]
        if missing and not known:
            err(w, f"нет display для {', '.join(missing)} (родитель minecraft:block/block их даёт)")


def state_space(assets, template):
    """Свойства блока по ванильному blockstate шаблона: {свойство: [значения]}."""
    ns, path = split(template)
    p = assets.path(ns, "blockstates", path, ".json")
    if not p or not os.path.isfile(p):
        return KNOWN_STATES.get(template)
    data = load(p)
    props = {}
    for key in (data or {}).get("variants", {}):
        for pair in filter(None, key.split(",")):
            k, _, v = pair.partition("=")
            props.setdefault(k, set()).add(v)
    for part in (data or {}).get("multipart", []):
        conds = part.get("when", {})
        for c in conds.get("OR", [conds]) if isinstance(conds, dict) else []:
            for k, v in c.items():
                if k != "OR":
                    props.setdefault(k, set()).update(str(v).split("|"))
    return {k: sorted(v) for k, v in props.items()}


def variant_matches(key, state):
    for pair in filter(None, key.split(",")):
        k, _, v = pair.partition("=")
        if state.get(k) != v:
            return False
    return True


def when_matches(cond, state):
    """Условие части multipart: `OR`/`AND` списком или свойства с `a|b` и
    `!a` — как его читает Horizon (`horizon-assets`, `Condition::decode`)."""
    if cond is None:
        return True
    if isinstance(cond, dict) and len(cond) == 1 and isinstance(cond.get("OR"), list):
        return any(when_matches(c, state) for c in cond["OR"])
    if isinstance(cond, dict) and len(cond) == 1 and isinstance(cond.get("AND"), list):
        return all(when_matches(c, state) for c in cond["AND"])
    for k, v in cond.items():
        value = str(v).lower() if isinstance(v, bool) else str(v)
        have = state.get(k)
        hit = False
        for term in value.split("|"):
            neg = term.startswith("!")
            hit = hit or ((have != term[1:]) if neg else (have == term))
        if not hit:
            return False
    return True


def when_terms(cond):
    """Все пары (свойство, значение) условия — для сверки со свойствами шаблона."""
    if not isinstance(cond, dict):
        return
    if len(cond) == 1 and any(isinstance(cond.get(k), list) for k in ("OR", "AND")):
        for c in cond.get("OR") or cond.get("AND"):
            yield from when_terms(c)
        return
    for k, v in cond.items():
        value = str(v).lower() if isinstance(v, bool) else str(v)
        for term in value.split("|"):
            yield k, term.lstrip("!")


def check_blockstate(assets, ns, block, path, template, checked_tex, checked_models):
    where = rel(path)
    data = load(path)
    if data is None:
        return []
    machine = template in CUBE_TEMPLATES
    models = []
    variants = data.get("variants")
    if variants is not None:
        for key, value in variants.items():
            for v in value if isinstance(value, list) else [value]:
                if not isinstance(v, dict) or "model" not in v:
                    err(where, f"вариант «{key}» без model")
                    continue
                for axis in ("x", "y"):
                    if v.get(axis, 0) % 90:
                        err(where, f"вариант «{key}»: {axis} = {v[axis]} не кратен 90")
                models.append((key, v["model"]))
        space = state_space(assets, template) if template else None
        if space is not None:
            names = sorted(space)
            for combo in itertools.product(*(space[n] for n in names)):
                state = dict(zip(names, combo))
                if not any(variant_matches(k, state) for k in variants):
                    label = ",".join(f"{k}={v}" for k, v in state.items()) or "(без свойств)"
                    err(where, f"у состояния {label} нет модели")
            for key in variants:
                for pair in filter(None, key.split(",")):
                    k, _, v = pair.partition("=")
                    if k not in space or v not in space[k]:
                        err(where, f"вариант «{key}»: у {template} нет {k}={v}")
    elif "multipart" in data:
        parts = data["multipart"] if isinstance(data["multipart"], list) else []
        for i, part in enumerate(parts):
            for v in part.get("apply", []) if isinstance(part.get("apply"), list) else [part.get("apply")]:
                if not isinstance(v, dict) or "model" not in v:
                    err(where, f"часть {i} без model")
                    continue
                for axis in ("x", "y"):
                    if v.get(axis, 0) % 90:
                        err(where, f"часть {i}: {axis} = {v[axis]} не кратен 90")
                models.append((f"часть {i}", v["model"]))
        # Условия — по свойствам шаблона, и у каждого состояния что-то рисуется.
        space = state_space(assets, template) if template else None
        if space is not None:
            for i, part in enumerate(parts):
                for k, v in when_terms(part.get("when")):
                    if k not in space or v not in space[k]:
                        err(where, f"часть {i}: у {template} нет {k}={v}")
            names = sorted(space)
            for combo in itertools.product(*(space[n] for n in names)):
                state = dict(zip(names, combo))
                if not any(when_matches(p.get("when"), state) for p in parts):
                    label = ",".join(f"{k}={v}" for k, v in state.items()) or "(без свойств)"
                    err(where, f"у состояния {label} нет ни одной части")
    else:
        err(where, "нет variants и multipart")
    # Своя модель из элементов у кубического шаблона — это машина: строгие правила.
    for key, ref in models:
        strict = machine and own_elements(assets, ref)
        check_model(assets, ref, f"{where} «{key}»", strict, checked_tex, checked_models)
    return [ref for _, ref in models]


def own_elements(assets, ref):
    """Элементы модели заданы в своём (не ванильном) звене цепочки родителей."""
    seen = set()
    while ref and ref not in seen:
        seen.add(ref)
        m = assets.model(ref)
        if m is None or split(ref)[0] == "minecraft":
            return False
        if "elements" in m:
            return True
        ref = m.get("parent")
    return False


def model_refs(node):
    if isinstance(node, dict):
        for k, v in node.items():
            if k == "model" and isinstance(v, str):
                yield v
            else:
                yield from model_refs(v)
    elif isinstance(node, list):
        for v in node:
            yield from model_refs(v)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--vanilla", help="папка assets ванили 26.x (с подпапкой minecraft)")
    ap.add_argument("-v", "--verbose", action="store_true", help="перечислить проверенные блоки")
    args = ap.parse_args()
    vanilla = find_vanilla(args.vanilla)
    assets = Assets(vanilla)
    print(f"ваниль: {vanilla or 'не найдена — ссылки на minecraft: не проверяются'}")

    # Все JSON читаются.
    n_json = 0
    for ns_dir in sorted(glob.glob(os.path.join(CONTENT, "assets", "*"))):
        for kind in ("blockstates", "models", "items"):
            for p in sorted(glob.glob(os.path.join(ns_dir, kind, "**", "*.json"), recursive=True)):
                n_json += 1
                load(p)

    checked_tex, checked_models = set(), set()
    blocks = {}
    for p in sorted(glob.glob(os.path.join(CONTENT, "data", "*", "horizon", "block", "*.json"))):
        ns = p.split(os.sep)[-4]
        data = load(p) or {}
        blocks[(ns, os.path.splitext(os.path.basename(p))[0])] = data
    n_states = n_machines = 0
    for (ns, block), data in sorted(blocks.items()):
        path = os.path.join(CONTENT, "assets", ns, "blockstates", block + ".json")
        template = data.get("template")
        if not os.path.isfile(path):
            err(f"{ns}:{block}", f"нет {rel(path)}")
            continue
        n_states += 1
        if template in CUBE_TEMPLATES:
            n_machines += 1
        check_blockstate(assets, ns, block, path, template, checked_tex, checked_models)
        if args.verbose:
            print(f"  {ns}:{block} ({template})")
        if data.get("item", False):
            item = os.path.join(CONTENT, "assets", ns, "items", block + ".json")
            if not os.path.isfile(item):
                err(f"{ns}:{block}", f"блок с предметом, но нет {rel(item)}")

    n_items = 0
    for p in sorted(glob.glob(os.path.join(CONTENT, "assets", "*", "items", "*.json"))):
        data = load(p)
        if data is None:
            continue
        n_items += 1
        refs = list(model_refs(data.get("model")))
        if not refs:
            err(rel(p), "нет ссылки на модель")
        for ref in refs:
            check_model(assets, ref, rel(p), False, checked_tex, checked_models)

    print(f"JSON: {n_json}, blockstates: {n_states} (машин на шаблонах-кубах: {n_machines}), "
          f"предметов: {n_items}, моделей: {len({r for r, _ in checked_models})}, "
          f"текстур: {len(checked_tex)}")
    for w in dict.fromkeys(warnings):
        print("предупреждение:", w)
    for e in dict.fromkeys(errors):
        print("ОШИБКА:", e)
    print("итог:", "ошибок нет" if not errors else f"ошибок {len(dict.fromkeys(errors))}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
