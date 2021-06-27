#version 450

layout(location = 0) in vec2 pos;
layout(location = 0) out vec2 uv;
layout(location = 1) out vec4 col;

layout(push_constant) uniform PushData {
  // The offset onscreen
  vec2 offset;
  // The offset within the texture
  vec2 uv_offset;
  // The size onscreen and on texture
  vec2 size;
  // The color to render with
  vec4 col;
} pc;

void main() {
  col = pc.col;
  uv = pos * pc.size + pc.uv_offset;
  gl_Position = vec4(pos * pc.size + pc.offset, 0.0, 1.0);
}
