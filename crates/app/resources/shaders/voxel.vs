#version 140

in uint corner;
in uint position;
in uint light;
in vec2 uv;
in vec4 color;

out vec2 v_tex_coords;
out vec4 v_color;

uniform ivec2 origin;
uniform mat4 matrix;
uniform vec3 sun_position;

void main() {
  vec3 corn = vec3(((corner >> uint(2)) & uint(1)),
                   ((corner >> uint(1)) & uint(1)), (corner & uint(1)));
  vec2 or = vec2(origin * 16);
  vec3 pos = vec3((position >> uint(12)) & uint(15), position & uint(255),
                  (position >> uint(8)) & uint(15));

  float block_light = (float(light & uint(15)) + 1.0) / 16.0;
  float sun_light = (float((light >> uint(4)) & uint(15)) + 1.0) / 16.0;

  float light_intensity =
      block_light + sun_light * max(sun_position.y * 0.96 + 0.6, 0.02);

  gl_Position = matrix * vec4(vec3(or.x, 0.0, or.y) + pos + corn, 1.0);
  v_color = (color / 255.0) *
            vec4(light_intensity, light_intensity, light_intensity, 1.0);

  v_tex_coords = uv;
}
