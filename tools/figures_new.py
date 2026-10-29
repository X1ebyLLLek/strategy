"""The troops' figures made from the free models found in the 32nd session (OpenGameArt; authors and licences in
assets/figures/CREDITS.txt). `figures3d.py` made the first set from poly.pizza (which the cloud container cannot reach);
this one replaces the figures whose models did not please: the tank.

    python -I tools/figures_new.py SRC_DIR

SRC_DIR holds the unpacked downloads in folders named as their OpenGameArt pages
(https://opengameart.org/content/<name>): t-12-soviet-light-tank-lowpoly (its .fbx converted by `assimp export` to t12.obj).
The format of the files is `figures3d.py`'s (FIG1).
"""
import os, struct, sys
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import meshbake

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'assets', 'figures')
PAINTED = 5


def srgb(c):
    return np.clip(c, 0, 1) ** (1 / 2.2)


def own(c):
    """A colour of its own (sRGB 0..1) as a paint code (1000 + r·4096 + g·64 + b, six bits each)."""
    q = np.clip(np.round(np.asarray(c) * 63), 0, 63).astype(int)
    return 1000 + q[..., 0] * 4096 + q[..., 1] * 64 + q[..., 2]


def fig_bytes(P, N, paint, length=None, height=None):
    """P, N (n,3,3) in the game's axes (x front, y left, z up); standing on z = 0, its middle at x = y = 0, scaled to a
    length or a height; the light a vertex gets from the sky: less near the ground."""
    lo, hi = P.reshape(-1, 3).min(0), P.reshape(-1, 3).max(0)
    k = length / (hi[0] - lo[0]) if length else height / (hi[2] - lo[2])
    mid = (lo + hi) / 2
    Q = (P - np.array([mid[0], mid[1], lo[2]])) * k
    z = Q[..., 2]
    ao = 0.72 + 0.28 * np.clip(z / max(z.max(), 1e-6), 0, 1)
    v = np.zeros((len(Q), 3, 8), dtype=np.float32)
    v[..., :3] = Q
    v[..., 3:6] = N
    v[..., 6] = paint[:, None]
    v[..., 7] = ao
    flat = v.reshape(-1, 8)
    return b'FIG1' + struct.pack('<I', len(flat)) + flat.tobytes()


def to_game(X, yaw_deg):
    """A glTF/OBJ frame (+y up) turned by `yaw` about the up axis, then to the game's: the model's +z to +x, +x to +y, up to +z."""
    a = np.radians(yaw_deg)
    c, s = np.cos(a), np.sin(a)
    R = np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])
    return (X @ R.T)[..., [2, 0, 1]]


def tank(src):
    """The T-12, a light tank of the thirties: its olive plates take the country's paint, the tracks, the rust and the
    black stay as they are."""
    P, N, C = meshbake.bake_textured(os.path.join(src, 't-12-soviet-light-tank-lowpoly', 't12.obj'), tris=9000)
    Pg, Ng = to_game(P, 0), to_game(N, 0)
    camo = (C[:, 0] > 0.06) & (C[:, 0] < 0.09) & (C[:, 2] < 0.04)
    # (the materials' numbers are linear; the figure's own colours are as the screen shows them)
    own_c = own(srgb(C * np.array([1.0, 1.0, 1.0]) * 1.6))
    paint = np.where(camo, PAINTED, own_c).astype(np.float64)
    return fig_bytes(Pg, Ng, paint, length=1.9)


def main():
    src = sys.argv[1]
    os.makedirs(OUT, exist_ok=True)
    data = tank(src)
    open(os.path.join(OUT, 'tank.bin'), 'wb').write(data)
    print('tank:', (len(data) - 8) // 32 // 3, 'triangles')


if __name__ == '__main__':
    main()
