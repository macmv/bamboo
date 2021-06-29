#version 450

layout(location = 0) in vec3 pos;
layout(location = 1) in vec2 uv;
layout(location = 0) out vec2 f_uv;

layout(push_constant) uniform PushData {
  mat4 model;
  mat4 proj;
  mat4 view;
} pc;

void main() {
  f_uv = uv;
  gl_Position = pc.proj * pc.view * pc.model * vec4(pos, 1.0);
}
