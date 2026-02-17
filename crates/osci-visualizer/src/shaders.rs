// ── Line rendering shaders ──────────────────────────────────────────

pub const LINE_VERTEX: &str = r#"#version 330 core

layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_other;
layout(location = 2) in float a_perp;  // -1 or +1
layout(location = 3) in float a_along; // 0 or 1

uniform float u_sigma;

out vec2 v_pos;        // position in UV space
out vec2 v_seg_a;      // segment start in UV space
out vec2 v_seg_b;      // segment end in UV space
out float v_sigma;

void main() {
    vec2 dir = a_other - a_pos;
    float seg_len = length(dir);
    vec2 tang = seg_len > 0.0001 ? dir / seg_len : vec2(1.0, 0.0);
    vec2 norm = vec2(-tang.y, tang.x);

    // Expand along segment + perpendicular by 4*sigma for Gaussian coverage
    float expand = 4.0 * u_sigma;
    vec2 point = mix(a_pos, a_other, a_along);
    point += tang * (a_along * 2.0 - 1.0) * expand; // extend past endpoints
    point += norm * a_perp * expand;

    v_pos = point;
    v_seg_a = a_pos;
    v_seg_b = a_other;
    v_sigma = u_sigma;

    // Map from [0,1] UV to [-1,1] clip space
    gl_Position = vec4(point * 2.0 - 1.0, 0.0, 1.0);
}
"#;

pub const LINE_FRAGMENT: &str = r#"#version 330 core

in vec2 v_pos;
in vec2 v_seg_a;
in vec2 v_seg_b;
in float v_sigma;

uniform float u_intensity;

out vec4 frag_color;

// Approximation of erf() for Gaussian integral
float erf_approx(float x) {
    float a = 0.278393;
    float b = 0.230389;
    float c = 0.000972;
    float d = 0.078108;
    float ax = abs(x);
    float denom = 1.0 + ax * (a + ax * (b + ax * (c + ax * d)));
    float val = 1.0 - 1.0 / (denom * denom * denom * denom);
    return sign(x) * val;
}

void main() {
    vec2 seg = v_seg_b - v_seg_a;
    float seg_len = length(seg);

    float brightness;

    if (seg_len < 0.00001) {
        // Point: 2D Gaussian
        float dist = length(v_pos - v_seg_a);
        brightness = exp(-0.5 * (dist * dist) / (v_sigma * v_sigma));
    } else {
        vec2 tang = seg / seg_len;
        vec2 norm = vec2(-tang.y, tang.x);

        // Project fragment onto segment coordinate system
        vec2 d = v_pos - v_seg_a;
        float along = dot(d, tang);
        float perp = dot(d, norm);

        // Gaussian perpendicular to line
        float gauss_y = exp(-0.5 * (perp * perp) / (v_sigma * v_sigma));

        // erf integral along line (analytical Gaussian beam)
        float inv_sigma_sqrt2 = 1.0 / (v_sigma * 1.41421356);
        float erf_end = erf_approx((seg_len - along) * inv_sigma_sqrt2);
        float erf_start = erf_approx(-along * inv_sigma_sqrt2);
        float integral_x = 0.5 * (erf_end - erf_start);

        brightness = gauss_y * integral_x;
    }

    brightness *= u_intensity;
    frag_color = vec4(brightness, brightness, brightness, 1.0);
}
"#;

// ── Fullscreen quad shaders (shared by blur, persistence, compositor) ─

pub const FULLSCREEN_VERTEX: &str = r#"#version 330 core

layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_uv;

out vec2 v_uv;

void main() {
    v_uv = a_uv;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

// ── Blur shader ─────────────────────────────────────────────────────

pub const BLUR_FRAGMENT: &str = r#"#version 330 core

in vec2 v_uv;

uniform sampler2D u_texture;
uniform vec2 u_direction;   // (1/w, 0) for horizontal, (0, 1/h) for vertical
uniform int u_tap_count;    // half-size: 8 for 17-tap, 32 for 65-tap

out vec4 frag_color;

void main() {
    float sigma = float(u_tap_count) / 3.0;
    float sigma2 = 2.0 * sigma * sigma;

    vec4 color = texture(u_texture, v_uv) * 1.0; // center weight = 1
    float total_weight = 1.0;

    for (int i = 1; i <= u_tap_count; i++) {
        float w = exp(-float(i * i) / sigma2);
        vec2 offset = u_direction * float(i);
        color += texture(u_texture, v_uv + offset) * w;
        color += texture(u_texture, v_uv - offset) * w;
        total_weight += 2.0 * w;
    }

    frag_color = color / total_weight;
}
"#;

// ── Persistence shader ──────────────────────────────────────────────

pub const PERSISTENCE_FRAGMENT: &str = r#"#version 330 core

in vec2 v_uv;

uniform sampler2D u_current;
uniform sampler2D u_previous;
uniform float u_fade;  // decay factor per frame

out vec4 frag_color;

void main() {
    vec4 cur = texture(u_current, v_uv);
    vec4 prev = texture(u_previous, v_uv);
    frag_color = cur + prev * u_fade;
}
"#;

// ── Compositor shader ───────────────────────────────────────────────

pub const COMPOSITE_FRAGMENT: &str = r#"#version 330 core

in vec2 v_uv;

uniform sampler2D u_persisted;   // persisted line texture
uniform sampler2D u_tight_blur;  // 512x512 tight bloom
uniform sampler2D u_wide_blur;   // 128x128 wide bloom

uniform vec3 u_color;
uniform float u_exposure;
uniform float u_glow_amount;
uniform float u_scatter_amount;
uniform float u_overexposure;
uniform float u_saturation;
uniform float u_ambient;
uniform float u_noise;
uniform float u_time;

out vec4 frag_color;

// Simple hash for noise
float hash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

void main() {
    float line_val = texture(u_persisted, v_uv).r;
    float tight = texture(u_tight_blur, v_uv).r;
    float wide = texture(u_wide_blur, v_uv).r;

    // Combine line + bloom
    float bloom = u_glow_amount * (tight + u_scatter_amount * wide);
    float L = line_val + bloom;

    // Tone mapping: 1 - exp(-exposure * L)
    float mapped = 1.0 - exp(-u_exposure * L);

    // Apply color
    vec3 col = u_color * mapped;

    // Overexposure: blend toward white at high intensity
    float overex = smoothstep(0.7, 1.0, mapped) * u_overexposure;
    col = mix(col, vec3(1.0), overex);

    // Saturation adjustment
    float lum = dot(col, vec3(0.299, 0.587, 0.114));
    col = mix(vec3(lum), col, u_saturation);

    // Ambient tint
    col += u_color * u_ambient;

    // Noise grain
    float n = hash(v_uv * 1000.0 + u_time) * u_noise;
    col += vec3(n);

    frag_color = vec4(col, 1.0);
}
"#;
