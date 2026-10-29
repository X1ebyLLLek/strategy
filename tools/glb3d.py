"""A small glTF 2.0 binary (.glb) reader: triangles in world space with a colour each corner (from
the material's base colour, its texture at the triangle's middle, or vertex colours), normals; the
node tree with its transforms; skins posed by an animation at a time (for rigged characters)."""
import io, json, struct
import numpy as np

COMP = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}
NUM = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}


class Glb:
    def __init__(self, path):
        data = open(path, 'rb').read()
        magic, ver, length = struct.unpack_from('<4sII', data, 0)
        assert magic == b'glTF', path
        off = 12
        self.bin = b''
        while off < length:
            clen, ctype = struct.unpack_from('<II', data, off)
            chunk = data[off + 8:off + 8 + clen]
            if ctype == 0x4E4F534A:
                self.j = json.loads(chunk.decode('utf-8'))
            elif ctype == 0x004E4942:
                self.bin = chunk
            off += 8 + clen
        self.textures = {}

    def acc(self, i):
        a = self.j['accessors'][i]
        n = NUM[a['type']]
        dt = COMP[a['componentType']]
        cnt = a['count']
        if 'bufferView' not in a:
            return np.zeros((cnt, n), dtype=np.float32)
        bv = self.j['bufferViews'][a['bufferView']]
        start = bv.get('byteOffset', 0) + a.get('byteOffset', 0)
        item = np.dtype(dt).itemsize * n
        stride = bv.get('byteStride', item)
        raw = np.frombuffer(self.bin, dtype=np.uint8, count=stride * (cnt - 1) + item, offset=start)
        rows = np.lib.stride_tricks.as_strided(raw, shape=(cnt, item), strides=(stride, 1))
        out = np.frombuffer(rows.copy().tobytes(), dtype=dt).reshape(cnt, n).astype(np.float32)
        if a.get('normalized'):
            if dt == np.uint8:
                out /= 255.0
            elif dt == np.uint16:
                out /= 65535.0
        return out

    def image(self, tex):
        if tex in self.textures:
            return self.textures[tex]
        from PIL import Image
        t = self.j['textures'][tex]
        img = self.j['images'][t['source']]
        bv = self.j['bufferViews'][img['bufferView']]
        raw = self.bin[bv.get('byteOffset', 0):bv.get('byteOffset', 0) + bv['byteLength']]
        im = np.asarray(Image.open(io.BytesIO(raw)).convert('RGBA'), dtype=np.float32) / 255.0
        self.textures[tex] = im
        return im

    def local(self, node, pose=None):
        n = self.j['nodes'][node]
        if pose is not None and node in pose:
            t, r, s = pose[node]
        elif 'matrix' in n:
            return np.array(n['matrix'], dtype=np.float64).reshape(4, 4).T
        else:
            t = n.get('translation', [0, 0, 0])
            r = n.get('rotation', [0, 0, 0, 1])
            s = n.get('scale', [1, 1, 1])
        return trs(t, r, s)

    def worlds(self, pose=None):
        """Each node's world matrix."""
        out = {}
        parent = {}
        for i, n in enumerate(self.j['nodes']):
            for c in n.get('children', []):
                parent[c] = i
        def w(i):
            if i in out:
                return out[i]
            m = self.local(i, pose)
            if i in parent:
                m = w(parent[i]) @ m
            out[i] = m
            return m
        for i in range(len(self.j['nodes'])):
            w(i)
        return out

    def pose(self, anim_name=None, t=0.0):
        """The nodes' TRS at time t of an animation (by name part; None: the rest pose)."""
        if anim_name is None or 'animations' not in self.j:
            return None
        anims = self.j['animations']
        pick = [a for a in anims if anim_name.lower() in a.get('name', '').lower()]
        if not pick:
            return None
        a = pick[0]
        pose = {}
        for i, n in enumerate(self.j['nodes']):
            pose[i] = [list(n.get('translation', [0, 0, 0])), list(n.get('rotation', [0, 0, 0, 1])), list(n.get('scale', [1, 1, 1]))]
        for ch in a['channels']:
            smp = a['samplers'][ch['sampler']]
            times = self.acc(smp['input'])[:, 0]
            vals = self.acc(smp['output'])
            node = ch['target'].get('node')
            path = ch['target']['path']
            if node is None or path == 'weights':
                continue
            tt = min(max(t * times[-1] if t <= 1.0 else t, times[0]), times[-1])
            k = int(np.searchsorted(times, tt, side='right') - 1)
            k = max(0, min(k, len(times) - 2)) if len(times) > 1 else 0
            if smp.get('interpolation') == 'CUBICSPLINE':
                vals = vals.reshape(len(times), 3, -1)[:, 1, :]
            if len(times) == 1:
                v = vals[0]
            else:
                f = (tt - times[k]) / max(times[k + 1] - times[k], 1e-9)
                f = min(max(f, 0.0), 1.0)
                if path == 'rotation':
                    v = slerp(vals[k], vals[k + 1], f)
                else:
                    v = vals[k] * (1 - f) + vals[k + 1] * f
            idx = {'translation': 0, 'rotation': 1, 'scale': 2}[path]
            pose[node][idx] = list(v)
        return pose

    def animations(self):
        return [a.get('name', '') for a in self.j.get('animations', [])]

    def triangles(self, anim=None, t=0.0, skip=()):
        """All triangles: (N,3,3) positions, (N,3,3) normals, (N,3) colours (linear), material
        names, node names."""
        pose = self.pose(anim, t)
        W = self.worlds(pose)
        P, N, C, M, NN = [], [], [], [], []
        UV = []
        scene = self.j.get('scene', 0)
        roots = self.j['scenes'][scene]['nodes'] if 'scenes' in self.j else list(range(len(self.j['nodes'])))
        seen = set()
        stack = list(roots)
        order = []
        while stack:
            i = stack.pop()
            if i in seen:
                continue
            seen.add(i)
            order.append(i)
            stack.extend(self.j['nodes'][i].get('children', []))
        for i in order:
            node = self.j['nodes'][i]
            if 'mesh' not in node:
                continue
            name = node.get('name', '')
            if any(s.lower() in name.lower() for s in skip):
                continue
            mesh = self.j['meshes'][node['mesh']]
            skin = node.get('skin')
            for prim in mesh['primitives']:
                if prim.get('mode', 4) != 4:
                    continue
                at = prim['attributes']
                pos = self.acc(at['POSITION'])[:, :3].astype(np.float64)
                nrm = self.acc(at['NORMAL'])[:, :3].astype(np.float64) if 'NORMAL' in at else None
                if skin is not None and 'JOINTS_0' in at:
                    sk = self.j['skins'][skin]
                    inv = self.acc(sk['inverseBindMatrices']).reshape(-1, 4, 4).transpose(0, 2, 1).astype(np.float64)
                    joints = sk['joints']
                    jm = np.stack([W[joints[k]] @ inv[k] for k in range(len(joints))])
                    J = self.acc(at['JOINTS_0']).astype(np.int64)
                    Wt = self.acc(at['WEIGHTS_0']).astype(np.float64)
                    Wt /= np.maximum(Wt.sum(1, keepdims=True), 1e-9)
                    mats = np.einsum('vk,vkij->vij', Wt, jm[J])
                    ph = np.concatenate([pos, np.ones((len(pos), 1))], 1)
                    pos = np.einsum('vij,vj->vi', mats, ph)[:, :3]
                    if nrm is not None:
                        nrm = np.einsum('vij,vj->vi', mats[:, :3, :3], nrm)
                else:
                    m = W[i]
                    pos = (np.concatenate([pos, np.ones((len(pos), 1))], 1) @ m.T)[:, :3]
                    if nrm is not None:
                        nrm = nrm @ np.linalg.inv(m[:3, :3])
                idx = self.acc(prim['indices'])[:, 0].astype(np.int64) if 'indices' in prim else np.arange(len(pos))
                tri = idx.reshape(-1, 3)
                tp = pos[tri]
                if nrm is None:
                    fn = np.cross(tp[:, 1] - tp[:, 0], tp[:, 2] - tp[:, 0])
                    fn /= np.maximum(np.linalg.norm(fn, axis=1, keepdims=True), 1e-12)
                    tn = np.repeat(fn[:, None, :], 3, 1)
                else:
                    tn = nrm[tri]
                    tn /= np.maximum(np.linalg.norm(tn, axis=2, keepdims=True), 1e-12)
                # colour: the material's factor, times its texture at the triangle's middle, times
                # vertex colours
                mat = self.j['materials'][prim['material']] if 'material' in prim else {}
                pbr = mat.get('pbrMetallicRoughness', {})
                base = np.array(pbr.get('baseColorFactor', [1, 1, 1, 1])[:3], dtype=np.float64)
                col = np.tile(base, (len(tri), 1))
                if 'baseColorTexture' in pbr and 'TEXCOORD_0' in at:
                    uv = self.acc(at['TEXCOORD_0'])[tri].mean(1)
                    im = self.image(pbr['baseColorTexture']['index'])
                    h, w = im.shape[:2]
                    x = np.clip((np.mod(uv[:, 0], 1.0) * w).astype(int), 0, w - 1)
                    y = np.clip((np.mod(uv[:, 1], 1.0) * h).astype(int), 0, h - 1)
                    tc = im[y, x, :3].astype(np.float64)
                    col = col * srgb_to_lin(tc)
                if 'COLOR_0' in at:
                    vc = self.acc(at['COLOR_0'])[:, :3].astype(np.float64)
                    col = col * vc[tri].mean(1)
                if 'TEXCOORD_0' in at:
                    UV.append(self.acc(at['TEXCOORD_0'])[tri].mean(1))
                else:
                    UV.append(np.zeros((len(tri), 2)))
                P.append(tp)
                N.append(tn)
                C.append(col)
                M.extend([mat.get('name', '')] * len(tri))
                NN.extend([name] * len(tri))
        # (each triangle's middle in the texture, in the same order — for a model whose texture came apart from it)
        self.last_uv = np.concatenate(UV)
        return np.concatenate(P), np.concatenate(N), np.concatenate(C), M, NN


def srgb_to_lin(c):
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def lin_to_srgb(c):
    c = np.clip(c, 0, 1)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)


def trs(t, r, s):
    x, y, z, w = r
    R = np.array([
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
    ])
    m = np.eye(4)
    m[:3, :3] = R * np.array(s)[None, :]
    m[:3, 3] = t
    return m


def slerp(a, b, f):
    a = np.array(a, dtype=np.float64)
    b = np.array(b, dtype=np.float64)
    d = np.dot(a, b)
    if d < 0:
        b, d = -b, -d
    if d > 0.9995:
        v = a + f * (b - a)
        return v / np.linalg.norm(v)
    th = np.arccos(d)
    return (np.sin((1 - f) * th) * a + np.sin(f * th) * b) / np.sin(th)
