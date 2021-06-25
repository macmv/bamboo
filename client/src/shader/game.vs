#version 450

layout(location = 0) in vec2 pos;
layout(location = 0) out vec2 uv;

layout(push_constant) uniform PushData {
  vec2 offset;
} pc;

void main() {
  uv = pos;
  gl_Position = vec4(pos + pc.offset, 0.0, 1.0);
}
