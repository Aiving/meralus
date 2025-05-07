#version 140

in vec2 v_character;

out vec4 f_color;

uniform sampler2D font;
uniform vec4 text_color;

void main() { f_color = texture2D(font, v_character) * (text_color); }
