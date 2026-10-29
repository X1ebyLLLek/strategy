"""Bakes a textured model (glTF/OBJ/FBX-converted, anything `trimesh` reads) into the game's figure triangles: each
triangle gets the colour of its texture at its middle (the game paints a triangle one colour, as `figures3d.py`
does), the mesh is thinned to a budget of triangles (quadric decimation; the colours carried over from the
nearest original triangle), the normals smoothed over gentle edges and left hard on sharp ones.

    from meshbake import bake
    P, N, C = bake('model.gltf', tris=6000)      # P, N: (n, 3, 3) arrays; C: (n, 3) linear-ish sRGB 0..1 per triangle

Used by `tools/figures_ph.py`.
"""
import numpy as np
import trimesh


def _texture_colours(mesh):
    """The colour (sRGB 0..1) of each face of one mesh: its texture at the face's UV middle, times the base factor."""
    vis = mesh.visual
    nf = len(mesh.faces)
    mat = getattr(vis, 'material', None)
    factor = np.ones(3)
    img = None
    if mat is not None:
        f = getattr(mat, 'baseColorFactor', None)
        if f is not None:
            factor = np.asarray(f[:3], dtype=float) / 255.0 if np.max(f) > 1.5 else np.asarray(f[:3], dtype=float)
        img = getattr(mat, 'baseColorTexture', None)
        if img is None:
            img = getattr(mat, 'image', None)
    uv = getattr(vis, 'uv', None)
    if img is not None and uv is not None and len(uv) == len(mesh.vertices):
        im = np.asarray(img.convert('RGB'), dtype=float) / 255.0
        h, w = im.shape[:2]
        fu = uv[mesh.faces].mean(1)
        x = np.mod(fu[:, 0], 1.0) * (w - 1)
        y = (1.0 - np.mod(fu[:, 1], 1.0)) * (h - 1)
        x0, y0 = np.floor(x).astype(int), np.floor(y).astype(int)
        x1, y1 = np.minimum(x0 + 1, w - 1), np.minimum(y0 + 1, h - 1)
        tx, ty = (x - x0)[:, None], (y - y0)[:, None]
        c = (im[y0, x0] * (1 - tx) * (1 - ty) + im[y0, x1] * tx * (1 - ty) + im[y1, x0] * (1 - tx) * ty + im[y1, x1] * tx * ty)
        return c * factor
    if hasattr(vis, 'face_colors') and vis.kind in ('face', 'vertex'):
        fc = np.asarray(vis.face_colors[:, :3], dtype=float) / 255.0
        if vis.kind == 'face' or fc.std() > 1e-3:
            return fc
    return np.broadcast_to(factor, (nf, 3)).copy()


def load_faces(path):
    """All triangles of a model in world space with a colour each: (P (n,3,3), C (n,3))."""
    sc = trimesh.load(path, force='scene')
    P, C = [], []
    for node in sc.graph.nodes_geometry:
        T, gname = sc.graph[node]
        g = sc.geometry[gname]
        if not isinstance(g, trimesh.Trimesh) or len(g.faces) == 0:
            continue
        col = _texture_colours(g)
        v = trimesh.transformations.transform_points(g.vertices, T)
        P.append(v[g.faces])
        C.append(col)
    return np.concatenate(P), np.concatenate(C)


def _smooth_normals(P, angle=38.0):
    """Per-corner normals: the mean of the normals of the faces meeting at the corner within `angle` of this face."""
    n = len(P)
    fn = np.cross(P[:, 1] - P[:, 0], P[:, 2] - P[:, 0])
    area = np.linalg.norm(fn, axis=1)
    fn = fn / np.maximum(area[:, None], 1e-12)
    key = np.round(P.reshape(-1, 3) / 1e-5).astype(np.int64)
    _, inv = np.unique(key, axis=0, return_inverse=True)
    inv = inv.reshape(-1)
    order = np.argsort(inv, kind='stable')
    starts = np.r_[0, np.flatnonzero(np.diff(inv[order])) + 1, len(inv)]
    out = np.repeat(fn[:, None, :], 3, axis=1).copy()
    cosl = np.cos(np.radians(angle))
    for a, b in zip(starts[:-1], starts[1:]):
        if b - a < 2:
            continue
        idx = order[a:b]
        faces = idx // 3
        nn = fn[faces] * area[faces][:, None]
        for k, i in enumerate(idx):
            m = (fn[faces] @ fn[faces[k]]) > cosl
            s = nn[m].sum(0)
            ln = np.linalg.norm(s)
            if ln > 1e-12:
                out[faces[k], i % 3] = s / ln
    return out


