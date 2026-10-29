#version 330 core
in vec2 vWorld;
in float vDepth;
uniform sampler2D uHeight;
uniform sampler2D uNormal;
uniform sampler2D uBase;
uniform sampler2D uIds;
uniform sampler2D uPal;
uniform sampler2D uSplatA;
uniform sampler2D uSplatB;
uniform sampler2DArray uPhotos;
uniform sampler2DArray uAerial;
uniform sampler2D uDetail;
uniform sampler2D uLines;
uniform sampler2D uWays;
uniform sampler2D uLight;
uniform vec2 uSize;
uniform float uK;
uniform float uTime;
uniform float uOverlay;
uniform float uBorders;
uniform float uClouds;
uniform float uWar;
uniform float uRelief;
uniform float uDist;
uniform float uStripe;
uniform float uEra;
uniform float uHypso;
uniform int uDbg;
uniform vec3 uSeason;
uniform int uPalW;
uniform vec3 uEye;
out vec4 frag;

const vec3 SUN = normalize(vec3(-0.45, -0.55, 0.70));

// value noise that goes round with the world (period in lattice cells along x); the hash of a
// lattice cell: Dave Hoskins' "hash without sine" (the cheaper one before went flat along whole
// rows of cells — the sea showed stripes)
float hash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}
float vnoise(vec2 p, float period) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    float x0 = mod(i.x, period);
    float x1 = mod(i.x + 1.0, period);
    float a = hash(vec2(x0, i.y));
    float b = hash(vec2(x1, i.y));
    float c = hash(vec2(x0, i.y + 1.0));
    float d = hash(vec2(x1, i.y + 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}
// fbm of world pixels at a base frequency (cells per world pixel; uSize.x * freq must be whole)
float fbm(vec2 w, float freq, int octaves) {
    float s = 0.0;
    float a = 0.5;
    float t = 0.0;
    for (int i = 0; i < octaves; i++) {
        s += a * vnoise(w * freq, uSize.x * freq);
        t += a;
        freq *= 2.0;
        a *= 0.5;
    }
    return s / t;
}

// value noise with its gradient (per lattice cell), in one pass
vec3 vnoised(vec2 p, float period) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    vec2 du = 6.0 * f * (1.0 - f);
    float x0 = mod(i.x, period);
    float x1 = mod(i.x + 1.0, period);
    float a = hash(vec2(x0, i.y));
    float b = hash(vec2(x1, i.y));
    float c = hash(vec2(x0, i.y + 1.0));
    float d = hash(vec2(x1, i.y + 1.0));
    float k = a - b - c + d;
    return vec3(a + (b - a) * u.x + (c - a) * u.y + k * u.x * u.y, du * vec2((b - a) + k * u.y, (c - a) + k * u.x));
}
// fbm with its gradient per world pixel
vec3 fbmd(vec2 w, float freq, int octaves) {
    vec3 s = vec3(0.0);
    float a = 0.5;
    float t = 0.0;
    for (int i = 0; i < octaves; i++) {
        vec3 n = vnoised(w * freq, uSize.x * freq);
        s += a * vec3(n.x, n.yz * freq);
        t += a;
        freq *= 2.0;
        a *= 0.5;
    }
    return s / t;
}

int idAt(vec2 w) {
    vec4 t = texture(uIds, w / uSize);
    return int(t.r * 255.0 + 0.5) + 256 * int(t.g * 255.0 + 0.5);
}
vec4 pal(int id, int row) {
    return texelFetch(uPal, ivec2(id % uPalW, (id / uPalW) * 3 + row), 0);
}
// the distance to a line kept as the step to its nearest point (two channels, 128 ± 127 for
// ± range; both 255 — none near): the steps of neighbouring texels mix exactly, so thin lines
// stay whole
float lineDist(vec2 c, float range) {
    if (c.x > 0.999 && c.y > 0.999) return range * 2.0;
    return length((c * 255.0 - 128.0) / 127.0 * range);
}

// the colour of a height as an atlas shows it (of the highest peak; below 0 — under the sea's
// level): deep green below the sea, greens of the lowlands, yellows and browns of the uplands, the
// grey and white of the high ranges (`map::Mode::ramp` gives the legend the same)
vec3 hypsoColour(float e) {
    const float AT[8] = float[8](-0.06, 0.0, 0.04, 0.10, 0.20, 0.36, 0.62, 1.0);
    const vec3 C[8] = vec3[8](vec3(0.20, 0.38, 0.30), vec3(0.36, 0.56, 0.36), vec3(0.56, 0.70, 0.42), vec3(0.82, 0.80, 0.52),
                              vec3(0.80, 0.64, 0.40), vec3(0.62, 0.44, 0.30), vec3(0.62, 0.58, 0.56), vec3(0.96, 0.96, 0.96));
    if (e <= AT[0]) return C[0];
    for (int i = 0; i < 7; i++) {
        if (e <= AT[i + 1]) return mix(C[i], C[i + 1], (e - AT[i]) / (AT[i + 1] - AT[i]));
    }
    return C[7];
}

// how far from the equator a world point lies (0 — on it, 1 — at a pole)
float latc_of(vec2 w) {
    return abs(w.y / uSize.y - 0.5) * 2.0;
}

// how bright the photographs of the forests are against the photographs of the ground
const float FOREST_GAIN = 1.15;
// the land seen from above (`uAerial`: a photograph — broad-leaved forest 0, coniferous 1, marsh 2),
// a photograph `AERIAL` world pixels across, laid with its repeats hidden: the land in patches,
// each patch the photograph shifted its own way, neighbouring patches blended (after Inigo Quilez's
// texture variation); `dx`, `dy` — the world's change across a screen pixel
const float AERIAL = 32.0;
vec3 aerialAt(float layer, vec2 w, vec2 dx, vec2 dy) {
    vec2 uv = w / AERIAL;
    float l = vnoise(w * 0.0625, uSize.x * 0.0625) * 8.0;
    float ia = floor(l);
    float f = fract(l);
    vec2 oa = fract(sin(vec2(3.0, 7.0) * ia) * 43.17);
    vec2 ob = fract(sin(vec2(3.0, 7.0) * (ia + 1.0)) * 43.17);
    vec3 a = textureGrad(uAerial, vec3(uv + oa, layer), dx / AERIAL, dy / AERIAL).rgb;
    vec3 b = textureGrad(uAerial, vec3(uv + ob, layer), dx / AERIAL, dy / AERIAL).rgb;
    return mix(a, b, smoothstep(0.2, 0.8, f - 0.1 * dot(a - b, vec3(1.0))));
}

// the tilled land seen from above (`uAerial` layers 3 and 4: two farmlands of Lancaster County,
// Pennsylvania — strips of crops, hedges, farmsteads), a photograph `FARM` world pixels across
// (a whole number of them round the world), laid in shifted patches as `aerialAt`
const float FARM = 32.0;
vec3 farmAt(float layer, vec2 w, vec2 dx, vec2 dy) {
    vec2 uv = w / FARM;
    float l = vnoise(w * 0.03125, uSize.x * 0.03125) * 8.0;
    float ia = floor(l);
    float f = fract(l);
    vec2 oa = fract(sin(vec2(5.0, 11.0) * ia) * 43.17);
    vec2 ob = fract(sin(vec2(5.0, 11.0) * (ia + 1.0)) * 43.17);
    vec3 a = textureGrad(uAerial, vec3(uv + oa, layer), dx / FARM, dy / FARM).rgb;
    vec3 b = textureGrad(uAerial, vec3(uv + ob, layer), dx / FARM, dy / FARM).rgb;
    return mix(a, b, smoothstep(0.2, 0.8, f - 0.1 * dot(a - b, vec3(1.0))));
}

