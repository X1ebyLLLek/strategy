"""Dresses the soldiers of assets/figures (made by `figures3d.py` from one modern character) in the clothes of their ages: the
musketeer in white breeches, white crossbelts, a knapsack, coat-tails; the rifleman of the factories' age in a greatcoat,
leather webbing, a pack with a bedroll, puttees. The tool recolours the triangles by where they are (the figure stands one
unit high, the front to +x) and adds a few solids.

    python -I tools/figures_dress.py

It works ONCE on the files `figures3d.py` wrote (a sign, assets/figures/DRESSED, is left; remove it after running
`figures3d.py` again, which makes the figures anew). The raiders are left as they are.
"""
import os, struct, sys
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import figures3d as F

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIR = os.path.join(ROOT, 'assets', 'figures')
FRAMES = ['stand', 'm0', 'm1', 'm2', 'm3', 'fight']
KIT = 88248      # the black webbing and trousers of the base figure (own colour .09 .075 .05)
LEATHER, CLOTH, SKIN, WHITE, STEEL = 2, 0, 1, 6, 3


def load(path):
    d = open(path, 'rb').read()
    assert d[:4] == b'FIG1'
    n = struct.unpack_from('<I', d, 4)[0]
    a = np.frombuffer(d, dtype=np.float32, count=n * 8, offset=8).reshape(-1, 3, 8).astype(np.float64)
    return a


def save(path, a):
    flat = a.astype(np.float32).reshape(-1, 8)
    open(path, 'wb').write(b'FIG1' + struct.pack('<I', len(flat)) + flat.tobytes())


def add(a, P, N, paint, ao=0.9):
    v = np.zeros((len(P), 3, 8))
    v[..., :3] = P
    v[..., 3:6] = N
    v[..., 6] = np.broadcast_to(paint, (len(P),))[:, None]
    v[..., 7] = ao
    return np.concatenate([a, v])


def skirt(a, z0, z1, rx, ry, paint, seg=14, centre=(0.0, 0.0)):
    """A coat's skirt: a ring of a flattened cone from z1 (the waist) down to z0, open at both ends (two faces' worth)."""
    tris, nrm = [], []
    for i in range(seg):
        a0, a1 = i / seg * 2 * np.pi, (i + 1) / seg * 2 * np.pi
        top = lambda t: np.array([centre[0] + np.cos(t) * rx * 0.80, centre[1] + np.sin(t) * ry * 0.80, z1])
        bot = lambda t: np.array([centre[0] + np.cos(t) * rx, centre[1] + np.sin(t) * ry, z0])
        n0 = np.array([np.cos(a0) / rx, np.sin(a0) / ry, 0.12])
        n1 = np.array([np.cos(a1) / rx, np.sin(a1) / ry, 0.12])
        n0, n1 = n0 / np.linalg.norm(n0), n1 / np.linalg.norm(n1)
        tris += [[bot(a0), bot(a1), top(a1)], [bot(a0), top(a1), top(a0)]]
        nrm += [[n0, n1, n1], [n0, n1, n0]]
    return add(a, np.array(tris), np.array(nrm), paint, 0.82)


def centres(a):
    """Where the body is in this frame: the hips' middle (x, y) and the back's x (the torso's rearmost part)."""
    z = a[..., 2].mean(1)
    hip = a[(z > 0.45) & (z < 0.56)][..., :2].reshape(-1, 2)
    torso = a[(z > 0.58) & (z < 0.80)][..., :3].reshape(-1, 3)
    return hip.mean(0), np.percentile(torso[:, 0], 4), torso[:, 1].mean()


def musket(a):
    z = a[..., 2].mean(1)
    paint = a[:, 0, 6]
    kit = paint == KIT
    out = a.copy()
    (hx, hy), back, ty = centres(a)
    cream = F.own([0.80, 0.77, 0.66])
    # breeches (the whole leg above the boot) and the plastron / crossbelts: the base figure's black kit above the hip
    out[((paint == KIT) | (paint == CLOTH)) & (z < 0.47) & (z > 0.14), :, 6] = cream
    out[kit & (z >= 0.47), :, 6] = F.own([0.78, 0.76, 0.68])
    # black gaiters over the boots; the boots themselves black
    out[(paint == LEATHER) & (z < 0.16), :, 6] = F.own([0.035, 0.03, 0.03])
    # coat-tails over the hips, the coat's colour
    out = skirt(out, 0.38, 0.52, 0.115, 0.14, CLOTH, centre=(hx, hy))
    # a knapsack on the back (−x) with a rolled blanket
    bx = back - 0.035
    t, n = F.box([bx, ty, 0.665], [0.035, 0.08, 0.095], np.eye(3))
    out = add(out, t, n, F.own([0.26, 0.18, 0.10]), 0.9)
    t, n = F.cylinder([bx - 0.005, ty - 0.095, 0.775], [bx - 0.005, ty + 0.095, 0.775], 0.032, 0.032, seg=8)
    out = add(out, t, n, F.own([0.55, 0.50, 0.40]), 0.95)
    return out


def industrial(a):
    z = a[..., 2].mean(1)
    paint = a[:, 0, 6]
    kit = paint == KIT
    out = a.copy()
    # trousers in the country's cloth, the webbing leather brown, puttees on the shins
    (hx, hy), back, ty = centres(a)
    out[kit & (z < 0.47), :, 6] = CLOTH
    out[kit & (z >= 0.47), :, 6] = F.own([0.36, 0.27, 0.16])
    out[(paint == CLOTH) & (z < 0.18) & (z > 0.05), :, 6] = F.own([0.30, 0.27, 0.18])
    out[(paint == LEATHER) & (z < 0.14), :, 6] = F.own([0.10, 0.07, 0.05])
    # the greatcoat's skirt
    out = skirt(out, 0.31, 0.52, 0.125, 0.15, CLOTH, centre=(hx, hy))
    # a pack with a bedroll, a canteen at the hip
    bx = back - 0.04
    t, n = F.box([bx, ty, 0.655], [0.04, 0.085, 0.10], np.eye(3))
    out = add(out, t, n, F.own([0.28, 0.27, 0.17]), 0.9)
    t, n = F.cylinder([bx - 0.01, ty - 0.10, 0.775], [bx - 0.01, ty + 0.10, 0.775], 0.036, 0.036, seg=8)
    out = add(out, t, n, F.own([0.40, 0.36, 0.26]), 0.95)
    t, n = F.cylinder([hx, hy + 0.115, 0.47], [hx, hy + 0.115, 0.55], 0.024, 0.024, seg=8)
    out = add(out, t, n, F.own([0.30, 0.30, 0.22]), 0.9)
    return out


def main():
    mark = os.path.join(DIR, 'DRESSED')
    if os.path.exists(mark):
        print('already dressed (remove assets/figures/DRESSED after figures3d.py made the figures anew)')
        return
    for era, fn in (('musket', musket), ('industrial', industrial)):
        for fr in FRAMES:
            p = os.path.join(DIR, f'soldier_{era}_{fr}.bin')
            a = load(p)
            b = fn(a)
            save(p, b)
            print(f'{era} {fr}: {len(a)} -> {len(b)} triangles')
    open(mark, 'w').write('figures_dress.py has been applied\n')


if __name__ == '__main__':
    main()
