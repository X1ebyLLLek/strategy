"""Builds the troops' figures for the map (assets/figures/*.bin) from free 3D models (glTF binary,
poly.pizza; authors and licences in assets/figures/CREDITS.txt): a soldier of each era (a rigged
character posed by its own animations — standing, four steps of the march, firing — with a rifle,
a helmet, a shako or a hood added, the uniform in the country's colour), a horseman (a horse
walking and galloping, a rider made here), a tank, a field gun and a cannon (one carriage, steel or
bronze), a technical and an armoured rig of the wastes.

    python -I tools/figures3d.py DIR_WITH_GLB [--preview OUT.png]

DIR holds <id>.glb, each fetched from the model's page https://poly.pizza/m/<id> (the page names
https://static.poly.pizza/<uuid>.glb). The figures' format: b'FIG1', the vertex count (u32), then
eight f32 a vertex — position (x front, y left, z up; the figure stands on z = 0), normal, the
paint, the light. A paint below 1000 is one of the game's (0 the country's cloth, 5 its painted
metal, 1 skin, 2 leather, 3 steel, 4 wood, 6 white, 7 brass, 9 rubber); from 1000 up — a colour of
its own: 1000 + r·4096 + g·64 + b (six bits each, as the screen shows it).
"""
import os, struct, sys
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from glb3d import Glb, lin_to_srgb

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'assets', 'figures')

CLOTH, SKIN, LEATHER, STEEL, WOOD, PAINTED, WHITE, BRASS, HORSE, RUBBER = 0, 1, 2, 3, 4, 5, 6, 7, 8, 9

# the sources: id, title, author, licence
SOURCES = {
    'Btfn3G5Xv4': ('SWAT', 'Quaternius', 'CC0'),
    'qvTrSG9pZF': ('Horse', 'Quaternius', 'CC0'),
    'cW3zvvkMOM': ('Tank', 'Quaternius', 'CC0'),
    '1VfiCTmbc8_': ('Cannon', 'poly.pizza author of model 1VfiCTmbc8_', 'CC-BY 3.0'),
    'RUwMItmU4B': ('Pickup Armored', 'Quaternius', 'CC0'),
    'VvX8nmoCN5': ('Truck Armored', 'Quaternius', 'CC0'),
}


def own(rgb):
    """A colour of its own (linear) as a paint code."""
    s = lin_to_srgb(np.asarray(rgb, dtype=np.float64))
    q = np.clip(np.round(s * 63), 0, 63).astype(int)
    return 1000 + q[..., 0] * 4096 + q[..., 1] * 64 + q[..., 2]


class Fig:
    """Triangles in the game's axes with a paint each."""

    def __init__(self):
        self.P = np.zeros((0, 3, 3))
        self.N = np.zeros((0, 3, 3))
        self.paint = np.zeros(0)

    def add(self, P, N, paint):
        self.P = np.concatenate([self.P, P])
        self.N = np.concatenate([self.N, N])
        self.paint = np.concatenate([self.paint, np.broadcast_to(paint, (len(P),)).astype(np.float64)])

    def bin(self):
        z = self.P[..., 2]
        top = max(z.max(), 1e-6)
        ao = 0.72 + 0.28 * np.clip(z / top, 0, 1)
        v = np.zeros((len(self.P), 3, 8), dtype=np.float32)
        v[..., :3] = self.P
        v[..., 3:6] = self.N
        v[..., 6] = self.paint[:, None]
        v[..., 7] = ao
        flat = v.reshape(-1, 8)
        return b'FIG1' + struct.pack('<I', len(flat)) + flat.tobytes()


def to_game(P, N, yaw_deg):
    """glTF (+y up) turned by `yaw` about the up axis, then to the game's axes: the model's front
    (+z after the turn) to +x, its left (+x) to +y, up to +z."""
    a = np.radians(yaw_deg)
    c, s = np.cos(a), np.sin(a)
    R = np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])
    P = P @ R.T
    N = N @ R.T
    return P[..., [2, 0, 1]], N[..., [2, 0, 1]]


def place(P, length=None, height=None):
    """Scale to a length (front to back) or a height; stand it on z = 0, its middle at x = y = 0;
    the scale."""
    lo, hi = P.reshape(-1, 3).min(0), P.reshape(-1, 3).max(0)
    k = length / (hi[0] - lo[0]) if length else height / (hi[2] - lo[2])
    mid = (lo + hi) / 2
    Q = (P - np.array([mid[0], mid[1], lo[2]])) * k
    return Q, k, np.array([mid[0], mid[1], lo[2]])


