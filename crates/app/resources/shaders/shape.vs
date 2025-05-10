#version 140

in vec3 position;
in vec4 color;

out vec4 v_color;

uniform mat4 matrix;

vec4 toLinear(vec4 sRGB) {
  bvec3 cutoff = lessThan(sRGB.rgb, vec3(0.04045));
  vec3 higher = pow((sRGB.rgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
  vec3 lower = sRGB.rgb / vec3(12.92);

  return vec4(mix(higher, lower, cutoff), sRGB.a);
}

void main() {
  gl_Position = matrix * vec4(position, 1.0);
  v_color = toLinear(color / 255.0);
}
