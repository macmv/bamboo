#version 450

layout(location = 0) in vec2 pos;
layout(location = 0) out vec2 uv;
layout(location = 1) out float corner_size;
layout(location = 2) out float ratio;

layout(push_constant) uniform PushData {
  vec2 pos;
  vec2 size;
  float corner_size;
} pc;

void main() {
  uv = (pos + 1) / 2;
  corner_size = pc.corner_size;
  ratio = pc.size.y / pc.size.x;
  gl_Position = vec4(pos * pc.size + pc.pos, 0.0, 1.0);
}
