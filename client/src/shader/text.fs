#version 450

layout(location = 0) in vec2 uv;
layout(location = 1) in vec4 col;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D tex;

void main() {
  // f_color = col * texture(tex, uv)[0];
  f_color = col;
}
