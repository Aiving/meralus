#version 140

in vec2 screen_position;
in vec3 position;
in vec2 size;

uniform mat4 matrix;

void main() {
  gl_Position =
      matrix *
      vec4(vec3(screen_position + position.xy * size, position.z), 1.0);
}
