#version 140

in vec3 position;
in uint light;
in vec2 uv;
in vec4 color;
in int visible;

out vec2 v_tex_coords;
out vec4 v_color;

uniform mat4 matrix;
uniform vec3 sun_position;

vec4 toLinear(vec4 sRGB) {
    bvec3 cutoff = lessThan(sRGB.rgb, vec3(0.04045));
    vec3 higher = pow((sRGB.rgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
    vec3 lower = sRGB.rgb / vec3(12.92);

    return vec4(mix(higher, lower, cutoff), sRGB.a);
}

void main() {
    if (visible == 1) {
        float block_light = (float(light & uint(15)) + 1.0) / 16.0;
        float sun_light = (float((light >> uint(4)) & uint(15)) + 1.0) / 16.0;

        float light_intensity =
            block_light + sun_light * max(sun_position.y * 0.96 + 0.3, 0.02);

        vec4 linear_color = toLinear(color / 255.0);

        gl_Position = matrix * vec4(position, 1.0);

        v_color = vec4(linear_color.rgb * light_intensity, linear_color.a);
        v_tex_coords = uv;
    }
}
