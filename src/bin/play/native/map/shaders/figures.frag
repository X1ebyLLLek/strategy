#version 330 core
in vec3 vN;
in vec3 vCol;
in float vPaint;
in float vAo;
in vec2 vPlate;
in vec2 vAt;
in float vDepth;
in vec3 vOwn;
uniform sampler2D uLight;
uniform float uDist;
uniform int uMode;
uniform vec3 uToward;
out vec4 frag;
const vec3 SUN = normalize(vec3(-0.45, -0.55, 0.70));
void main() {
    float haze = smoothstep(1.15, 2.6, vDepth / uDist);
    float vSun = mix(0.52, 1.0, clamp(textureLod(uLight, vAt, 0.0).r * 1.15, 0.0, 1.0));
    if (uMode == 4) {
        float r = length(vPlate);
        if (r > 1.0) discard;
        float ring = smoothstep(0.70, 0.82, r) * (1.0 - smoothstep(0.92, 1.0, r));
        float fill = 0.18 * (1.0 - smoothstep(0.66, 0.88, r));
        vec3 c = vCol;
        float l = dot(c, vec3(0.3, 0.59, 0.11));
        c = clamp(mix(vec3(l), c, 1.7) * 1.45 + 0.06, 0.0, 1.0);
        frag = vec4(c, (ring * 0.9 + fill) * (1.0 - haze));
        return;
    }
    if (uMode == 3) {
        float r = length(vPlate);
        float a = (1.0 - smoothstep(0.30, 1.0, r)) * 0.42 * (0.3 + 0.7 * vSun) * (1.0 - haze);
        frag = vec4(0.0, 0.0, 0.0, a);
        return;
    }
    // the paints: the uniform (the country's), skin, leather, steel, wood, painted metal, white,
    // brass, a bay horse, rubber
    int k = int(vPaint + 0.5);
    vec3 base = k == 0 ? mix(vCol, vec3(dot(vCol, vec3(0.3, 0.59, 0.11))), 0.15) * 1.18 + 0.03
              : k == 1 ? vec3(0.78, 0.58, 0.45)
              : k == 2 ? vec3(0.19, 0.15, 0.12)
              : k == 3 ? vec3(0.19, 0.20, 0.21)
              : k == 4 ? vec3(0.40, 0.25, 0.13)
              : k == 5 ? vCol * vec3(0.95, 0.97, 0.95)
              : k == 6 ? vec3(0.86, 0.84, 0.78)
              : k == 7 ? vec3(0.72, 0.54, 0.24)
              : k == 8 ? vec3(0.34, 0.20, 0.11)
              : k == 10 ? clamp(mix(vOwn * 1.35 + 0.05, vCol * 1.15, 0.22), 0.0, 1.0)
              : vec3(0.07, 0.07, 0.07);
    vec3 n = normalize(vN);
    float diff = max(dot(n, SUN), 0.0);
    vec3 sunc = vec3(1.06, 1.0, 0.88);
    vec3 skyc = vec3(0.86, 0.91, 1.0);
    // (a little light round the edges, as a painted miniature catches it, to stand out of the land)
    float rim = pow(1.0 - abs(dot(n, -uToward)), 2.5) * 0.30;
    vec3 col = base * (sunc * 0.95 * diff * vSun + skyc * 0.62 * (0.62 + 0.38 * n.z) * vAo + vec3(0.05, 0.045, 0.035) * (1.0 - n.z)) + vec3(rim) * (0.4 + 0.6 * vSun);
    // (metal and leather shine where the sun glints)
    if (k == 3 || k == 5 || k == 7 || k == 2) {
        vec3 h = normalize(SUN - uToward);
        col += sunc * pow(max(dot(n, h), 0.0), 24.0) * (k == 7 ? 0.45 : 0.22) * vSun;
    }
    // (30th session: the figures read as black cut-outs on the land — the shadows lifted a little)
    col = pow(clamp(col, 0.0, 1.0), vec3(0.82));
    col = mix(col, vec3(0.64, 0.72, 0.80), haze * 0.55);
    frag = vec4(col, 1.0);
}
