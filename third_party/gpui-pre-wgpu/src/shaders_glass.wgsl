// Liquid glass (LastHarness local patch). Only in the storage-buffer variant;
// WebGL2 has no glass pipeline and skips glass batches.
//
// The renderer copies the frame drawn so far into a backdrop texture, bound as
// `t_sprite`, then draws each glass rect sampling that copy: pulled inward near
// the rounded edge (lens bend), blurred a little, tinted, with a bright rim.
// `glass_sdf` and `glass_bend` are mirrored in Rust tests in wgpu_renderer.rs.
//
// The material model (variants, lens profile, rim and shade, chroma-adaptive
// warmth, interaction energy) follows gpui-liquid-glass by 1234igor (MIT); see
// THIRD-PARTY-NOTICES.md. Only the maths is ported, not its Metal code.

struct Glass {
    order: u32,
    pad: u32,
    bounds: Bounds,
    content_mask: Bounds,
    corner_radius: f32,
    blur: f32,
    refraction: f32,
    rim: f32,
    tint: Hsla,
    variant: u32,
    energy: f32,
}

@group(1) @binding(0) var<storage, read> b_glasses: array<Glass>;

struct GlassVarying {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) glass_id: u32,
    @location(1) clip_distances: vec4<f32>,
    @location(2) @interpolate(flat) tint: vec4<f32>,
}

