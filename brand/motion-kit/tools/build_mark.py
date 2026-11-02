#!/usr/bin/env python3
"""LastHarness mark: "appear and glance" as a real keyframed Lottie animation.

Everything is authored on the 64-unit brand grid and scaled by S into a 512x512 comp.
Run:  python3 build_mark.py            -> writes lottie/*.json for every palette
Edit the BEATS section to retime the animation; every keyframe has an explicit easing.
"""
import json, os, sys

S = 8                    # 64 grid -> 512 px
FPS = 60
W = H = 64 * S

# ---------- palettes (face, eyes, slot, ring) ----------
PALETTES = {
    "grove-dark":   dict(bg="#15251c", face="#b2c27c", eye="#15251c", slot="#e0a050"),
    "grove-light":  dict(bg="#f2ead6", face="#15251c", eye="#f2ead6", slot="#e0a050"),
    "tokyo-night":  dict(bg="#1a1b26", face="#c0caf5", eye="#1a1b26", slot="#7aa2f7"),
    "mono-black":   dict(bg="#ffffff", face="#000000", eye="#ffffff", slot="#000000"),
}

# ---------- easing (cubic-bezier, like CSS) ----------
EASE = {
    "linear":   ((0.0, 0.0), (1.0, 1.0)),
    "inOut":    ((0.65, 0.0), (0.35, 1.0)),
    "out":      ((0.22, 1.0), (0.36, 1.0)),
    "in":       ((0.55, 0.0), (1.0, 0.45)),
    "outBack":  ((0.34, 1.56), (0.64, 1.0)),   # overshoot
    "inBack":   ((0.36, 0.0), (0.66, -0.56)),  # pull back before leaving
    "hold":     None,
}

def hexrgb(h):
    h = h.lstrip("#"); return [int(h[i:i+2], 16) / 255 for i in (0, 2, 4)] + [1]

def anim(keys):
    """keys: list of (frame, value, ease_to_next). value is a number or list."""
    if len(keys) == 1:
        v = keys[0][1]; return {"a": 0, "k": v}
    out = []
    for i, (f, v, e) in enumerate(keys):
        v = v if isinstance(v, list) else [v]
        k = {"t": f, "s": v}
        if i < len(keys) - 1:
            if e == "hold":
                k["h"] = 1
            else:
                (ox, oy), (ix, iy) = EASE[e]
                n = len(v)
                k["o"] = {"x": [ox] * n, "y": [oy] * n}
                k["i"] = {"x": [ix] * n, "y": [iy] * n}
        out.append(k)
    return {"a": 1, "k": out}

def st(a=None, p=None, s=None, r=None, o=None):
    return {"a": a or {"a": 0, "k": [0, 0]}, "p": p or {"a": 0, "k": [0, 0]},
            "s": s or {"a": 0, "k": [100, 100]}, "r": r or {"a": 0, "k": 0},
            "o": o or {"a": 0, "k": 100}, "sk": {"a": 0, "k": 0}, "sa": {"a": 0, "k": 0}}

def g(v):  # grid -> px
    return [x * S for x in v] if isinstance(v, (list, tuple)) else v * S

# ======================================================================================
# BEATS  (frames at 60 fps).  0-84 = spawn (plays once).  84-492 = loop.
# ======================================================================================
SPAWN_END, LOOP_END = 84, 492
MARKERS = [
    ("spawn", 0, 84), ("loop", 84, 408),
    ("slot-in", 126, 33), ("glance", 151, 30), ("hold", 181, 101),
    ("slot-out", 282, 27), ("look-back", 294, 66), ("idle", 360, 132),
]

# Face: anchored at its bottom centre (27,56) so squash and lean pivot on the "floor".
FACE_PIVOT = g([27, 56])
face_scale = [
    (0, [0, 0], "out"), (18, [116, 86], "inOut"), (28, [92, 110], "inOut"),
    (38, [104, 96], "inOut"), (48, [100, 100], "hold"),
    # surprise when the slot appears: stretch up
    (131, [100, 100], "out"), (141, [97, 105], "inOut"), (151, [100, 100], "hold"),
    # small squash when turning back to us
    (336, [100, 100], "out"), (346, [104, 95], "inOut"), (360, [100, 100], "hold"),
    (LOOP_END, [100, 100], "hold"),
]
face_opacity = [(0, 0, "out"), (8, 100, "hold"), (LOOP_END, 100, "hold")]
# lean toward the slot while glancing (rotation about the floor pivot + a little lift)
face_rot = [(0, 0, "hold"), (159, 0, "inOut"), (181, -3.5, "hold"), (294, -3.5, "inOut"),
            (330, 0, "hold"), (LOOP_END, 0, "hold")]
