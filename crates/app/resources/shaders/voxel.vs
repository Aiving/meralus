#version 140

in uint corner;
in uint position;
in vec2 uv;
in vec4 color;

out vec2 v_tex_coords;
out vec4 v_color;

uniform ivec2 origin;
uniform mat4 matrix;

void main() {
  vec3 corn = vec3(((corner >> uint(2)) & uint(1)), ((corner >> uint(1)) & uint(1)), (corner & uint(1)));
  vec2 or = vec2(origin * 16);
  vec3 pos = vec3((position >> uint(12)) & uint(15), position & uint(255), (position >> uint(8)) & uint(15));

  gl_Position = matrix * vec4(vec3(or.x, 0.0, or.y) + pos + corn, 1.0);
  v_color = color / 255.0;
  v_tex_coords = uv;
}
