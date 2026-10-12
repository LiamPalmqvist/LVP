"""Dump the visible text of a frame in reading order, with absolute positions.

usage: python3 texts.py canvas.json images_dir guid [guid ...]

Instances are expanded and text overrides applied, so the output matches what
the rendered frame shows. Each line is "y x w h | text".
"""
import sys

import render


def mul(a, b):
    a0, a1, a2, a3, a4, a5 = a
    b0, b1, b2, b3, b4, b5 = b
    return (a0 * b0 + a1 * b3, a0 * b1 + a1 * b4, a0 * b2 + a1 * b5 + a2,
            a3 * b0 + a4 * b3, a3 * b1 + a4 * b4, a3 * b2 + a4 * b5 + a5)


def walk(nid, path, ov, dv, m, out, root=False):
    base = render.by[nid]
    n = dict(base)
    key = tuple(path)
    n.update(ov.get(key, {}))
    n.update(dv.get(key, {}))
    if n.get("visible") is False:
        return
    mm = m if root else mul(m, render.mat(n.get("transform")))
    if n.get("type") == "TEXT":
        chars = (n.get("textData") or {}).get("characters", "")
        sz = n.get("size", {})
        out.append((round(mm[5]), round(mm[2]), round(sz.get("x", 0)), round(sz.get("y", 0)), chars.replace("\n", " / ")))
        return
    child_ids = render.kids.get(nid, [])
    cov, cdv = ov, dv
    if n.get("type") == "INSTANCE":
        sd = n.get("symbolData", {}) or {}
        sym = n.get("overriddenSymbolID") or sd.get("symbolID")
        cov, cdv = dict(ov), dict(dv)
        for o in sd.get("symbolOverrides", []) or []:
            k = key + tuple(render.G(g) for g in o.get("guidPath", {}).get("guids", []))
            vals = {a: b for a, b in o.items() if a != "guidPath"}
            if k in cov:
                vals.update(cov[k])
            cov[k] = vals
        for d in n.get("derivedSymbolData", []) or []:
            k = key + tuple(render.G(g) for g in d.get("guidPath", {}).get("guids", []))
            cdv.setdefault(k, {a: b for a, b in d.items() if a != "guidPath"})
        child_ids = render.kids.get(render.G(sym), []) if sym else []
    for c in child_ids:
        walk(c, list(path) + [c], cov, cdv, mm, out)


def main():
    src, images = sys.argv[1:3]
    render.load(src, images)
    for gid in sys.argv[3:]:
        out = []
        walk(gid, [], {}, {}, (1, 0, 0, 0, 1, 0), out, root=True)
        print(f"== {render.by[gid].get('name')} [{gid}] {render.by[gid]['size']['x']:.0f}x{render.by[gid]['size']['y']:.0f}")
        for y, x, w, h, t in sorted(out):
            print(f"{y:5d} {x:5d} {w:4d}x{h:<3d} | {t}")


if __name__ == "__main__":
    main()