def ellipsoid(c, r, upper=False, seg=12, rings=6):
    """Triangles of an ellipsoid (its upper half: a dome)."""
    tris, nrm = [], []
    lo = 0.0 if upper else -np.pi / 2
    def pt(i, j):
        a = i / seg * 2 * np.pi
        b = lo + j / rings * (np.pi / 2 - lo)
        n = np.array([np.cos(b) * np.cos(a), np.cos(b) * np.sin(a), np.sin(b)])
        return c + n * r, n / np.maximum(r, 1e-9) / np.linalg.norm(n / np.maximum(r, 1e-9))
    for j in range(rings):
        for i in range(seg):
            p00, n00 = pt(i, j)
            p10, n10 = pt(i + 1, j)
            p11, n11 = pt(i + 1, j + 1)
            p01, n01 = pt(i, j + 1)
            tris += [[p00, p10, p11], [p00, p11, p01]]
            nrm += [[n00, n10, n11], [n00, n11, n01]]
    if upper:
        # (the brim: a flat ring under the dome)
        for i in range(seg):
            a0, a1 = i / seg * 2 * np.pi, (i + 1) / seg * 2 * np.pi
            e = lambda a, k: c + np.array([np.cos(a) * r[0] * k, np.sin(a) * r[1] * k, 0.0])
            tris += [[e(a0, 1.0), e(a1, 1.18), e(a1, 1.0)], [e(a0, 1.0), e(a0, 1.18), e(a1, 1.18)]]
            nrm += [[[0, 0, -1]] * 3, [[0, 0, -1]] * 3]
    return np.array(tris), np.array(nrm, dtype=np.float64)


def cylinder(a, b, r0, r1, seg=10, caps=True):
    """Triangles of a tapered cylinder from a to b."""
    a, b = np.asarray(a, float), np.asarray(b, float)
    d = b - a
    d /= max(np.linalg.norm(d), 1e-9)
    side = np.array([0, 0, 1.0]) if abs(d[2]) < 0.9 else np.array([1.0, 0, 0])
    u = np.cross(d, side)
    u /= np.linalg.norm(u)
    v = np.cross(d, u)
    tris, nrm = [], []
    for i in range(seg):
        a0, a1 = i / seg * 2 * np.pi, (i + 1) / seg * 2 * np.pi
        n0 = u * np.cos(a0) + v * np.sin(a0)
        n1 = u * np.cos(a1) + v * np.sin(a1)
        tris += [[a + n0 * r0, a + n1 * r0, b + n1 * r1], [a + n0 * r0, b + n1 * r1, b + n0 * r1]]
        nrm += [[n0, n1, n1], [n0, n1, n0]]
        if caps:
            tris += [[a, a + n1 * r0, a + n0 * r0], [b, b + n0 * r1, b + n1 * r1]]
            nrm += [[-d, -d, -d], [d, d, d]]
    return np.array(tris), np.array(nrm)


def box(c, half, axes):
    """Triangles of a box about c with half-sizes along three axes (columns of `axes`)."""
    c = np.asarray(c, float)
    tris, nrm = [], []
    for k in range(3):
        for sgn in (-1, 1):
            n = axes[:, k] * sgn
            o = [(k + 1) % 3, (k + 2) % 3]
            ctr = c + n * half[k]
            e1, e2 = axes[:, o[0]] * half[o[0]], axes[:, o[1]] * half[o[1]]
            q = [ctr - e1 - e2, ctr + e1 - e2, ctr + e1 + e2, ctr - e1 + e2]
            if sgn < 0:
                q = q[::-1]
            tris += [[q[0], q[1], q[2]], [q[0], q[2], q[3]]]
            nrm += [[n] * 3, [n] * 3]
    return np.array(tris), np.array(nrm)


def frame_of(d):
    """An orthonormal frame whose first axis is d."""
    d = d / max(np.linalg.norm(d), 1e-9)
    side = np.array([0, 0, 1.0]) if abs(d[2]) < 0.9 else np.array([0, 1.0, 0])
    u = np.cross(side, d)
    u /= np.linalg.norm(u)
    w = np.cross(d, u)
    return np.stack([d, u, w], 1)


