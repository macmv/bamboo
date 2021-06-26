#version 450

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 col;
layout(location = 0) out vec2 f_uv;
layout(location = 1) out vec4 f_col;

void main() {
  f_uv = uv;
  f_col = col;
  gl_Position = vec4(pos, 0.0, 1.0);
}
