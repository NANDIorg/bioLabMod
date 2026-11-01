#!/usr/bin/env python3
"""Проверка моделей предметов BioLab (без зависимостей).

Запуск из корня репозитория:
    python3 tools/check_item_models.py [--vanilla <путь к assets/minecraft>]

Что проверяет:
  * каждый JSON в content/assets/biolab читается;
  * определения items/*.json: известные типы узлов, существующие модели,
    оттенки (tints), у select по minecraft:display_context — допустимые
    имена контекстов, без повторов, и каждый из девяти контекстов получает
    модель (через случай или fallback);
  * модели models/**: цепочка parent разрешается и не зациклена, все
    ссылки на текстуры (#переменные и пути) ведут к существующим PNG;
  * элементы: from/to в пределах −16…32 и from ≤ to, поворот по одной оси
    на −45, −22.5, 0, 22.5 или 45, грани с допустимым именем, uv в 0…16
    и текстурой;
  * display: известные положения, тройки чисел, translation в ±80,
    scale в ±4;
  * текстуры textures/item/3d — PNG 16×16; неиспользуемые текстуры и
    модели из models/item/3d — предупреждение.

Ванильные ссылки (minecraft:item/generated и т. п.) проверяются по ресурсам
версии Horizon, если они найдены (или указаны --vanilla); иначе — по
короткому списку встроенных моделей.

Код выхода 1, если есть ошибки.
"""
import glob
import json
import os
import struct
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS = os.path.join(ROOT, "content", "assets")
NS = "biolab"

CONTEXTS = ["none", "thirdperson_lefthand", "thirdperson_righthand", "firstperson_lefthand",
            "firstperson_righthand", "head", "gui", "ground", "fixed", "on_shelf"]
DISPLAY_KEYS = set(CONTEXTS) - {"none"}
FACES = {"north", "south", "east", "west", "up", "down"}
ANGLES = {-45, -22.5, 0, 22.5, 45}
BUILTIN = {"builtin/generated", "builtin/entity", "item/generated", "item/handheld",
           "block/block", "block/cube", "block/cube_all"}

errors = []
warnings = []


def err(where, msg):
    line = f"{where}: {msg}"
    if line not in errors:
        errors.append(line)


def warn(where, msg):
    warnings.append(f"{where}: {msg}")


def rel(path):
    return os.path.relpath(path, ROOT)


def split_id(ref):
    ns, _, path = ref.partition(":") if ":" in ref else ("minecraft", "", ref)
    return ns, path


def find_vanilla():
    if "--vanilla" in sys.argv:
        return sys.argv[sys.argv.index("--vanilla") + 1]
    pattern = os.path.expanduser(
        "~/Library/Application Support/horizon-launcher/versions/*/.local/horizon-client/*/assets/minecraft")
    found = sorted(glob.glob(pattern))
    return found[-1] if found else None


VANILLA = find_vanilla()


def load_json(path):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError) as e:
        err(rel(path), f"не читается: {e}")
        return None


def model_path(ref):
    ns, path = split_id(ref)
    if ns == NS:
        return os.path.join(ASSETS, NS, "models", path + ".json")
    if VANILLA:
        return os.path.join(VANILLA, "models", path + ".json")
    return None


def texture_exists(ref):
    ns, path = split_id(ref)
    if ns == NS:
        return os.path.isfile(os.path.join(ASSETS, NS, "textures", path + ".png"))
    if VANILLA:
        return os.path.isfile(os.path.join(VANILLA, "textures", path + ".png"))
    return True


def png_size(path):
    with open(path, "rb") as f:
        head = f.read(24)
    if head[:8] != b"\x89PNG\r\n\x1a\n" or head[12:16] != b"IHDR":
        return None
    return struct.unpack(">II", head[16:24])


_models = {}


