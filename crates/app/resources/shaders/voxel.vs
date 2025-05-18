#version 140

in vec3 corner;
in uint position;
in uint light;
in vec2 uv;
in vec4 color;

out vec2 v_tex_coords;
out vec4 v_color;

uniform ivec2 origin;
uniform mat4 matrix;
uniform vec3 sun_position;

vec4 toLinear(vec4 sRGB) {
  bvec3 cutoff = lessThan(sRGB.rgb, vec3(0.04045));
  vec3 higher = pow((sRGB.rgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
  vec3 lower = sRGB.rgb / vec3(12.92);

  return vec4(mix(higher, lower, cutoff), sRGB.a);
}

void main() {
  vec2 or = vec2(origin * 16);
  vec3 pos = vec3((position >> uint(12)) & uint(15), position & uint(255),
                  (position >> uint(8)) & uint(15));

  float block_light = (float(light & uint(15)) + 1.0) / 16.0;
  float sun_light = (float((light >> uint(4)) & uint(15)) + 1.0) / 16.0;

  float light_intensity =
      block_light + sun_light * max(sun_position.y * 0.96 + 0.3, 0.02);

  vec4 linear_color = toLinear(color / 255.0);

  gl_Position = matrix * vec4(vec3(or.x, 0.0, or.y) + pos + corner, 1.0);
  v_color = vec4(linear_color.rgb * light_intensity, linear_color.a);

  v_tex_coords = uv;
}