B = 4  # idle bob, px
face_pos = [
    (0, FACE_PIVOT, "hold"), (84, FACE_PIVOT, "inOut"),
    (105, [FACE_PIVOT[0], FACE_PIVOT[1] - B], "inOut"), (126, FACE_PIVOT, "hold"),
    (159, FACE_PIVOT, "inOut"), (181, [FACE_PIVOT[0] + 7, FACE_PIVOT[1] - 6], "hold"),
    (294, [FACE_PIVOT[0] + 7, FACE_PIVOT[1] - 6], "inOut"), (330, FACE_PIVOT, "hold"),
    (380, FACE_PIVOT, "inOut"), (410, [FACE_PIVOT[0], FACE_PIVOT[1] - B], "inOut"),
    (440, FACE_PIVOT, "inOut"), (470, [FACE_PIVOT[0], FACE_PIVOT[1] - B], "inOut"),
    (LOOP_END, FACE_PIVOT, "hold"),
]

# Eye group (child of the face): the glance = translate (2.4,-2) + rotate -8 about the face centre (27,37).
EYE_PIVOT = g([27, 37])
def eoff(e): return [EYE_PIVOT[0] + 2.4 * S * e, EYE_PIVOT[1] - 2 * S * e]
eyes_pos = [
    (0, eoff(0), "hold"), (151, eoff(0), "inOut"), (159, eoff(-0.08), "inOut"),   # anticipation
    (181, eoff(1), "hold"), (294, eoff(1), "out"), (309, eoff(1.14), "inOut"),     # look for the slot
    (336, eoff(-0.14), "inOut"), (354, eoff(0), "hold"), (LOOP_END, eoff(0), "hold"),
]
eyes_rot = [
    (0, 0, "hold"), (151, 0, "inOut"), (159, 0.64, "inOut"), (181, -8, "hold"),
    (294, -8, "out"), (309, -9.1, "inOut"), (336, 1.1, "inOut"), (354, 0, "hold"), (LOOP_END, 0, "hold"),
]

# Each eye: rect size (shape), group position (centre), group scale (blinks / surprise).
NEAR_C = [18.6 + 3, 30.6 + 5.5]
FAR_FRONT_C, FAR_GLANCE_C = [28.4 + 3, 30.6 + 5.5], [30.1 + 2.3, 31.4 + 5.1]
near_size = [(0, g([6, 11]), "hold")]
far_size = [(0, g([6, 11]), "hold"), (159, g([6, 11]), "inOut"), (181, g([4.6, 10.2]), "hold"),
            (309, g([4.6, 10.2]), "inOut"), (336, g([6, 11]), "hold"), (LOOP_END, g([6, 11]), "hold")]
far_pos = [(0, g(FAR_FRONT_C), "hold"), (159, g(FAR_FRONT_C), "inOut"), (181, g(FAR_GLANCE_C), "hold"),
           (309, g(FAR_GLANCE_C), "inOut"), (336, g(FAR_FRONT_C), "hold"), (LOOP_END, g(FAR_FRONT_C), "hold")]