def _decimate(P, C, tris):
    """Thins triangles P (n,3,3) to about `tris` by quadric decimation; each new triangle takes the colour of the nearest
    original one."""
    import fast_simplification
    from scipy.spatial import cKDTree
    v = P.reshape(-1, 3)
    key = np.round(v / 1e-6).astype(np.int64)
    uniq, first, inv = np.unique(key, axis=0, return_index=True, return_inverse=True)
    inv = inv.reshape(-1)
    verts = v[first]
    faces = inv.reshape(-1, 3)
    ok = (faces[:, 0] != faces[:, 1]) & (faces[:, 1] != faces[:, 2]) & (faces[:, 0] != faces[:, 2])
    faces, C0, P0 = faces[ok], C[ok], P[ok]
    red = 1.0 - tris / len(faces)
    nv, nf = fast_simplification.simplify(verts.astype(np.float32), faces.astype(np.int32), target_reduction=float(red), agg=5)
    Q = nv[nf].astype(np.float64)
    _, j = cKDTree(P0.mean(1)).query(Q.mean(1))
    return Q, C0[j]


def bake(path, tris=6000, smooth=38.0, drop_below=None):
    """The model as game triangles, thinned to about `tris`: P (n,3,3), N (n,3,3), C (n,3)."""
    P, C = load_faces(path)
    if drop_below is not None:
        keep = P[..., 2].min(1) >= drop_below
        P, C = P[keep], C[keep]
    if len(P) > tris:
        P, C = _decimate(P, C, tris)
    N = _smooth_normals(P, smooth)
    return P, N, C


def _faces_with_uv(path, texture=None, only=None, skip=None):
    """Per mesh part: (P (n,3,3) world, UV (n,3,2) or None, image array or None, flat colour (n,3))."""
    sc = trimesh.load(path, force='scene')
    parts = []
    for node in sc.graph.nodes_geometry:
        T, gname = sc.graph[node]
        g = sc.geometry[gname]
        if not isinstance(g, trimesh.Trimesh) or len(g.faces) == 0:
            continue
        if only is not None and not any(o == gname or o == node for o in only):
            continue
        if skip is not None and any(o in gname for o in skip):
            continue
        vis = g.visual
        mat = getattr(vis, 'material', None)
        factor = np.ones(3)
        img = None
        if mat is not None:
            f = getattr(mat, 'baseColorFactor', None)
            if f is not None:
                f = np.asarray(f[:3], dtype=float)
                factor = f / 255.0 if f.max() > 1.5 else f
            img = getattr(mat, 'baseColorTexture', None)
            if img is None:
                img = getattr(mat, 'image', None)
        if img is None and mat is not None and getattr(mat, 'main_color', None) is not None:
            factor = np.asarray(mat.main_color[:3], dtype=float) / 255.0
        uv = getattr(vis, 'uv', None)
        if texture is not None:
            from PIL import Image
            img = Image.open(texture)
            factor = np.ones(3)
        v = trimesh.transformations.transform_points(g.vertices, T)
        P = v[g.faces]
        if img is not None and uv is not None and len(uv) == len(g.vertices):
            im = np.asarray(img.convert('RGB'), dtype=float) / 255.0
            parts.append((P, np.asarray(uv)[g.faces], im, factor, None))
        else:
            fc = None
            if vis.kind in ('face', 'vertex'):
                fc = np.asarray(vis.face_colors[:, :3], dtype=float) / 255.0
            if fc is None or fc.std() < 1e-4:
                fc = np.broadcast_to(factor, (len(g.faces), 3)).copy()
            parts.append((P, None, None, factor, fc))
    return parts


