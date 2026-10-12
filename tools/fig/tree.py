import json, sys
from collections import defaultdict
msg = json.load(open(sys.argv[1]))
nodes = msg["nodeChanges"]
gid = lambda g: f'{g["sessionID"]}:{g["localID"]}'
by = {gid(n["guid"]): n for n in nodes}
kids = defaultdict(list)
for n in nodes:
    p = n.get("parentIndex")
    if p: kids[gid(p["guid"])].append((p["position"], gid(n["guid"])))
for k in kids: kids[k].sort()
maxd = int(sys.argv[2]) if len(sys.argv) > 2 else 3
root = sys.argv[3] if len(sys.argv) > 3 else None
def show(i, d):
    n = by[i]
    sz = n.get("size", {})
    t = n.get("transform", {})
    extra = ""
    if n.get("type") == "TEXT":
        extra = " text=" + repr(n.get("textData", {}).get("characters", "")[:60])
    print("  " * d + f'{n.get("type")} "{n.get("name")}" [{i}] {sz.get("x",0):.0f}x{sz.get("y",0):.0f} @({t.get("m02",0):.0f},{t.get("m12",0):.0f}){extra}' + (f' ({len(kids[i])} kids)' if kids[i] and d >= maxd else ""))
    if d < maxd:
        for _, c in kids[i]: show(c, d + 1)
if root: show(root, 0)
else:
    for n in nodes:
        if n.get("type") == "DOCUMENT": show(gid(n["guid"]), 0)
