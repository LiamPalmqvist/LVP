"""Render frames of a decoded .fig (canvas.json) to SVG.

usage: python3 render.py canvas.json images_dir out_dir [guid ...]
With no guids, renders every top-level frame on Page 1.
Also importable: call load() then render_frame(guid).
"""
import base64, json, math, os, re, struct, sys
from collections import defaultdict

G = lambda g: f'{g["sessionID"]}:{g["localID"]}'
imgdir = None
msg = nodes = blobs = by = kids = None


def load(src, images_dir):
    global imgdir, msg, nodes, blobs, by, kids
    imgdir = images_dir
    msg = json.load(open(src))
    nodes = msg["nodeChanges"]
    blobs = [base64.b64decode(b["bytes"]) for b in msg["blobs"]]
    by = {G(n["guid"]): n for n in nodes}
    k = defaultdict(list)
    for n in nodes:
        p = n.get("parentIndex")
        if p:
            k[G(p["guid"])].append((p["position"], G(n["guid"])))
    kids = {key: [c for _, c in sorted(v)] for key, v in k.items()}


def path_d(idx, sx=1.0, sy=1.0):
    b = blobs[idx]
    i, out = 0, []
    def fl(n):
        nonlocal i
        v = struct.unpack_from("<%df" % n, b, i)
        i += 4 * n
        return v
    while i < len(b):
        c = b[i]; i += 1
        if c == 0:
            if out:
                out.append("Z")
        elif c == 1:
            x, y = fl(2); out.append(f"M{x*sx:.3f} {y*sy:.3f}")
        elif c == 2:
            x, y = fl(2); out.append(f"L{x*sx:.3f} {y*sy:.3f}")
        elif c == 3:
            v = fl(4); out.append("Q" + " ".join(f"{v[j]*(sx if j%2==0 else sy):.3f}" for j in range(4)))
        elif c == 4:
            v = fl(6); out.append("C" + " ".join(f"{v[j]*(sx if j%2==0 else sy):.3f}" for j in range(6)))
        else:
            break
    return " ".join(out)


def mat(t):
    if not t:
        return (1, 0, 0, 0, 1, 0)
    return (t["m00"], t["m01"], t["m02"], t["m10"], t["m11"], t["m12"])


def svgmat(m):
    a, c, e, b, d, f = m
    return f"matrix({a:.5f} {b:.5f} {c:.5f} {d:.5f} {e:.3f} {f:.3f})"


def inv(m):
    a, b, c, d, e, f = m
    det = a * e - b * d
    if abs(det) < 1e-12:
        return (1, 0, 0, 0, 1, 0)
    return (e / det, -b / det, (b * f - c * e) / det, -d / det, a / det, (c * d - a * f) / det)


def rgba(c, op=1.0):
    r, g, b = (max(0, min(255, round(c[k] * 255))) for k in "rgb")
    return f"rgb({r},{g},{b})", c.get("a", 1.0) * op


imgcache = {}
def image_href(h):
    if h in imgcache:
        return imgcache[h]
    p = os.path.join(imgdir, h)
    if not os.path.exists(p):
        imgcache[h] = None
        return None
    data = open(p, "rb").read()
    mime = "image/png" if data[:4] == b"\x89PNG" else "image/jpeg"
    imgcache[h] = f"data:{mime};base64," + base64.b64encode(data).decode()
    return imgcache[h]


class Ctx:
    def __init__(self):
        self.defs = []
        self.n = 0
    def uid(self, p="d"):
        self.n += 1
        return f"{p}{self.n}"


def style_fill(guid_ref):
    if not guid_ref or "guid" not in guid_ref:
        return None
    s = by.get(G(guid_ref["guid"]))
    return s.get("fillPaints") if s else None


