#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 f_color;

void main() {
  f_color = vec4(uv.x / 2 + 0.5, uv.y / 2 + 0.5, 1.0, 1.0);
}
