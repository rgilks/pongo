# WebGPU Surface Creation Research

## Problem
Creating a WebGPU surface from `HtmlCanvasElement` in wgpu 0.20 for web/WASM targets.

## Attempted Approaches

### 1. Direct `create_surface(canvas)`
- **Error**: `HtmlCanvasElement` doesn't implement `Into<SurfaceTarget>`
- **Reason**: Missing `HasWindowHandle` and `HasDisplayHandle` traits
- **Status**: ❌ Failed

### 2. `SurfaceTarget::Canvas(canvas)`
- **Error**: Variant doesn't exist in wgpu 0.20
- **Status**: ❌ Failed

### 3. `SurfaceTargetUnsafe::Canvas(canvas_js)`
- **Error**: Variant doesn't exist in `SurfaceTargetUnsafe` enum
- **Note**: Compiler suggests `SurfaceTargetUnsafe::from_window` but that requires `HasWindowHandle`
- **Status**: ❌ Failed

### 4. Feature flags
- **Attempted**: `wgpu = { version = "0.20", features = ["web"] }`
- **Error**: `wgpu` doesn't have a `web` feature
- **Status**: ❌ Failed

## Current Understanding

- wgpu 0.20 uses `raw_window_handle` traits for surface creation
- `HtmlCanvasElement` from `web-sys` doesn't implement these traits
- The unsafe API (`create_surface_unsafe`) exists but `SurfaceTargetUnsafe` enum variants are unclear

## Next Steps to Research

1. Check wgpu GitHub examples repository for actual working web/WASM code
2. Review wgpu 0.20 changelog for web surface creation changes
3. Check if there's a `wgpu-web` or similar helper crate
4. Review `raw_window_handle` crate documentation for web support
5. Consider if we need to use a different wgpu version or approach

## References

- [wgpu documentation](https://wgpu.rs/doc/wgpu/)
- [wgpu GitHub](https://github.com/gfx-rs/wgpu)
- [raw_window_handle](https://docs.rs/raw-window-handle/)

