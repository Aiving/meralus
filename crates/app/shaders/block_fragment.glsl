#version 450

/* layout(location = 0)  */in vec3 v_position;
/* layout(location = 1)  */in vec3 v_normal;
/* layout(location = 2)  */in vec2 v_tex_coord;
in vec4 v_color0;
// /* layout(location = 3)  */in vec2 v_tile_uv;
/* layout(location = 4)  */in float v_ambient_occlusion;

/* layout(location = 0)  */out vec4 f_color;

/* layout(set = 0, binding = 1)  */uniform sampler2D block_texture;

void main() {
  vec3 ao_color;

  if (v_ambient_occlusion < 1.0) ao_color = vec3(1.0, 0.0, 0.0);
  else if (v_ambient_occlusion < 2.0) ao_color = vec3(0.0, 1.0, 0.0);
  else if (v_ambient_occlusion < 3.0) ao_color = vec3(0.0, 0.0, 1.0);
  else if (v_ambient_occlusion < 4.0) ao_color = vec3(1.0, 1.0, 1.0);

  f_color = texture(block_texture, v_tex_coord) * (v_color0 / 255.0);
  f_color.rgb = mix(f_color.rgb, vec3(0.05, 0.05, 0.05),
                    0.3 * v_ambient_occlusion /* * distance(v_tile_uv, vec2(0.5)) */);
}