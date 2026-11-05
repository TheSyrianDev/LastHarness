#!/usr/bin/env python3
"""LastHarness motion v1: the mark (spawn, working loop, six moods) and the six worker bots, as Lottie.

Source of truth for out/lottie/. Never hand-edit the JSON; change this file and re-run:
    python3 tools/build_v1.py            (from brand/motion-kit/)
Geometry and colours come from source/tokens.json, source/brand-geometry.js and source/bot-tools.js
(transcribed below; the shapes are not redesigned). Authoring is on the 64-unit grid, scaled x8 into 512 px.
Only shape layers, solids and nulls; no expressions, effects, images, text or precomps.
"""
import json, math, os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))
KIT = os.path.dirname(HERE)
TOK = json.load(open(os.path.join(KIT, "source", "tokens.json")))
S, FPS, W = 8, 60, 512

# ---------------------------------------------------------------------------------------------
# easing (cubic-bezier, CSS style). Each key's ease shapes the move to the NEXT key.
EASE = {
    "linear": ((0, 0), (1, 1)),
    "inOut": ((0.65, 0), (0.35, 1)),
    "sine": ((0.37, 0), (0.63, 1)),          # breathing, soft both ends
    "out": ((0.22, 1), (0.36, 1)),
    "outSoft": ((0.33, 1), (0.68, 1)),
    "in": ((0.55, 0), (1, 0.45)),
    "inQuad": ((0.5, 0), (0.75, 0)),          # falling
    "outBack": ((0.34, 1.56), (0.64, 1)),
    "inBack": ((0.36, 0), (0.66, -0.56)),
}


def rgb(h):
    h = h.lstrip("#")
    return [round(int(h[i:i + 2], 16) / 255, 4) for i in (0, 2, 4)] + [1]


def lerp(a, b, t):
    return [x + (y - x) * t for x, y in zip(a, b)] if isinstance(a, list) else a + (b - a) * t


class Track:
    """Keys are (frame, value, ease_to_next, arc). arc (px) bends a 2D move sideways (spatial tangents)."""

    def __init__(self, v):
        self.k = {0: (v, "hold", None)}

    def key(self, f, v, e="hold", arc=None):
        self.k[round(f)] = (v, e, arc)
        return self

    def move(self, f0, f1, a, b, e="inOut", arc=None):
        self.key(f0, a, e, arc)
        if round(f1) not in self.k or self.k[round(f1)][1] == "hold":
            self.key(f1, b, "hold")
        return self

    def value_at(self, f):
        ks = sorted(self.k.items())
        prev = ks[0]
        for fr, kv in ks:
            if fr <= f:
                prev = (fr, kv)
        return prev[1][0]

    def lottie(self, end):
        ks = sorted(self.k.items())
        if ks[-1][0] < end:
            ks.append((end, (ks[-1][1][0], "hold", None)))
        if len(ks) == 1 or all(kv[0] == ks[0][1][0] for _, kv in ks):
            return {"a": 0, "k": ks[0][1][0]}
        spatial = any(kv[2] is not None for _, kv in ks)
        out = []
        for i, (f, (v, e, arc)) in enumerate(ks):
            vv = v if isinstance(v, list) else [v]
            k = {"t": f, "s": vv}
            if i < len(ks) - 1:
                if e == "hold":
                    k["h"] = 1
                else:
                    (ox, oy), (ix, iy) = EASE[e]
                    n = len(vv)
                    if spatial:     # spatial: a single eased value along a (possibly bent) path
                        n = 1
                    k["o"] = {"x": [ox] * n, "y": [oy] * n}
                    k["i"] = {"x": [ix] * n, "y": [iy] * n}
                    if arc is not None:
                        nv = ks[i + 1][1][0]
                        dx, dy = nv[0] - v[0], nv[1] - v[1]
                        L = math.hypot(dx, dy) or 1
                        px, py = -dy / L * arc * 4 / 3, dx / L * arc * 4 / 3
                        k["to"] = [dx / 3 + px, dy / 3 + py]
                        k["ti"] = [-dx / 3 + px, -dy / 3 + py]
            out.append(k)
        # a spatial key needs neighbours with tangents too (lottie-web reads them pairwise)
        if any("to" in k for k in out):
            for k in out[:-1]:
                if "h" not in k and "to" not in k:
                    k["to"], k["ti"] = [0, 0], [0, 0]
        return {"a": 1, "k": out}


def T(**kw):
    d = {"a": [0, 0], "p": [0, 0], "s": [100, 100], "r": 0, "o": 100}
    d.update(kw)
    return {k: (v if isinstance(v, dict) and "a" in v else {"a": 0, "k": v}) for k, v in d.items()} | \
        {"sk": {"a": 0, "k": 0}, "sa": {"a": 0, "k": 0}}


def g(v):
    return [x * S for x in v] if isinstance(v, (list, tuple)) else v * S


def fill(c, o=100):
    return {"ty": "fl", "c": c if isinstance(c, dict) else {"a": 0, "k": rgb(c)}, "o": o if isinstance(o, dict) else {"a": 0, "k": o}, "r": 1}


def stroke(c, w, o=100, cap=2):
    return {"ty": "st", "c": {"a": 0, "k": rgb(c)}, "o": o if isinstance(o, dict) else {"a": 0, "k": o},
            "w": w if isinstance(w, dict) else {"a": 0, "k": w}, "lc": cap, "lj": 2, "ml": 4}


def grp(name, items, tr=None):
    return {"ty": "gr", "nm": name, "it": items + [{"ty": "tr", **(tr or T())}]}


def rect(p, s, r):
    return {"ty": "rc", "d": 1, "p": p if isinstance(p, dict) else {"a": 0, "k": p},
            "s": s if isinstance(s, dict) else {"a": 0, "k": s}, "r": r if isinstance(r, dict) else {"a": 0, "k": r}}


def ell(p, s):
    return {"ty": "el", "d": 1, "p": p if isinstance(p, dict) else {"a": 0, "k": p}, "s": s if isinstance(s, dict) else {"a": 0, "k": s}}


def shape(verts, ins, outs, closed):
    return {"i": ins, "o": outs, "v": verts, "c": closed}


def sh(k):
    return {"ty": "sh", "d": 1, "ks": k if isinstance(k, dict) and "a" in k else {"a": 0, "k": k}}


