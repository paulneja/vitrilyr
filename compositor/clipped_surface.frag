#version 100
// SPDX-License-Identifier: GPL-3.0-or-later
// Vitrilyr optical surface, integrated through Niri-glass uniform plumbing.
//_DEFINES_
#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
#endif
precision highp float;
#if defined(EXTERNAL)
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif
uniform float alpha;
varying vec2 v_coords;
#if defined(DEBUG_FLAGS)
uniform float tint;
#endif
uniform float niri_scale;
uniform vec2 geo_size;
uniform vec4 corner_radius;
uniform mat3 input_to_geo;
uniform float lg_refraction_strength;
uniform float lg_refraction_power;
uniform float lg_edge_lighting;
uniform float lg_fringing;
uniform float lg_lens_distortion;
uniform float lg_brightness;
uniform float lg_contrast;
uniform float lg_saturation;
uniform float lg_edge_thickness;
uniform float lg_padding_pixels;
float niri_rounding_alpha(vec2 coords, vec2 size, vec4 radius);
vec4 postprocess(vec4 color);

float distanceToGlass(vec2 p, vec2 size) {
    vec2 centered = p - size * 0.5;
    float r = centered.x > 0.0
        ? (centered.y > 0.0 ? corner_radius.z : corner_radius.y)
        : (centered.y > 0.0 ? corner_radius.w : corner_radius.x);
    r = min(r, min(size.x, size.y) * 0.5);
    vec2 q = abs(centered) - size * 0.5 + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

// Local pixels follow the framebuffer transform, including flipped outputs.
vec2 pixelToTexture(vec2 offset) {
    vec2 local = offset / geo_size;
    float a = input_to_geo[0][0];
    float b = input_to_geo[1][0];
    float c = input_to_geo[0][1];
    float d = input_to_geo[1][1];
    float determinant = a * d - b * c;
    if (abs(determinant) < 0.000001) return vec2(0.0);
    return vec2(d * local.x - b * local.y, a * local.y - c * local.x) / determinant;
}

void main() {
    vec2 local = (input_to_geo * vec3(v_coords, 1.0)).xy * geo_size;
    vec2 size = geo_size - vec2(lg_padding_pixels * 2.0);
    vec2 p = local - vec2(lg_padding_pixels);
    vec4 color = texture2D(tex, v_coords);
    if (lg_refraction_strength > 0.0001) {
        float depth = max(0.0, -distanceToGlass(p, size));
        float bevel = clamp(min(size.x, size.y) * lg_edge_thickness * 0.5,
                            8.0, 28.0);
        float t = clamp(depth / bevel, 0.0, 1.0);
        vec2 gradient = vec2(
            distanceToGlass(p + vec2(0.5, 0.0), size) - distanceToGlass(p - vec2(0.5, 0.0), size),
            distanceToGlass(p + vec2(0.0, 0.5), size) - distanceToGlass(p - vec2(0.0, 0.5), size));
        vec2 outward = gradient / max(length(gradient), 0.001);
        float slope = (1.0 - t) * (1.0 - t);
        vec3 normal = normalize(vec3(outward * slope * 2.4, 1.0));
        vec3 ray = refract(vec3(0.0, 0.0, -1.0), normal, 1.0 / 1.46);
        float thickness = lg_refraction_strength * (1.0 + lg_refraction_power);
        vec2 offsetPx = ray.xy / max(abs(ray.z), 0.1) * thickness;
        offsetPx += (p - size * 0.5) * (-0.025 * lg_lens_distortion) * smoothstep(0.0, 1.0, t);
        vec2 displacement = pixelToTexture(offsetPx);
        float dispersion = clamp(lg_fringing * 0.12, 0.0, 0.2);
        vec2 uv = clamp(v_coords + displacement, 0.0, 1.0);
        color = texture2D(tex, uv);
        color.r = texture2D(tex, clamp(v_coords + displacement * (1.0 + dispersion), 0.0, 1.0)).r;
        color.b = texture2D(tex, clamp(v_coords + displacement * (1.0 - dispersion), 0.0, 1.0)).b;
        float luminance = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
        color.rgb = mix(vec3(luminance), color.rgb, lg_saturation);
        color.rgb = (color.rgb - 0.5) * lg_contrast + 0.5;
        color.rgb *= lg_brightness;
        float light = pow(abs(dot(outward, normalize(vec2(-0.65, -0.76)))), 5.0);
        float reflection = exp(-depth / 1.25) * (0.12 + 0.52 * light);
        color.rgb = mix(color.rgb, vec3(1.0), clamp(reflection * lg_edge_lighting, 0.0, 0.8));
    }
#if defined(NO_ALPHA)
    color.a = 1.0;
#endif
    color = postprocess(color);
    float inside = step(0.0, p.x) * step(p.x, size.x) * step(0.0, p.y) * step(p.y, size.y);
    color *= niri_rounding_alpha(p, size, corner_radius) * inside * alpha;
#if defined(DEBUG_FLAGS)
    if (tint == 1.0) color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif
    gl_FragColor = color;
}