def model(ref, where):
    """Модель по id с разрешённой цепочкой parent: (textures, elements,
    display, chain) или None, если не нашлась."""
    ns, path = split_id(ref)
    key = f"{ns}:{path}"
    if key in _models:
        return _models[key]
    if ns != NS and (path in BUILTIN or path.startswith("builtin/")) and not VANILLA:
        _models[key] = ({}, None, {}, [key])
        return _models[key]
    if path.startswith("builtin/"):
        _models[key] = ({}, None, {}, [key])
        return _models[key]
    file = model_path(ref)
    if file is None:
        _models[key] = ({}, None, {}, [key])
        return _models[key]
    if not os.path.isfile(file):
        err(where, f"нет модели {key} ({rel(file) if ns == NS else file})")
        _models[key] = None
        return None
    _models[key] = "pending"
    data = load_json(file)
    if data is None:
        _models[key] = None
        return None
    textures, elements, display, chain = {}, None, {}, []
    if "parent" in data:
        parent = model(data["parent"], rel(file) if ns == NS else key)
        if parent == "pending":
            err(key, f"цикл в parent через {data['parent']}")
            parent = None
        if parent:
            textures, elements, display, chain = dict(parent[0]), parent[1], dict(parent[2]), list(parent[3])
    textures.update(data.get("textures", {}))
    if "elements" in data:
        elements = data["elements"]
    display.update(data.get("display", {}))
    chain = [key] + chain
    _models[key] = (textures, elements, display, chain)
    return _models[key]


def resolve_texture(textures, name, where, seen=()):
    if name in seen:
        err(where, f"цикл текстурных переменных {name}")
        return None
    if name not in textures:
        err(where, f"переменная текстуры #{name} не задана")
        return None
    value = textures[name]
    if value.startswith("#"):
        return resolve_texture(textures, value[1:], where, seen + (name,))
    return value


def num3(v):
    return isinstance(v, list) and len(v) == 3 and all(isinstance(c, (int, float)) for c in v)


def check_model_file(path):
    data = load_json(path)
    if data is None:
        return
    ref = f"{NS}:" + os.path.relpath(path, os.path.join(ASSETS, NS, "models"))[:-5].replace(os.sep, "/")
    where = rel(path)
    m = model(ref, where)
    if not m or m == "pending":
        return
    textures, elements, display, chain = m
    for name, value in textures.items():
        if not value.startswith("#"):
            if not texture_exists(value):
                err(where, f"нет текстуры {value} (переменная {name})")
    for i, e in enumerate(data.get("elements", [])):
        w = f"{where} элемент {i}"
        fr, to = e.get("from"), e.get("to")
        if not (num3(fr) and num3(to)):
            err(w, "from/to — не тройки чисел")
            continue
        for a in range(3):
            if not (-16 <= fr[a] <= 32 and -16 <= to[a] <= 32):
                err(w, f"координаты вне −16…32: {fr} → {to}")
                break
            if fr[a] > to[a]:
                err(w, f"from больше to по оси {'xyz'[a]}")
        r = e.get("rotation")
        if r is not None:
            if not isinstance(r, dict):
                err(w, "rotation — не объект")
            elif "axis" in r or "angle" in r:
                if r.get("axis") not in ("x", "y", "z"):
                    err(w, f"ось поворота {r.get('axis')!r}")
                if r.get("angle") not in ANGLES:
                    err(w, f"угол поворота {r.get('angle')!r} (допустимо {sorted(ANGLES)})")
                if not num3(r.get("origin")):
                    err(w, "у поворота нет origin")
            else:
                err(w, "поворот не по одной оси (x/y/z без axis+angle)")
        faces = e.get("faces")
        if not isinstance(faces, dict) or not faces:
            err(w, "нет граней")
            continue
        for fname, face in faces.items():
            fw = f"{w} грань {fname}"
            if fname not in FACES:
                err(fw, "неизвестная грань")
            uv = face.get("uv")
            if uv is not None and not (isinstance(uv, list) and len(uv) == 4
                                       and all(isinstance(c, (int, float)) and 0 <= c <= 16 for c in uv)):
                err(fw, f"uv вне 0…16: {uv}")
            tex = face.get("texture")
            if not isinstance(tex, str) or not tex.startswith("#"):
                err(fw, f"texture {tex!r} — нужна #переменная")
            else:
                # Существование файла проверено выше по всей карте textures.
                resolve_texture(textures, tex[1:], fw)
            if "tintindex" in face and not isinstance(face["tintindex"], int):
                err(fw, "tintindex не целое")
            if face.get("rotation", 0) not in (0, 90, 180, 270):
                err(fw, f"поворот грани {face.get('rotation')}")
            if "cullface" in face and face["cullface"] not in FACES:
                err(fw, f"cullface {face['cullface']!r}")
    for ctx, t in data.get("display", {}).items():
        w = f"{where} display.{ctx}"
        if ctx not in DISPLAY_KEYS:
            err(w, "неизвестное положение")
            continue
        for key, lim in (("rotation", None), ("translation", 80), ("scale", 4)):
            if key in t:
                if not num3(t[key]):
                    err(w, f"{key} — не тройка чисел")
                elif lim and any(abs(c) > lim for c in t[key]):
                    err(w, f"{key} вне ±{lim}: {t[key]}")
    if "gui_light" in data and data["gui_light"] not in ("front", "side"):
        err(where, f"gui_light {data['gui_light']!r}")
    # Модель, которую можно нарисовать: есть элементы или это спрайт (layer0).
    if elements is None and not any(c.endswith("generated") for c in chain) \
            and not any(c.startswith("minecraft:builtin") for c in chain):
        if VANILLA or not any(split_id(c)[0] == "minecraft" for c in chain):
            err(where, "нет ни elements, ни parent на item/generated")
    if any(c.endswith("generated") for c in chain) and elements is None and "layer0" not in textures:
        err(where, "спрайтовая модель без layer0")


