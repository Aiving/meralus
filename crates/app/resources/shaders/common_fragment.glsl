#version 140

in vec2 v_tex_coords;
in vec4 v_color;
in vec2 v_overlay_tex_coords;
in vec4 v_overlay_color;
flat in uint v_have_overlay;

out vec4 f_color;

uniform sampler2D tex;
uniform bool with_tex;

void main() {
  if (with_tex) {
    vec4 baseTexel = texture2D(tex, v_tex_coords);

    if (v_have_overlay == uint(1)) {
      vec4 overlayTexel = texture2D(tex, v_overlay_tex_coords);

      if (overlayTexel.a == 0.0)
        f_color = baseTexel * v_color;
      else {
        overlayTexel.a = 1;

        f_color = overlayTexel * v_overlay_color;
      }
    } else
      f_color = baseTexel * v_color;
  } else
    f_color = v_color;
}
