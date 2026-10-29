#version 330 core
layout(location = 0) in vec3 aPos;
layout(location = 1) in vec3 aNrm;
layout(location = 2) in vec2 aPart;
layout(location = 3) in vec4 aInst;
layout(location = 4) in vec4 aCol;
uniform mat4 uMvp;
uniform sampler2D uHeight;
uniform vec2 uSize;
uniform float uRelief;
uniform float uTime;
uniform int uMode;
uniform vec2 uShadow[9];
out vec3 vN;
out vec3 vCol;
out float vPaint;
out float vAo;
out vec2 vPlate;
out vec2 vAt;
out float vDepth;
out vec3 vOwn;
const vec3 SUN = normalize(vec3(-0.45, -0.55, 0.70));
float ground(vec2 w) {
    return textureLod(uHeight, w / uSize, 0.0).r * uRelief;
}
// a point turned about the axis across the body (y) at height `pivot`, by `a`
vec3 swing(vec3 p, float pivot, float a) {
    vec2 q = vec2(p.x, p.z - pivot);
    q = mat2(cos(a), -sin(a), sin(a), cos(a)) * q;
    return vec3(q.x, p.y, q.y + pivot);
}
void main() {
    float size = aInst.z;
    float ang = aInst.w;
    int model = int(aCol.w / 4.0);
    float doing = aCol.w - float(model) * 4.0;
    mat2 R = mat2(cos(ang), sin(ang), -sin(ang), cos(ang));
    vCol = aCol.rgb;
    vAt = aInst.xy / uSize;
    if (uMode == 3) {
        // a shadow: an oval on the land, away from the sun, as long as the figure is tall
        vec2 dir = -normalize(SUN.xy);
        vec2 sh = uShadow[clamp(model, 0, 8)];
        float hgt = sh.x * size;
        float half_len = 0.42 * hgt + sh.y * size * 0.6;
        vec2 mid = aInst.xy + dir * (0.40 * hgt);
        vec2 q = mid + dir * aPos.y * 2.0 * half_len + vec2(-dir.y, dir.x) * aPos.x * 2.0 * sh.y * size;
        vPlate = aPos.xy * 2.0;
        gl_Position = uMvp * vec4(q, ground(q) + 0.02, 1.0);
        vDepth = gl_Position.w;
        return;
    }
    if (uMode == 4) {
        // the ring: a circle on the land round the feet, as wide as the figure's breadth asks
        vec2 sh = uShadow[clamp(model, 0, 8)];
        float rad = size * (0.20 + 0.60 * sh.y);
        vec2 q = aInst.xy + aPos.xy * 2.0 * rad;
        vPlate = aPos.xy * 2.0;
        gl_Position = uMvp * vec4(q, ground(q) + 0.02, 1.0);
        vDepth = gl_Position.w;
        return;
    }
    vec3 p = aPos;
    vec3 n = aNrm;
    // the paint: one of the paints (below 1000, by tens the limb that moves), or a colour of the
    // model's own (1000 + r*4096 + g*64 + b, six bits each; by 262144 the limb)
    float limb;
    vOwn = vec3(0.0);
    if (aPart.x > 999.5) {
        float v = aPart.x - 1000.0;
        limb = floor(v / 262144.0 + 1e-4);
        float c = v - limb * 262144.0;
        float r = floor(c / 4096.0 + 1e-4);
        float g = floor((c - r * 4096.0) / 64.0 + 1e-4);
        float b = c - r * 4096.0 - g * 64.0;
        vOwn = vec3(r, g, b) / 63.0;
        vPaint = 10.0;
    } else {
        limb = floor(aPart.x / 10.0 + 0.001);
        vPaint = aPart.x - limb * 10.0;
    }
    vAo = aPart.y;
    // marching: the legs step (a man's about the hip, a horse's about the shoulder and the
    // haunch), the body bobs; fighting: a man kneels a little to his aim
    float phase = uTime * 6.0 + fract(sin(dot(aInst.xy, vec2(12.9898, 78.233))) * 43758.5453) * 6.283;
    if (doing > 0.5 && doing < 1.5) {
        float s = sin(phase);
        if (limb > 0.5 && limb < 1.5) { p = swing(p, 0.47, 0.42 * s); n = swing(n, 0.0, 0.42 * s); }
        if (limb > 1.5 && limb < 2.5) { p = swing(p, 0.47, -0.42 * s); n = swing(n, 0.0, -0.42 * s); }
        if (limb > 2.5 && limb < 3.5) { p = swing(p, 0.55, 0.35 * s); n = swing(n, 0.0, 0.35 * s); }
        if (limb > 3.5 && limb < 4.5) { p = swing(p, 0.55, -0.35 * s); n = swing(n, 0.0, -0.35 * s); }
        p.z += 0.012 * abs(cos(phase));
    }
    p.xy = R * p.xy;
    n.xy = R * n.xy;
    vN = n;
    vec3 at = vec3(aInst.xy + p.xy * size, ground(aInst.xy) + p.z * size);
    gl_Position = uMvp * vec4(at, 1.0);
    if (uMode == 5) {
        // the outline's shell: out along the normal by a screen point or so (the world's size of a point grows with the distance)
        // (33rd session: along the face's normal the shell tore at the hard edges — the outline ran in dashes; now out from the body's
        // middle by position, so that corners shared by faces move together)
        vec3 rel = vec3(p.xy, p.z - 0.5);
        at += normalize(rel + vec3(1e-4)) * gl_Position.w * 0.0012;
        gl_Position = uMvp * vec4(at, 1.0);
    }
    vDepth = gl_Position.w;
}
