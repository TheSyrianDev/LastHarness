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
// smoothstep profile like a rounded bevel. Kept for reference and tests.
fn glass_bend(depth: f32, zone: f32) -> f32 {
    let t = saturate(1.0 - depth / max(zone, 0.001));
    return t * t * (3.0 - 2.0 * t);
}

// Lens bevel: the glass surface is a quarter circle `bezel` pixels wide at the
// rim and flat inside. Returns (displacement 0..1, surface normal z 0..1) at
// `depth` pixels inside the edge. The displacement is the height lost to the
// curve, so it is steep at the very rim and gone where the surface is flat,
// which is how a real lens squeezes what is behind it into the edge.
fn glass_lens(depth: f32, bezel: f32) -> vec2<f32> {
    let x = saturate(depth / max(bezel, 0.001));
    let u = 1.0 - x;
    let nz = sqrt(max(1.0 - u * u, 0.0));
    return vec2<f32>(1.0 - nz, nz);
}

// Snell, honestly: at a surface tilted `theta` from flat, light entering glass of
// index `eta` leaves at a shallower angle, and a thick slab displaces the ray by
// the *difference of the tangents* of the two angles. Scaling the tilt (`slope *
// (1 - 1/eta)`) overestimates the peak by a fifth and the grazing edge by more
// than double, which is what made rims look like a smear instead of a lens.
// Returns displacement in units of the slab's thickness, 0 flat out to 1 at the
// extreme that can actually happen (`theta` = 90 degrees).
fn glass_refract(cos_theta: f32, eta: f32) -> f32 {
    let theta = acos(clamp(cos_theta, 0.0, 1.0));
    let bent = asin(clamp(sin(theta) / eta, 0.0, 1.0));
    return tan(clamp(theta - bent, 0.0, 1.2));
}

