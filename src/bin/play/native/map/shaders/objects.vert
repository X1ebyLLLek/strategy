#version 330 core
layout(location = 0) in vec3 aPos;
layout(location = 1) in vec3 aNrm;
layout(location = 2) in vec2 aPart;
layout(location = 3) in vec4 aInst;
uniform mat4 uMvp;
uniform sampler2D uHeight;
uniform vec2 uSize;
uniform float uRelief;
uniform float uGrow;
uniform float uKind;
uniform float uEra;
uniform float uTime;
uniform vec3 uUp;
uniform int uMode;
out vec3 vN;
out vec3 vLocal;
out vec2 vCard;
out vec3 vAt;
out float vPart;
out float vAo;
out float vSeed;
out float vDepth;
out float vDead;
out vec3 vMN;
out float vTop;
out float vRuin;
out vec2 vRot;
const vec3 SUN = normalize(vec3(-0.45, -0.55, 0.70));
// a tree's picture in the atlas: across (model units either side of the trunk), and up the
// screen from below the foot to above the top; the camera's tilt it is drawn at (radians from
// straight down: as the camera looks where the trees are drawn)
const float CARD_X = 0.62;
const float CARD_LOW = -0.32;
const float CARD_HIGH = 1.88;
const float PIC_PITCH = 0.72;
// how tall each kind of tree stands and how wide its crown (for its shadow)
const vec2 SHAPE[5] = vec2[5](vec2(1.62, 0.40), vec2(1.40, 0.50), vec2(1.44, 0.34), vec2(1.46, 0.24), vec2(0.55, 0.32));
// and each kind of building (cottage, town house, barn, hipped house, church, flats, factory,
// windmill), and the height of its walls (model units)
const vec2 BSHAPE[8] = vec2[8](vec2(0.40, 0.24), vec2(0.60, 0.25), vec2(0.40, 0.30), vec2(0.55, 0.30), vec2(0.95, 0.24), vec2(0.70, 0.36), vec2(0.50, 0.46), vec2(0.80, 0.22));
const float TOP[8] = float[8](0.19, 0.34, 0.15, 0.30, 0.32, 0.64, 0.30, 0.70);
float ground(vec2 w) {
    return textureLod(uHeight, w / uSize, 0.0).r * uRelief;
}
void main() {
    bool tree = uKind < 4.5;
    float w = aInst.w;
    // (a tree turns to one of the four sides its picture was drawn from; a building any way)
    float side = floor(fract(w * 7.77) * 4.0);
    float a = tree ? side * 1.5707963 + 0.35 : w * 6.2831853;
    mat2 R = mat2(cos(a), sin(a), -sin(a), cos(a));
    vRot = vec2(cos(a), sin(a));
    float stretch = tree ? 0.84 + 0.36 * fract(w * 5.31) : 1.0;
    if (uMode == 1) stretch = 1.0;
    // (a building's own shades from where it stands: its turn is its street's)
    float bseed = fract(sin(dot(aInst.xy, vec2(12.9898, 78.233))) * 43758.5453);
    vSeed = tree ? fract(w * 31.7) : bseed;
    int bk = int(uKind - 10.0 + 0.5);
    // (every other town house three storeys)
    float zs = !tree && bk == 1 && fract(bseed * 3.1) > 0.5 ? 1.32 : 1.0;
    vTop = tree ? 0.0 : TOP[clamp(bk, 0, 7)] * zs;
    vMN = aNrm;
    // (the wastes: a third of the houses roofless, their walls broken)
    vRuin = !tree && bk < 6 && uEra > 0.5 && uEra < 1.5 && fract(bseed * 7.3) < 0.32 ? 1.0 : 0.0;
    vPart = aPart.x;
    vAo = aPart.y;
    vLocal = vec3(aPos.xy, aPos.z * zs);
    vCard = vec2(-1.0);
    vN = vec3(R * aNrm.xy, aNrm.z);
    vAt = vec3(aInst.xy / uSize, max(textureLod(uHeight, aInst.xy / uSize, 0.0).r, 0.0));
    if (uMode == 1) {
        // a picture for the atlas: the model turned to its side, seen as the camera sees the
        // land where trees are drawn, filling the picture's frame
        vec3 p = aPos;
        p.xy = R * p.xy;
        vec3 up = vec3(0.0, -cos(PIC_PITCH), sin(PIC_PITCH));
        vec3 fwd = vec3(0.0, -sin(PIC_PITCH), -cos(PIC_PITCH));
        float yy = (dot(p, up) - CARD_LOW) / (CARD_HIGH - CARD_LOW) * 2.0 - 1.0;
        gl_Position = vec4(p.x / CARD_X, yy, dot(p, fwd) / 3.0, 1.0);
        vDepth = 0.0;
        return;
    }
    float size = aInst.z * uGrow;
    vec3 at;
    if (uMode == 3) {
        // a shadow: an oval on the land, away from the sun, as long as the thing is tall
        vec2 dir = -normalize(SUN.xy);
        vec2 sh = tree ? SHAPE[int(uKind + 0.5)] : BSHAPE[clamp(bk, 0, 7)] * vec2(zs, 1.0);
        float hgt = sh.x * stretch * size;
        float half_len = 0.42 * hgt + sh.y * size;
        vec2 mid = aInst.xy + dir * (0.48 * hgt);
        vec2 q = mid + dir * aPos.y * 2.0 * half_len + vec2(-dir.y, dir.x) * aPos.x * 2.0 * sh.y * size * 1.1;
        at = vec3(q, ground(q) + 0.03);
        vCard = aPos.xy * 2.0;
        gl_Position = uMvp * vec4(at, 1.0);
        vDepth = gl_Position.w;
        return;
    }
    vec3 p = aPos * size;
    p.z *= stretch * zs;
    // (a windmill's sails turn about the hub)
    if (bk == 7 && aPart.x > 1.5) {
        vec3 hub = vec3(0.0, -0.2, 0.66);
        vec3 q = aPos - hub;
        float b = uTime * 0.9 + w * 6.2831853;
        q.xz = mat2(cos(b), sin(b), -sin(b), cos(b)) * q.xz;
        p = (hub + q) * size;
    }
    // (the wastes: a third of the trees dead — their crowns gone, the trunks left)
    bool dead = tree && uEra > 0.5 && uEra < 1.5 && fract(w * 13.7) < 0.35;
    vDead = dead ? 1.0 : 0.0;
    if (dead && aPart.x > 0.5 && aPart.x < 2.5) p = vec3(0.0, 0.0, 0.3 * aInst.z);
    // (the wind: the crowns sway a little, each in its own time)
    if (tree) p.xy += vec2(0.018, 0.012) * sin(uTime * 1.6 + w * 40.0) * max(p.z - 0.25 * size, 0.0);
    p.xy = R * p.xy;
    if (uMode == 2) {
        // a card: across the screen and up it, framing the tree as its picture does
        float hi = mix(CARD_LOW, CARD_HIGH, aPos.z);
        p = vec3(aPos.x * 2.0 * CARD_X * size, 0.0, 0.0) + uUp * hi * size * stretch;
        p.xy += vec2(0.018, 0.012) * sin(uTime * 1.6 + w * 40.0) * max(hi - 0.25, 0.0) * size;
        vCard = vec2((side + aPos.x + 0.5) / 4.0, (uKind + aPos.z) / 5.0);
    }
    at = vec3(aInst.xy + p.xy, ground(aInst.xy) + p.z - 0.04);
    gl_Position = uMvp * vec4(at, 1.0);
    vDepth = gl_Position.w;
}