def paint_attr(ctx, paint, size, kind="fill"):
    """Return (attr string) for a paint, or None if invisible."""
    if paint.get("visible") is False:
        return None
    op = paint.get("opacity", 1.0)
    t = paint.get("type")
    if t == "SOLID":
        col, a = rgba(paint["color"], op)
        return f'{kind}="{col}" {kind}-opacity="{a:.3f}"'
    if t and t.startswith("GRADIENT"):
        gid = ctx.uid("g")
        w, h = size
        T = inv(mat(paint.get("transform")))
        S = (w, 0, 0, 0, h, 0)
        a, b, c, d, e, f = T
        # compose S*T
        M = (S[0] * a, S[0] * b, S[0] * c, S[4] * d, S[4] * e, S[4] * f)
        stops = "".join(
            f'<stop offset="{s["position"]:.4f}" stop-color="{rgba(s["color"])[0]}" stop-opacity="{s["color"].get("a",1)*op:.3f}"/>'
            for s in paint.get("stops", []))
        if t == "GRADIENT_LINEAR":
            ctx.defs.append(f'<linearGradient id="{gid}" gradientUnits="userSpaceOnUse" x1="0" y1="0.5" x2="1" y2="0.5" gradientTransform="{svgmat(M)}">{stops}</linearGradient>')
        else:
            ctx.defs.append(f'<radialGradient id="{gid}" gradientUnits="userSpaceOnUse" cx="0.5" cy="0.5" r="0.5" gradientTransform="{svgmat(M)}">{stops}</radialGradient>')
        return f'{kind}="url(#{gid})"'
    return None


def image_fill(ctx, paint, size, clip_d):
    if paint.get("visible") is False:
        return ""
    img = paint.get("image", {})
    h = img.get("hash")
    if isinstance(h, str):
        h = base64.b64decode(h).hex()
    elif isinstance(h, list):
        h = bytes(h).hex()
    href = image_href(h) if h else None
    w, hgt = size
    if not href:
        return f'<path d="{clip_d}" fill="#444"/>'
    cid = ctx.uid("c")
    ctx.defs.append(f'<clipPath id="{cid}"><path d="{clip_d}"/></clipPath>')
    mode = paint.get("imageScaleMode", "FILL")
    par = {"FILL": "xMidYMid slice", "FIT": "xMidYMid meet"}.get(mode, "none")
    op = paint.get("opacity", 1.0)
    return f'<g clip-path="url(#{cid})" opacity="{op:.3f}"><image href="{href}" x="0" y="0" width="{w:.2f}" height="{hgt:.2f}" preserveAspectRatio="{par}"/></g>'


def rect_d(n, size):
    w, h = size
    r = n.get("cornerRadius", 0) or 0
    if n.get("rectangleCornerRadiiIndependent"):
        tl = n.get("rectangleTopLeftCornerRadius", 0); tr = n.get("rectangleTopRightCornerRadius", 0)
        br = n.get("rectangleBottomRightCornerRadius", 0); bl = n.get("rectangleBottomLeftCornerRadius", 0)
    else:
        tl = tr = br = bl = r
    m = min(w, h) / 2
    tl, tr, br, bl = (min(x, m) for x in (tl, tr, br, bl))
    return (f"M{tl} 0 L{w-tr} 0 Q{w} 0 {w} {tr} L{w} {h-br} Q{w} {h} {w-br} {h} "
            f"L{bl} {h} Q0 {h} 0 {h-bl} L0 {tl} Q0 0 {tl} 0 Z")


