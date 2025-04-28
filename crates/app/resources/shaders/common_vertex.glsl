#version 140

in vec3 position;
in vec2 uv;
in vec4 color;
in vec2 overlay_uv;
in vec4 overlay_color;
in uint have_overlay;

out vec2 v_tex_coords;
out vec4 v_color;
out vec2 v_overlay_tex_coords;
out vec4 v_overlay_color;
flat out uint v_have_overlay;

uniform mat4 matrix;

void main() {
  gl_Position = matrix * vec4(position, 1.0);
  v_color = color / 255.0;
  v_tex_coords = uv;
  v_overlay_tex_coords = overlay_uv;
  v_overlay_color = overlay_color / 255.0;
  v_have_overlay = have_overlay;
}
