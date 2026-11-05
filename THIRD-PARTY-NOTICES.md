# Third-party notices

## Geist and Geist Pixel fonts

`crates/lh-gui/fonts/Geist-*.ttf` and `GeistPixel-Square.ttf` are from the Geist font family by the Geist Project Authors, https://github.com/vercel/geist-font. The font files are licensed under the SIL Open Font License, Version 1.1. The full license text is in `crates/lh-gui/fonts/OFL-Geist.txt`.

## JetBrains Mono Nerd Font

`crates/lh-gui/fonts/JetBrainsMonoNerdFontMono-*.ttf` are JetBrains Mono patched with Nerd Fonts glyphs. The font is by the JetBrains Mono Project Authors, https://github.com/JetBrains/JetBrainsMono, and is licensed under the SIL Open Font License, Version 1.1. The full license text is in `crates/lh-gui/fonts/OFL-JetBrainsMono.txt`.

## gpui-liquid-glass

The liquid glass material model in `third_party/gpui-pre-wgpu/src/shaders_glass.wgsl`
and `GlassVariant` / `glass_energy` in `third_party/gpui-pre/src/scene.rs` (variants,
bevel lens profile, rim and shade, chroma-adaptive warmth, interaction energy) are
ported from https://github.com/1234igor/gpui-liquid-glass. Only the maths was ported.

```
MIT License

Copyright (c) 2026 1234igor

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## motion

`crates/lh-core/web/motion.js` is motion 12.23.12 from https://motion.dev (npm package `motion`, file `dist/motion.js`), vendored unchanged and served by the web UI. No copy is loaded from a CDN.

```
The MIT License (MIT)

Copyright (c) 2024 [Motion](https://motion.dev) B.V.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
