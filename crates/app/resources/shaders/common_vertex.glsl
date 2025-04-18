#version 140

in vec3 position;
in vec2 uv;
in vec4 color;

out vec2 v_tex_coords;
out vec4 v_color;

uniform mat4 matrix;

void main() {
  gl_Position = matrix * vec4(position, 1.0);
  v_color = color / 255.0;
  v_tex_coords = uv;
}
