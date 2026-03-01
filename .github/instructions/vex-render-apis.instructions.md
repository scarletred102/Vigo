---
description: "Use when writing GPU rendering, text rendering, or image decoding code for the Vex render pipeline. Covers wgpu 23, cosmic-text 0.12, and image 0.25 API patterns to avoid deprecated or incorrect usage."
applyTo: "crates/vex-render/**/*.rs"
---

# Vex Engine — Render Crate API Patterns

## wgpu 23

### Texture Copy (ImageCopyTexture / ImageDataLayout)

Use `ImageCopyTexture` and `ImageDataLayout` — the `TexelCopy*` aliases were **removed** in wgpu 23:

```rust
// ✅ Correct
queue.write_texture(
    wgpu::ImageCopyTexture {
        texture: &texture,
        mip_level: 0,
        origin: wgpu::Origin3d { x, y, z: 0 },
        aspect: wgpu::TextureAspect::All,
    },
    data,
    wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(bytes_per_row),
        rows_per_image: None,
    },
    extent,
);

// ❌ Wrong — removed in wgpu 23
// wgpu::TexelCopyTextureInfo { ... }
// wgpu::TexelCopyBufferLayout { ... }
```

### Renderer Construction

`Renderer::new` takes `(&device, &queue, format)` and creates atlas textures on construction. Do not defer atlas creation.

## cosmic-text 0.12

### Getting Glyph Images (Avoid Borrow Conflicts)

Use `get_image_uncached()` to get an **owned** `SwashImage`. The cached variant (`get_image()`) returns a borrow that conflicts with the mutable font system:

```rust
// ✅ Correct — returns owned SwashImage, no borrow conflict
let image: Option<SwashImage> = cache.get_image_uncached(&mut font_system, image_key);

// ❌ Problematic — borrows cache, conflicts with font_system borrow
// let image = cache.get_image(&mut font_system, image_key);
```

## image 0.25

Supported decode formats (via `image::load_from_memory`): PNG, JPEG, WebP, GIF, BMP.  
Returns `DynamicImage` — convert with `.into_rgba8()` for GPU upload.

```rust
let img = image::load_from_memory(bytes)?.into_rgba8();
let (w, h) = img.dimensions();
let data: &[u8] = img.as_raw();
```

## Full Render Pipeline Order

```
parse_html(html)
  → collect <style> elements → parse_stylesheet()
  → compute_styles(document, [stylesheet], viewport)
  → layout_document(document, &computed, viewport_size)
  → build_display_list(&layout_root, viewport)
  → renderer.prepare(&display_list, &mut font_system, &device, &queue)
  → renderer.render(&mut encoder, &view)
```

Do not skip steps or combine phases — each stage is independently testable.

## Shader Files

WGSL shaders live in `crates/vex-render/src/shaders/`. One file per pipeline:

| File | Pipeline |
|------|---------|
| `rect.wgsl` | Solid-color rectangles |
| `text.wgsl` | Alpha-tested glyph quads |
| `image.wgsl` | Textured image quads |

Use `include_str!` to embed shaders at compile time — no runtime file I/O.

## Atlas Sizes

| Atlas | Size | Format |
|-------|------|--------|
| Glyph atlas | 2048×2048 | R8Unorm |
| Image atlas | 4096×4096 | Rgba8UnormSrgb |

Do not change these without profiling memory impact.