// the country shown on a province (+1; 0 — none), and its side in the war
ivec2 who(int id) {
    if (id == 0) return ivec2(0, 9);
    vec4 p = pal(id, 2);
    if (p.a > 0.9) return ivec2(-1, 9);
    return ivec2(int(p.r * 255.0 + 0.5) + 256 * int(p.g * 255.0 + 0.5), int(p.b * 255.0 + 0.5));
}

void main() {
    vec2 uv = vWorld / uSize;
    // the land's height as shown (a basin may lie below the sea), and the world's own: water < 0
    vec2 hv = texture(uHeight, uv).rg;
    float h = hv.r;
    float hs = hv.g;
    if (vWorld.y < 0.0 || vWorld.y > uSize.y) {
        h = 0.0;
        hs = -0.5;
    }
    vec3 eye = uEye;
    vec3 pos = vec3(vWorld, h * uRelief);
    vec3 V = normalize(eye - pos);
    vec3 col;
    // how many world pixels a screen pixel takes here
    vec2 px = max(fwidth(vWorld), vec2(1e-4));
    vec2 wdx = dFdx(vWorld);
    vec2 wdy = dFdy(vWorld);
    float near = clamp((uK - 1.5) / 4.0, 0.0, 1.0);
    if (hs < 0.0) {
        // the sea: deeper is darker; waves (two layers drifting, as fine as the screen allows —
        // they fade before they would shimmer), the sun's glint, the sky in it; surf close up
        float depth = clamp(-hs, 0.0, 1.0);
        vec3 shallow = vec3(0.19, 0.42, 0.50);
        vec3 deep = vec3(0.05, 0.14, 0.29);
        col = mix(shallow, deep, smoothstep(0.0, 0.32, sqrt(depth)));
        float pxw = max(px.x, px.y);
        float lod = smoothstep(0.50, 0.16, pxw);
        vec2 q = vWorld;
        float t = uTime;
        vec2 o1 = vec2(t * 0.9, t * 0.5);
        vec2 o2 = vec2(-t * 0.6, t * 0.8);
        vec3 n = vec3(0.0, 0.0, 1.0);
        if ((uDbg & 8) != 0) lod = 0.0;
        if (lod > 0.01) {
            vec2 g = fbmd(q + o1, 0.125, 3).yz + 0.5 * fbmd(q + o2, 0.25, 2).yz;
            n = normalize(vec3(-g * 3.0 * lod, 1.0));
        }
        // the swell: long low waves from four quarters (their slopes exact, no noise's seams),
        // patchy over the sea; seen at the middle distances where the small ones are gone
        float lod2 = smoothstep(3.2, 1.1, pxw) * (1.0 - 0.6 * lod);
        if (lod2 > 0.01) {
            vec2 slope = vec2(0.0);
            for (int i = 0; i < 4; i++) {
                vec2 d = normalize(i == 0 ? vec2(0.8, 0.6) : i == 1 ? vec2(-0.5, 0.86) : i == 2 ? vec2(0.95, -0.3) : vec2(0.2, 0.98));
                float len = i == 0 ? 41.0 : i == 1 ? 29.0 : i == 2 ? 19.0 : 13.0;
                float kk = 6.2831853 / len;
                float ph = dot(d, q) * kk + t * sqrt(9.8 * kk) * 0.6 + float(i) * 1.7;
                slope += d * cos(ph) * (0.34 - 0.05 * float(i));
            }
            float patchy = 0.75 + 0.35 * sin(dot(q, vec2(0.023, 0.017)) + t * 0.15) * sin(dot(q, vec2(-0.013, 0.029)) - t * 0.11);
            vec2 s = slope * patchy * lod2 * 0.35;
            n = normalize(n + vec3(-s, 0.0));
        }
        vec3 H = normalize(V + SUN);
        // (31st session: from afar the glints are broad and faint — sharp ones lay white shards on the open sea)
        float spec = pow(max(dot(n, H), 0.0), mix(18.0, 60.0, lod)) * max(lod, 0.35 * lod2) * mix(0.55, 1.0, lod);
        float fres = pow(1.0 - max(dot(n, V), 0.0), 4.0);
        col = mix(col, vec3(0.42, 0.55, 0.68), fres * 0.22 * max(lod, lod2));
        col *= 1.0 + 0.18 * max(lod, lod2) * (dot(n, SUN) - SUN.z);
        col += vec3(1.0, 0.95, 0.85) * spec * 0.16;
        // the shelf: a lighter band along the coast; surf right at the shore, close up
        float shore = smoothstep(-0.012, 0.0, hs);
        col = mix(col, vec3(0.30, 0.52, 0.55), smoothstep(-0.05, 0.0, hs) * 0.30);
        float foam = smoothstep(0.55, 0.9, fbm(q * 1.0 + vec2(t * 0.4, 0.0), 0.5, 2) + 0.35 * sin(t * 1.5 - depth * 900.0));
        col = mix(col, vec3(0.86, 0.90, 0.88), shore * foam * 0.5 * lod * smoothstep(1.5, 3.5, uK));
    } else {
        // the land: the photographs of its kinds of ground, each as much as lies there (the
        // borders between kinds broken by the photographs' own light and dark), at two scales so
        // that no tile repeats to the eye; tinted a little by the land's colour; lit by the sun
        vec4 b = texture(uBase, uv);
        vec4 sa = texture(uSplatA, uv);
        vec4 sb = texture(uSplatB, uv);
        float wk[8] = float[8](sa.r, sa.g, sa.b, sa.a, sb.r, sb.g, sb.b, sb.a);
        // (MAPDBG 32: the kinds of ground as colours — red: plains and forest, green: hills, mountains, tundra, blue: desert, marsh, snow; with 64 added — red: tundra, green: hills, blue: mountains)
        if ((uDbg & 32) != 0) {
            frag = (uDbg & 64) != 0 ? vec4(wk[6], wk[2], wk[3], 1.0) : vec4(wk[0] + wk[1], wk[2] + wk[3] + wk[6], wk[4] + wk[5] + wk[7], 1.0);
            return;
        }
        vec2 t1 = vWorld / 16.0;
        vec2 t2 = vWorld / 64.0 + vec2(0.37, 0.61);
        vec3 c = vec3(0.0);
        vec3 nd = vec3(0.0);
        float sum = 0.0;
        float bestw = 0.0;
        float bestl = 0.0;
        for (int i = 0; i < 8; i++) {
            if (wk[i] < 0.02 || (uDbg & 2) != 0) continue;
            // (the forests' own ground — their glades, round the canopy — is the meadow's)
            float li = i == 1 ? 0.0 : float(i);
            vec3 a = texture(uPhotos, vec3(t1, li)).rgb;
            vec3 a2 = texture(uPhotos, vec3(t2, li)).rgb;
            vec3 tc = mix(a, a2, 0.35);
            if (wk[i] > bestw) {
                bestw = wk[i];
                bestl = li;
            }
            float lift = dot(tc, vec3(0.33));
            float wgt = wk[i] * wk[i] * (0.35 + lift);
            c += wgt * tc;
            nd += wgt * (texture(uPhotos, vec3(t1, 8.0 + li)).xyz * 2.0 - 1.0);
            sum += wgt;
        }
        c /= max(sum, 1e-4);
        nd /= max(sum, 1e-4);
        float lc = dot(c, vec3(0.3, 0.59, 0.11));
        float lb = dot(b.rgb, vec3(0.3, 0.59, 0.11));
        // (not in the forests' zone: their dark colour by the world's pixels showed in steps round the
        // canopy)
        col = mix(c, b.rgb * (lc / max(lb, 1e-3)), 0.30 * (1.0 - wk[1]));
        float grain = texture(uNormal, uv).a;
        col *= 0.86 + 0.28 * grain;
        // (32nd session: close up the ground was soft paint — one photograph per 16 world pixels, stretched. A second look
        // at the dominant ground's photograph at four times the frequency adds its small light and dark (the
        // photograph's own, graded against its mean), and a fine grain of blades and grit over it: the ground
        // gets a crisp tooth as the camera comes down, and nothing at the middle zoom, where it would only glitter)
        {
            float pxd = max(px.x, px.y);
            float closeD = smoothstep(0.13, 0.035, pxd);
            if (closeD > 0.01 && (uDbg & 2) == 0) {
                vec3 dp = texture(uPhotos, vec3(vWorld / 3.7 + vec2(0.21, 0.47), bestl)).rgb;
                vec3 mean = textureLod(uPhotos, vec3(0.5, 0.5, bestl), 9.0).rgb;
                float dl = dot(dp, vec3(0.3, 0.59, 0.11)) / max(dot(mean, vec3(0.3, 0.59, 0.11)), 0.05);
                float fine = vnoise(vWorld * 23.0, uSize.x * 23.0) - 0.5;
                col *= mix(1.0, clamp(dl, 0.62, 1.55), 0.55 * closeD) * (1.0 + 0.16 * fine * closeD);
            }
        }
        // broad dark patches of growth on gentle ground (thicker grass, copses, damp hollows):
        // darker and a little greener, as a painted map shows them from afar
        // (made once with the world's layers: `ground::layers`)
        float lumc = dot(col, vec3(0.3, 0.59, 0.11));
        // (30th session: the dark growth lighter and greener — under a country's tint it was grey-violet mud)
        vec3 lush = mix(col, vec3(lumc * 0.72, lumc * 1.06, lumc * 0.58), 0.50) * 0.88;
        col = mix(col, lush, b.a * 0.75);
        // (33rd session: the deserts and dry basins were one grey-lilac smear from the middle zoom on. Now broad swells of
        // sand, ochre and pale crust, and wind-combed dunes lit from the sun's side, as an aerial photograph of a sandy waste;
        // the dunes give way to the swells as the pixel grows)
        float des = smoothstep(0.20, 0.70, wk[4]);
        if (des > 0.01 && (uDbg & 2) == 0) {
            float ld = dot(col, vec3(0.3, 0.59, 0.11));
            float sw = fbm(vWorld + vec2(41.0, 17.0), 1.0 / 14.0, 3);
            vec3 sandc = vec3(ld * 1.26, ld * 1.06, ld * 0.80);
            vec3 ochre = vec3(ld * 1.34, ld * 0.99, ld * 0.70);
            vec3 pale = vec3(ld * 1.10, ld * 1.05, ld * 0.97);
            vec3 tone = mix(mix(sandc, ochre, smoothstep(0.35, 0.70, sw)), pale, 0.8 * smoothstep(0.55, 0.85, fbm(vWorld + 9.0, 1.0 / 22.0, 2)));
            col = mix(col, tone, des * 0.65);
            float fineD = 1.0 - smoothstep(0.35, 1.3, max(px.x, px.y));
            if (fineD > 0.01) {
                vec2 dq = vec2(dot(vWorld, vec2(0.87, 0.5)), dot(vWorld, vec2(-0.5, 0.87)));
                vec2 s2 = vec2(dq.x * 0.22, dq.y * 1.6) + 2.2 * (fbm(vWorld + 5.0, 1.0 / 24.0, 2) - 0.5);
                float d0 = fbm(s2, 1.0 / 5.0, 3);
                float d1 = fbm(s2 + vec2(0.0, 0.35), 1.0 / 5.0, 3);
                col *= 1.0 + 0.19 * clamp((d1 - d0) * 6.0, -1.0, 1.0) * fineD * des;
            }
        }
        // (33rd session: the tundra was a dark grey smear — its photograph has no light and dark to speak of. Now moss, heath and
        // pale lichen in mottled patches, as the tundra is from above; the small patches give way to the large as the pixel grows)
        float tun = smoothstep(0.25, 0.75, wk[6]);
        if (tun > 0.01 && (uDbg & 2) == 0) {
            float lt = dot(col, vec3(0.3, 0.59, 0.11));
            float m1 = fbm(vWorld + vec2(13.0, 71.0), 1.0 / 5.0, 3);
            float m2 = fbm(vWorld + vec2(88.0, 3.0), 1.0 / 18.0, 3);
            float fineT = 1.0 - smoothstep(0.5, 2.2, max(px.x, px.y));
            vec3 tc = mix(mix(vec3(0.40, 0.46, 0.27), vec3(0.47, 0.36, 0.26), smoothstep(0.42, 0.62, m2)), vec3(0.68, 0.69, 0.55), 0.75 * smoothstep(0.50, 0.72, m1) * fineT);
            tc *= 0.86 + 0.40 * (m1 - 0.5) * fineT + 0.8 * (lt - 0.25);
            col = mix(col, tc, tun * 0.72);
        }
        vec4 dt = texture(uDetail, uv);
        vec4 ln = texture(uLines, uv);
        vec4 wy = texture(uWays, uv);
        // (30th session, the second hand: the mountains were crumpled foil at the middle zoom — the small folds, the
        // snow's breakup and the rock's grain are finer than a screen pixel there and only glitter; they give way
        // to the large forms as the pixel grows)
        float coarse = smoothstep(0.28, 0.95, max(px.x, px.y));
        float cav = (dt.r * 2.0 - 1.0) * mix(1.0, 0.40, coarse);
        // the tilled land round the towns and villages: fields in patches, each patch of fields
        // turned its own way, each field its own crop, hedges between, furrows close up
        if ((uDbg & 4) != 0) { dt = vec4(0.5, 0.0, 0.0, 0.0); ln = vec4(1.0); wy = vec4(1.0); }
        float farm = dt.g * smoothstep(1.2, 2.2, uK) * (uEra > 0.5 && uEra < 1.5 ? 0.55 : 1.0);
        if (farm > 0.01) {
            // (two farmlands, region by region; the photographs graded to the ground's light)
            float which = smoothstep(0.35, 0.65, vnoise(vWorld * 0.03125, uSize.x * 0.03125));
            vec3 crop = mix(farmAt(3.0, vWorld + 13.0, wdx, wdy), farmAt(4.0, vWorld + 71.0, wdx, wdy), which);
            float lc = dot(crop, vec3(0.3, 0.59, 0.11));
            crop = mix(vec3(lc), crop, 1.55) * (uEra > 1.5 ? vec3(1.08, 1.0, 0.86) : vec3(1.02, 1.04, 0.92));
            crop = clamp((crop - 0.45) * 1.35 + 0.47, 0.0, 1.0);
            crop *= 0.94 + 0.12 * grain;
            col = mix(col, crop, min(farm * 1.25, 0.95));
        }
        // the towns' built-up ground: trodden earth and cobbles, yards and gardens between
        float town = dt.a * smoothstep(1.4, 2.4, uK);
        if (town > 0.01) {
            vec3 earth = uEra > 1.5 ? vec3(0.50, 0.42, 0.31) : uEra > 0.5 ? vec3(0.36, 0.33, 0.30) : vec3(0.47, 0.44, 0.40);
            float cob = mix(1.0, 0.86 + 0.24 * hash(floor(vWorld * 14.0)), smoothstep(0.05, 0.02, px.x));
            float yards = smoothstep(0.45, 0.70, fbm(vWorld, 1.0 / 1.5, 2)) * (1.0 - smoothstep(0.7, 1.0, dt.a));
            vec3 ground = mix(earth * cob * (0.86 + 0.28 * grain), col * vec3(0.92, 1.0, 0.86), yards);
            col = mix(col, ground, town * 0.85);
        }
        // banks of the rivers: wet earth, and greener growth along them
        float dc = lineDist(ln.rg, 6.0);
        // (33rd session: between two rivers a few pixels apart the steps to the two lines point opposite ways, and the card's mixing
        // of them reads «on the line» along the whole middle — a ghost river of jagged dashes. The true distance is never
        // shorter than the shortest step of the four texels round the point less the way to that texel.)
        {
            vec2 ts = vec2(textureSize(uLines, 0));
            ivec2 t0 = ivec2(floor(uv * ts - 0.5));
            float low = 1e9;
            for (int j = 0; j < 2; j++) {
                for (int i = 0; i < 2; i++) {
                    ivec2 tc = ivec2((t0.x + i + int(ts.x)) % int(ts.x), clamp(t0.y + j, 0, int(ts.y) - 1));
                    low = min(low, lineDist(texelFetch(uLines, tc, 0).rg, 6.0));
                }
            }
            dc = max(dc, low - 0.85 * uSize.x / ts.x);
        }
        float rhw = dt.b * 4.0;
        float pxr = max(px.x, px.y);
        float least = uK < 1.2 ? 0.30 : uK < 2.0 ? 0.14 : 0.0;
        float vis = rhw >= least ? max(rhw, 0.95 * pxr) : rhw;
        float near_r = step(dc, 5.99);
        // (33rd session: the great rivers are many pixels wide; their banks and flood plains widen with them)
        float great = smoothstep(0.35, 1.8, rhw);
        float water = (1.0 - smoothstep(vis - 0.5 * pxr, vis + 0.5 * pxr, dc)) * near_r;
        float bank = (1.0 - smoothstep(vis, vis + 0.30 + 0.12 * rhw + pxr, dc)) * (1.0 - water) * near_r;
        float meadow = (1.0 - smoothstep(vis, min(vis + 1.6 + 0.9 * rhw, 5.6), dc)) * near_r;
        float lumm = dot(col, vec3(0.3, 0.59, 0.11));
        col = mix(col, vec3(lumm * 0.80, lumm * 1.05, lumm * 0.60), meadow * (0.30 + 0.22 * great) * (1.0 - farm) * (1.0 - smoothstep(0.55, 0.80, lumm)));
        col = mix(col, col * vec3(0.66, 0.62, 0.55), bank * 0.8);
        float pxc = max(px.x, px.y);
        // the land's slopes to the sun (the photographs' own small relief less on the rock: up
        // close its every crack lit and black read as hammered metal)
        vec3 n = texture(uNormal, uv).xyz * 2.0 - 1.0;
        n = normalize(n + vec3(nd.xy * (0.25 + 0.75 * near) * (1.0 - 0.45 * wk[3]), 0.0));
        float steep = 1.0 - n.z;
        float lat = abs(vWorld.y / uSize.y - 0.5) * 2.0;
        float sline = mix(0.78, 0.46, smoothstep(0.30, 0.90, lat));
        // the ranges are not bare rock: on their gentle ground below the snow — alpine meadow
        // and scrub in patches, greener low down, browner and thinner up high
        float mtn = wk[3] + 0.45 * wk[2];
        if (mtn > 0.02) {
            float gentle = smoothstep(0.26, 0.08, steep);
            float up = smoothstep(sline - 0.45, sline - 0.04, h);
            float patches = smoothstep(0.30, 0.70, fbm(vWorld, 1.0 / 12.0, 3) + 0.25 * cav);
            vec3 grassm = texture(uPhotos, vec3(t1, 0.0)).rgb * vec3(0.92, 0.95, 0.78);
            vec3 scrub = texture(uPhotos, vec3(t1, 2.0)).rgb * vec3(1.0, 0.92, 0.80);
            vec3 meadow = mix(grassm, scrub, 0.35 + 0.55 * up);
            col = mix(col, meadow, mtn * gentle * (1.0 - 0.75 * up) * (0.30 + 0.60 * patches));
        }
        // the marshes: a photograph of a marsh from above (pools of dark water among sedge, reeds and
        // scrub); its water glints after the light (below)
        float marsh = wk[5] * (1.0 - farm) * (1.0 - town);
        float pool = 0.0;
        float deep = 0.0;
        if (marsh > 0.02) {
            vec3 mp = aerialAt(2.0, vWorld + 57.0, wdx, wdy);
            float k = smoothstep(0.05, 0.45, marsh);
            col = mix(col, mp, k * 0.95);
            // (the water: pools in the low wet places, their shores ragged, smooth to the eye)
            float wet = fbm(vWorld, 1.0 / 2.0, 4);
            float thr = mix(0.70, 0.63, smoothstep(0.4, 0.9, marsh));
            float e = 0.012 + fwidth(wet);
            pool = smoothstep(thr - e, thr + e, wet) * smoothstep(0.3, 0.7, marsh) * (1.0 - water);
            deep = smoothstep(thr, thr + 0.05, wet);
        }
        // the basins below the sea: their floors a crust of salt where it is dry (cracked into
        // plates close up), of baked clay elsewhere
        float sunk = texture(uLight, uv).a;
        // (the salt's edge sharp and ragged, as a dried shore is; a band of wet mud round it)
        float sv = sunk + 0.22 * (fbm(vWorld, 1.0 / 1.5, 3) - 0.5);
        float se = 0.012 + fwidth(sv);
        float salt = smoothstep(0.74 - se, 0.74 + se, sv) * step(0.5, sunk);
        float mud = smoothstep(0.64, 0.73, sv) * (1.0 - salt) * step(0.45, sunk);
        // (a basin's long gentle slopes are neither rock walls nor gullies)
        float basin = smoothstep(0.0, 0.1, sunk);
        if (mud > 0.01) {
            col = mix(col, col * vec3(0.58, 0.55, 0.50), mud * 0.7 * smoothstep(0.15, 0.5, wk[4] + wk[0]));
        }
        if (salt > 0.01) {
            vec2 cell = vWorld * 6.0;
            vec2 ci = floor(cell);
            float d1 = 9.0;
            float d2 = 9.0;
            for (int j = -1; j <= 1; j++) {
                for (int i = -1; i <= 1; i++) {
                    vec2 g = ci + vec2(float(i), float(j));
                    vec2 gw = vec2(mod(g.x, uSize.x * 6.0), g.y);
                    float dd = length(cell - g - vec2(hash(gw), hash(gw + 7.1)));
                    if (dd < d1) {
                        d2 = d1;
                        d1 = dd;
                    } else if (dd < d2) {
                        d2 = dd;
                    }
                }
            }
            float crack = (1.0 - smoothstep(0.02, 0.07, d2 - d1)) * smoothstep(0.06, 0.02, pxr);
            float dry = clamp(wk[4] * 1.5 + 0.35, 0.0, 1.0);
            vec3 crust = mix(vec3(0.62, 0.55, 0.44), vec3(0.80, 0.78, 0.72), dry) * (0.88 + 0.16 * vnoise(vWorld * 0.9, uSize.x * 0.9) + 0.08 * (grain - 0.5));
            crust = mix(crust, crust * 0.62, crack);
            // (salt only where it is dry; elsewhere the floor a damp, greener lowland)
            float arid = smoothstep(0.15, 0.5, wk[4]);
            col = mix(col, crust, salt * arid);
            col = mix(col, col * vec3(0.86, 1.0, 0.80), salt * (1.0 - arid) * 0.5);
        }
        // (the era's ground: the wastes burnt to ash and rust in patches; old paintings warmer)
        float gl = dot(col, vec3(0.3, 0.59, 0.11));
        if (uEra > 0.5 && uEra < 1.5) {
            float burnt = smoothstep(0.35, 0.65, fbm(vWorld, 1.0 / 16.0, 3));
            vec3 ash = vec3(gl * 1.05, gl * 0.95, gl * 0.82);
            col = mix(ash, vec3(gl * 1.25, gl * 0.85, gl * 0.55), burnt * 0.6);
        } else if (uEra > 1.5) {
            col = mix(col, vec3(gl * 1.12, gl * 1.02, gl * 0.78), 0.28);
        }
        // (the plains' low swells are in the normals themselves: `ground::layers`)
        // snow on the high peaks (lower towards the poles; it holds on the gentler slopes and the
        // crests, not in the gullies), rock on the steep slopes — in layers along the height, bent
        // as the rock was folded, lighter on the crests, darker in the gullies
        // (on the steep walls the rock laid from the side, not stretched down them from above)
        vec3 gn = abs(normalize(cross(dFdx(pos), dFdy(pos))));
        vec3 bw = pow(gn, vec3(4.0));
        bw /= bw.x + bw.y + bw.z + 1e-5;
        vec3 rock = mix(texture(uPhotos, vec3(t1, 3.0)).rgb, texture(uPhotos, vec3(t2, 3.0)).rgb, 0.4) * bw.z
                  + texture(uPhotos, vec3(vec2(vWorld.y, pos.z) / 16.0, 3.0)).rgb * bw.x
                  + texture(uPhotos, vec3(vec2(vWorld.x, pos.z) / 16.0, 3.0)).rgb * bw.y;
        rock *= vec3(1.42, 1.36, 1.26);
        float layer = sin(h * 70.0 + 7.0 * fbm(vWorld, 1.0 / 20.0, 2));
        rock *= (0.93 + 0.09 * layer) * (1.0 + 0.12 * cav);
        col = mix(col, rock, smoothstep(0.12, 0.34, steep + 0.08 * max(cav, 0.0)) * 0.85 * (1.0 - basin));
        // the shore: a beach of sand where the land comes low to the water (wet and darker at the
        // waterline, broken by the grass behind it), pale rock where it falls steeply into it
        {
            float coast = 1.0 - smoothstep(0.004, 0.018 + 0.010 * fbm(vWorld, 1.0 / 8.0, 2), hs);
            float marshy = wk[5] + wk[7];
            float beach = coast * smoothstep(0.30, 0.0, steep) * (1.0 - 0.8 * marshy);
            vec3 sand = mix(texture(uPhotos, vec3(t1, 4.0)).rgb, texture(uPhotos, vec3(t2, 4.0)).rgb, 0.4) * vec3(1.05, 1.0, 0.88);
            sand = mix(sand, sand * vec3(0.62, 0.60, 0.58), smoothstep(0.004, 0.0, hs));
            col = mix(col, sand, beach * 0.9);
            float cliff = coast * smoothstep(0.18, 0.40, steep);
            col = mix(col, rock * vec3(1.12, 1.10, 1.04), cliff * 0.85);
        }
        // (its edge sharp and broken — by the rock showing through, by the wind's drifts — the
        // snow's own photograph in it)
        float drift = (fbm(vWorld, 1.0 / 6.0, 3) - 0.5) * mix(1.0, 0.45, coarse);
        float snow = smoothstep(sline, sline + 0.045, h + 0.10 * drift + 0.03 * grain + 0.16 * cav) * smoothstep(0.62, 0.82, n.z + 0.10 * drift) * smoothstep(-0.20, 0.15, cav);
        vec3 snowp = mix(texture(uPhotos, vec3(t1, 7.0)).rgb, texture(uPhotos, vec3(t2, 7.0)).rgb, 0.4);
        // (32nd session: the peaks' snow lay as one flat white sheet from afar; now its hollows are cooler and shaded, its
        // crests bright, wind-combed in drifts — the same sculpting the winter plain has)
        vec2 wq = vec2(dot(vWorld, vec2(0.94, 0.34)), dot(vWorld, vec2(-0.34, 0.94)));
        float sd = fbm(vWorld, 1.0 / 7.0, 3);
        float comb = fbm(vec2(wq.x * 0.35, wq.y * 1.9), 1.0 / 5.0, 2);
        float crest = smoothstep(0.38, 0.62, 0.55 * sd + 0.45 * comb + 0.22 * (grain - 0.5) + 0.30 * cav * (1.0 - coarse));
        col = mix(col, snowp * vec3(0.98, 0.99, 1.0) * mix(vec3(0.74, 0.80, 0.90), vec3(1.0), crest), snow);
        float snowcov = 0.0;
        // the time of year: winter's snow lies where it is cold enough (towards the poles, up
        // high; less on steep slopes, broken at its edge), autumn yellows the grass and browns
        // the fields, spring's mud darkens the low wet ground
        if (uSeason.x + uSeason.y + uSeason.z > 0.01) {
            float coldness = latc_of(vWorld) + 0.55 * h;
            float lies = smoothstep(0.16, 0.36, coldness + 0.14 * (fbm(vWorld, 1.0 / 16.0, 3) - 0.5));
            float winter = uSeason.x * lies * smoothstep(0.30, 0.75, n.z) * (1.0 - water);
            // (32nd session: the winter plain was one blown-out white — no drifts, no shade, the country's colour
            // gone. Now the snow is a little darker and bluer in its hollows and bright on its crests, wind-combed
            // in long drifts; the ground (a forest, a field's furrows) shows faintly through)
            vec3 snowc = mix(vec3(0.70, 0.76, 0.86), vec3(0.93, 0.945, 0.97), crest) * (0.94 + 0.10 * grain);
            snowcov = winter;
            col = mix(col, snowc, winter * 0.90);
            float grass = clamp(wk[0] + 0.8 * wk[2] + 0.6 * wk[6], 0.0, 1.0);
            float lumy = dot(col, vec3(0.3, 0.59, 0.11));
            col = mix(col, vec3(lumy * 1.30, lumy * 1.05, lumy * 0.55), uSeason.y * (0.55 * grass + 0.45 * farm) * (1.0 - winter));
            float low = smoothstep(0.35, 0.05, h);
            col = mix(col, col * vec3(0.66, 0.56, 0.44), uSeason.z * (0.5 * grass + 0.6 * farm + 0.4 * wk[5]) * low);
        }
        // the forests: where the canopy lies (the light layer's third channel: `ground::layers`), a
        // photograph of a forest from above — broad-leaved where it is warm, coniferous where it is
        // cold, in stands between; the time of year and the era on it
        float ck = 0.0;
        if ((uDbg & 16) == 0 && textureLod(uLight, uv, 2.5).b > 0.003) {
            float craw = textureLod(uLight, uv, 0.0).b;
            if (craw > 0.004) {
                float latc = latc_of(vWorld);
                float firs = clamp((wk[6] + wk[7]) * 2.0 + (latc - 0.45) * 1.6, 0.0, 1.0);
                if (latc < 0.28 && wk[6] + wk[7] < 0.05) firs = 0.0;
                float stand = smoothstep(0.40, 0.60, vnoise(vWorld * 0.125, uSize.x * 0.125) + (firs - 0.5) * 1.3);
                vec3 fc = mix(aerialAt(0.0, vWorld, wdx, wdy), aerialAt(1.0, vWorld + 31.0, wdx, wdy), stand);
                float lum = dot(fc, vec3(0.3, 0.59, 0.11));
                // (the edge of the forest goes round the photograph's own crowns: the dark ones stand
                // out of it, the light gaps fall back — sharp, as a forest's edge is)
                float thin = uEra > 0.5 && uEra < 1.5 ? 0.22 : 0.0;
                float v = craw + 0.30 * (vnoise(vWorld * 2.25, uSize.x * 2.25) - 0.5) + 2.2 * (0.21 - lum);
                float e = 0.03 + fwidth(v);
                ck = smoothstep(0.49 + thin - e, 0.49 + thin + e, v);
                // (close up the photograph's own detail at a finer scale over it: crisp leaves)
                float close = smoothstep(0.10, 0.03, pxr);
                if (close > 0.01) {
                    vec2 q = vWorld * 3.0 + vec2(0.37, 0.71);
                    vec3 d = mix(textureGrad(uAerial, vec3(q / AERIAL, 0.0), wdx * 3.0 / AERIAL, wdy * 3.0 / AERIAL).rgb, textureGrad(uAerial, vec3(q / AERIAL, 1.0), wdx * 3.0 / AERIAL, wdy * 3.0 / AERIAL).rgb, stand);
                    float ld = dot(d, vec3(0.3, 0.59, 0.11));
                    fc *= mix(1.0, clamp(ld / mix(0.235, 0.18, stand), 0.75, 1.35), 0.30 * close);
                    lum = dot(fc, vec3(0.3, 0.59, 0.11));
                }
                float lies = smoothstep(0.16, 0.36, latc + 0.55 * h);
                if (uSeason.y > 0.01) fc = mix(fc, vec3(lum * 1.55, lum * 1.05, lum * 0.42), uSeason.y * (1.0 - stand) * 0.75);
                if (uSeason.x > 0.01) {
                    vec3 bare = mix(vec3(lum * 1.05, lum * 0.95, lum * 0.85), vec3(0.78, 0.80, 0.84), 0.45 * lies);
                    fc = mix(fc, bare, uSeason.x * lies * (1.0 - stand) * 0.85);
                    fc = mix(fc, vec3(0.80, 0.82, 0.86), uSeason.x * lies * stand * 0.25);
                }
                if (uEra > 0.5 && uEra < 1.5) {
                    float dead = smoothstep(0.45, 0.60, vnoise(vWorld * 0.5, uSize.x * 0.5));
                    fc = mix(fc, vec3(lum * 1.05, lum, lum * 0.85), 0.45 + 0.4 * dead);
                } else if (uEra > 1.5) {
                    fc = mix(fc, fc * vec3(1.12, 1.04, 0.82), 0.5);
                }
                col = mix(col, fc * FOREST_GAIN, ck);
                // (30th session: a forest has an edge — a dark rim where the canopy meets the open ground, so that it reads as
                // a shape from afar, not as a stain)
                col *= 1.0 - 0.20 * (4.0 * ck * (1.0 - ck)) * smoothstep(0.05, 0.9, uK);
            }
        }
        // the sun, and the sky's light from above (a little less in the folds; the crests catch
        // more light, the gullies less)
        // (the sun warm, where no range shades it; the sky cool, less of it deep in the valleys:
        // the shaded side of a range blue-grey, not black)
        vec4 lt = texture(uLight, uv);
        // (a range's shadow takes most of the sun, not all: the map must still read in it)
        float diff = max(dot(n, SUN), 0.0) * mix(0.30, 1.0, lt.r);
        vec3 lightf = (vec3(1.06, 1.0, 0.90) * (0.82 * diff) + vec3(0.86, 0.91, 1.0) * (0.46 * (0.55 + 0.45 * n.z) * mix(1.0, lt.g, 0.70))) * clamp(1.0 + 0.30 * cav * (1.0 - basin), 0.70, 1.25);
        col *= lightf;
        // the rivers: water in the land — deep in the middle, shallow and silty at the edges, the current in
        // long streaks along the stream, sandbars in the wide reaches, ice along the shore in the cold, the
        // sky and the sun in it (33rd session)
        if (water > 0.001) {
            float lodw = smoothstep(0.35, 0.06, pxr);
            vec3 wn = vec3(0.0, 0.0, 1.0);
            if (lodw > 0.01) {
                vec2 g = fbmd(vWorld * 2.0 + vec2(uTime * 0.25, uTime * 0.1), 0.5, 2).yz;
                wn = normalize(vec3(-g * 0.9 * lodw * (1.0 - 0.65 * great), 1.0));
            }
            // across the river: 0 in the middle, 1 at the edge; the way along it from the step to the line
            // (its sign fixed, so that the streaks do not break on the middle line)
            float across = clamp(dc / max(vis, 1e-3), 0.0, 1.0);
            float deep = 1.0 - across * across;
            vec2 sd = (ln.rg * 255.0 - 128.0) / 127.0 * 6.0;
            vec2 cd = sd / max(length(sd), 1e-3);
            vec2 ad = vec2(-cd.y, cd.x);
            if (ad.x + 0.37 * ad.y < 0.0) ad = -ad;
            vec2 cr = vec2(-ad.y, ad.x);
            float lodS = smoothstep(0.55, 0.10, pxr);
            float cw = max(rhw, 0.5);
            float streak = vnoise(vec2(dot(vWorld, ad) * 0.55, dot(vWorld, cr) * 2.6 / cw), 1e5) * 0.6
                         + vnoise(vec2(dot(vWorld, ad) * 1.4 + 5.0, dot(vWorld, cr) * 6.5 / cw), 1e5) * 0.4;
            vec3 wc = mix(vec3(0.17, 0.40, 0.50), vec3(0.11, 0.30, 0.43), smoothstep(0.15, 0.9, rhw));
            // (the shallows at the edges: lighter and greener, silt in them)
            float edge = 1.0 - smoothstep(0.0, 0.50, deep);
            wc = mix(wc, vec3(0.26, 0.50, 0.54), edge * 0.50 * great);
            wc = mix(wc, vec3(0.40, 0.47, 0.37), edge * edge * 0.30 * great);
            wc *= 1.0 + (streak - 0.5) * 0.36 * great * lodS;
            // sandbars: patches of pale sand in the shallows of the wide reaches
            float barn = vnoise(vWorld * 0.22, 1e5) * 0.7 + 0.3 * vnoise(vWorld * 0.55 + 11.0, 1e5);
            float bars = smoothstep(0.52, 0.78, barn + 0.55 * (across - 0.55)) * smoothstep(0.50, 0.90, across) * smoothstep(1.2, 2.0, rhw) * lodS;
            wc = mix(wc, vec3(0.58, 0.55, 0.44) * (0.94 + 0.12 * grain), bars * 0.55);
            float fres = pow(1.0 - max(dot(wn, V), 0.0), 5.0);
            wc = mix(wc, vec3(0.42, 0.54, 0.66), fres * 0.35);
            vec3 Hh = normalize(V + SUN);
            // (the sun's glints sparser and softer on the wide waters: confetti, not sparks)
            wc += vec3(1.0, 0.95, 0.85) * pow(max(dot(wn, Hh), 0.0), mix(80.0, 260.0, great)) * mix(0.45, 0.09, great) * lodw;
            wc = mix(wc, vec3(0.20, 0.30, 0.28), smoothstep(vis * 0.5, vis, dc) * 0.30 * (1.0 - great));
            // (a wet, bright rim where the water meets the bank)
            float rim = (1.0 - smoothstep(0.0, 0.10 + 1.2 * pxr, vis - dc)) * great;
            wc = mix(wc, vec3(0.62, 0.72, 0.74), rim * 0.22);
            // (ice grows from the banks in the cold: the middle stays open water)
            float riceCold = uSeason.x * smoothstep(0.16, 0.36, latc_of(vWorld) + 0.55 * h);
            float ice = riceCold * smoothstep(0.35, 0.95, across + 0.35 * (vnoise(vWorld * 1.7, 1e5) - 0.5)) * (0.4 + 0.6 * great);
            wc = mix(wc, vec3(0.78, 0.84, 0.90) * (0.92 + 0.12 * streak), ice * 0.85);
            col = mix(col, wc, water);
        }
        // the marshes' water (the photograph's dark pools): the sky in it as the view lowers, the sun's
        // glint; ice in a cold winter
        if (pool > 0.001) {
            float fres = 0.02 + 0.98 * pow(1.0 - max(V.z, 0.0), 5.0);
            // (the sky in still water even from above; at the shore shallow, the mud and the weed
            // showing through)
            vec3 pc = mix(vec3(0.08, 0.10, 0.10), vec3(0.44, 0.53, 0.62), clamp(0.30 + 0.7 * fres, 0.0, 0.8));
            pc *= mix(0.75, 1.0, lt.r);
            pc = mix(col * vec3(0.62, 0.66, 0.64), pc, 0.35 + 0.65 * deep);
            vec3 Hh = normalize(V + SUN);
            pc += vec3(1.0, 0.95, 0.85) * pow(max(Hh.z, 0.0), 120.0) * 0.5;
            float ice = uSeason.x * smoothstep(0.16, 0.36, latc_of(vWorld) + 0.55 * h);
            pc = mix(pc, vec3(0.74, 0.80, 0.86) * (0.9 + 0.1 * grain), ice * 0.9);
            col = mix(col, pc, pool);
        }
        // the ways, as the camera comes near: main roads, then the village roads; railways
        // (ballast and two rails close up); over a river a road is a bridge, with its shadow
        float rv = smoothstep(1.15, 1.9, uK);
        float dm = lineDist(ln.ba, 2.0);
        float dl = lineDist(wy.rg, 2.0);
        float dr = lineDist(wy.ba, 2.0);
        if (rv > 0.0 && min(dm, min(dl, dr)) < 1.99) {
            float hm = max(0.13, 0.75 * pxr);
            float hl = max(0.08, 0.55 * pxr);
            float vl = smoothstep(1.9, 3.0, uK);
            float am = 1.0 - smoothstep(hm - 0.5 * pxr, hm + 0.5 * pxr, dm);
            float al = (1.0 - smoothstep(hl - 0.5 * pxr, hl + 0.5 * pxr, dl)) * vl;
            float verge = (1.0 - smoothstep(hm, hm + 0.08 + pxr, dm)) * (1.0 - am);
            vec3 dirt = uEra > 1.5 ? vec3(0.50, 0.39, 0.26) : uEra > 0.5 ? vec3(0.30, 0.29, 0.27) : vec3(0.46, 0.43, 0.38);
            vec3 track = uEra > 0.5 && uEra < 1.5 ? vec3(0.38, 0.33, 0.27) : vec3(0.47, 0.38, 0.27);
            float road = max(am, al) * rv;
            if (uEra > 0.5 && uEra < 1.5) road *= 0.35 + 0.65 * smoothstep(0.30, 0.45, fbm(vWorld, 0.5, 2));
            vec3 rc = mix(track, dirt, am / max(am + al, 1e-3));
            rc *= 0.85 + 0.3 * grain;
            col = mix(col, col * 0.72, verge * rv * 0.8);
            col = mix(col, rc * lightf, road);
            // (a bridge: its parapets dark, its shadow on the water)
            float br = water * rv;
            if (br > 0.01) {
                float hw2 = am > al ? hm : hl;
                float dd = am > al ? dm : dl;
                float parapet = (1.0 - smoothstep(0.02, 0.02 + pxr, abs(dd - hw2))) * max(am, al);
                col = mix(col, vec3(0.16, 0.13, 0.10), parapet * br);
                float shade = (1.0 - smoothstep(hw2, hw2 + 0.18, dd)) * (1.0 - road);
                col *= 1.0 - 0.35 * shade * br;
            }
            float hr = max(0.10, 0.5 * pxr);
            float ar = (1.0 - smoothstep(hr - 0.5 * pxr, hr + 0.5 * pxr, dr)) * rv;
            float close = smoothstep(0.06, 0.025, pxr);
            vec3 ballast = mix(vec3(0.20, 0.18, 0.17), vec3(0.46, 0.43, 0.40), close);
            col = mix(col, ballast * lightf, ar);
            float steel = (1.0 - smoothstep(0.012, 0.012 + pxr, abs(dr - 0.055))) * close * rv;
            col = mix(col, vec3(0.10, 0.10, 0.11), steel * step(dr, 0.1));
        }
        // the heights in colour (the mode «Рельеф»): the land lit as it is, coloured as an atlas
        // colours it, lines of equal height every ~250 m (thicker every 1000), the sea's level
        // round the basins below it
        if (uHypso > 0.01) {
            float lum = dot(lightf, vec3(0.33));
            vec3 hc = hypsoColour(h) * clamp(lum, 0.45, 1.25);
            // (33rd session: the rivers keep their water in the atlas colours: the great ones read as rivers on the relief)
            col = mix(col, hc, 0.62 * uHypso * (1.0 - 0.80 * water));
            float e = h / 0.045;
            float fw = max(fwidth(e), 1e-4);
            float iso = 1.0 - smoothstep(0.5, 1.5, abs(fract(e + 0.5) - 0.5) / fw);
            float big = 1.0 - smoothstep(0.8, 2.0, abs(fract(e / 4.0 + 0.5) - 0.5) * 4.0 / fw);
            col = mix(col, col * 0.55, (0.35 * iso + 0.45 * big) * uHypso * smoothstep(0.6, 0.2, fw));
            float zero = 1.0 - smoothstep(0.6, 1.8, abs(h) / max(fwidth(h), 1e-5));
            col = mix(col, vec3(0.10, 0.30, 0.55), zero * uHypso * step(0.01, sunk) * 0.85);
        }
        // the countries' colours over the land
        int id = idAt(vWorld);
        ivec2 me = who(id);
        if (id > 0 && me.x >= 0 && (uDbg & 128) == 0) {
            vec4 p0 = pal(id, 0);
            vec4 p1 = pal(id, 1);
            vec3 tint = p0.a > 0.5 ? p0.rgb : vec3(0.59, 0.57, 0.53);
            float on = p0.a > 0.5 ? 1.0 : 0.55;
            if (p1.a > 0.5 && mod(floor((vWorld.x + vWorld.y) / uStripe), 2.0) == 1.0) tint = p1.rgb;
            float lum = dot(col, vec3(0.3, 0.59, 0.11)) / 0.42;
            // each province a little lighter or darker than its neighbours, so that the land falls
            // into provinces even where no line is drawn
            tint *= 0.93 + 0.14 * hash(vec2(float(id), 17.0));
            // (30th session: under a forest the country's colour is weaker — the forest keeps its green and the country
            // its tint at the edge and in the open: it was a purple-grey mush with no forest in it)
            // (32nd session: on snow the tint is stronger — the white ground carries it less)
            // (33rd session: and the rivers keep their water under the country's colour — a great river is the map's own line)
            float tk = min(uOverlay * (1.0 + 0.38 * snowcov), 0.92) * on * (1.0 - 0.30 * ck) * (1.0 - 0.70 * water);
            col = mix(col, tint * clamp(lum, 0.80, 1.25), tk);
            // borders (far away; close up egui draws them smooth): the countries' dark, the
            // provinces' faint, the front red
            if (uBorders > 0.0) {
                float edge = 0.0;
                float prov = 0.0;
                float front = 0.0;
                for (int i = 0; i < 4; i++) {
                    vec2 d = i == 0 ? vec2(1.0, 0.0) : i == 1 ? vec2(-1.0, 0.0) : i == 2 ? vec2(0.0, 1.0) : vec2(0.0, -1.0);
                    int id2 = idAt(vWorld + d * px * 1.1);
                    if (id2 == id) continue;
                    ivec2 o = who(id2);
                    if (o.x < 0 || id2 == 0) continue;
                    prov = 1.0;
                    if (o.x != me.x) edge = 1.0;
                    if (uWar > 0.5 && me.y != 9 && o.y != 9 && o.y != me.y) front = 1.0;
                }
                // At strategic zoom this is the only province contour.  Make it a firm ink line:
                // a faint, terrain-coloured line vanishes completely in a forest or in mountain
                // shadow before the smooth close-up contour takes over.
                // (dark ink on light land, a lighter line on dark land — a black hairline vanishes in a
                // forest's or a range's shade, and a thick one cuts the eye on a plain)
                float lb = dot(col, vec3(0.3, 0.59, 0.11));
                vec3 ink = mix(col * 0.38, col * 0.55 + vec3(0.11), smoothstep(0.26, 0.10, lb));
                // (29th session, the second hand: a hierarchy — the countries' borders at full ink, the provinces' lighter,
                // so that 9000 of them do not make a net over the whole map)
                col = mix(col, ink, prov * uBorders * 0.9 * mix(0.62, 1.0, edge));
                // (and a soft shade just inside the edge, so that a province reads as a tile)
                float rim = 0.0;
                for (int i = 0; i < 4; i++) {
                    vec2 d = i == 0 ? vec2(1.0, 0.0) : i == 1 ? vec2(-1.0, 0.0) : i == 2 ? vec2(0.0, 1.0) : vec2(0.0, -1.0);
                    int id3 = idAt(vWorld + d * px * 3.4);
                    if (id3 != id && id3 != 0 && who(id3).x >= 0) rim = 1.0;
                }
                col *= 1.0 - 0.16 * rim * (1.0 - prov) * uBorders;
                edge += front * 0.0;
            }
        }
    }
    // the fog of war: a dark drifting haze over the enemy's land the staff does not see (soft at
    // its edge: the province and four points round it)
    if (uWar > 0.5 && hs >= 0.0) {
        float fogv = 0.0;
        for (int i = 0; i < 5; i++) {
            vec2 o = i == 0 ? vec2(0.0) : i == 1 ? vec2(1.4, 0.0) : i == 2 ? vec2(-1.4, 0.0) : i == 3 ? vec2(0.0, 1.4) : vec2(0.0, -1.4);
            int id2 = idAt(vWorld + o);
            float a = id2 > 0 ? pal(id2, 2).a : 0.0;
            fogv += (a > 0.3 && a < 0.7) ? 0.2 : 0.0;
        }
        if (fogv > 0.0) {
            float mist = 0.55 + 0.45 * smoothstep(0.25, 0.75, fbm(vWorld + vec2(uTime * 0.8, uTime * 0.3), 1.0 / 32.0, 4));
            float lumf = dot(col, vec3(0.3, 0.59, 0.11));
            // (30th session: a veil, not a hole — the unseen land kept its colours half, only greyed and cooled)
            vec3 hazed = mix(vec3(lumf) * 0.80, vec3(0.24, 0.27, 0.34), 0.40);
            col = mix(col, hazed, fogv * 0.58 * mist);
        }
    }
    // clouds far away, and their shadows
    if (uClouds > 0.0) {
        float t = uTime * 0.6;
        float c = smoothstep(0.52, 0.78, fbm(vWorld + vec2(t * 2.0, t * 0.6), 1.0 / 64.0, 5));
        float sh = smoothstep(0.52, 0.78, fbm(vWorld + vec2(t * 2.0 + 10.0, t * 0.6 + 8.0), 1.0 / 64.0, 5));
        col *= 1.0 - 0.25 * sh * uClouds;
        col = mix(col, vec3(0.97, 0.97, 0.98), c * 0.62 * uClouds);
    }
    // haze far up a tilted view
    float haze = smoothstep(1.15, 2.6, vDepth / uDist);
    col = mix(col, vec3(0.64, 0.72, 0.80), haze * 0.55);
    // (30th session, the grade of the whole picture: the shadows lifted and the colours a little
    // fuller, so that a forest or a range in shade is dark but not mud, and a country's tint
    // still reads on it; the sea is left as it is)
    if (hs >= 0.0) {
        float gl2 = dot(col, vec3(0.3, 0.59, 0.11));
        col = mix(vec3(gl2), col, 1.14);
        col = pow(clamp(col, 0.0, 1.0), vec3(0.84));
    }
    frag = vec4(col, 1.0);
}
