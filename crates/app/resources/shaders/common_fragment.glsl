#version 140

in vec2 v_tex_coords;
in vec4 v_color;

out vec4 f_color;

uniform sampler2D tex;

void main() { f_color = v_color * texture2D(tex, v_tex_coords); }
