# /// script
# dependencies = ["zstandard"]
# ///
"""Decode a Figma .fig (fig-kiwi) canvas into JSON."""
import json, struct, sys, zlib, base64
import zstandard

def decompress(b):
    if b[:4] == b"\x28\xb5\x2f\xfd":
        return zstandard.ZstdDecompressor().decompress(b, max_output_size=1 << 30)
    return zlib.decompress(b, -15)

class R:
    def __init__(s, b): s.b, s.i = b, 0
    def byte(s):
        v = s.b[s.i]; s.i += 1; return v
    def vu(s):
        r = sh = 0
        while True:
            c = s.byte(); r |= (c & 127) << sh; sh += 7
            if not c & 128 or sh >= 35: return r
    def vi(s):
        v = s.vu(); return ~(v >> 1) if v & 1 else v >> 1
    def vu64(s):
        r = sh = 0
        while True:
            c = s.byte(); r |= (c & 127) << sh; sh += 7
            if not c & 128 or sh >= 70: return r
    def vi64(s):
        v = s.vu64(); return ~(v >> 1) if v & 1 else v >> 1
    def f(s):
        first = s.byte()
        if first == 0: return 0.0
        bits = first | (s.byte() << 8) | (s.byte() << 16) | (s.byte() << 24)
        bits = ((bits << 23) | (bits >> 9)) & 0xFFFFFFFF
        return struct.unpack("<f", struct.pack("<I", bits))[0]
    def s(s):
        e = s.b.index(0, s.i); v = s.b[s.i:e].decode("utf8", "replace"); s.i = e + 1; return v

def schema(b):
    r = R(b); defs = []
    for _ in range(r.vu()):
        name = r.s(); kind = r.byte(); fields = []
        for _ in range(r.vu()):
            fields.append(dict(name=r.s(), type=r.vi(), arr=r.byte(), val=r.vu()))
        defs.append(dict(name=name, kind=kind, fields=fields))
    return defs

def decode(defs, data, root="Message"):
    r = R(data)
    byname = {d["name"]: d for d in defs}
    def val(t, arr):
        if arr:
            n = r.vu()
            if t == -2: v = bytes(r.b[r.i:r.i + n]); r.i += n; return base64.b64encode(v).decode()
            return [val(t, 0) for _ in range(n)]
        if t == -1: return bool(r.byte())
        if t == -2: return r.byte()
        if t == -3: return r.vi()
        if t == -4: return r.vu()
        if t == -5: return r.f()
        if t == -6: return r.s()
        if t == -7: return r.vi64()
        if t == -8: return r.vu64()
        d = defs[t]
        if d["kind"] == 0:
            v = r.vu()
            for f in d["fields"]:
                if f["val"] == v: return f["name"]
            return v
        if d["kind"] == 1:
            return {f["name"]: val(f["type"], f["arr"]) for f in d["fields"]}
        out = {}
        while True:
            fid = r.vu()
            if fid == 0: return out
            f = next(f for f in d["fields"] if f["val"] == fid)
            out[f["name"]] = val(f["type"], f["arr"])
    d = byname[root]
    return val(defs.index(d), 0)

b = open(sys.argv[1], "rb").read()
assert b[:8] == b"fig-kiwi"
i = 12; chunks = []
while i < len(b):
    n = struct.unpack("<I", b[i:i + 4])[0]; chunks.append(b[i + 4:i + 4 + n]); i += 4 + n
defs = schema(decompress(chunks[0]))
msg = decode(defs, decompress(chunks[1]))
json.dump(msg, open(sys.argv[2], "w"))
json.dump(defs, open(sys.argv[2] + ".schema.json", "w"))
print("nodes:", len(msg.get("nodeChanges", [])), "blobs:", len(msg.get("blobs", [])))