def check_tints(tints, where):
    if not isinstance(tints, list):
        err(where, "tints — не список")
        return
    for t in tints:
        kind = t.get("type", "")
        if kind == "minecraft:constant":
            v = t.get("value")
            if not (isinstance(v, int) or num3(v)):
                err(where, f"constant без value: {t}")
        elif not kind.startswith("minecraft:"):
            err(where, f"неизвестный оттенок {kind!r}")


def walk(node, where, used, contexts=None):
    """Обходит дерево модели предмета. contexts — множество контекстов,
    в которых узел может быть выбран (None — любые)."""
    if not isinstance(node, dict):
        err(where, "узел модели — не объект")
        return
    kind = node.get("type", "")
    if kind == "minecraft:model":
        ref = node.get("model")
        if not isinstance(ref, str):
            err(where, "minecraft:model без model")
            return
        used.setdefault(ref, set()).update(contexts or CONTEXTS)
        model(ref, where)
        if "tints" in node:
            check_tints(node["tints"], where)
    elif kind == "minecraft:select":
        prop = node.get("property")
        cases = node.get("cases", [])
        if not isinstance(cases, list):
            err(where, "cases — не список")
            return
        if prop == "minecraft:display_context":
            covered = set()
            for i, case in enumerate(cases):
                when = case.get("when")
                when = when if isinstance(when, list) else [when]
                for c in when:
                    if c not in CONTEXTS:
                        err(where, f"случай {i}: неизвестный контекст {c!r}")
                    elif c in covered:
                        err(where, f"случай {i}: контекст {c} уже занят")
                    covered.add(c)
                walk(case.get("model"), f"{where} случай {i}", used, set(when))
            rest = set(CONTEXTS) - covered
            if "fallback" in node:
                walk(node["fallback"], f"{where} fallback", used, rest)
            elif rest:
                err(where, f"нет fallback, без модели остаются: {', '.join(sorted(rest))}")
        else:
            for i, case in enumerate(cases):
                walk(case.get("model"), f"{where} случай {i}", used, contexts)
            if "fallback" in node:
                walk(node["fallback"], f"{where} fallback", used, contexts)
    elif kind == "minecraft:condition":
        for key in ("on_true", "on_false"):
            if key not in node:
                err(where, f"condition без {key}")
            else:
                walk(node[key], f"{where} {key}", used, contexts)
    elif kind == "minecraft:range_dispatch":
        for i, entry in enumerate(node.get("entries", [])):
            walk(entry.get("model"), f"{where} entry {i}", used, contexts)
        if "fallback" in node:
            walk(node["fallback"], f"{where} fallback", used, contexts)
    elif kind == "minecraft:composite":
        for i, sub in enumerate(node.get("models", [])):
            walk(sub, f"{where} models[{i}]", used, contexts)
    elif kind == "minecraft:special":
        if "base" in node:
            used.setdefault(node["base"], set()).update(contexts or CONTEXTS)
            model(node["base"], where)
    elif kind in ("minecraft:empty", "minecraft:bundle/selected_item"):
        pass
    else:
        err(where, f"неизвестный тип узла {kind!r}")


