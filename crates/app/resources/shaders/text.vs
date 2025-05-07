#version 140

in vec3 position;
in vec2 screen_position;
in vec2 character;
in vec2 offset;
in vec2 size;

out vec2 v_character;

uniform mat4 matrix;

void main() {
  gl_Position =
      matrix *
      vec4(vec3(screen_position + position.xy * (size * 4096), position.z),
           1.0);

  v_character = offset + character * size;
}
