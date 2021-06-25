#version 450

layout(location = 0) in vec2 uv;
layout(location = 1) in float cs;
layout(location = 2) in float ratio;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D img;

void main() {
  vec2 mapped = uv;
  float cs_x = cs * ratio;
  float cs_y = cs / ratio;
  if (uv.x < cs_x) {
    // mapped.x is within 0 - cs. We want it at 0 to 0.333
    mapped.x /= cs_x;
    mapped.x /= 3;
  } else if (uv.x > 1 - cs_x) {
    // mapped.x is within (1-cs) - 1. We want it at 0.666 to 1.
    mapped.x -= 1 - cs_x;
    mapped.x /= cs_x;
    // It is now within the range 0-1
    mapped.x /= 3;
    mapped.x += 0.666;
  } else {
    mapped.x -= cs_x;
    mapped.x /= (1 - cs_x * 2);
    // mapped.x is now within the range 0-1. We want it to be within 0.333 to 0.666
    mapped.x /= 3;
    mapped.x += 0.333;
  }
  if (uv.y < cs_y) {
    // mapped.y is within 0 - cs. We want it at 0 to 0.333
    mapped.y /= cs_y;
    mapped.y /= 3;
  } else if (uv.y > 1 - cs_y) {
    // mapped.y is within (1-cs) - 1. We want it at 0.666 to 1.
    mapped.y -= 1 - cs_y;
    mapped.y /= cs_y;
    // It is now within the range 0-1
    mapped.y /= 3;
    mapped.y += 0.666;
  } else {
    mapped.y -= cs_y;
    mapped.y /= (1 - cs_y * 2);
    // mapped.y is now within the range 0-1. We want it to be within 0.333 to 0.666
    mapped.y /= 3;
    mapped.y += 0.333;
  }
  vec4 col = texture(img, mapped);
  f_color = col;
}