def rifle(fig, butt, muzzle, kind):
    """A long arm from its butt to its muzzle: a wooden stock, a steel barrel (a musket's longer, with
    its bayonet; a raider's shorter and all dark)."""
    d = muzzle - butt
    L = np.linalg.norm(d)
    ax = frame_of(d)
    wood = own([0.28, 0.14, 0.06]) if kind != 'raider' else own([0.06, 0.06, 0.06])
    stock_len = 0.42 * L
    t, n = box(butt + ax[:, 0] * stock_len / 2, [stock_len / 2, 0.018, 0.035], ax)
    fig.add(t, n, wood)
    t, n = cylinder(butt + ax[:, 0] * stock_len * 0.9, muzzle, 0.014, 0.011, seg=6)
    fig.add(t, n, own([0.07, 0.07, 0.075]))
    if kind == 'musket':
        t, n = cylinder(muzzle, muzzle + ax[:, 0] * 0.22 * L, 0.006, 0.002, seg=4)
        fig.add(t, n, own([0.55, 0.56, 0.58]))


def pose_points(g, anim, t):
    """The world positions of the joints in a pose (glTF frame)."""
    pose = g.pose(anim, t)
    W = g.worlds(pose)
    out = {}
    for i, n in enumerate(g.j['nodes']):
        out[n.get('name', str(i))] = W[i][:3, 3]
    return out


def soldier(g, anim, t, era):
    """A soldier in a pose of the animation, of an era: 'industrial' (a steel helmet, a rifle),
    'musket' (a shako with its plume, a musket with its bayonet), 'raider' (his hood, a short gun)."""
    P, N, C, M, NN = g.triangles(anim, t)
    joints = pose_points(g, anim, t)
    fig = Fig()
    paint = np.zeros(len(P))
    for i, (m, node) in enumerate(zip(M, NN)):
        if m == 'Swat':
            paint[i] = CLOTH
        elif m == 'Skin':
            paint[i] = SKIN
        elif m == 'Visor':
            paint[i] = own([0.02, 0.02, 0.02]) if era == 'raider' else SKIN
        else:
            # the black kit: webbing and boots (a raider's hood stays black)
            paint[i] = own([0.02, 0.018, 0.016]) if era == 'raider' else (LEATHER if 'Feet' in node else own([0.09, 0.075, 0.05]))
    # into the game's axes: the character faces +z
    Pg, Ng = to_game(P, N, 0)
    fig.add(Pg, Ng, paint)
    J = {k: to_game(v[None, None, :], v[None, None, :], 0)[0][0, 0] for k, v in joints.items()}
    # the head's top: a helmet, a shako
    head = Pg[[i for i, nn in enumerate(NN) if 'Head' in nn]].reshape(-1, 3)
    lo, hi = head.min(0), head.max(0)
    hc = (lo + hi) / 2
    hr = (hi - lo) / 2
    if era == 'industrial':
        t_, n_ = ellipsoid(np.array([hc[0], hc[1], hi[2] - hr[2] * 0.55]), np.array([hr[0] * 1.12, hr[1] * 1.12, hr[2] * 0.62]), upper=True)
        fig.add(t_, n_, PAINTED)
    elif era == 'musket':
        top = hi[2] - hr[2] * 0.25
        t_, n_ = cylinder([hc[0], hc[1], top], [hc[0] + 0.01, hc[1], top + hr[2] * 1.25], hr[1] * 0.95, hr[1] * 1.08, seg=12)
        fig.add(t_, n_, own([0.03, 0.03, 0.035]))
        t_, n_ = ellipsoid(np.array([hc[0] + 0.02, hc[1], top + hr[2] * 1.45]), np.array([0.025, 0.025, 0.05]))
        fig.add(t_, n_, WHITE)
    # the arm: held across the body between the hands (standing, firing), slung on the march
    wr, wl = J.get('Wrist.R'), J.get('Wrist.L')
    long = {'industrial': 1.15, 'musket': 1.45, 'raider': 0.85}[era]
    kind = {'industrial': 'rifle', 'musket': 'musket', 'raider': 'raider'}[era]
    sh = J.get('UpperArm.R', J.get('Shoulder.R'))
    if anim in ('Idle_Gun_Shoot', 'Gun_Shoot', 'Idle_Gun_Pointing') and wr is not None and wl is not None and sh is not None:
        # aimed: the butt at the right shoulder, along the right arm (the character aims one
        # handed; the arm is the line of the aim)
        d = wr - sh
        d /= max(np.linalg.norm(d), 1e-9)
        butt = sh - d * 0.06
        rifle(fig, butt, butt + d * long, kind)
    elif anim.startswith('Idle') and wr is not None:
        # at ease: the butt by the right foot, the muzzle up past the shoulder
        foot = np.array([wr[0] + 0.04, wr[1] - 0.04, 0.03])
        rifle(fig, foot, foot + np.array([0.03, 0.0, long]), kind)
    else:
        ch = J.get('Chest', hc)
        back = np.array([ch[0] - 0.16, ch[1], ch[2]])
        rifle(fig, back + np.array([0.0, -0.18, -0.38]), back + np.array([0.0, 0.18, -0.38 + long * 0.85]), kind)
    return fig


