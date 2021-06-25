#version 450

layout(location = 0) in vec2 uv;
layout(location = 1) in vec2 cs;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D img;

float map_9patch(float v, float cs) {
  float mapped = v;
  if (v < cs) {
    // mapped.x is within 0 - cs. We want it at 0 to 1
    mapped /= cs;
  } else if (v > 1 - cs) {
    // mapped.x is within (1-cs) - 1. We want it at 2 to 3.
    mapped -= 1 - cs;
    mapped /= cs;
    mapped += 2;
  } else {
    mapped -= cs;
    mapped /= (1 - cs * 2);
    // mapped.x is now within the range 0-1. We want it to be within 1 to 2
    mapped += 1;
  }
  mapped /= 3;
  return mapped;
}

void main() {
  vec2 mapped = vec2(map_9patch(uv.x, cs.x), map_9patch(uv.y, cs.y));

  vec4 col = texture(img, mapped);
  f_color = col;
}
