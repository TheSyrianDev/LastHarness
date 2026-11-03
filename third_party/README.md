# Third-party code

`gpui-pre/`, `gpui-pre-wgpu/`, `gpui-pre-apple/` and `gpui-pre-windows/` are copies of GPUI from the Zed project (https://github.com/zed-industries/zed),
licensed under Apache-2.0 (see `LICENSE-APACHE` in each). The workspace patches `crates-io` to use them.

Files changed from upstream:
- `gpui-pre-wgpu/src/cosmic_text_system.rs`
- `gpui-pre/src/text_system/line_layout_bidi.rs`
- `gpui-pre-apple/src/metal_renderer.rs` and `gpui-pre-windows/src/directx_renderer.rs` (skip the new glass primitive)