def rider(fig, seat, scale):
    """A horseman on his saddle (made here): his coat in the country's colour, a shako, a sabre."""
    s = scale
    x, y, z = seat
    t, n = box([x, y, z + 0.02 * s], [0.12 * s, 0.13 * s, 0.025 * s], np.eye(3))
    fig.add(t, n, own([0.32, 0.06, 0.05]))
    t, n = cylinder([x - 0.01 * s, y, z + 0.04 * s], [x, y, z + 0.32 * s], 0.075 * s, 0.09 * s)
    fig.add(t, n, CLOTH)
    for sg in (-1, 1):
        t, n = cylinder([x, y + 0.07 * s * sg, z + 0.05 * s], [x + 0.12 * s, y + 0.12 * s * sg, z - 0.08 * s], 0.035 * s, 0.03 * s)
        fig.add(t, n, own([0.75, 0.73, 0.66]))
        t, n = cylinder([x + 0.12 * s, y + 0.12 * s * sg, z - 0.08 * s], [x + 0.10 * s, y + 0.13 * s * sg, z - 0.25 * s], 0.03 * s, 0.027 * s)
        fig.add(t, n, LEATHER)
    t, n = ellipsoid(np.array([x + 0.01 * s, y, z + 0.40 * s]), np.array([0.05, 0.045, 0.058]) * s)
    fig.add(t, n, SKIN)
    t, n = cylinder([x, y, z + 0.43 * s], [x, y, z + 0.56 * s], 0.05 * s, 0.058 * s)
    fig.add(t, n, own([0.03, 0.03, 0.035]))
    t, n = cylinder([x + 0.01 * s, y + 0.11 * s, z + 0.24 * s], [x + 0.16 * s, y + 0.17 * s, z + 0.58 * s], 0.006 * s, 0.003 * s, seg=4)
    fig.add(t, n, own([0.6, 0.62, 0.64]))
    t, n = cylinder([x, y + 0.10 * s, z + 0.27 * s], [x + 0.08 * s, y + 0.14 * s, z + 0.22 * s], 0.026 * s, 0.024 * s)
    fig.add(t, n, CLOTH)


def horseman(g, anim, t):
    P, N, C, M, NN = g.triangles(anim, t)
    joints = pose_points(g, anim, t)
    paint = own(C)
    Pg, Ng = to_game(P, N, 0)
    fig = Fig()
    fig.add(Pg, Ng, paint)
    back = joints.get('Back')
    if back is not None:
        b = to_game(back[None, None, :], back[None, None, :], 0)[0][0, 0]
        top = Pg.reshape(-1, 3)
        near = top[np.linalg.norm(top[:, :2] - b[:2], axis=1) < 0.6]
        z = near[:, 2].max() if len(near) else b[2] + 0.5
        rider(fig, np.array([b[0] + 0.15, b[1], z]), 4.6)
    return fig


def vehicle(g, yaw, recolour):
    """A vehicle as it is, turned to face +x; `recolour(colour, material, node)` → a paint."""
    P, N, C, M, NN = g.triangles()
    Pg, Ng = to_game(P, N, yaw)
    paint = np.array([recolour(C[i], M[i], NN[i]) for i in range(len(P))], dtype=np.float64)
    fig = Fig()
    fig.add(Pg, Ng, paint)
    return fig


def near(c, ref, tol=0.06):
    return np.abs(lin_to_srgb(np.asarray(c)) - np.asarray(ref) / 255.0).max() < tol