// Signed distance to a rounded rect centred at the origin: negative inside.
fn glass_sdf(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + radius;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// How strongly the backdrop is pulled inward, 0 to 1, at `depth` pixels inside
// the edge. Full at the edge, fading to nothing `zone` pixels in, with a
// smoothstep profile like a rounded bevel.
fn glass_bend(depth: f32, zone: f32) -> f32 {
    let t = saturate(1.0 - depth / max(zone, 0.001));
    return t * t * (3.0 - 2.0 * t);
}

@vertex
fn vs_glass(@builtin(vertex_index) vertex_id: u32, @builtin(instance_index) instance_id: u32) -> GlassVarying {
    let unit_vertex = vec2<f32>(f32(vertex_id & 1u), 0.5 * f32(vertex_id & 2u));
    let glass = b_glasses[instance_id];
    var out = GlassVarying();
    out.position = to_device_position(unit_vertex, glass.bounds);
    out.glass_id = instance_id;
    out.clip_distances = distance_from_clip_rect(unit_vertex, glass.bounds, glass.content_mask);
    out.tint = hsla_to_rgba(glass.tint);
    return out;
}

// Backdrop colour at a pixel position, premultiplied, kept inside the copied area.
fn glass_backdrop(position: vec2<f32>, lo: vec2<f32>, hi: vec2<f32>) -> vec4<f32> {
    let p = clamp(position, lo, hi);
    let s = textureSampleLevel(t_sprite, s_sprite, p / globals.viewport_size, 0.0);
    // The target holds premultiplied colour only in premultiplied mode.
    return select(vec4<f32>(s.rgb * s.a, s.a), s, globals.premultiplied_alpha != 0u);
}

@fragment
fn fs_glass(input: GlassVarying) -> @location(0) vec4<f32> {
    if (any(input.clip_distances < vec4<f32>(0.0))) {
        return vec4<f32>(0.0);
    }
    let glass = b_glasses[input.glass_id];
    let half_size = glass.bounds.size * 0.5;
    let p = input.position.xy - (glass.bounds.origin + half_size);
    let d = glass_sdf(p, half_size, glass.corner_radius);
    let coverage = saturate(0.5 - d);
    if (coverage <= 0.0) {
        return vec4<f32>(0.0);
    }

    // Outward normal from the SDF gradient.
    let e = vec2<f32>(0.5, 0.0);
    var n = vec2<f32>(
        glass_sdf(p + e.xy, half_size, glass.corner_radius) - glass_sdf(p - e.xy, half_size, glass.corner_radius),
        glass_sdf(p + e.yx, half_size, glass.corner_radius) - glass_sdf(p - e.yx, half_size, glass.corner_radius),
    );
    n = n / max(length(n), 0.0001);

    let clear = glass.variant == 1u || glass.variant == 3u;
    let tinted = glass.variant >= 2u;
    let energy = saturate(glass.energy);

    // Lens: sample from further inside the closer we are to the edge. The bevel
    // is at most 16 px wide; Clear bends harder.
    let depth = max(-d, 0.0);
    let zone = min(min(half_size.x, half_size.y), 16.0);
    let edge = saturate(1.0 - depth / max(zone, 0.001));
    let strength = select(1.0, 1.5, clear);
    let center = input.position.xy - n * glass.refraction * strength * glass_bend(depth, zone);

    // Same margin as the copy in wgpu_renderer.rs.
    let margin = glass.blur + 2.0;
    let lo = max(glass.bounds.origin - margin, vec2<f32>(0.5));
    let hi = min(glass.bounds.origin + glass.bounds.size + margin, globals.viewport_size - 0.5);

    // Small blur: the centre plus two rings of four taps.
    var color = glass_backdrop(center, lo, hi);
    if (glass.blur > 0.0) {
        let b = glass.blur;
        let h = b * 0.5;
        color += glass_backdrop(center + vec2<f32>(b, 0.0), lo, hi);
        color += glass_backdrop(center - vec2<f32>(b, 0.0), lo, hi);
        color += glass_backdrop(center + vec2<f32>(0.0, b), lo, hi);
        color += glass_backdrop(center - vec2<f32>(0.0, b), lo, hi);
        color += glass_backdrop(center + vec2<f32>(h, h), lo, hi);
        color += glass_backdrop(center + vec2<f32>(-h, h), lo, hi);
        color += glass_backdrop(center + vec2<f32>(h, -h), lo, hi);
        color += glass_backdrop(center + vec2<f32>(-h, -h), lo, hi);
        color /= 9.0;
    }

    // Grade the backdrop in straight colour.
    var c = color.rgb / max(color.a, 0.0001);
    let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    if (!clear) {
        // Regular lifts saturation a touch.
        c = saturate((luma + (c - luma) * 1.05) * 1.01 + 2.0 / 255.0);
    }

    // Tint. Regular warms on colourful backdrops (chroma-adaptive); tinted
    // variants let the tint colour carry the body, shaded by the backdrop.
    var tint = input.tint.rgb;
    if (!clear) {
        let chroma = max(max(c.r, c.g), c.b) - min(min(c.r, c.g), c.b);
        let warmth = 0.45 * saturate((chroma * 255.0 - 10.0) / 46.0);
        tint *= vec3<f32>(1.0, 1.0 - 0.067 * warmth, 1.0 - 0.165 * warmth);
    }
    let inner_glow = pow(edge, 1.7) * select(0.025, 0.008, clear);
    var ta = saturate(input.tint.a + inner_glow + energy * 0.025);
    if (tinted) {
        tint *= 0.6 + 0.6 * luma;
        ta = 0.35 + 0.65 * ta;
    }
    var rgb = c * (1.0 - ta) + saturate(tint) * ta;
    var alpha = color.a * (1.0 - ta) + ta;

    // Light from the top left: 1 on edges facing it, 0 facing away.
    let light = saturate((-n.x - n.y) * 0.5 + 0.5);
    // Specular bevel: a broad bright band near the edge, sharper on Clear.
    let spec = select(pow(edge, 5.0) * (0.06 + 0.18 * light), pow(edge, 6.0) * (0.12 + 0.58 * light), clear);
    // Shade on the bevel facing away from the light.
    let shade = pow(edge, 4.0) * (1.0 - light) * select(0.04, 0.13, clear);
    // Rim: a thin bright line at the very edge, scaled by `rim`.
    let band = saturate(1.0 - depth / 1.5);
    let line = band * (0.35 + 0.65 * light * light);
    let rim = saturate(glass.rim * (line + spec));
    rgb = (rgb * (1.0 - rim) + vec3<f32>(rim)) * (1.0 - shade);
    alpha = alpha * (1.0 - rim) + rim;

    // Interaction energy brightens the body, most near the edge.
    let glow = energy * (0.055 + pow(edge, 2.2) * 0.12);
    rgb = rgb * (1.0 - glow) + vec3<f32>(glow);
    alpha = alpha * (1.0 - glow) + glow;
    rgb *= alpha;

    let a = alpha * coverage;
    if (globals.premultiplied_alpha != 0u) {
        return vec4<f32>(rgb * coverage, a);
    }
    return vec4<f32>(rgb / max(alpha, 0.0001), a);
}