// The bend at the extreme, so the result can be normalised to 0..1.
fn glass_refract_max(eta: f32) -> f32 {
    return tan(clamp(1.5707963 - asin(1.0 / eta), 0.0, 1.2));
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

    // Lens. The bevel scales with the piece: a 16 px knob gets a 3 px curve, a
    // whole panel all the refraction asks for. A fixed 12 px floor turned every
    // small control into one smeared edge.
    let depth = max(-d, 0.0);
    let min_half = min(half_size.x, half_size.y);
    let bezel = clamp(min_half * 0.42, 2.0, max(glass.refraction * 2.0, 3.0));
    let lens = glass_lens(depth, bezel);
    let edge = lens.x;
    // Where the surface is actually tilted. Inside the bevel a letter bends; the
    // flat middle magnifies instead, which is the difference between a lens and
    // a blur radius.
    let tilt = select(0.0, 1.0 - lens.y, bezel > 0.0);
    let refract = glass_refract(lens.y, 1.55) / glass_refract_max(1.55);
    let strength = select(1.25, 1.75, clear) * (1.0 + 0.3 * energy);
    // The bevel's own width is the slab thickness: the shift can never be wider
    // than the curve it comes from.
    let shift = bezel * strength * refract * saturate(tilt + 0.15 * edge * edge);
    // The flat middle magnifies a little too, so what is behind reads as seen
    // through a thick lens and not only at the rim.
    let magnify = select(0.035, 0.05, clear) * min(glass.refraction / 8.0, 1.5);
    let center = input.position.xy - n * shift - p * magnify;
    // Slight dispersion: red bends less than blue, only where the rim bends.
    let spread = n * shift * 0.12;

    // Same margin as the copy in wgpu_renderer.rs.
    let margin = glass.blur + 2.0;
    let lo = max(glass.bounds.origin - margin, vec2<f32>(0.5));
    let hi = min(glass.bounds.origin + glass.bounds.size + margin, globals.viewport_size - 0.5);

    // Blur: a Gaussian on two rings of eight taps, the rings turned to a different
    // angle than the pixel grid. Nine taps on the axes left square cells that read
    // as a grid through the glass; a rotated two-ring sample does not.
    var color = glass_backdrop(center, lo, hi) * 0.12;
    if (glass.blur > 0.0) {
        let b = glass.blur;
        // a per-piece turn, so a row of knobs does not all sample the same grid
        let turn = fract((glass.bounds.origin.x + glass.bounds.origin.y) * 0.0073) * 6.2831853;
        let r1 = b * 0.55;
        let r2 = b * 1.15;
        // 0.12 centre, 8 x 0.075 near, 8 x 0.04 far: sums to 1, tails included
        let near = 0.075;
        let far = 0.04;
        var i = 0;
        loop {
            if (i >= 8) { break; }
            let a = turn + f32(i) * 0.7853982;
            let dir = vec2<f32>(cos(a), sin(a));
            color += glass_backdrop(center + dir * r1, lo, hi) * near;
            color += glass_backdrop(center + dir * r2, lo, hi) * far;
            i = i + 1;
        }
        color *= 1.0 / 1.04;
    }
    if (shift > 0.5) {
        // Two extra taps, only near the rim: red from less bend, blue from more.
        let red = glass_backdrop(center + spread, lo, hi);
        let blue = glass_backdrop(center - spread, lo, hi);
        color = vec4<f32>(mix(color.r, red.r, 0.6), color.g, mix(color.b, blue.b, 0.6), color.a);
    }

    // Grade the backdrop in straight colour.
    var c = color.rgb / max(color.a, 0.0001);
    let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    // A thick lens lifts saturation; Clear only a little.
    c = saturate((luma + (c - luma) * select(1.12, 1.05, clear)) * select(1.01, 1.0, clear) + 2.0 / 255.0);

    // Tint. Regular warms on colourful backdrops (chroma-adaptive); tinted
    // variants let the tint colour carry the body, shaded by the backdrop.
    var tint = input.tint.rgb;
    if (!clear) {
        let chroma = max(max(c.r, c.g), c.b) - min(min(c.r, c.g), c.b);
        let warmth = 0.45 * saturate((chroma * 255.0 - 10.0) / 46.0);
        tint *= vec3<f32>(1.0, 1.0 - 0.067 * warmth, 1.0 - 0.165 * warmth);
    }
    let inner_glow = pow(edge, 1.2) * select(0.03, 0.01, clear);
    // Adaptive fill: over a dark backdrop the glass leans opaque so the words on
    // it stay readable; over a bright one it thins, so a bright page is not
    // milked. This is why one fill value never looked right on both.
    let adapt = saturate(1.15 - 1.3 * luma);
    // Never thinner than the look asked for: a menu over a bright page must stay
    // a menu. It only thickens, over a dark backdrop.
    var ta = saturate(input.tint.a * (0.95 + 0.3 * adapt) + inner_glow + energy * 0.025);
    if (tinted) {
        tint *= 0.6 + 0.6 * luma;
        ta = 0.35 + 0.65 * ta;
    }
    // The bevel stays clearer than the body, so the bend of what is behind
    // (words, edges) shows at the rim even on a nearly solid pop-up. The middle,
    // where the panel's own words sit, keeps its full tint.
    ta *= 1.0 - 0.45 * pow(edge, 0.7);
    var rgb = c * (1.0 - ta) + saturate(tint) * ta;
    var alpha = color.a * (1.0 - ta) + ta;

    // Light from the top left. `facing` is 1 on rims facing it, `away` on the
    // opposite rim, which catches a softer bounce like real glass.
    let light_dir = vec2<f32>(-0.6, -0.8);
    let facing = saturate(dot(n, light_dir));
    let away = saturate(-dot(n, light_dir));
    // Moving or pressed glass catches more light.
    let lit = glass.rim * (1.0 + 0.9 * energy);
    // Fresnel: the surface turns edge-on at the rim and reflects more there.
    let fresnel = pow(1.0 - lens.y, 2.0);
    // Specular on the bevel: bright on the lit side, softer opposite.
    let spec = fresnel * (select(0.12, 0.2, clear) + select(0.6, 0.9, clear) * facing * facing + 0.3 * away * away);
    // A broad sheen across the lit part of the bevel, like light caught in the
    // curve, reaching well inside the rim and not only the last pixel.
    let sheen = pow(edge, 0.6) * pow(facing, 1.5) * 0.32 + pow(edge, 0.8) * away * away * 0.1;
    // One rim: the Fresnel glow of the curve, brightest where it faces the light,
    // with no separate two-pixel line drawn on top. Stacking a line, a spec, a
    // sheen and a ring gave every control a chrome outline.
    let rim = saturate(lit * (spec * 0.85 + sheen * 0.65));
    // Inner shadow: a soft dark ring just inside the rim, deeper away from the light.
    let ring = saturate(1.0 - abs(depth - 3.0) / 3.5) * saturate(depth / 1.5);
    let shade = ring * (0.04 + 0.08 * (1.0 - facing)) * select(1.0, 1.3, clear) * saturate(glass.rim * 1.2);
    rgb = (rgb * (1.0 - rim) + vec3<f32>(rim)) * (1.0 - shade);
    alpha = alpha * (1.0 - rim) + rim;

    // Interaction energy brightens the body, most near the edge.
    let glow = energy * (0.055 + pow(edge, 1.5) * 0.12);
    rgb = rgb * (1.0 - glow) + vec3<f32>(glow);
    alpha = alpha * (1.0 - glow) + glow;
    rgb *= alpha;

    let a = alpha * coverage;
    if (globals.premultiplied_alpha != 0u) {
        return vec4<f32>(rgb * coverage, a);
    }
    return vec4<f32>(rgb / max(alpha, 0.0001), a);
}
