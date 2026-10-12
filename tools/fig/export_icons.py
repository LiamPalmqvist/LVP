"""Export every variant of the design's Icon component set as a standalone SVG.

usage: python3 export_icons.py canvas.json images_dir out_dir

File names come from the variant properties, dropping any that are "default":
"name=volume, value=high, two-tone=no" becomes volume-high.svg, and two-tone
variants get a "-2tone" suffix. A manifest.json maps file names to the original
variant strings.
"""
import json, os, re, sys

import render

ICON_SET = "42:356"


def slug(variant):
    props = dict(p.split("=", 1) for p in (x.strip() for x in variant.split(",")) if "=" in p)
    parts = [props.get("name", "icon")]
    for k in ("direction", "state", "value", "EQ"):
        v = props.get(k, "default")
        if v != "default":
            parts.append(v)
    if props.get("two-tone") == "yes":
        parts.append("2tone")
    return re.sub(r"[^a-z0-9]+", "-", "-".join(parts).lower()).strip("-")


def main():
    src, images, out = sys.argv[1:4]
    render.load(src, images)
    os.makedirs(out, exist_ok=True)
    manifest, seen = {}, {}
    colors = {}
    for gid in render.kids[ICON_SET]:
        n = render.by[gid]
        if n.get("type") != "SYMBOL":
            continue
        name = slug(n["name"])
        if name in seen:
            seen[name] += 1
            name = f"{name}-{seen[name]}"
        else:
            seen[name] = 1
        svg = render.render_frame(gid)
        for c in re.findall(r'(?:fill|stroke)="(rgb\([^)]*\))"', svg):
            colors[c] = colors.get(c, 0) + 1
        open(os.path.join(out, name + ".svg"), "w").write(svg)
        manifest[name] = n["name"]
    json.dump(manifest, open(os.path.join(out, "manifest.json"), "w"), indent=1, sort_keys=True)
    print(len(manifest), "icons")
    for c, k in sorted(colors.items(), key=lambda x: -x[1]):
        print(k, c)


if __name__ == "__main__":
    main()