# ---------------------------------------------------------------------------------------------
# SVG path data -> Lottie bezier shapes (M L H V Q C A Z, absolute and relative)
def _arc_to_cubic(x1, y1, rx, ry, phi, fa, fs, x2, y2):
    if rx == 0 or ry == 0:
        return [((x1, y1), (x2, y2), (x2, y2))]
    cp, sp = math.cos(math.radians(phi)), math.sin(math.radians(phi))
    dx, dy = (x1 - x2) / 2, (y1 - y2) / 2
    x1p, y1p = cp * dx + sp * dy, -sp * dx + cp * dy
    lam = x1p ** 2 / rx ** 2 + y1p ** 2 / ry ** 2
    if lam > 1:
        rx, ry = rx * math.sqrt(lam), ry * math.sqrt(lam)
    num = rx ** 2 * ry ** 2 - rx ** 2 * y1p ** 2 - ry ** 2 * x1p ** 2
    den = rx ** 2 * y1p ** 2 + ry ** 2 * x1p ** 2
    co = math.sqrt(max(0, num / den)) * (-1 if fa == fs else 1)
    cxp, cyp = co * rx * y1p / ry, -co * ry * x1p / rx
    cx, cy = cp * cxp - sp * cyp + (x1 + x2) / 2, sp * cxp + cp * cyp + (y1 + y2) / 2
    ang = lambda ux, uy, vx, vy: math.atan2(ux * vy - uy * vx, ux * vx + uy * vy)
    t1 = ang(1, 0, (x1p - cxp) / rx, (y1p - cyp) / ry)
    dt = ang((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry)
    if not fs and dt > 0:
        dt -= 2 * math.pi
    elif fs and dt < 0:
        dt += 2 * math.pi
    n = max(1, math.ceil(abs(dt) / (math.pi / 2)))
    d = dt / n
    k = 4 / 3 * math.tan(d / 4)
    segs = []
    for i in range(n):
        a0, a1 = t1 + i * d, t1 + (i + 1) * d
        p = lambda a: (cx + rx * math.cos(a) * cp - ry * math.sin(a) * sp, cy + rx * math.cos(a) * sp + ry * math.sin(a) * cp)
        dp = lambda a: (-rx * math.sin(a) * cp - ry * math.cos(a) * sp, -rx * math.sin(a) * sp + ry * math.cos(a) * cp)
        P0, P1, D0, D1 = p(a0), p(a1), dp(a0), dp(a1)
        segs.append(((P0[0] + k * D0[0], P0[1] + k * D0[1]), (P1[0] - k * D1[0], P1[1] - k * D1[1]), P1))
    return segs


def svg_path(d):
    toks = re.findall(r"[MmLlHhVvQqCcAaZz]|-?\d*\.?\d+(?:e-?\d+)?", d)
    subs, cur, pos, start, i, cmd = [], None, (0, 0), (0, 0), 0, None

    def num():
        nonlocal i
        v = float(toks[i]); i += 1
        return v

    while i < len(toks):
        if re.match(r"[A-Za-z]", toks[i]):
            cmd = toks[i]; i += 1
        rel = cmd.islower()
        C = cmd.upper()
        ox, oy = pos if rel else (0, 0)
        if C == "M":
            pos = (num() + ox, num() + oy); start = pos
            cur = {"v": [pos], "i": [(0, 0)], "o": [], "c": False}; subs.append(cur)
            cmd = "l" if rel else "L"
        elif C in "LHV":
            if C == "L": p = (num() + ox, num() + oy)
            elif C == "H": p = (num() + (pos[0] if rel else 0), pos[1])
            else: p = (pos[0], num() + (pos[1] if rel else 0))
            cur["o"].append((0, 0)); cur["v"].append(p); cur["i"].append((0, 0)); pos = p
        elif C == "Q":
            q = (num() + ox, num() + oy); p = (num() + ox, num() + oy)
            c1 = (pos[0] + 2 / 3 * (q[0] - pos[0]), pos[1] + 2 / 3 * (q[1] - pos[1]))
            c2 = (p[0] + 2 / 3 * (q[0] - p[0]), p[1] + 2 / 3 * (q[1] - p[1]))
            cur["o"].append((c1[0] - pos[0], c1[1] - pos[1])); cur["v"].append(p); cur["i"].append((c2[0] - p[0], c2[1] - p[1])); pos = p
        elif C == "C":
            c1 = (num() + ox, num() + oy); c2 = (num() + ox, num() + oy); p = (num() + ox, num() + oy)
            cur["o"].append((c1[0] - pos[0], c1[1] - pos[1])); cur["v"].append(p); cur["i"].append((c2[0] - p[0], c2[1] - p[1])); pos = p
        elif C == "A":
            rx, ry, phi, fa, fs = num(), num(), num(), num(), num(); p = (num() + ox, num() + oy)
            for c1, c2, e in _arc_to_cubic(pos[0], pos[1], rx, ry, phi, int(fa), int(fs), p[0], p[1]):
                cur["o"].append((c1[0] - pos[0], c1[1] - pos[1])); cur["v"].append(e); cur["i"].append((c2[0] - e[0], c2[1] - e[1])); pos = e
        elif C == "Z":
            cur["c"] = True
            if math.dist(cur["v"][-1], cur["v"][0]) < 1e-6 and len(cur["v"]) > 1:
                cur["i"][0] = cur["i"].pop(); cur["v"].pop()
            pos = start
    out = []
    for s_ in subs:
        while len(s_["o"]) < len(s_["v"]):
            s_["o"].append((0, 0))
        out.append(shape([list(v) for v in s_["v"]], [list(v) for v in s_["i"]], [list(v) for v in s_["o"]], s_["c"]))
    return out


def path_items(d, scale=1.0):
    res = []
    for s_ in svg_path(d):
        for kk in ("i", "o", "v"):
            s_[kk] = [[round(a * scale, 3), round(b * scale, 3)] for a, b in s_[kk]]
        res.append(sh(s_))
    return res


# ---------------------------------------------------------------------------------------------
# THE MARK
PALS = TOK["palettes"]
MARK_PALETTES = {
    "grove-dark": dict(bg=PALS["grove"]["forest"], face=PALS["grove"]["lichen"], eye=PALS["grove"]["forest"],
                       slot=PALS["grove"]["busy"], busy=PALS["grove"]["busy"], ready=PALS["grove"]["ready"],
                       needs=PALS["grove"]["needsYou"], dim=PALS["grove"]["moss"]),
    "grove-light": dict(bg=PALS["grove"]["cream"], face=PALS["grove"]["forest"], eye=PALS["grove"]["cream"],
                        slot=PALS["grove"]["busy"], busy=PALS["grove"]["busy"], ready=PALS["grove"]["ready"],
                        needs=PALS["grove"]["needsYou"], dim=PALS["grove"]["moss"]),
    # tokyo-night keeps v0's slot (the theme accent, as markColours.omarchy says: slot = accent)
    "tokyo-night": dict(bg=PALS["tokyoNight"]["forest"], face=PALS["tokyoNight"]["cream"], eye=PALS["tokyoNight"]["forest"],
                        slot=PALS["tokyoNight"]["lichen"], busy=PALS["tokyoNight"]["busy"], ready=PALS["tokyoNight"]["ready"],
                        needs=PALS["tokyoNight"]["needsYou"], dim=PALS["tokyoNight"]["moss"]),
    # one colour only: status lives in motion and opacity, never in a new colour
    "mono-black": dict(bg="#ffffff", face="#000000", eye="#ffffff", slot="#000000", busy="#000000",
                       ready="#000000", needs="#000000", dim="#000000"),
}

M = TOK["mark"]
FACE_PIVOT = g([27, 56])
EYE_PIVOT = g([27, 37])
NEAR_C = [M["eyeNear"]["x"] + 3, M["eyeNear"]["y"] + 5.5]
FAR_C = [M["eyeFarFront"]["x"] + 3, M["eyeFarFront"]["y"] + 5.5]
FAR_GC = [M["eyeFarGlance"]["x"] + 2.3, M["eyeFarGlance"]["y"] + 5.1]
EYE_S, FAR_GS = [6, 11], [M["eyeFarGlance"]["w"], M["eyeFarGlance"]["h"]]
WIDE_S, WIDE_N, WIDE_F = [7, 12.6], [17.8 + 3.5, 29.4 + 6.3], [29.2 + 3.5, 29.4 + 6.3]   # 'ask' eyes
GL = M["glance"]


def eyes_at(e, extra_rot=0.0):
    """Eye-group pose for a glance amount e (0 = at you, 1 = logo glance)."""
    return [EYE_PIVOT[0] + GL["translate"][0] * S * e, EYE_PIVOT[1] + GL["translate"][1] * S * e], GL["rotate"] * e + extra_rot


THINK = ([EYE_PIVOT[0] + 1.4 * S, EYE_PIVOT[1] - 3.6 * S], -4)          # brand-geometry 'think'
THINK_SLOT = ([EYE_PIVOT[0] + 2.6 * S, EYE_PIVOT[1] - 3.2 * S], -8)     # up and toward the slot

SEGMENTS = []          # (name, start, length)
END = 0


def build_mark_tracks(P):
    tr = {
        "face_p": Track(FACE_PIVOT), "face_s": Track([100, 100]), "face_r": Track(0), "face_o": Track(100),
        "eyes_p": Track(EYE_PIVOT), "eyes_r": Track(0),
        "near_c": Track(g(NEAR_C)), "near_s": Track(g(EYE_S)), "near_k": Track([100, 100]),
        "far_c": Track(g(FAR_C)), "far_s": Track(g(EYE_S)), "far_k": Track([100, 100]),
        "pill_o": Track(100),
        "happy_o": Track(0), "happy_k": Track([100, 100]),
        "sleep_o": Track(0), "x_o": Track(0), "x_k": Track([100, 100]),
        "slot_s": Track([0, 0]), "slot_c": Track(rgb(P["slot"])), "slot_o": Track(100),
        "ring_s": Track(g([12, 12])), "ring_o": Track(0),
    }
    R = tr
    segs = []

    def blink(f, close=3, open_=6, lag=1, depth=8):
        for k, d in (("near_k", 0), ("far_k", lag)):
            R[k].key(f + d, [100, 100], "in").key(f + d + close, [100, depth], "out").key(f + d + close + open_, [100, 100])

    def neutral(f):   # pin every track to the neutral pose at frame f (start/end of each segment)
        for k, v in (("face_p", FACE_PIVOT), ("face_s", [100, 100]), ("face_r", 0), ("face_o", 100),
                     ("eyes_p", EYE_PIVOT), ("eyes_r", 0), ("near_c", g(NEAR_C)), ("near_s", g(EYE_S)),
                     ("near_k", [100, 100]), ("far_c", g(FAR_C)), ("far_s", g(EYE_S)), ("far_k", [100, 100]),
                     ("pill_o", 100), ("happy_o", 0), ("sleep_o", 0), ("x_o", 0), ("slot_s", [0, 0]),
                     ("slot_o", 100), ("ring_o", 0)):
            if f not in R[k].k:
                R[k].key(f, v)
        if f not in R["slot_c"].k:
            R["slot_c"].key(f, rgb(P["slot"]))

    def swap_eyes(f, to_key, scale_key=None, pop=6):
        """Close the pills, swap to a drawn eye at the closed moment, pop it open."""
        R["near_k"].key(f, [100, 100], "in").key(f + 3, [100, 8])
        R["far_k"].key(f + 1, [100, 100], "in").key(f + 4, [100, 8])
        R["pill_o"].key(f + 4, 100).key(f + 5, 0)
        R[to_key].key(f + 4, 0).key(f + 5, 100)
        if scale_key:
            R[scale_key].key(f + 4, [100, 30], "outBack").key(f + 5 + pop + 4, [100, 100])
        R["near_k"].key(f + 5, [100, 8]); R["far_k"].key(f + 5, [100, 8])

    def unswap_eyes(f, from_key, scale_key=None):
        if scale_key:
            R[scale_key].key(f, [100, 100], "in").key(f + 4, [100, 30])
        R[from_key].key(f + 4, 100).key(f + 5, 0)
        R["pill_o"].key(f + 4, 0).key(f + 5, 100)
        R["near_k"].key(f + 4, [100, 8]).key(f + 5, [100, 8], "outBack").key(f + 14, [100, 100])
        R["far_k"].key(f + 5, [100, 8]).key(f + 6, [100, 8], "outBack").key(f + 15, [100, 100])

    def slot_in(f, dur=20, colour=None):
        R["slot_s"].key(f, [0, 0], "out").key(f + dur * 0.7, [108, 108], "inOut").key(f + dur, [97, 97], "inOut").key(f + dur + 7, [100, 100])
        if colour:
            R["slot_c"].key(f, rgb(colour))

    def slot_out(f, dur=18):
        R["slot_s"].key(f, [100, 100], "out").key(f + 6, [116, 116], "inBack").key(f + dur, [0, 0])

    def recolour(f0, f1, a, b):
        R["slot_c"].key(f0, rgb(a), "inOut").key(f1, rgb(b))

    # ---------------- spawn 0-84: drop in, land, squash, settle, eyes open, first blink
    R["face_o"].key(0, 0, "out").key(6, 100)
    R["face_p"].key(0, [FACE_PIVOT[0], FACE_PIVOT[1] - 3 * S], "inQuad").key(14, FACE_PIVOT, "hold")
    R["face_s"].key(0, [55, 70], "inQuad").key(14, [88, 114], "out")            # falling: stretched
    R["face_s"].key(18, [118, 84], "inOut").key(28, [94, 108], "inOut").key(37, [103, 97], "inOut").key(46, [99, 101], "inOut").key(52, [100, 100])
    for k, lag in (("near_k", 0), ("far_k", 1)):
        R[k].key(0, [100, 8]).key(30 + lag, [100, 8], "outBack").key(42 + lag, [100, 100])
    blink(62)
    neutral(84)
    segs.append(("spawn", 0, 84))

    # ---------------- working 84-492: the polished appear-and-glance loop
    b = 84
    R["face_p"].key(b, FACE_PIVOT, "sine").key(105, [FACE_PIVOT[0], FACE_PIVOT[1] - 4], "sine").key(126, FACE_PIVOT)
    # slot pops in with a secondary bounce; ring burst
    slot_in(126, 22)
    R["ring_s"].key(127, g([12, 12]), "out").key(165, g([30, 30]))
    R["ring_o"].key(127, 75, "out").key(165, 0)
    # surprise: face stretches up, eyes widen a touch (eyes lead by 2 frames)
    R["face_s"].key(133, [100, 100], "out").key(142, [97, 105], "inOut").key(153, [100, 100])
    for k in ("near_k", "far_k"):
        R[k].key(131, [100, 100], "out").key(139, [112, 118], "inOut").key(151, [100, 100])
    # anticipation (subtle) then the glance on an arc; eyes overshoot and settle after the face
    p0, r0 = eyes_at(0); pa, ra = eyes_at(-0.1, 0.8); p1, r1 = eyes_at(1.05); p2, r2 = eyes_at(1)
    R["eyes_p"].key(151, p0, "inOut").key(160, pa, "inOut", arc=-3).key(179, p1, "inOut").key(190, p2)
    R["eyes_r"].key(151, r0, "inOut").key(160, ra, "inOut").key(179, r1, "inOut").key(190, r2)
    R["far_s"].key(161, g(EYE_S), "inOut").key(181, g(FAR_GS))
    R["far_c"].key(161, g(FAR_C), "inOut").key(181, g(FAR_GC))
    R["face_r"].key(164, 0, "inOut").key(186, -3.5)
    R["face_p"].key(164, FACE_PIVOT, "inOut", arc=-2).key(186, [FACE_PIVOT[0] + 7, FACE_PIVOT[1] - 6])
    blink(230)
    # slot leaves: swell, shrink; second ring
    R["slot_s"].key(282, [100, 100], "out").key(290, [120, 120], "inBack").key(307, [0, 0])
    R["ring_s"].key(293, g([8, 8]), "out").key(330, g([28, 28]))
    R["ring_o"].key(293, 55, "out").key(330, 0)
    # double-take: look where it was, small pull back, a second quicker look, then turn back to you
    pb1, rb1 = eyes_at(1.12); pb2, rb2 = eyes_at(1.04); pb3, rb3 = eyes_at(1.17); pb4, rb4 = eyes_at(-0.14)
    R["eyes_p"].key(296, p2, "out").key(307, pb1, "inOut").key(313, pb2, "out").key(319, pb3, "inOut", arc=3).key(346, pb4, "inOut").key(362, p0)
    R["eyes_r"].key(296, r2, "out").key(307, rb1, "inOut").key(313, rb2, "out").key(319, rb3, "inOut").key(346, rb4 + 1.1, "inOut").key(362, 0)
    R["far_s"].key(321, g(FAR_GS), "inOut").key(344, g(EYE_S))
    R["far_c"].key(321, g(FAR_GC), "inOut").key(344, g(FAR_C))
    R["face_r"].key(312, -3.5, "inOut").key(342, 0)
    R["face_p"].key(312, [FACE_PIVOT[0] + 7, FACE_PIVOT[1] - 6], "inOut", arc=2).key(342, FACE_PIVOT)
    R["face_s"].key(342, [100, 100], "out").key(350, [104, 95], "inOut").key(364, [100, 100])
    blink(370)
    R["face_p"].key(386, FACE_PIVOT, "sine").key(414, [FACE_PIVOT[0], FACE_PIVOT[1] - 4], "sine").key(442, FACE_PIVOT, "sine") \
        .key(468, [FACE_PIVOT[0], FACE_PIVOT[1] - 4], "sine").key(486, FACE_PIVOT)
    blink(430); blink(443)
    neutral(492)
    segs.append(("working", 84, 408))
    for n, f, d in (("working:slot-in", 126, 33), ("working:glance", 151, 39), ("working:hold", 190, 92),
                    ("working:slot-out", 282, 25), ("working:look-back", 296, 68), ("working:idle", 364, 128)):
        segs.append((n, f, d))

    # ---------------- idle 492-612: at you, breathing, one blink
    b = 492
    R["face_s"].key(b, [100, 100], "sine").key(b + 60, [101.5, 98.5], "sine").key(b + 120, [100, 100])
    R["eyes_p"].key(b, EYE_PIVOT, "sine").key(b + 60, [EYE_PIVOT[0], EYE_PIVOT[1] - 0.6 * S], "sine").key(b + 120, EYE_PIVOT)
    blink(b + 78)
    neutral(b + 120)
    segs.append(("idle", b, 120))

    # ---------------- thinking 612-792: eyes up, between you and the slot; slot pulses in busy
    b = 612
    slot_in(b + 2, 20, P["slot"])
    recolour(b + 6, b + 26, P["slot"], P["busy"])
    R["eyes_p"].key(b + 8, EYE_PIVOT, "inOut", arc=-2).key(b + 26, THINK[0])
    R["eyes_r"].key(b + 8, 0, "inOut").key(b + 26, THINK[1])
    for i, (f0, pose) in enumerate(((b + 52, THINK_SLOT), (b + 86, THINK), (b + 112, THINK_SLOT), (b + 140, None))):
        prev = R["eyes_p"].value_at(f0)
        tgt = pose[0] if pose else EYE_PIVOT
        R["eyes_p"].key(f0, prev, "inOut", arc=(2 if i % 2 == 0 else -2)).key(f0 + 18, tgt)
        R["eyes_r"].key(f0, R["eyes_r"].value_at(f0), "inOut").key(f0 + 18, pose[1] if pose else 0)
    R["far_s"].key(b + 52, g(EYE_S), "inOut").key(b + 70, g([5.3, 10.6])).key(b + 86, g([5.3, 10.6]), "inOut").key(b + 104, g(EYE_S))
    R["far_s"].key(b + 112, g(EYE_S), "inOut").key(b + 130, g([5.3, 10.6])).key(b + 140, g([5.3, 10.6]), "inOut").key(b + 158, g(EYE_S))
    for f in range(b + 30, b + 150, 40):       # pulse: scale and light
        R["slot_s"].key(f, [100, 100], "sine").key(f + 20, [80, 80], "sine").key(f + 40, [100, 100])
        R["slot_o"].key(f, 100, "sine").key(f + 20, 68, "sine").key(f + 40, 100)
    blink(b + 96)
    recolour(b + 150, b + 160, P["busy"], P["slot"])
    slot_out(b + 158, 18)
    neutral(b + 180)
    segs.append(("thinking", b, 180))

    # ---------------- done 792-912: slot turns ready, happy eyes, a small hop
    b = 792
    slot_in(b + 2, 18, P["slot"])
    recolour(b + 14, b + 32, P["slot"], P["ready"])
    swap_eyes(b + 18, "happy_o", "happy_k")
    R["face_s"].key(b + 26, [100, 100], "out").key(b + 32, [106, 94], "out").key(b + 38, [96, 105])          # crouch, spring
    R["face_p"].key(b + 32, FACE_PIVOT, "out").key(b + 46, [FACE_PIVOT[0], FACE_PIVOT[1] - 3 * S], "in").key(b + 58, FACE_PIVOT)
    R["face_s"].key(b + 38, [96, 105], "inOut").key(b + 50, [100, 100], "in").key(b + 58, [100, 100], "out") \
        .key(b + 62, [108, 92], "inOut").key(b + 72, [98, 102], "inOut").key(b + 80, [100, 100])
    unswap_eyes(b + 90, "happy_o", "happy_k")
    slot_out(b + 98, 18)
    recolour(b + 116, b + 118, P["ready"], P["slot"])
    neutral(b + 120)
    segs.append(("done", b, 120))

    # ---------------- asleep 912-1092: eyes close to lines, slow breathing, slot dimmed
    b = 912
    slot_in(b + 4, 22, P["slot"])
    recolour(b + 14, b + 44, P["slot"], P["dim"])
    R["slot_o"].key(b + 14, 100, "sine").key(b + 44, 45, "sine").key(b + 90, 62, "sine").key(b + 136, 45, "inOut").key(b + 150, 100)
    # eyes droop shut slowly, then swap to the closed-line eyes
    R["near_k"].key(b + 10, [100, 100], "inOut").key(b + 26, [100, 40], "inOut").key(b + 32, [100, 46], "in").key(b + 38, [100, 8])
    R["far_k"].key(b + 11, [100, 100], "inOut").key(b + 27, [100, 40], "inOut").key(b + 33, [100, 46], "in").key(b + 39, [100, 8])
    R["pill_o"].key(b + 39, 100).key(b + 40, 0)
    R["sleep_o"].key(b + 39, 0).key(b + 40, 100)
    R["near_k"].key(b + 40, [100, 8]); R["far_k"].key(b + 40, [100, 8])
    R["face_s"].key(b + 30, [100, 100], "sine").key(b + 72, [103, 97], "sine").key(b + 114, [100, 100], "sine").key(b + 140, [101.5, 98.5], "sine").key(b + 158, [100, 100])
    R["face_p"].key(b + 30, FACE_PIVOT, "sine").key(b + 72, [FACE_PIVOT[0], FACE_PIVOT[1] + 2], "sine").key(b + 114, FACE_PIVOT)
    # wake: lines off, pills open
    R["sleep_o"].key(b + 146, 100).key(b + 147, 0)
    R["pill_o"].key(b + 146, 0).key(b + 147, 100)
    R["near_k"].key(b + 147, [100, 8], "outBack").key(b + 160, [100, 100])
    R["far_k"].key(b + 148, [100, 8], "outBack").key(b + 161, [100, 100])
    recolour(b + 140, b + 150, P["dim"], P["slot"])
    slot_out(b + 152, 18)
    neutral(b + 180)
    segs.append(("asleep", b, 180))

    # ---------------- needs-you 1092-1212: eyes wide on you, slot in needs-you clay, attention bounce
    b = 1092
    slot_in(b + 2, 18, P["slot"])
    recolour(b + 8, b + 26, P["slot"], P["needs"])
    for c, s0, s1, c1 in (("near", EYE_S, WIDE_S, WIDE_N), ("far", EYE_S, WIDE_S, WIDE_F)):
        R[c + "_s"].key(b + 6, g(s0), "outBack").key(b + 18, g(s1)).key(b + 92, g(s1), "inOut").key(b + 104, g(s0))
        R[c + "_c"].key(b + 6, g(NEAR_C if c == "near" else FAR_C), "outBack").key(b + 18, g(c1)).key(b + 92, g(c1), "inOut").key(b + 104, g(NEAR_C if c == "near" else FAR_C))
    for f0, h, sq in ((b + 24, 12, [106, 94]), (b + 54, 7, [103, 97])):    # two bounces, the second smaller
        R["face_s"].key(f0, [100, 100], "out").key(f0 + 4, sq, "out").key(f0 + 9, [97, 104], "inOut").key(f0 + 20, [100, 100]) \
            .key(f0 + 22, [100, 100], "out").key(f0 + 25, [sq[0] - 2, sq[1] + 2], "inOut").key(f0 + 30, [100, 100])
        R["face_p"].key(f0 + 4, FACE_PIVOT, "out").key(f0 + 13, [FACE_PIVOT[0], FACE_PIVOT[1] - h], "in").key(f0 + 22, FACE_PIVOT)
    R["slot_s"].key(b + 40, [100, 100], "out").key(b + 46, [112, 112], "inOut").key(b + 56, [100, 100])
    blink(b + 86)
    recolour(b + 96, b + 104, P["needs"], P["slot"])
    slot_out(b + 100, 18)
    neutral(b + 120)
    segs.append(("needs-you", b, 120))

    # ---------------- failed 1212-1302: x eyes, a small shake (sideways; the face never rotates)
    b = 1212
    slot_in(b + 2, 16, P["slot"])
    recolour(b + 8, b + 22, P["slot"], P["needs"])
    swap_eyes(b + 10, "x_o", "x_k", pop=4)
    xs = [0, 9, -8, 6, -4, 2, 0]
    for i in range(len(xs) - 1):
        R["face_p"].key(b + 20 + i * 5, [FACE_PIVOT[0] + xs[i], FACE_PIVOT[1]], "inOut").key(b + 25 + i * 5, [FACE_PIVOT[0] + xs[i + 1], FACE_PIVOT[1]])
    R["face_s"].key(b + 50, [100, 100], "inOut").key(b + 58, [102, 97], "inOut").key(b + 66, [100, 100])
    unswap_eyes(b + 64, "x_o", "x_k")
    recolour(b + 66, b + 72, P["needs"], P["slot"])
    slot_out(b + 70, 16)
    neutral(b + 90)
    segs.append(("failed", b, 90))
    return tr, segs, b + 90


def mark_layers(P, tr, end):
    R = {k: v.lottie(end) for k, v in tr.items()}

    def pill_r(skey):
        trk = tr[skey]
        t2 = Track(min(trk.k[0][0]) / 2)
        t2.k = {f: (min(v) / 2, e, None) for f, (v, e, a) in trk.k.items()}
        return t2.lottie(end)

    def eye(name, ckey, skey, kkey):
        return grp(name, [rect([0, 0], R[skey], pill_r(skey)), fill(P["eye"])], T(p=R[ckey], s=R[kkey]))

    def line_eye(name, ds, widths, op, scl, tr_kw=None):
        items = []
        for d, w in zip(ds, widths):
            items.append(grp("stroke", path_items(d, S) + [stroke(P["eye"], w * S)]))
        kw = dict(a=EYE_PIVOT, p=EYE_PIVOT, o=op, s=scl)
        kw.update(tr_kw or {})
        return grp(name, items, T(**kw))

    pills = grp("eyes-pill", [eye("eye-near", "near_c", "near_s", "near_k"), eye("eye-far", "far_c", "far_s", "far_k")], T(o=R["pill_o"]))
    happy = line_eye("eyes-happy", ["M18.4 38.2Q21.6 32.6 24.8 38.2", "M29.8 38.2Q32.4 33.4 35 38.2"], [3, 2.7], R["happy_o"], R["happy_k"],
                     dict(p=[EYE_PIVOT[0] + 1.2 * S, EYE_PIVOT[1] - 1 * S], r=-6))
    sleep = line_eye("eyes-asleep", ["M18.6 38.6H24.6", "M29.4 38.6H35"], [2.8, 2.8], R["sleep_o"], {"a": 0, "k": [100, 100]})
    xx = line_eye("eyes-failed", ["M18.8 33.2L24.4 38.8M24.4 33.2L18.8 38.8", "M29.6 33.2L35.2 38.8M35.2 33.2L29.6 38.8"], [2.6, 2.6], R["x_o"], R["x_k"])

    def L(ind, nm, shapes, ks, parent=None, ty=4):
        d = {"ddd": 0, "ind": ind, "ty": ty, "nm": nm, "sr": 1, "ks": ks, "ao": 0, "ip": 0, "op": end, "st": 0, "bm": 0}
        if ty == 4:
            d["shapes"] = shapes
        if parent:
            d["parent"] = parent
        return d

    eyes = L(2, "eyes", [happy, sleep, xx, pills], T(a=EYE_PIVOT, p=R["eyes_p"], r=R["eyes_r"]), parent=3)
    face = L(3, "face", [grp("face", [rect(g([27, 37]), g([38, 38]), g(15)), fill(P["face"])])],
             T(a=FACE_PIVOT, p=R["face_p"], s=R["face_s"], r=R["face_r"], o=R["face_o"]))
    slot = L(4, "slot", [grp("slot", [ell([0, 0], g([12, 12])), fill({"a": 1, "k": R["slot_c"]["k"]} if R["slot_c"]["a"] else R["slot_c"])])],
             T(p=g([49, 15]), s=R["slot_s"], o=R["slot_o"]))
    ring = L(5, "slot-ring", [grp("ring", [ell([0, 0], R["ring_s"]), {**stroke(P["slot"], 1.4 * S), "c": R["slot_c"]}])],
             T(p=g([49, 15]), o=R["ring_o"]))
    return [ring, slot, eyes, face]


def comp(name, layers, end, markers, bg=None, meta=""):
    if bg:
        layers = layers + [{"ddd": 0, "ind": 99, "ty": 1, "nm": "background", "sc": bg, "sw": W, "sh": W, "ks": T(),
                            "ip": 0, "op": end, "st": 0, "bm": 0, "ao": 0}]
    return {"v": "5.12.2", "fr": FPS, "ip": 0, "op": end, "w": W, "h": W, "ddd": 0, "nm": name, "assets": [],
            "layers": layers, "markers": [{"tm": t, "cm": n, "dr": d} for n, t, d in markers],
            "meta": {"g": "LastHarness tools/build_v1.py", "d": meta}}


# ---------------------------------------------------------------------------------------------
# THE SIX WORKER BOTS (ball, two pill eyes, tool where the slot is). Shapes from bot-tools.js.
BOTS = TOK["bots"]
BOT_BG = BOTS["background"]
ROLES = [("lead", "lead"), ("researcher", "research"), ("writer", "writer"), ("coder", "coder"),
         ("designer", "designer"), ("planner", "planner")]
RB, YAW, PIT = 21.5, math.radians(56), math.radians(38)


def tool_shapes(key, c, k, R):
    """Each tool in its own 64 coords (bot-tools.js). Returns shape items; R holds animated parts."""
    F = lambda col, o=100: fill(col, o)
    Sk = lambda col, w, o=100: stroke(col, w, o)
    P = lambda d: path_items(d)
    if key == "lead":
        needle = grp("needle", [grp("n1", P("M32 13L39 32H25z") + [F(c)]), grp("n2", P("M25 32H39L32 51z") + [F(c, 45)])],
                     T(a=[32, 32], p=[32, 32], r=R["needle_r"]))
        return [grp("hub", [ell([32, 32], [6.8, 6.8]), F(k)]), needle, grp("dial", [ell([32, 32], [42, 42]), F(k)]),
                grp("case", [ell([32, 32], [58, 58]), F(c)])]
    if key == "research":
        return [grp("glint", P("M18 21a9.5 9.5 0 0 1 7-6") + [Sk(c, 3.6, 75)]), grp("lens", [ell([26, 26], [27, 27]), F(k)]),
                grp("rim", [ell([26, 26], [42, 42]), F(c)]),
                grp("handle", P("M37 39l6-6 15 15a4 4 0 0 1 0 5.5l-.5.5a4 4 0 0 1-5.5 0z") + [F(c)])]
    if key == "writer":
        return [grp("hole", [ell([32, 48.5], [5.6, 5.6]), F(k)]), grp("slit", P("M32 49V59") + [Sk(k, 2.8)]),
                grp("bands", P("M21 13H43M21 42.5H43") + [{**Sk(k, 3.2), "lc": 1}]),
                grp("nib", P("M21 42H43L35 58Q32 62 29 58z") + [F(c)]), grp("barrel", [rect([32, 23], [22, 40], 8), F(c)])]
    if key == "coder":
        return [grp("cursor", [rect([35.5, 43.5], [15, 5], 2.5), fill(k, R["cursor_o"])]),
                grp("prompt", P("M14 30l8 7-8 7") + [Sk(k, 5)]), grp("bar", P("M3 21H61") + [{**Sk(k, 3.6), "lc": 1}]),
                grp("window", [rect([32, 32], [58, 46], 8), F(c)])]
    if key == "designer":
        return [grp("bands", P("M21 21H43M21 30.5H43") + [{**Sk(k, 3.2), "lc": 1}]),
                grp("tip", P("M17 31H47V36C47 47 39 54 32 62C25 54 17 47 17 36z") + [F(c)]),
                grp("ferrule", [rect([32, 25.5], [22, 11], 2), F(c)]), grp("handle", [rect([32, 12], [9, 24], 4.5), F(c)])]
    if key == "planner":
        tick = grp("tick", P("M40 46l4 4 7-7") + [{"ty": "tm", "s": R["tick_s"], "e": R["tick_e"], "o": {"a": 0, "k": 0}, "m": 1},
                                                 Sk(k, 4)])
        return [tick, grp("lines", P("M18 30h15M18 40h20M18 50h12") + [Sk(k, 4)]),
                grp("clip", [rect([32, 9], [24, 14], 4), {**Sk(k, 3.6), "lc": 1}, F(c)]),
                grp("board", [rect([32, 35], [46, 54], 7), F(c)])]
    raise KeyError(key)


TOOL_ROT = {"lead": 0, "research": 0, "writer": 45, "coder": 0, "designer": 45, "planner": 0}


def ball(lam, phi, yaw, pit):
    x0, y0, z0 = math.sin(lam) * math.cos(phi), math.sin(phi), math.cos(lam) * math.cos(phi)
    x1, z1 = x0 * math.cos(yaw) + z0 * math.sin(yaw), -x0 * math.sin(yaw) + z0 * math.cos(yaw)
    return x1, y0 * math.cos(pit) + z1 * math.sin(pit), z1 * math.cos(pit) - y0 * math.sin(pit)


def bot_eye(a_deg, e):
    """Eye rect (centre, size, rotation, visible) on the ball for turn amount e, as on the identity page."""
    yaw, pit = e * YAW, e * PIT
    a = math.radians(a_deg)
    x, y, z = ball(a, math.radians(-3), yaw, pit)
    w = 6.5 * max(0.3, max(z, 0) ** 0.85)
    h = 12 * (0.55 + 0.45 * z)
    sx, sy = 24 + 16.5 * x, 40 - 16.5 * y
    rot = 0.35 * math.degrees(math.atan2(math.sin(a + yaw) * math.sin(pit), math.cos(a + yaw)))
    return [sx, sy], [w, h], rot, z


def seam_shape(lam_deg, e):
    """Faint seam meridian. Fixed vertex count; hidden points collapse onto the nearest visible end."""
    yaw, pit = e * YAW, e * PIT
    pts, vis = [], []
    for i in range(-90, 91, 6):
        x, y, z = ball(math.radians(lam_deg), math.radians(i), yaw, pit)
        pts.append([24 + RB * x, 40 - RB * y]); vis.append(z > 0.02)
    idx = [i for i, v in enumerate(vis) if v]
    if not idx:
        idx = [0]
    lo, hi = idx[0], idx[-1]
    pts = [pts[min(max(i, lo), hi)] for i in range(len(pts))]
    pts = [[round(a * S, 2), round(b * S, 2)] for a, b in pts]
    z = [[0, 0]] * len(pts)
    return shape(pts, z, z, False)


def smooth(x):
    x = min(1, max(0, x))
    return x * x * (3 - 2 * x)


def ease_bezier(name, t):
    (x1, y1), (x2, y2) = EASE[name]
    lo, hi = 0.0, 1.0
    for _ in range(40):
        m = (lo + hi) / 2
        bx = 3 * (1 - m) ** 2 * m * x1 + 3 * (1 - m) * m * m * x2 + m ** 3
        lo, hi = (m, hi) if bx < t else (lo, m)
    m = (lo + hi) / 2
    return 3 * (1 - m) ** 2 * m * y1 + 3 * (1 - m) * m * m * y2 + m ** 3


BOT_SPAWN, BOT_IDLE, BOT_WORK = (0, 84), (84, 120), (204, 360)
BOT_END = 564


def bot_turn(f):
    """Turn amount e(f) during the work segment (page timing: up at 32-45%, back at 62-74%)."""
    s0, n = BOT_WORK
    if not (s0 <= f <= s0 + n):
        return 0.0
    p = (f - s0) / n
    if p < 0.32: return 0.0
    if p < 0.45: return ease_bezier("inOut", (p - 0.32) / 0.13) * 1.0
    if p < 0.62: return 1.0
    if p < 0.74: return 1 - ease_bezier("inOut", (p - 0.62) / 0.12)
    return 0.0


def build_bot(role, key, with_bg):
    c, k = BOTS[role], BOT_BG
    end = BOT_END
    s0, n = BOT_WORK
    pf = lambda p: s0 + round(p * n)
    # head (ball) transform: translate (2.4e, -2.4e) toward the tool; squash/stretch about the floor (24,62)
    HP = g([24, 62])
    head_p = Track(HP); head_s = Track([100, 100]); head_o = Track(100)
    head_o.key(0, 0, "out").key(6, 100)
    head_p.key(0, [HP[0], HP[1] - 3 * S], "inQuad").key(14, HP)
    head_s.key(0, [55, 70], "inQuad").key(14, [88, 114], "out").key(18, [116, 86], "inOut").key(28, [95, 107], "inOut") \
        .key(37, [103, 97], "inOut").key(46, [100, 100])
    b = BOT_IDLE[0]
    head_s.key(b, [100, 100], "sine").key(b + 60, [101.5, 98.5], "sine").key(b + 120, [100, 100])
    # eyes and seams: baked per 2 frames while turning, plus blink keys at rest
    eyes = {"L": {"c": Track(None), "s": Track(None), "r": Track(0), "o": Track(100)},
            "R": {"c": Track(None), "s": Track(None), "r": Track(0), "o": Track(100)}}
    seams = {"L": Track(seam_shape(-96, 0)), "R": Track(seam_shape(96, 0))}
    hl_p = Track(g([17.4, 31.2]))
    for side, a in (("L", -24), ("R", 24)):
        cc, ss, rr, zz = bot_eye(a, 0)
        E = eyes[side]
        E["c"].k = {0: ([round(v * S, 2) for v in cc], "hold", None)}
        E["s"].k = {0: ([round(v * S, 2) for v in ss], "hold", None)}
        E["r"].k = {0: (round(rr, 2), "hold", None)}
    rest_s = {sd: eyes[sd]["s"].k[0][0] for sd in "LR"}

    def blink(f, lag_side="R"):
        for sd in "LR":
            d = 1 if sd == lag_side else 0
            w, h = rest_s[sd]
            eyes[sd]["s"].key(f + d, [w, h], "in").key(f + d + 3, [w, round(h * 0.12, 2)], "out").key(f + d + 9, [w, h])

    for sd in "LR":   # eyes closed during the drop, open after landing
        w, h = rest_s[sd]
        d = 1 if sd == "R" else 0
        eyes[sd]["s"].key(0, [w, round(h * 0.12, 2)]).key(30 + d, [w, round(h * 0.12, 2)], "outBack").key(42 + d, [w, h])
    blink(62); blink(b + 78)
    f_up0, f_dn1 = pf(0.32), pf(0.74)
    head_p.key(f_up0, HP)
    for f in list(range(f_up0, f_dn1, 2)) + [f_dn1]:
        e = bot_turn(f)
        head_p.key(f, [round(HP[0] + 2.4 * e * S, 2), round(HP[1] - 2.4 * e * S, 2)], "linear")
        for sd, a in (("L", -24), ("R", 24)):
            cc, ss, rr, zz = bot_eye(a, e)
            E = eyes[sd]
            E["c"].key(f, [round(v * S, 2) for v in cc], "linear")
            E["s"].key(f, [round(v * S, 2) for v in ss], "linear")
            E["r"].key(f, round(rr, 2), "linear")
            E["o"].key(f, 100 if zz >= 0.1 else 0, "hold")
        seams["L"].key(f, seam_shape(-96, e), "linear"); seams["R"].key(f, seam_shape(96, e), "linear")
        # the soft highlight slides a little against the turn, selling the rotation
        hl_p.key(f, [round((17.4 - 3.2 * e) * S, 2), round((31.2 + 2.0 * e) * S, 2)], "linear")
    for trk in [head_p, hl_p, seams["L"], seams["R"]] + [eyes[sd][q] for sd in "LR" for q in "csro"]:
        last = max(trk.k)
        trk.k[last] = (trk.k[last][0], "hold", None)
    blink(pf(0.88))

    # tool actions (page CSS, 6 s cycle = 360 f), each a transform on the tool or a part of it
    act_p = Track([0, 0]); act_r = Track(0)
    R = {}
    if key == "lead":
        nr = Track(0); nr.key(s0, 0).key(pf(0.22), 0, "inOut").key(pf(0.52), 380, "inOut").key(pf(0.58), 360)
        nr.key(s0 + n, 360); R["needle_r"] = nr.lottie(end)
        # needle resets 360 -> 0 at the cycle edge: identical pose, so the loop seam is clean
        R["needle_r"]["k"][-1]["s"] = [360]
    elif key == "research":
        for p, v in ((0.22, [0, 0]), (0.30, [-2, 1.5]), (0.40, [1.5, -1]), (0.50, [-1, -1.5]), (0.60, [0, 0])):
            act_p.key(pf(p), [v[0] * S, v[1] * S], "inOut")
        act_p.key(pf(0.60), [0, 0])
    elif key == "writer":
        for p, v, r in ((0.20, [0, 0], 0), (0.26, [-1.5, 1], -6), (0.32, [1, -0.5], 4), (0.38, [-1.5, 1], -6), (0.44, [1, -0.5], 4), (0.50, [0, 0], 0)):
            act_p.key(pf(p), [v[0] * S, v[1] * S], "inOut"); act_r.key(pf(p), r, "inOut")
        act_p.key(pf(0.5), [0, 0]); act_r.key(pf(0.5), 0)
    elif key == "coder":
        for p in (0.24, 0.32, 0.40):
            act_p.key(pf(p), [0, 0], "out").key(pf(p + 0.04), [0, 0.8 * S], "inOut").key(pf(p + 0.08), [0, 0])
    elif key == "designer":
        for p, r in ((0.22, 0), (0.30, -14), (0.38, 6), (0.46, -14), (0.54, 0)):
            act_r.key(pf(p), r, "inOut")
        act_r.key(pf(0.54), 0)
    if key == "coder":
        co = Track(100)
        for f in range(0, end, 30):
            co.key(f, 100 if (f // 30) % 2 == 0 else 0)
        R["cursor_o"] = co.lottie(end)
    if key == "planner":
        # neutral: tick drawn. Work: it rubs out, then draws itself again (page: draws at 22-40%)
        ts, te = Track(0), Track(100)
        ts.key(pf(0.04), 0, "inOut").key(pf(0.16), 100).key(pf(0.20), 0)
        te.key(pf(0.20), 100).key(pf(0.201), 0, "inOut").key(pf(0.40), 100)
        R["tick_s"], R["tick_e"] = ts.lottie(end), te.lottie(end)
    tool_s = Track([0, 0])
    tool_s.key(0, [0, 0]).key(40, [0, 0], "outBack").key(58, [100, 100])
    TS = 0.37 * S * 100
    tsc = {f: ([v[0] * TS / 100, v[1] * TS / 100], e, None) for f, (v, e, a) in tool_s.k.items()}
    tool_s.k = tsc
    base = g([51, 13])
    tp = Track(base)
    tp.k = {f: ([base[0] + v[0], base[1] + v[1]], e, None) for f, (v, e, a) in act_p.k.items()}
    tr_ = Track(TOOL_ROT[key])
    tr_.k = {f: (TOOL_ROT[key] + v, e, None) for f, (v, e, a) in act_r.k.items()}

    def L(ind, nm, shapes, ks, parent=None):
        d = {"ddd": 0, "ind": ind, "ty": 4, "nm": nm, "sr": 1, "ks": ks, "ao": 0, "ip": 0, "op": end, "st": 0, "bm": 0, "shapes": shapes}
        if parent:
            d["parent"] = parent
        return d

    tool = L(1, "tool", tool_shapes(key, c, k, R), T(a=[32, 32], p=tp.lottie(end), r=tr_.lottie(end), s=tool_s.lottie(end)))

    def beye(sd):
        E = eyes[sd]
        rk = Track(None)
        rk.k = {f: (round(min(min(v[0], v[1]) / 2, 3.25 * S), 2), e, None) for f, (v, e, a) in E["s"].k.items()}
        return grp("eye-" + sd, [rect([0, 0], E["s"].lottie(end), rk.lottie(end)), fill(k)],
                   T(p=E["c"].lottie(end), r=E["r"].lottie(end), o=E["o"].lottie(end)))

    shade = {"ty": "gf", "o": {"a": 0, "k": 100}, "r": 1, "t": 2, "nm": "shade",
             "s": hl_p.lottie(end), "e": {"a": 0, "k": g([17.4 + 35.2, 31.2])},
             "h": {"a": 0, "k": 0}, "a": {"a": 0, "k": 0},
             "g": {"p": 3, "k": {"a": 0, "k": [0, 1, 1, 1, 0.5, 1, 1, 1, 1, 0, 0, 0, 0, 0.30, 0.5, 0, 1, 0.24]}}}
    seam_items = [grp("seam-" + sd, [sh(seams[sd].lottie(end)), stroke("#000000", 0.9 * S, 20)]) for sd in "LR"]
    head_shapes = [beye("L"), beye("R")] + seam_items + [grp("shade", [ell(g([24, 40]), g([44, 44])), shade]),
                                                       grp("ball", [ell(g([24, 40]), g([44, 44])), fill(c)])]
    head = L(2, "head", head_shapes, T(a=HP, p=head_p.lottie(end), s=head_s.lottie(end), o=head_o.lottie(end)))
    markers = [("spawn", *BOT_SPAWN), ("idle", *BOT_IDLE), ("work", *BOT_WORK)]
    return comp(f"lastharness-bot-{role}", [tool, head], end, markers, BOT_BG if with_bg else None,
                f"{role} worker: spawn, idle, work. Background {BOT_BG}.")


# ---------------------------------------------------------------------------------------------
if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(KIT, "out", "lottie")
    os.makedirs(out, exist_ok=True)
    written = []
    for pal, P in MARK_PALETTES.items():
        tr, segs, end = build_mark_tracks(P)
        layers = mark_layers(P, tr, end)
        for bg in (False, True):
            fn = os.path.join(out, f"lastharness-mark-{pal}{'-bg' if bg else ''}.json")
            json.dump(comp(f"lastharness-mark-{pal}", json.loads(json.dumps(layers)), end, segs, P["bg"] if bg else None,
                           "Mark: spawn, working loop, idle, thinking, done, asleep, needs-you, failed. 64 grid x8."),
                      open(fn, "w"), separators=(",", ":"))
            written.append(fn)
    for role, key in ROLES:
        for bg in (False, True):
            fn = os.path.join(out, f"lastharness-bot-{role}{'-bg' if bg else ''}.json")
            json.dump(build_bot(role, key, bg), open(fn, "w"), separators=(",", ":"))
            written.append(fn)
    for fn in written:
        print(f"{os.path.getsize(fn):8d}  {os.path.relpath(fn, KIT)}")
