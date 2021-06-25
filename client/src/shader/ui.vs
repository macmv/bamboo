#version 450

layout(location = 0) in vec2 pos;
layout(location = 0) out vec2 uv;
layout(location = 1) out vec2 corner_size;

layout(push_constant) uniform PushData {
  vec2 pos;
  vec2 size;
  float corner_size;
} pc;

void main() {
  uv = (pos + 1) / 2;
  // pc.corner_size is in absolute coordinates, and we want it to be within uv coordinates.
  corner_size = vec2(pc.corner_size / pc.size.x, pc.corner_size / pc.size.y);
  gl_Position = vec4(pos * pc.size + pc.pos, 0.0, 1.0);
}
