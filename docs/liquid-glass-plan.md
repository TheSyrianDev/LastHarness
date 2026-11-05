# Liquid glass plan

Status: plan only, nothing built from it yet. Written 2026-11-05.

The user's test: it is not liquid glass unless content scrolls under a glass bar and visibly bends. Today our
glass sits on flat panels with little sharp content behind it, so the lens has nothing to bend.

## (a) What Apple's Liquid Glass does

Sources:
- WWDC25 "Meet Liquid Glass" (session 219): https://developer.apple.com/videos/play/wwdc2025/219/
- WWDC25 "Build a SwiftUI app with the new design" (session 323): https://developer.apple.com/videos/play/wwdc2025/323/
- Apple HIG, Materials: https://developer.apple.com/design/human-interface-guidelines/materials
- SwiftUI `glassEffect(_:in:)`, `GlassEffectContainer`, `glassEffectID(_:in:)`, `scrollEdgeEffectStyle(_:for:)` in the
  SwiftUI reference on developer.apple.com.
- Community shaders: liquidGL (https://github.com/naughtyduk/liquidGL), liquid-glass-js
  (https://github.com/dashersw/liquid-glass-js), ybouane LiquidGlass (https://liquid-glass.ybouane.com/),
  @sohumsuthar/liquid-glass (npm), html-in-canvas.dev tutorial (https://html-in-canvas.dev/liquid-glass-effect/).

Verified from the sessions and search summaries; the rest is from the docs named above.

1. **Lensing first, blur second.** Apple says the material defines itself mainly through lensing: light from
   behind is bent and concentrated, strongest at the curved rim, not frosted like the old blur materials.
   Content behind stays recognisable.
2. **A floating layer for controls.** Glass is for the navigation and control layer (toolbars, tab bars,
   sidebars, sheets, pop-ups, buttons) that floats above content. Content itself is not glass. Glass on glass
   is avoided.
3. **Content scrolls under it.** Lists and pages run full height behind the bars. A scroll edge effect (soft
   blur and fade by default, `.hard` option for a crisper edge) dissolves content near the bar so titles and
   icons stay readable.
4. **Two variants.** Regular adapts its tint and contrast to what is behind and works anywhere. Clear is more
   see-through, for controls over rich media, and needs a dimming layer under it for legibility. A tint
   colour can be added for prominent actions.
5. **Adaptive.** Small glass flips between light and dark looks depending on the brightness of what is
   behind it. Bigger glass keeps its look but shadows and tint deepen. Shadows grow when text scrolls under.
6. **Specular highlights.** Highlights sit on the rim and move with device motion (and with the pointer and
   window focus on Mac).
7. **Thickness scales with size.** Larger or flexed glass reads as thicker material: deeper shadow, stronger
   lensing.
8. **Interactive.** `.glassEffect(.regular.interactive())` makes a control scale up, bounce and shimmer when
   touched, with light glowing from the touch point and spreading to nearby glass.
9. **Morph and merge.** Glass shapes inside one `GlassEffectContainer` blend when near each other like liquid
   drops (metaball style), and `glassEffectID` morphs one shape into another during transitions. One container
   also means one shared sampling region, which is how Apple keeps cost down.
10. **Concentric corners.** Corner radii follow the container: inner radius = outer radius minus padding, so
    nested shapes look concentric with the window and device corners.

Community recreations agree on the maths: an SDF of a rounded rect, a height profile at the bevel (circle or
squircle), displacement of the backdrop along the SDF normal by that slope (Snell-style), chromatic split of
red and blue strongest at the bevel and zero in the flat middle, Fresnel rim brightening, a specular band from a
light direction, and a modest blur. liquid-glass-js composites edge, rim and base layers with a 13x13 Gaussian.

## (b) Gap list: ours vs real

Ours today: `third_party/gpui-pre-wgpu/src/shaders_glass.wgsl` (SDF, quarter-circle bevel `glass_lens`,
displacement along normal, small 9-tap blur, red/blue split, chroma-adaptive warmth, four variants, rim line,
Fresnel spec, sheen, inner shade, `energy` for hover and press). The renderer copies the frame drawn so far for
each glass batch (`copy_glass_backdrop`, margin `blur + 2`). UI: `crates/lh-gui/src/style.rs` `surface_glass`,
`glass_layer`, `PanelExt` (`panel`, `elevated`, `side_panel`, `glass_fill`); `crates/lh-gui/src/ui.rs` `glass()`
for the slider and switch thumbs; `theme.rs` `tint_glass` for colour picks.

| Real Liquid Glass | Ours | Gap |
| --- | --- | --- |
| Content scrolls under floating bars | Panels sit beside content; scroll areas end at the bar | **Biggest gap.** Nothing sharp is behind most glass, so no visible bend |
| Glass only on the control layer | Panels, side panel, composer, pop-ups and knobs are all glass | Too much glass; content panels should go flat |
| Scroll edge effect | None | Missing |
| Lensing strong at rim, thickness grows with size | Bevel = `3 * refraction`, min 12 px; fixed per surface | Bevel does not scale with size; profile is a quarter circle (a squircle reads softer) |
| Regular adapts light/dark to backdrop | Tint is fixed per theme; only warmth adapts | No luminance adaptation |
| Clear needs a dimming layer | Clear has no dim | Readability risk |
| Specular moves with motion or pointer | Fixed light from top left | Light is static |
| Interactive: scale, bounce, glow from touch point | `energy` brightens evenly; knob gets a lift shadow | No touch-point glow, no scale flex |
| Merge and morph in a container | Each glass is separate; overlapping glass samples a copy without the other glass | Missing |
| One sampling region per container | One frame copy per batch; batches split by draw order | Can mean several copies per frame |
| Concentric corners | Radii from `p.radius` / `p.inner`, not derived | Partly |
| Windows | DirectX skips glass (`directx_renderer.rs:383`) | Glass looks become flat tints there |

## (c) Layout change: content under floating glass

Goal: the chat transcript, page lists and settings scroll full height, and the bars float over them.

- **Top bar.** Becomes an overlay (absolute, top) over the scroll area. The scroll area gets top padding equal
  to the bar height so the first line starts below it, but scrolled lines pass under it.
- **Composer.** Floats at the bottom over the transcript. Transcript gets bottom padding equal to the composer
  height plus a gap, so the last message can scroll clear of it.
- **Sidebar footers** (account, settings buttons) float over the end of the conversation list.
- **Pop-ups and menus** already float; make sure the content under them is the real page, not a solid panel.
- **Content panels go flat.** The transcript and page bodies stop being glass (`panel` becomes a plain surface
  in glass looks), so glass is only the floating layer. This also removes glass on glass.
- **Scroll edge effect.** A fade band under each bar (say 16 to 24 px) where content is dimmed toward the bar
  tint, drawn as a gradient quad before the glass. Soft is the default; a `hard` option for dense bars.

What is sampled: the backdrop copy is the frame drawn so far up to the glass batch, so transcript text, code
blocks, images and the window background all bend under the bar. That is exactly what we want, and it needs no
renderer change, only that bars are drawn after (on top of) the scroll content.

Cost: one region copy per glass batch, sized to the union of the batch's glass plus margin. A top bar (say
1200 x 48) plus composer (800 x 120) is under 0.2 MP per frame, small for the RTX 3050 and fine for iGPUs. The
risk is batch splitting: if text is drawn between two glass pieces, each gets its own copy and pass break. Keep
all floating glass at one layer order so it lands in one batch (see phase 4). While scrolling, the bars repaint
every frame anyway, so no new redraw cost.

## (d) Shader changes

All in `shaders_glass.wgsl`, mirrored where tests mirror maths (`glass_sdf`, `glass_bend` in `wgpu_renderer.rs`).

1. **Lens profile.** Replace the quarter circle with a squircle height `h(x) = (1 - (1 - x)^4)^(1/4)` and
   displace by its slope. Bevel width scales with size: `bezel = clamp(0.25 * min_half, 8, 32)`, so a big bar
   reads thicker than a knob (Apple's "thickness scales with size"). Keep `magnify` small for the flat middle.
2. **Specular.** Light direction becomes a uniform per glass (new `light: vec2` field), fed from the pointer
   position relative to the glass on desktop, top left at rest. Add a narrow specular band (`pow(facing, 8)`)
   on top of the broad sheen. Animate only on pointer move, no constant redraw.
3. **Adaptive tint (Regular).** Sample backdrop luminance over the glass (average of a few wide taps). Over
   bright content shift tint toward dark, over dark toward light, with hysteresis so it does not flicker while
   scrolling. Large glass only deepens shadow and tint alpha, as Apple does.
4. **Clear dimming.** Clear gets a dim term (darken backdrop by 10 to 20 percent in its middle) so the
   control's own icons stay readable.
5. **Interactive flex.** Add `touch: vec2` (press point, local). Glow is a radial falloff from the touch point
   times `energy`, not an even brightening. Scale (1.0 to about 1.06) is done in layout by the caller, with the
   existing spring in `ui.rs` `glass()`; the shader reads a larger bevel when energy is high.
6. **Merge and morph.** Pass up to N (say 4) neighbour rects in one container. In the shader use a smooth
   minimum of their SDFs (`smin(d1, d2, k)`, k about 12 px) so close shapes join with a liquid neck. Morph is
   then just animating rect bounds and k. Container glass is drawn as one quad over the union, one batch.
7. **Wider blur only where asked.** Regular keeps 9 taps; the scroll edge band can use a cheap 2-pass box
   blur in the copy if the fade alone is not enough.

## (e) Phases, small to big

Each phase is tested headless with `crates/lh-gui/tests/glass_shots.rs` (offscreen wgpu, no mouse, PNGs with
`LH_GLASS_SHOTS=<dir>`), plus the usual unit tests. Real app check is by the user.

1. **Text-behind test scene.** Add a shot with sharp black text and thin lines (a grid) behind a bar-shaped
   glass. Assert pixels inside the bevel differ from the same pixels without glass by more than a threshold,
   and that a straight line behind becomes curved (column of a vertical line shifts near the rim). Done: the
   test fails on a blur-only shader and passes on ours.
2. **Floating top bar and composer.** Layout change in `crates/lh-gui` only. Transcript runs full height,
   bars overlay it, padding keeps first and last lines reachable. Done: unit test of layout sizes; user sees
   text bend under the bar when scrolling.
3. **Flat content, glass controls.** `panel` becomes flat in glass looks; side panel footer, top bar,
   composer, pop-ups, knobs stay glass. Contrast test (`readability_problems`) still passes for every look.
4. **Scroll edge effect** and one-batch ordering for floating glass. Done: a shot shows the fade band; a
   renderer test counts one backdrop copy for bar plus composer.
5. **Lens profile and size-scaled bevel.** Done: shot of small knob vs wide bar, bevel width measured.
6. **Pointer specular and touch glow.** Done: shots with light at left vs right differ on the expected rim;
   touch glow peaks at the touch point.
7. **Adaptive tint and Clear dim.** Done: same glass over a white and a black backdrop picks different tints,
   text contrast in the glass stays above the budget in both.
8. **Merge and morph.** Done: two pills 8 px apart render a joined neck; 40 px apart render separate; morph
   test steps bounds and checks no gaps.

## (f) Risks

- **Windows.** The DirectX renderer has no glass (`third_party/gpui-pre-windows/src/directx_renderer.rs:383`).
  With content under bars, Windows bars would be a flat tint over text, which can hurt reading. Mitigation: on
  Windows, a near-solid bar fill plus the scroll edge fade, until an HLSL port of the shader exists (part of
  Milestone 10). WebGL2 also has no glass pipeline.
- **No COPY_SRC.** Some surfaces cannot be copied; then glass shows only tint and rim (already warned in
  `wgpu_renderer.rs:1232`). Same mitigation as Windows.
- **Performance.** Pass breaks and copies per glass batch; more glass pieces at different orders mean more
  breaks. Keep floating glass in one batch, cap the neighbour count for merge, avoid constant animation frames
  (only on scroll, pointer move, press).
- **Text readability.** Text scrolling under a bar competes with the bar's own labels. The scroll edge fade,
  adaptive tint and Clear dim exist for this. Keep the contrast test as the gate; never ship a look that fails
  it. Pop-ups keep their 0.9 minimum cover.
- **Reduced transparency / motion.** Respect a setting that turns glass into solid fills and stops specular
  motion.
- **Look and branding are the user's.** This plan is about the material and layout mechanics. Bar heights,
  fade sizes and tint strengths shown here are starting values to try, not decisions.
- **System prompt.** Not affected.
