#version 330 core
in vec3 vN;
in vec3 vLocal;
in vec2 vCard;
in vec3 vAt;
in float vPart;
in float vAo;
in float vSeed;
in float vDepth;
in float vDead;
in vec3 vMN;
in float vTop;
in float vRuin;
in vec2 vRot;
uniform sampler2D uAtlas;
uniform sampler2DArray uMat;
uniform sampler2D uLight;
uniform vec3 uSeason;
uniform float uKind;
uniform float uEra;
uniform float uDist;
uniform float uGrow;
uniform int uMode;
out vec4 frag;
const vec3 SUN = normalize(vec3(-0.45, -0.55, 0.70));
// the materials' photographs: model units a photograph spans (clay tiles, slate, thatch, iron,
// brick, plaster, boards, stone, flat tiles, bark)
const float MSCALE[10] = float[10](0.10, 0.07, 0.12, 0.125, 0.12, 0.25, 0.15, 0.20, 0.10, 0.10);
float hash3(vec3 p) {
    p = fract(p * 0.1031);
    p += dot(p, p.zyx + 31.32);
    return fract((p.x + p.y) * p.z);
}
// leaves: a speckle of light and dark small clusters over a crown
float leafy(vec3 p) {
    vec3 i = floor(p);
    vec3 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    float n = mix(mix(mix(hash3(i), hash3(i + vec3(1, 0, 0)), f.x), mix(hash3(i + vec3(0, 1, 0)), hash3(i + vec3(1, 1, 0)), f.x), f.y),
                  mix(mix(hash3(i + vec3(0, 0, 1)), hash3(i + vec3(1, 0, 1)), f.x), mix(hash3(i + vec3(0, 1, 1)), hash3(i + vec3(1, 1, 1)), f.x), f.y), f.z);
    return n;
}
// what the light makes of a tree's place: the sun (leaves let it through a little on the
// shaded side), the sky (less inside the crown), speckled as leaves are
vec2 lightOf(vec3 n, bool leaf, float ao, vec3 local) {
    float lam = dot(n, SUN);
    float sun = leaf ? clamp(lam * 0.62 + 0.38, 0.0, 1.0) : max(lam, 0.0);
    float sky = ao * (0.62 + 0.38 * n.z);
    if (leaf) {
        float s = 0.70 + 0.55 * leafy(local * 22.0) * (0.6 + 0.4 * leafy(local * 7.0 + 3.0));
        sun *= s * (0.55 + 0.45 * ao);
        sky *= s;
    }
    return vec2(sun, sky);
}
vec3 treeColour(float kind, bool leaf, float seed, float local, float nz) {
    bool wastes = uEra > 0.5 && uEra < 1.5;
    bool old = uEra > 1.5;
    vec3 base;
    bool conifer = kind < 0.5 || (kind > 1.5 && kind < 2.5);
    if (!leaf) {
        base = kind > 2.5 && kind < 3.5 ? (fract(local * 9.0 + seed * 5.0) < 0.18 ? vec3(0.16, 0.14, 0.12) : vec3(0.82, 0.80, 0.74))
             : kind > 1.5 && kind < 2.5 ? mix(vec3(0.30, 0.20, 0.13), vec3(0.52, 0.30, 0.16), smoothstep(0.4, 1.0, local))
             : wastes ? vec3(0.22, 0.18, 0.15) : vec3(0.27, 0.20, 0.13);
    } else if (kind < 0.5) {
        // (spruces: dark blue-green, each its own shade)
        base = mix(vec3(0.05, 0.13, 0.09), vec3(0.11, 0.20, 0.12), seed);
    } else if (kind < 1.5) {
        // (broad trees: greens from olive to deep, a few already yellowing)
        float t = fract(seed * 7.13);
        base = t < 0.3 ? vec3(0.17, 0.28, 0.09) : t < 0.6 ? vec3(0.22, 0.32, 0.10) : t < 0.85 ? vec3(0.13, 0.24, 0.10) : t < 0.95 ? vec3(0.29, 0.35, 0.12) : vec3(0.40, 0.39, 0.14);
        base *= 0.88 + 0.24 * seed;
    } else if (kind < 2.5) {
        // (pines: grey-green, the needles thinner)
        base = mix(vec3(0.10, 0.17, 0.10), vec3(0.16, 0.23, 0.12), seed);
    } else if (kind < 3.5) {
        // (birches: light, yellow-green)
        base = mix(vec3(0.27, 0.37, 0.12), vec3(0.35, 0.44, 0.16), seed);
    } else {
        base = mix(vec3(0.18, 0.27, 0.10), vec3(0.25, 0.33, 0.12), seed);
    }
    if (wastes && leaf) base = mix(base, vec3(0.30, 0.26, 0.16), 0.65);
    if (old) base = mix(base, base * vec3(1.15, 1.05, 0.80), 0.6);
    // (the time of year: autumn turns the broad leaves yellow, orange and red; winter strips
    // them and lays snow on the conifers' boughs where it is cold)
    float coldness = abs(vAt.y - 0.5) * 2.0 + 0.55 * vAt.z;
    float lies = smoothstep(0.16, 0.36, coldness);
    if (leaf && !conifer) {
        if (uSeason.y > 0.01) {
            float t2 = fract(seed * 3.71);
            vec3 fall = t2 < 0.4 ? vec3(0.62, 0.48, 0.12) : t2 < 0.75 ? vec3(0.66, 0.32, 0.08) : vec3(0.52, 0.16, 0.08);
            if (kind > 2.5 && kind < 3.5) fall = vec3(0.70, 0.58, 0.16);
            base = mix(base, fall, uSeason.y * (fract(seed * 5.3) < 0.25 ? 0.15 : 0.75));
        }
        if (uSeason.x > 0.01) base = mix(base, vec3(0.30, 0.25, 0.20), uSeason.x * lies * 0.85);
    }
    if (leaf && uSeason.x > 0.01) {
        base = mix(base, vec3(0.86, 0.88, 0.92), uSeason.x * lies * smoothstep(0.2, 0.8, nz) * (conifer ? 0.75 : 0.35));
    }
    return base;
}
void main() {
    float haze = smoothstep(1.15, 2.6, vDepth / uDist);
    // (a range's shadow over the place the thing stands)
    float vSun = uMode == 1 ? 1.0 : mix(0.30, 1.0, clamp(textureLod(uLight, vAt.xy, 0.0).r * 1.15, 0.0, 1.0));
    if (uMode == 3) {
        // a shadow: soft at its edge, weaker where a range already shades the land
        float r = length(vCard);
        float a = (1.0 - smoothstep(0.35, 1.0, r)) * 0.26 * clamp(uGrow * 1.5, 0.0, 1.0) * (0.3 + 0.7 * vSun) * (1.0 - haze);
        frag = vec4(0.0, 0.0, 0.0, a);
        return;
    }
    bool tree = uKind < 4.5;
    vec3 n = normalize(vN);
    vec3 col;
    if (uMode == 1) {
        // the picture for the atlas: the sun's and the sky's light, whether leaves, how much the
        // place faces up (for the snow)
        bool leaf = vPart > 0.5;
        vec2 l = lightOf(n, leaf, vAo, vLocal);
        frag = vec4(l.x / 1.3, l.y / 1.3, (leaf ? 0.5 : 0.0) + 0.49 * clamp(n.z, 0.0, 1.0), 1.0);
        return;
    }
    vec3 sunc = vec3(1.06, 1.0, 0.88);
    vec3 skyc = vec3(0.86, 0.91, 1.0);
    if (uMode == 2) {
        // a far tree: its picture (premultiplied by its cover: the cover's own light kept at the
        // edges of the smaller copies)
        vec4 t = texture(uAtlas, vCard);
        float edge = max(fwidth(t.a), 1e-4);
        if (t.a < 0.02) discard;
        vec3 v = t.rgb / t.a;
        bool leaf = v.b >= 0.5;
        // (the wastes' dead trees: the trunk alone)
        if (leaf && vDead > 0.5) discard;
        float nz = (v.b - (leaf ? 0.5 : 0.0)) / 0.49;
        vec3 base = treeColour(uKind, leaf, vSeed, 0.5, nz);
        col = base * (sunc * 1.30 * v.r * 1.3 * vSun + skyc * 0.74 * v.g * 1.3);
        col = mix(col, vec3(0.64, 0.72, 0.80), haze * 0.55);
        // (sharp edges under the window's samples: the cover's step widened to a pixel)
        float cover = clamp((t.a - 0.5) / edge + 0.5, 0.0, 1.0);
        frag = vec4(col, cover);
        return;
    }
    if (tree) {
        bool leaf = vPart > 0.5;
        vec3 base = treeColour(uKind, leaf, vSeed, vLocal.z, n.z);
        // (the bark's photograph round the trunk; the birch keeps its own white)
        if (!leaf && (uKind < 2.5 || uKind > 3.5)) {
            vec2 bu = vec2(atan(vLocal.y, vLocal.x) * 0.02, vLocal.z * 0.5) / 0.10;
            base *= texture(uMat, vec3(bu, 9.0)).rgb * 2.4;
        }
        vec2 l = lightOf(n, leaf, vAo, vLocal);
        col = base * (sunc * 1.30 * l.x * vSun + skyc * 0.74 * l.y);
        col = mix(col, vec3(0.64, 0.72, 0.80), haze * 0.55);
        frag = vec4(col, 1.0);
        return;
    }
    // a building: walls and roof of the era's materials — photographs (`materials`) laid along the
    // walls and up the slopes, their relief turned with the building, each house its own place on
    // the photograph and its own shade; windows by storeys in frames, a door in the front, a plinth
    // at the walls' foot, the eaves' shade at their top; snow on the roofs in a cold winter
    vec3 base;
    bool wastes = uEra > 0.5 && uEra < 1.5;
    bool old = uEra > 1.5;
    int kind = int(uKind - 10.0 + 0.5);
    vec3 L = vLocal;
    bool roof = vPart > 0.5 && vPart < 1.5;
    bool chim = vPart > 4.5 && vPart < 5.5;
    bool glass = vPart > 5.5;
    float s1 = vSeed;
    float s2 = fract(vSeed * 7.31);
    float s3 = fract(vSeed * 13.7);
    vec3 mn = normalize(vMN);
    // (along the wall it is: across the model or along it)
    float u = abs(mn.x) > abs(mn.y) ? L.y : L.x;
    if (vRuin > 0.5) {
        // (a ruin: no roof, the walls broken off unevenly, burnt black at the top)
        if (roof || vPart > 4.5) discard;
        float cut = vTop * (0.45 + 0.55 * fract(sin(floor(u * 28.0) * 12.9898 + s1 * 7.0) * 43758.5453));
        if (L.z > cut && L.z > 0.06) discard;
    }
    // the materials: walls 4 brick, 5 plaster, 6 boards, 7 stone; roofs 0 clay tiles, 1 slate,
    // 2 thatch, 3 corrugated iron, 8 flat tiles (the old age: boards, plaster, thatch; the
    // staff's: brick, slate, iron)
    int wm;
    int rm;
    if (old) {
        wm = kind == 2 ? 6 : kind == 4 ? 7 : kind == 0 ? (s3 < 0.5 ? 6 : 5) : kind == 3 ? (s3 < 0.35 ? 7 : 5) : 5;
        rm = kind == 0 || kind == 2 ? (s2 < 0.65 ? 2 : 0) : kind == 4 ? 1 : (s2 < 0.55 ? 0 : 8);
    } else {
        wm = kind == 2 ? 6 : kind == 4 ? 7 : kind == 6 ? 4 : kind == 0 ? (s3 < 0.4 ? 6 : 5) : kind == 1 || kind == 3 || kind == 5 ? (s3 < 0.45 ? 4 : 5) : 5;
        rm = kind == 6 ? 3 : kind == 2 ? (s2 < 0.5 ? 3 : 8) : kind == 4 || kind == 5 ? 1 : (s2 < 0.40 ? 0 : s2 < 0.75 ? 8 : 1);
        if (wastes && kind < 4 && s2 < 0.5) rm = 3;
    }
    if (kind == 7) {
        wm = 5;
        rm = 6;
    }
    // (their own shades: plaster washed in colours, boards tarred or painted red, the rest as
    // they are; the wastes' greyed)
    vec3 wt;
    if (wm == 5) {
        wt = old ? (s1 < 0.45 ? vec3(1.0, 0.98, 0.92) : s1 < 0.70 ? vec3(0.98, 0.88, 0.70) : s1 < 0.85 ? vec3(0.86, 0.84, 0.80) : vec3(0.90, 0.78, 0.62))
                 : (s1 < 0.22 ? vec3(1.0, 0.96, 0.86) : s1 < 0.40 ? vec3(0.98, 0.82, 0.58) : s1 < 0.55 ? vec3(1.0, 0.92, 0.72) : s1 < 0.70 ? vec3(0.88, 0.87, 0.84) : s1 < 0.85 ? vec3(0.86, 0.66, 0.56) : vec3(0.96, 0.95, 0.92));
        if (kind == 7) wt = vec3(1.0, 0.97, 0.90);
    } else if (wm == 6) {
        wt = s1 < 0.3 && !old ? vec3(1.9, 0.95, 0.70) : vec3(1.35, 1.25, 1.10) * (0.85 + 0.3 * s3);
    } else {
        wt = vec3(0.92 + 0.16 * s1);
    }
    vec3 rt = rm == 3 ? (wastes || s3 < 0.3 ? vec3(1.05, 0.72, 0.52) : vec3(0.80, 0.84, 0.88)) : vec3(0.85 + 0.30 * s2);
    if (rm == 6) rt = vec3(0.9, 0.8, 0.7);
    if (wastes) wt = mix(wt, vec3(dot(wt, vec3(0.33))) * 0.85, 0.5);
    // where the photograph lies: along the wall and up it; along the eaves and up the slope
    vec2 uv;
    vec3 T;
    vec3 B;
    if (roof) {
        vec2 d = length(mn.xy) > 0.05 ? normalize(mn.xy) : vec2(0.0, 1.0);
        vec2 t2 = vec2(-d.y, d.x);
        uv = vec2(dot(L.xy, t2), -dot(L.xy, d) / max(mn.z, 0.25));
        T = vec3(t2, 0.0);
        B = normalize(cross(mn, T));
    } else {
        vec2 t2 = length(mn.xy) > 0.05 ? vec2(-mn.y, mn.x) / length(mn.xy) : vec2(1.0, 0.0);
        uv = vec2(dot(L.xy, t2), L.z);
        T = vec3(t2, 0.0);
        B = vec3(0.0, 0.0, 1.0);
    }
    int m = roof ? rm : wm;
    if (chim) m = 4;
    vec2 tuv = uv / MSCALE[m] + vec2(s1 * 7.0, s2 * 5.0);
    tuv.y = -tuv.y;
    vec3 tex = texture(uMat, vec3(tuv, float(m))).rgb;
    vec3 tn = texture(uMat, vec3(tuv, float(10 + m))).xyz * 2.0 - 1.0;
    vec3 pn = normalize(T * tn.x + B * tn.y + mn * max(tn.z, 0.3));
    base = tex * (roof ? rt : wt);
    if (glass) {
        // the north lights: panes in iron frames, the sky in them
        float bars = step(0.85, fract(uv.x * 40.0)) + step(0.80, fract(uv.y * 14.0));
        base = mix(vec3(0.30, 0.38, 0.46), vec3(0.16, 0.16, 0.17), clamp(bars, 0.0, 1.0));
        pn = mn;
    } else if (kind == 7 && vPart > 1.5) {
        // the sails: canvas
        base = vec3(0.80, 0.74, 0.60);
        pn = mn;
    } else if (chim) {
        // chimneys: brick, sooty at the top
        base = mix(tex, vec3(0.13, 0.12, 0.11), smoothstep(0.55, 1.0, L.z / (kind == 6 ? 1.15 : 0.62)));
    } else if (roof) {
        float coldness = abs(vAt.y - 0.5) * 2.0 + 0.55 * vAt.z;
        base = mix(base, vec3(0.88, 0.90, 0.94), uSeason.x * smoothstep(0.16, 0.36, coldness) * smoothstep(0.3, 0.7, mn.z) * 0.85);
    } else {
        // the walls: windows by storeys (not on the gables), framed, the sky in the glass (far
        // away only a darker wall); the door in the front's middle; the old age's half-timbering;
        // the plinth; the eaves' shade
        float storey = kind == 5 ? 0.15 : kind == 4 || kind == 6 ? 0.30 : kind == 1 ? 0.165 : 0.17;
        float colw = kind == 4 ? 0.10 : kind == 2 ? 0.16 : kind == 6 ? 0.12 : 0.09;
        float wh = kind == 4 || kind == 6 ? 0.30 : kind == 2 ? 0.10 : 0.20;
        float hw = kind == 2 ? 0.12 : kind == 6 ? 0.26 : 0.22;
        vec2 g = vec2(u / colw + 0.5, L.z / storey);
        vec2 f = fract(g) - 0.5;
        float rows = step(0.07, L.z) * step(L.z, vTop - 0.03) * (kind == 7 ? 0.0 : 1.0);
        float win = step(abs(f.x), hw) * step(abs(f.y - 0.06), wh) * rows;
        float frame = step(abs(f.x), hw + 0.06) * step(abs(f.y - 0.06), wh + 0.05) * rows * (1.0 - win);
        float aa = clamp(1.0 - max(fwidth(g.x), fwidth(g.y)) * 1.5, 0.0, 1.0);
        vec3 glassc = mix(vec3(0.08, 0.09, 0.12), vec3(0.42, 0.50, 0.60), 0.25 + 0.25 * s3);
        vec3 framec = old ? vec3(0.32, 0.24, 0.16) : wm == 4 ? vec3(0.86, 0.84, 0.78) : vec3(0.95, 0.94, 0.90);
        float few = kind == 2 ? 0.6 : 1.0;
        base = mix(base, framec, frame * aa * few * 0.9);
        base = mix(base, mix(base * 0.80, glassc, aa), win * few);
        if (win * aa > 0.5) pn = mn;
        if (old && wm == 5 && (kind == 1 || (kind == 0 && s3 < 0.75))) {
            float beam = (1.0 - step(0.012, abs(fract(L.z / storey + 0.02) - 0.02) * storey)) + (1.0 - step(0.014, abs(abs(u) - (abs(mn.x) > abs(mn.y) ? 0.17 : 0.20))));
            base = mix(base, vec3(0.20, 0.14, 0.10), clamp(beam, 0.0, 1.0) * aa * step(L.z, vTop) * (1.0 - win));
        }
        if (mn.y < -0.5 && abs(L.x) < 0.032 && L.z < 0.11 && kind != 5 && kind != 7) {
            base = abs(L.x) > 0.026 || L.z > 0.104 ? framec * 0.8 : vec3(0.26, 0.17, 0.10) * (0.85 + 0.3 * tex.r);
            pn = mn;
        }
        float plinth = 1.0 - step(0.024, L.z);
        base = mix(base, vec3(0.36, 0.34, 0.31) * (0.8 + 0.4 * dot(tex, vec3(0.33))), plinth * (wm == 7 ? 0.0 : 0.85));
        base *= mix(1.0, 0.72, smoothstep(vTop - 0.05, vTop, L.z) * step(L.z, vTop + 0.002) * (kind == 5 || kind == 6 ? 0.0 : 1.0));
        if (vRuin > 0.5) base *= mix(1.0, 0.35, smoothstep(vTop * 0.3, vTop, L.z));
    }
    // (the photograph's relief, turned as the building is)
    vec3 n2 = vec3(vRot.x * pn.x - vRot.y * pn.y, vRot.y * pn.x + vRot.x * pn.y, pn.z);
    float diff = max(dot(n2, SUN), 0.0);
    // (30th session: the walls and roofs came out nearly black — a house is a light thing on a dark land, so
    // more of the sky's light in the shade, and the shadows lifted as the land's are)
    col = base * (sunc * 0.92 * diff * vSun + skyc * 0.66 * (0.62 + 0.38 * n2.z) * mix(0.8, 1.0, vSun) + vec3(0.06, 0.055, 0.045) * (1.0 - n2.z));
    col = pow(clamp(col, 0.0, 1.0), vec3(0.80));
    col = mix(col, vec3(0.64, 0.72, 0.80), haze * 0.55);
    frag = vec4(col, 1.0);
}