def render_node(ctx, nid, path, ov, dv, out):
    base = by[nid]
    n = dict(base)
    key = tuple(path)
    if key in ov:
        n.update(ov[key])
    if key in dv:
        n.update(dv[key])
    if n.get("visible") is False:
        return
    if n.get("type") in ("VARIABLE", "VARIABLE_SET", "BRUSH"):
        return
    size = (n.get("size", {}).get("x", 0), n.get("size", {}).get("y", 0))
    m = mat(n.get("transform"))
    attrs = [f'transform="{svgmat(m)}"'] if m != (1, 0, 0, 0, 1, 0) else []
    op = n.get("opacity", 1.0)
    if op < 1:
        attrs.append(f'opacity="{op:.3f}"')
    # effects (drop shadow only)
    effs = [e for e in n.get("effects", []) or [] if e.get("visible", True) and e.get("type") == "DROP_SHADOW"]
    filt = ""
    if effs:
        fid = ctx.uid("f")
        parts = "".join(
            f'<feDropShadow dx="{e["offset"]["x"]}" dy="{e["offset"]["y"]}" stdDeviation="{e["radius"]/2}" flood-color="{rgba(e["color"])[0]}" flood-opacity="{e["color"].get("a",1):.3f}"/>'
            for e in effs)
        ctx.defs.append(f'<filter id="{fid}" x="-50%" y="-50%" width="200%" height="200%">{parts}</filter>')
        filt = f' filter="url(#{fid})"'
    out.append(f'<g {" ".join(attrs)}{filt}><!-- {n.get("type")} {n.get("name","")} -->')

    t = n.get("type")
    fills = n.get("fillPaints")
    if fills is None or ("styleIdForFill" in (ov.get(key) or {}) and "fillPaints" not in (ov.get(key) or {})):
        sf = style_fill(n.get("styleIdForFill"))
        if sf is not None:
            fills = sf
    fills = fills or []
    strokes = n.get("strokePaints") or []
    if "styleIdForStrokeFill" in (ov.get(key) or {}) and "strokePaints" not in (ov.get(key) or {}):
        sf = style_fill(n.get("styleIdForStrokeFill"))
        if sf is not None:
            strokes = sf

    if t == "TEXT":
        dt = n.get("derivedTextData") or {}
        glyphs = dt.get("glyphs")
        if "textData" in ov.get(key, {}) and "derivedTextData" not in dv.get(key, {}) and ov[key]["textData"].get("characters") != (base.get("textData") or {}).get("characters"):
            glyphs = None
        paint = next((paint_attr(ctx, p, size) for p in fills if p.get("type") != "IMAGE" and paint_attr(ctx, p, size)), 'fill="#fff"')
        if glyphs:
            ds = []
            for gl in glyphs:
                if "commandsBlob" not in gl:
                    continue
                fs = gl.get("fontSize", n.get("fontSize", 14))
                px, py = gl["position"]["x"], gl["position"]["y"]
                ds.append(f'<path transform="translate({px:.3f} {py:.3f}) scale({fs} {-fs})" d="{path_d(gl["commandsBlob"])}"/>')
            out.append(f'<g {paint}>' + "".join(ds) + "</g>")
        else:
            txt = (n.get("textData") or {}).get("characters", "")
            fs = n.get("fontSize", 14)
            txt = txt.replace("&", "&amp;").replace("<", "&lt;")
            out.append(f'<text x="0" y="{fs*0.95:.1f}" font-family="Helvetica" font-size="{fs}" {paint}>{txt}</text>')
        out.append("</g>")
        return

    geo = n.get("fillGeometry")
    shape_d = None
    if geo:
        shape_d = " ".join(path_d(g["commandsBlob"]) for g in geo if "commandsBlob" in g)
    elif t in ("FRAME", "ROUNDED_RECTANGLE", "RECTANGLE", "SYMBOL", "INSTANCE") and size[0] > 0:
        shape_d = rect_d(n, size)
    elif t == "ELLIPSE":
        w, h = size
        shape_d = f"M0 {h/2} A{w/2} {h/2} 0 1 0 {w} {h/2} A{w/2} {h/2} 0 1 0 0 {h/2} Z"
    is_mask = n.get("mask")
    if shape_d and not is_mask:
        rule = ' fill-rule="evenodd"' if geo and geo[0].get("windingRule") == "ODD" else ""
        for p in fills:
            if p.get("type") == "IMAGE":
                out.append(image_fill(ctx, p, size, shape_d))
            else:
                a = paint_attr(ctx, p, size)
                if a:
                    out.append(f'<path d="{shape_d}" {a}{rule}/>')
    sgeo = n.get("strokeGeometry")
    if sgeo and strokes and not is_mask:
        sd = " ".join(path_d(g["commandsBlob"]) for g in sgeo if "commandsBlob" in g)
        for p in strokes:
            a = paint_attr(ctx, p, size)
            if a:
                out.append(f'<path d="{sd}" {a}/>')

    if t == "BOOLEAN_OPERATION":
        out.append("</g>")
        return

    # children
    child_ov, child_dv = ov, dv
    child_ids = kids.get(nid, [])
    child_prefix = list(path)
    if t == "INSTANCE":
        sd = n.get("symbolData", {}) or {}
        sym = n.get("overriddenSymbolID") or sd.get("symbolID")
        child_ov = dict(ov); child_dv = dict(dv)
        for o in sd.get("symbolOverrides", []) or []:
            gp = tuple(G(g) for g in o.get("guidPath", {}).get("guids", []))
            k = key + gp
            vals = {kk: vv for kk, vv in o.items() if kk != "guidPath"}
            if k in child_ov:
                merged = dict(vals); merged.update(child_ov[k]); child_ov[k] = merged
            else:
                child_ov[k] = vals
        for d in n.get("derivedSymbolData", []) or []:
            gp = tuple(G(g) for g in d.get("guidPath", {}).get("guids", []))
            k = key + gp
            vals = {kk: vv for kk, vv in d.items() if kk != "guidPath"}
            if k not in child_dv:
                child_dv[k] = vals
        child_ids = kids.get(G(sym), []) if sym else []
    clip = t in ("FRAME", "INSTANCE", "SYMBOL") and not n.get("frameMaskDisabled", False) and shape_d
    if clip:
        cid = ctx.uid("c")
        ctx.defs.append(f'<clipPath id="{cid}"><path d="{shape_d}"/></clipPath>')
        out.append(f'<g clip-path="url(#{cid})">')
    mask_clip = None
    for c in child_ids:
        cp = child_prefix + [c]
        cn = by[c]
        cov = child_ov.get(tuple(cp), {})
        if (cov.get("mask") if "mask" in cov else cn.get("mask")) and cov.get("visible", cn.get("visible", True)) is not False:
            # use mask geometry as a clip for following siblings
            sub = []
            render_node_geom(c, cp, child_ov, child_dv, sub)
            if sub:
                mid = ctx.uid("m")
                ctx.defs.append(f'<clipPath id="{mid}">{"".join(sub)}</clipPath>')
                if mask_clip:
                    out.append("</g>")
                out.append(f'<g clip-path="url(#{mid})">')
                mask_clip = mid
            continue
        render_node(ctx, c, cp, child_ov, child_dv, out)
    if mask_clip:
        out.append("</g>")
    if clip:
        out.append("</g>")
    out.append("</g>")