def build(src):
    figs = {}
    sol = Glb(os.path.join(src, 'Btfn3G5Xv4.glb'))
    frames = [('stand', 'Idle_Gun', 0.3), ('m0', 'Walk', 0.0), ('m1', 'Walk', 0.25), ('m2', 'Walk', 0.5), ('m3', 'Walk', 0.75), ('fight', 'Idle_Gun_Shoot', 0.15)]
    for era in ('industrial', 'musket', 'raider'):
        for name, anim, t in frames:
            figs[f'soldier_{era}_{name}'] = (soldier(sol, anim, t, era), 'height', 1.0)
    horse = Glb(os.path.join(src, 'qvTrSG9pZF.glb'))
    hframes = [('stand', 'AnimalArmature|Idle', 0.2), ('m0', 'AnimalArmature|Walk', 0.0), ('m1', 'AnimalArmature|Walk', 0.25), ('m2', 'AnimalArmature|Walk', 0.5), ('m3', 'AnimalArmature|Walk', 0.75), ('fight', 'AnimalArmature|Gallop', 0.3)]
    for name, anim, t in hframes:
        figs[f'rider_{name}'] = (horseman(horse, anim, t), 'height', 1.3)
    tank = Glb(os.path.join(src, 'cW3zvvkMOM.glb'))
    # (its body's main colours take the country's paint; tracks and details stay as they are)
    figs['tank'] = (vehicle(tank, 90, lambda c, m, n: PAINTED if m in ('Main', 'Main_Dark') else own(c)), 'length', 1.75)
    can = Glb(os.path.join(src, '1VfiCTmbc8_.glb'))
    def gun_paint(c, m, n, old):
        if m == 'mat19':  # the wheels
            return own([0.30, 0.17, 0.07]) if old else RUBBER
        if m == 'mat11':  # the carriage
            return own([0.20, 0.11, 0.05]) if old else PAINTED
        if m in ('mat15', 'mat21'):  # the barrel
            return BRASS if old else PAINTED
        return own(c)
    figs['cannon'] = (vehicle(can, 180, lambda c, m, n: gun_paint(c, m, n, True)), 'length', 1.45)
    gun = vehicle(can, 180, lambda c, m, n: gun_paint(c, m, n, False))
    # (a field gun of the factories: a shield of steel before its crew)
    q = gun.P.reshape(-1, 3)
    lo, hi = q.min(0), q.max(0)
    axle = q[np.abs(q[:, 1]) > 0.8 * hi[1]].mean(0)
    L = hi[0] - lo[0]
    x = axle[0] + 0.06 * L
    zc = axle[2] + 0.05 * L
    t, n = box([x, 0, zc + 0.05 * L], [0.006 * L, 0.29 * L, 0.11 * L], np.eye(3))
    gun.add(t, n, PAINTED)
    figs['gun'] = (gun, 'length', 1.45)
    pick = Glb(os.path.join(src, 'RUwMItmU4B.glb'))
    tech = vehicle(pick, 0, lambda c, m, n: own(c))
    # (a machine gun on a post over the bed)
    q = tech.P.reshape(-1, 3)
    lo, hi = q.min(0), q.max(0)
    bed = np.array([lo[0] + 0.28 * (hi[0] - lo[0]), 0.0, lo[2] + 0.55 * (hi[2] - lo[2])])
    L = hi[0] - lo[0]
    t, n = cylinder(bed, bed + np.array([0, 0, 0.22 * L]), 0.012 * L, 0.012 * L, seg=6)
    tech.add(t, n, STEEL)
    top = bed + np.array([0, 0, 0.24 * L])
    t, n = box(top + np.array([0.02 * L, 0, 0]), [0.07 * L, 0.025 * L, 0.03 * L], np.eye(3))
    tech.add(t, n, STEEL)
    t, n = cylinder(top + np.array([0.08 * L, 0, 0.005 * L]), top + np.array([0.36 * L, 0, 0.02 * L]), 0.011 * L, 0.009 * L, seg=6)
    tech.add(t, n, STEEL)
    figs['technical'] = (tech, 'length', 1.3)
    rig = Glb(os.path.join(src, 'VvX8nmoCN5.glb'))
    figs['rig'] = (vehicle(rig, 0, lambda c, m, n: own(c)), 'length', 1.7)
    return figs


def main():
    src = sys.argv[1]
    os.makedirs(OUT, exist_ok=True)
    figs = build(src)
    for name, (fig, how, val) in figs.items():
        P, k, origin = place(fig.P, length=val if how == 'length' else None, height=val if how == 'height' else None)
        fig.P = P
        open(os.path.join(OUT, name + '.bin'), 'wb').write(fig.bin())
        print(f'{name}: {len(fig.P)} triangles')
    lines = ['Фигуры войск на карте — из свободных моделей (переделаны: позы, оружие, головные уборы, окраска):', '']
    for mid, (title, author, lic) in SOURCES.items():
        lines.append(f'- «{title}» — {author}, {lic}: https://poly.pizza/m/{mid}')
    lines.append('')
    lines.append('Собрано tools/figures3d.py.')
    open(os.path.join(OUT, 'CREDITS.txt'), 'w', encoding='utf-8', newline='\n').write('\n'.join(lines) + '\n')


if __name__ == '__main__':
    main()
