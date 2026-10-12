# Figma tools

Scripts for reading `design/vlc-redesign.fig` without Figma. Python 3, standard
library only except `decode.py`, which needs `zstandard` (run it with `uv`).

```bash
# 1. Decode the .fig into JSON and extract its images (once).
unzip -o design/vlc-redesign.fig -d /tmp/fig
uv run tools/fig/decode.py /tmp/fig/canvas.fig design/canvas.json
cp -r /tmp/fig/images design/images

# 2. Render frames to SVG, then PNG.
python3 tools/fig/render.py design/canvas.json design/images /tmp/svg [guid ...]
for f in /tmp/svg/*.svg; do rsvg-convert "$f" -o design/frames/$(basename "${f%.svg}").png; done

# 3. Inspect a frame.
python3 tools/fig/tree.py design/canvas.json 2            # top-level frames and guids
python3 tools/fig/texts.py design/canvas.json design/images GUID   # visible text with positions
python3 tools/fig/geom.py design/canvas.json design/images GUID 5  # boxes, fills, radii, fonts

# 4. Re-export the icon set into assets/icons.
python3 tools/fig/export_icons.py design/canvas.json design/images assets/icons
```

`render.py` handles frames, component instances with overrides, vector
geometry, text glyphs, images, gradients, clipping and drop shadows. It does
not do background blur or blend modes, and a few colour overrides inside
nested instances come out wrong (toggles may render orange when off); the
PNG exports in the original zip are the colour reference for those.
