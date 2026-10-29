"""Bakes the free 3D buildings (OpenGameArt; authors and licences in assets/buildings/CREDITS.txt) into the models the map
draws in the towns (assets/buildings/*.bld).

    python -I tools/buildings_bake.py SRC_DIR

SRC_DIR holds the unpacked downloads, each in a folder named as its OpenGameArt page (https://opengameart.org/content/<name>):
  ordinary-house, brick-building (the glTF), farm / farm2 (medieval-farm-house, -2, converted to .obj by `assimp export`),
  small-old-house, simple-cottage, soviet-panel-apartment-house-3d, medieval-church (its church.dae -> church.obj),
  (the factory of OpenGameArt was left out: a slab of ground came with it).

A .bld file: b'BLD1', four f32 — half the width along the street, half the depth, the walls' height, the whole height (model
units; the model stands on z = 0, its front — the door — to −y, its long side along x) — then u32 vertices of eight f32:
position, normal, and two numbers that carry the colour of the triangle (the game paints a triangle one colour, the
texture's mean over it, `meshbake.bake_textured`): part = 100 + r·256 + g, light = b + the sky's share (0..0.999).
"""
import os, struct, sys
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import meshbake

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'assets', 'buildings')


def luma(c):
    return c @ np.array([0.3, 0.59, 0.11])


def mix(a, b, t):
    return a * (1 - t) + b * t


# colour graders: (P (n,3,3) in the model's own z-up frame, N, C (n,3)) -> C
def terracotta_roof(P, N, C):
    """The blue tin roof of the little cottage -> clay tiles."""
    blue = (C[:, 2] > C[:, 0] * 1.25) & (C[:, 2] > 0.35)
    out = C.copy()
    l = luma(C)
    out[blue] = np.stack([l * 1.55, l * 0.80, l * 0.55], 1)[blue]
    return np.clip(out, 0, 1)


def calm_thatch(P, N, C):
    """The cartoon's saturated yellow straw -> weathered thatch."""
    yel = (C[:, 0] > 0.55) & (C[:, 2] < C[:, 0] * 0.55)
    out = C.copy()
    l = luma(C)
    tgt = np.stack([l * 1.22, l * 1.0, l * 0.66], 1)
    out[yel] = mix(C[yel], tgt[yel], 0.8)
    return np.clip(out * 0.92, 0, 1)


def brick_factory(P, N, C):
    """The grey factory: walls in brick, a flat grey roof, dark chimneys."""
    out = C.copy()
    side = np.abs(N.mean(1)[:, 2]) < 0.5
    l = luma(C)
    out[side] = np.stack([0.55 + 0.25 * l, 0.27 + 0.16 * l, 0.20 + 0.10 * l], 1)[side]
    top = P[..., 2].mean(1) > 0.22 * P[..., 2].max()
    chimney = top & side
    out[chimney] = (np.stack([0.30 + 0.4 * l, 0.27 + 0.35 * l, 0.25 + 0.3 * l], 1))[chimney]
    return np.clip(out, 0, 1)


def slate_roof(P, N, C):
    """The flat pale roof of the brick building -> dark slate; the walls a little warmer."""
    out = C.copy()
    up = N.mean(1)[:, 2] > 0.55
    l = luma(C)
    out[up] = np.stack([0.20 + 0.25 * l, 0.20 + 0.25 * l, 0.22 + 0.25 * l], 1)[up]
    return np.clip(out * np.where(up, 1.0, 1.12)[:, None], 0, 1)


def lift(k):
    return lambda P, N, C: np.clip(C * k, 0, 1)


# name, source (relative to SRC_DIR), texture, only, yaw (degrees about z, counter-clockwise from above), width (model units), triangles, grader
MODELS = [
    ('ordinary_a', 'ordinary-house/house.obj', 'ordinary-house/texture1.png', None, 0, 0.44, 9000, None),
    ('ordinary_b', 'ordinary-house/house.obj', 'ordinary-house/texture2.png', None, 0, 0.44, 9000, None),
    ('oldhouse', 'small-old-house/House.obj', 'small-old-house/tex/house_tex.png', None, 90, 0.46, 7000, lift(1.35)),
    ('cottage', 'simple-cottage/cottage_mesh.obj', 'simple-cottage/cottage_diffuse.png', None, 0, 0.34, 5000, terracotta_roof),
    ('farm_a', 'farm/farmhouse.obj', 'farm/texture.png', None, 0, 0.44, 7000, calm_thatch),
    ('farm_b', 'farm2/farmhouse2.obj', 'farm2/texture.png', None, 0, 0.44, 7000, calm_thatch),
    ('brick', 'brick-building/scene.gltf', None, None, 0, 0.60, 9000, slate_roof),
    ('panel', 'soviet-panel-apartment-house-3d/spah9lvl.obj', 'soviet-panel-apartment-house-3d/spah9lvl.png', None, 0, 0.90, 9000, lift(1.2)),
    ('church', 'medieval-church/church.obj', None, None, -90, 0.92, 14000, lift(1.25)),
]


def own_rgb(c):
    q = np.clip(np.round(c * 255), 0, 255).astype(np.int64)
    return q


def bake_one(src, cfg):
    name, rel, tex, only, yaw, width, tris, grade = cfg
    P, N, C = meshbake.bake_textured(os.path.join(src, rel), tris=tris, texture=os.path.join(src, tex) if tex else None, only=only)
    # y-up -> z-up with the front (+z of a glTF/OBJ) to −y
    P = P[..., [0, 2, 1]] * np.array([1, -1, 1])
    N = N[..., [0, 2, 1]] * np.array([1, -1, 1])
    a = np.radians(yaw)
    c, s = np.cos(a), np.sin(a)
    R = np.array([[c, -s, 0], [s, c, 0], [0, 0, 1]])
    P, N = P @ R.T, N @ R.T
    if grade is not None:
        C = grade(P, N, C)
    lo, hi = P.reshape(-1, 3).min(0), P.reshape(-1, 3).max(0)
    k = width / (hi[0] - lo[0])
    mid = (lo + hi) / 2
    Q = (P - np.array([mid[0], mid[1], lo[2]])) * k
    hgt = (hi[2] - lo[2]) * k
    hx, hy = (hi[0] - lo[0]) * k / 2, (hi[1] - lo[1]) * k / 2
    top = 0.62 * hgt
    ao = np.clip(0.72 + 0.28 * np.clip(Q[..., 2] / max(hgt * 0.5, 1e-6), 0, 1), 0, 0.999)
    q = own_rgb(C)
    v = np.zeros((len(Q), 3, 8), dtype=np.float32)
    v[..., :3] = Q
    v[..., 3:6] = N
    v[..., 6] = (100 + q[:, 0] * 256 + q[:, 1])[:, None]
    v[..., 7] = q[:, 2][:, None] + ao
    flat = v.reshape(-1, 8)
    data = b'BLD1' + struct.pack('<4f', hx, hy, top, hgt) + struct.pack('<I', len(flat)) + flat.tobytes()
    return data, (hx, hy, top, hgt, len(Q))


def main():
    src = sys.argv[1]
    os.makedirs(OUT, exist_ok=True)
    for cfg in MODELS:
        data, info = bake_one(src, cfg)
        open(os.path.join(OUT, cfg[0] + '.bld'), 'wb').write(data)
        print(cfg[0], 'hx %.3f hy %.3f top %.3f height %.3f, %d triangles' % info)


if __name__ == '__main__':
    main()