def blink(f0, dur=10):
    return [(f0, [100, 100], "in"), (f0 + dur // 2, [100, 8], "out"), (f0 + dur, [100, 100], "hold")]
eye_scale = (
    [(0, [100, 8], "hold"), (37, [100, 8], "outBack"), (48, [100, 100], "hold")]   # open
    + blink(61)
    + [(131, [100, 100], "out"), (141, [112, 118], "inOut"), (151, [100, 100], "hold")]  # surprise
    + blink(228) + blink(426) + blink(439) + [(LOOP_END, [100, 100], "hold")]
)

# Slot: pops in, holds, swells, shrinks away.  Ring: two bursts.
SLOT_C = g([49, 15])
slot_scale = [(0, [0, 0], "hold"), (126, [0, 0], "outBack"), (150, [100, 100], "hold"),
              (282, [100, 100], "out"), (290, [120, 120], "inBack"), (307, [0, 0], "hold"),
              (LOOP_END, [0, 0], "hold")]
ring_size = [(0, g([12, 12]), "hold"), (127, g([12, 12]), "out"), (165, g([30, 30]), "hold"),
             (293, g([8, 8]), "out"), (330, g([28, 28]), "hold"), (LOOP_END, g([28, 28]), "hold")]
ring_op = [(0, 0, "hold"), (127, 75, "out"), (165, 0, "hold"),
           (293, 55, "out"), (330, 0, "hold"), (LOOP_END, 0, "hold")]

# ======================================================================================
def rect_shape(name, size, r, color, pos_anim, scale_anim):
    return {"ty": "gr", "nm": name, "it": [
        {"ty": "rc", "nm": name + "-rect", "d": 1, "p": {"a": 0, "k": [0, 0]}, "s": size,
         "r": {"a": 0, "k": r}},
        {"ty": "fl", "c": {"a": 0, "k": hexrgb(color)}, "o": {"a": 0, "k": 100}, "r": 1},
        {"ty": "tr", **st(p=pos_anim, s=scale_anim)}]}

def layer(ind, name, shapes, ks, parent=None):
    L = {"ddd": 0, "ind": ind, "ty": 4, "nm": name, "sr": 1, "ks": ks, "ao": 0,
         "shapes": shapes, "ip": 0, "op": LOOP_END, "st": 0, "bm": 0}
    if parent: L["parent"] = parent
    return L

def build(pal, bg=False):
    P = PALETTES[pal]
    # eye roundness: full pill. Lottie clamps r to half the short side, so a big value keeps it a pill.
    eyes = layer(2, "eyes", [
        rect_shape("eye-near", anim(near_size), 30, P["eye"], anim([(0, g(NEAR_C), "hold")]), anim(eye_scale)),
        rect_shape("eye-far", anim(far_size), 30, P["eye"], anim(far_pos), anim(eye_scale)),
    ], st(a={"a": 0, "k": EYE_PIVOT}, p=anim(eyes_pos), r=anim(eyes_rot)), parent=3)
    face = layer(3, "face", [{"ty": "gr", "nm": "face", "it": [
        {"ty": "rc", "nm": "face-rect", "d": 1, "p": {"a": 0, "k": g([27, 37])}, "s": {"a": 0, "k": g([38, 38])},
         "r": {"a": 0, "k": g(15)}},
        {"ty": "fl", "c": {"a": 0, "k": hexrgb(P["face"])}, "o": {"a": 0, "k": 100}, "r": 1},
        {"ty": "tr", **st()}]}],
        st(a={"a": 0, "k": FACE_PIVOT}, p=anim(face_pos), s=anim(face_scale), r=anim(face_rot), o=anim(face_opacity)))
    slot = layer(4, "slot", [{"ty": "gr", "nm": "slot", "it": [
        {"ty": "el", "nm": "slot-dot", "d": 1, "p": {"a": 0, "k": [0, 0]}, "s": {"a": 0, "k": g([12, 12])}},
        {"ty": "fl", "c": {"a": 0, "k": hexrgb(P["slot"])}, "o": {"a": 0, "k": 100}, "r": 1},
        {"ty": "tr", **st()}]}],
        st(p={"a": 0, "k": SLOT_C}, s=anim(slot_scale)))
    ring = layer(5, "slot-ring", [{"ty": "gr", "nm": "ring", "it": [
        {"ty": "el", "nm": "ring-ellipse", "d": 1, "p": {"a": 0, "k": [0, 0]}, "s": anim(ring_size)},
        {"ty": "st", "c": {"a": 0, "k": hexrgb(P["slot"])}, "o": {"a": 0, "k": 100}, "w": {"a": 0, "k": 1.4 * S},
         "lc": 2, "lj": 2},
        {"ty": "tr", **st()}]}],
        st(p={"a": 0, "k": SLOT_C}, o=anim(ring_op)))
    layers = [ring, slot, eyes, face]   # first = top
    if bg:
        layers.append({"ddd": 0, "ind": 9, "ty": 1, "nm": "background", "sc": P["bg"], "sw": W, "sh": H,
                       "ks": st(), "ip": 0, "op": LOOP_END, "st": 0, "bm": 0, "ao": 0})
    # renumber so the order is stable
    return {"v": "5.12.2", "fr": FPS, "ip": 0, "op": LOOP_END, "w": W, "h": H, "ddd": 0,
            "nm": f"lastharness-mark-{pal}", "assets": [], "layers": layers,
            "markers": [{"tm": t, "cm": n, "dr": d} for n, t, d in MARKERS],
            "meta": {"g": "LastHarness build_mark.py", "d": "Glance + Slot: appear and glance. 64 grid x8."}}

if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "lottie"
    os.makedirs(out, exist_ok=True)
    for p in PALETTES:
        for bg in (False, True):
            fn = f"{out}/lastharness-mark-{p}{'-bg' if bg else ''}.json"
            json.dump(build(p, bg), open(fn, "w"), separators=(",", ":"))
            print(fn, os.path.getsize(fn), "bytes")
