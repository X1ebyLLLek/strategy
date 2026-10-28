#version 330 core
layout(location = 0) in vec2 aPos;
uniform mat4 uMvp;
uniform sampler2D uHeight;
uniform vec2 uSize;
uniform float uRelief;
out vec2 vWorld;
out float vDepth;
void main() {
    float h = textureLod(uHeight, aPos / uSize, 0.0).r;
    if (aPos.y < 0.0 || aPos.y > uSize.y) h = 0.0;
    vWorld = aPos;
    gl_Position = uMvp * vec4(aPos, h * uRelief, 1.0);
    vDepth = gl_Position.w;
}
