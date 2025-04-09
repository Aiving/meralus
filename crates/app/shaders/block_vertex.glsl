#version 450

/* layout(location = 0)  */ in vec3 position;
/* layout(location = 2)  */ in vec2 texcoord;
/* layout(location = 1)  */ in vec4 normal;
in vec4 color0;
// /* layout(location = 3)  */ in vec2 tile_uv;
// /* layout(location = 4)  */ in float ambient_occlusion;

/* layout(location = 0)  */ out vec3 v_position;
/* layout(location = 1)  */ out vec3 v_normal;
/* layout(location = 2)  */ out vec2 v_tex_coord;
/* layout(location = 2)  */ out vec4 v_color0;
// /* layout(location = 3)  */ out vec2 v_tile_uv;
/* layout(location = 4)  */ out float v_ambient_occlusion;

uniform mat4 world;
uniform mat4 Model;
uniform mat4 Projection;
uniform vec2 tile_size;

void main() {
  mat4 worldview = Model * world;

  v_normal =
      mat3(transpose(inverse(world))) * vec3(normal.x, normal.y, normal.z);
  v_tex_coord = texcoord;
  v_color0 = color0;
  // v_tile_uv = tile_uv;
  v_position = vec3(world * vec4(position, 1.0));
  v_ambient_occlusion = normal.w;

  gl_Position =
      /* Projection * */ worldview * vec4(position.x, position.y, position.z, 1.0);
}