def render_node_geom(nid, path, ov, dv, out):
    n = dict(by[nid]); key = tuple(path)
    n.update(ov.get(key, {})); n.update(dv.get(key, {}))
    size = (n.get("size", {}).get("x", 0), n.get("size", {}).get("y", 0))
    geo = n.get("fillGeometry")
    d = " ".join(path_d(g["commandsBlob"]) for g in geo if "commandsBlob" in g) if geo else rect_d(n, size)
    out.append(f'<path transform="{svgmat(mat(n.get("transform")))}" d="{d}"/>')


def render_frame(fid):
    n = by[fid]
    ctx = Ctx()
    w, h = n["size"]["x"], n["size"]["y"]
    body = []
    t = dict(n)
    # place at origin
    saved = n.get("transform")
    by[fid] = dict(n); by[fid]["transform"] = None
    render_node(ctx, fid, [], {}, {}, body)
    by[fid]["transform"] = saved
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
           f'width="{w:.0f}" height="{h:.0f}" viewBox="0 0 {w:.2f} {h:.2f}"><defs>{"".join(ctx.defs)}</defs>{"".join(body)}</svg>')
    return svg


if __name__ == "__main__":
    src, images, outdir = sys.argv[1:4]
    load(src, images)
    want = sys.argv[4:] or kids["0:1"]
    os.makedirs(outdir, exist_ok=True)
    for fid in want:
        n = by[fid]
        if n.get("type") == "TEXT":
            continue
        name = re.sub(r"[^A-Za-z0-9._-]+", "_", n.get("name", "x"))
        fn = os.path.join(outdir, f"{name}__{fid.replace(':','-')}.svg")
        open(fn, "w").write(render_frame(fid))
        print(fn)