def main():
    base = os.path.join(ASSETS, NS)
    all_json = sorted(glob.glob(os.path.join(base, "**", "*.json"), recursive=True))
    for path in all_json:
        load_json(path)

    used = {}
    items = sorted(glob.glob(os.path.join(base, "items", "*.json")))
    by_context = 0
    for path in items:
        data = load_json(path)
        if data is None:
            continue
        if "model" not in data:
            err(rel(path), "нет поля model")
            continue
        root = data["model"]
        if root.get("type") == "minecraft:select" and root.get("property") == "minecraft:display_context":
            by_context += 1
        walk(root, rel(path), used)

    models = sorted(glob.glob(os.path.join(base, "models", "**", "*.json"), recursive=True))
    for path in models:
        check_model_file(path)

    # Текстуры частей 3D-моделей.
    tex3d = sorted(glob.glob(os.path.join(base, "textures", "item", "3d", "*.png")))
    for path in tex3d:
        size = png_size(path)
        if size is None:
            err(rel(path), "не PNG")
        elif size != (16, 16):
            err(rel(path), f"размер {size[0]}×{size[1]}, нужен 16×16")

    # Что из 3D-набора нигде не используется.
    referenced = set()
    for m in _models.values():
        if m and m != "pending":
            for v in m[0].values():
                if not v.startswith("#"):
                    ns, p = split_id(v)
                    referenced.add(f"{ns}:{p}")
    for path in tex3d:
        ref = f"{NS}:item/3d/" + os.path.basename(path)[:-4]
        if ref not in referenced:
            warn(rel(path), "текстура не используется ни одной моделью")
    parents = set()
    for m in _models.values():
        if m and m != "pending":
            parents.update(m[3][1:])
    models3d = sorted(glob.glob(os.path.join(base, "models", "item", "3d", "*.json")))
    for path in models3d:
        ref = f"{NS}:item/3d/" + os.path.basename(path)[:-5]
        if ref not in used and ref not in parents:
            warn(rel(path), "модель не используется ни одним предметом")

    hand_3d = sorted({os.path.basename(p)[:-5] for p in models3d
                      if f"{NS}:item/3d/" + os.path.basename(p)[:-5] in used})
    gui_3d = sorted(r.split("/")[-1] for r, ctx in used.items()
                    if r.startswith(f"{NS}:item/3d/") and "gui" in ctx)

    print(f"JSON прочитано: {len(all_json)}")
    print(f"определений предметов: {len(items)}, из них с выбором по display_context: {by_context}")
    print(f"моделей проверено: {len(models)}, из них 3D-форм: {len(models3d)}")
    print(f"текстур частей (item/3d): {len(tex3d)}")
    print(f"3D-модели в руке: {', '.join(hand_3d) or '—'}")
    print(f"3D и в инвентаре: {', '.join(gui_3d) or '—'}")
    print(f"ванильные ресурсы: {VANILLA or 'не найдены, minecraft:-ссылки проверены по списку встроенных'}")
    for w in warnings:
        print("ПРЕДУПРЕЖДЕНИЕ", w)
    for e in errors:
        print("ОШИБКА", e)
    print("итог:", "ошибок нет" if not errors else f"ошибок: {len(errors)}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
