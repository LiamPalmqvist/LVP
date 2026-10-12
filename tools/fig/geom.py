"""Dump node geometry for a frame: absolute box, fill, stroke, radius.

usage: python3 geom.py canvas.json images_dir guid [max_depth]

Instances are expanded like texts.py. Lines are indented by depth:
"name TYPE x,y wxh fill=#RRGGBB/a stroke=#..@w r=radius text='...'"
"""
import sys

import render
from texts import mul


def hexpaint(paints):
    out = []
    for p in paints or []:
        if p.get("visible") is False:
            continue
        if p.get("type") == "SOLID":
            c = p["color"]
            a = c.get("a", 1) * p.get("opacity", 1)
            h = "#%02X%02X%02X" % tuple(round(c[k] * 255) for k in "rgb")
            out.append(h + ("" if a >= 0.999 else f"/{a:.2f}"))
        else:
            out.append(p.get("type", "?").lower())
    return ",".join(out)


def walk(nid, path, ov, dv, m, depth, maxd, root=False):
    n = dict(render.by[nid])
    key = tuple(path)
    n.update(ov.get(key, {}))
    n.update(dv.get(key, {}))
    if n.get("visible") is False:
        return
    mm = m if root else mul(m, render.mat(n.get("transform")))
    sz = n.get("size", {})
    fills = n.get("fillPaints")
    if "styleIdForFill" in ov.get(key, {}) and "fillPaints" not in ov.get(key, {}):
        fills = render.style_fill(n.get("styleIdForFill")) or fills
    strokes = n.get("strokePaints")
    bits = [f'{"  " * depth}{n.get("name")} {n.get("type")} {mm[2]:.0f},{mm[5]:.0f} {sz.get("x", 0):.0f}x{sz.get("y", 0):.0f}']
    f = hexpaint(fills)
    if f:
        bits.append(f"fill={f}")
    s = hexpaint(strokes)
    if s and n.get("strokeGeometry"):
        bits.append(f"stroke={s}@{n.get('strokeWeight', 1):g}")
    r = n.get("cornerRadius")
    if r:
        bits.append(f"r={r:g}")
    if n.get("opacity", 1) < 1:
        bits.append(f"op={n['opacity']:.2f}")
    if n.get("type") == "TEXT":
        fn = n.get("fontName", {})
        bits.append(f"font={fn.get('style')}/{n.get('fontSize'):g}")
        bits.append("text=" + repr((n.get("textData") or {}).get("characters", "")[:40]))
    if n.get("stackMode"):
        bits.append(f"auto={n['stackMode']} gap={n.get('stackSpacing', 0):g} pad={n.get('stackHorizontalPadding', 0):g},{n.get('stackVerticalPadding', 0):g}")
    print(" ".join(bits))
    if depth >= maxd or n.get("type") in ("TEXT", "BOOLEAN_OPERATION", "VECTOR"):
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
        walk(c, list(path) + [c], cov, cdv, mm, depth + 1, maxd)


if __name__ == "__main__":
    src, images, gid = sys.argv[1:4]
    maxd = int(sys.argv[4]) if len(sys.argv) > 4 else 4
    render.load(src, images)
    walk(gid, [], {}, {}, (1, 0, 0, 0, 1, 0), 0, maxd, root=True)