def _pyramid(im):
    """Box-filtered levels of an image (H, W, 3): [full, half, quarter, ...]."""
    levels = [im]
    while min(levels[-1].shape[:2]) > 1:
        a = levels[-1]
        h, w = (a.shape[0] // 2) * 2, (a.shape[1] // 2) * 2
        a = a[:h, :w]
        levels.append((a[0::2, 0::2] + a[1::2, 0::2] + a[0::2, 1::2] + a[1::2, 1::2]) / 4)
    return levels


def _bilinear(im, uv):
    h, w = im.shape[:2]
    x = np.mod(uv[:, 0], 1.0) * (w - 1)
    y = (1.0 - np.mod(uv[:, 1], 1.0)) * (h - 1)
    x0, y0 = np.floor(x).astype(int), np.floor(y).astype(int)
    x1, y1 = np.minimum(x0 + 1, w - 1), np.minimum(y0 + 1, h - 1)
    tx, ty = (x - x0)[:, None], (y - y0)[:, None]
    return (im[y0, x0] * (1 - tx) * (1 - ty) + im[y0, x1] * tx * (1 - ty) + im[y1, x0] * (1 - tx) * ty + im[y1, x1] * tx * ty)


def _sample(im, uv, factor, texel_area=None):
    """The texture at the points `uv` (n, 2); with `texel_area` (n) — the area each triangle covers in texels — the
    mean of the texture over the triangle's size (a mip level), not one sample of it: bricks do not become speckle."""
    if texel_area is None:
        return _bilinear(im, uv) * factor
    pyr = _pyramid(im)
    lvl = np.clip(np.round(0.5 * np.log2(np.maximum(texel_area, 1.0)) - 0.5), 0, len(pyr) - 1).astype(int)
    out = np.zeros((len(uv), 3))
    for l in np.unique(lvl):
        m = lvl == l
        out[m] = _bilinear(pyr[l], uv[m])
    return out * factor


def _bisect(P, UV, mask):
    """Splits the masked triangles across their longest edge (two each); P (n,3,3), UV (n,3,2)."""
    keep = ~mask
    Pm, Um = P[mask], UV[mask]
    e = np.stack([np.linalg.norm(Pm[:, 1] - Pm[:, 0], axis=1), np.linalg.norm(Pm[:, 2] - Pm[:, 1], axis=1), np.linalg.norm(Pm[:, 0] - Pm[:, 2], axis=1)], 1)
    k = e.argmax(1)  # the longest edge i -> i+1; the opposite corner i+2
    idx = np.arange(len(Pm))
    a, b, c = k, (k + 1) % 3, (k + 2) % 3
    pa, pb, pc = Pm[idx, a], Pm[idx, b], Pm[idx, c]
    ua, ub, uc = Um[idx, a], Um[idx, b], Um[idx, c]
    pm, um = (pa + pb) / 2, (ua + ub) / 2
    P1 = np.stack([pa, pm, pc], 1)
    P2 = np.stack([pm, pb, pc], 1)
    U1 = np.stack([ua, um, uc], 1)
    U2 = np.stack([um, ub, uc], 1)
    return np.concatenate([P[keep], P1, P2]), np.concatenate([UV[keep], U1, U2])


def bake_textured(path, tris=9000, smooth=38.0, texture=None, only=None, skip=None):
    """Like `bake`, but the texture is kept: the triangles that carry the most texture (the largest in texels) are
    cut across their longest edge, again and again, till `tris` is reached; then each is painted from the texture at
    its middle. A wall of two triangles becomes a grid that carries the bricks and the windows."""
    parts = _faces_with_uv(path, texture, only, skip)
    Ps, Cs = [], []
    # the texel area of each triangle of the textured parts
    work = []
    for P, UV, im, factor, fc in parts:
        if UV is None:
            Ps.append(P)
            Cs.append(fc)
        else:
            h, w = im.shape[:2]
            work.append([P, UV, im, factor])
    fixed = sum(len(p) for p in Ps)
    budget = max(tris - fixed, 1000)

    def area(UV, im):
        d1, d2 = UV[:, 1] - UV[:, 0], UV[:, 2] - UV[:, 0]
        return np.abs(d1[:, 0] * d2[:, 1] - d1[:, 1] * d2[:, 0]) / 2 * im.shape[0] * im.shape[1]

    # a threshold T of texel area: every triangle above it is cut (repeatedly) — T found by trying
    def count_for(T):
        tot = 0
        for P, UV, im, factor in work:
            a = area(UV, im)
            # a triangle of area a cut by longest-edge bisection into pieces under T takes about 2^ceil(log2(a/T))
            tot += np.where(a > T, 2.0 ** np.ceil(np.log2(np.maximum(a / T, 1.0))), 1.0).sum()
        return tot
    lo, hi = 0.25, 1e7
    for _ in range(40):
        mid = np.sqrt(lo * hi)
        if count_for(mid) > budget:
            lo = mid
        else:
            hi = mid
    T = hi
    for P, UV, im, factor in work:
        for _ in range(14):
            m = area(UV, im) > T
            if not m.any():
                break
            P, UV = _bisect(P, UV, m)
        Ps.append(P)
        Cs.append(_sample(im, UV.mean(1), factor, area(UV, im)))
    P = np.concatenate(Ps)
    C = np.concatenate(Cs)
    if len(P) > tris * 1.1:
        P, C = _decimate(P, C, tris)
    N = _smooth_normals(P, smooth)
    return P, N, C
