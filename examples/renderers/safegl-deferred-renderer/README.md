# SafeGL classical deferred renderer

Classical deferred, one directional light, and no anti-aliasing.
Uses the published `safegl = "0.1.0"` crate with the `gl46` feature.

## Run

From this directory:

```sh
cargo run --release --locked
```

From the collection directory:

```sh
cargo run --release --locked -p safegl-deferred-renderer
```

## Rendering

OpenGL 4.6 Core, two rendering passes followed by the FPS overlay:

1. **Geometry:** depth-tested indexed meshes write three color attachments simultaneously, with a separate 24-bit depth buffer.
2. **Lighting:** a fullscreen triangle reads one G-buffer texel per pixel and computes ambient, diffuse, and Blinn–Phong specular from one directional light. Pixels without geometry receive the background color.

| Attachment | Format | Contents |
| --- | --- | --- |
| Position | RGBA16F | World-space position; alpha marks geometry coverage |
| Normal/specular | RGBA16F | World-space normal; alpha stores specular strength |
| Albedo | RGBA8 | Linear base color |
| Depth | Depth24 renderbuffer | Geometry-pass depth testing |

G-buffer attachments are single-sample, unfiltered, and recreated when the render target dimensions change. No MSAA is requested; multisampling is disabled in every draw pass. No FXAA, TAA, or other AA pass is applied.

Source base-color textures decode from sRGB when sampled. G-buffer color remains linear; only final output uses sRGB conversion. Material double-sided flags are respected. The lighting program explicitly sets `outputs_srgb: false` to enable framebuffer sRGB encoding of its linear output. This corrects an inherited configuration mismatch in the forward baseline, so this copy appears brighter. G-buffer precision and albedo quantization can also produce small differences.

## Model and materials

All examples load the shared `../assets/ferris.glb`, resolved relative to the
package directory at compile time. Keep the collection together when moving it.
See [ATTRIBUTION.md](../ATTRIBUTION.md) for the asset information.

The loader applies node transforms, groups static primitives by material, and
centers/scales the model uniformly to a two-unit largest dimension. Double-sided
materials retain both faces. Ferris uses an opaque base-color texture.

Lighting remains classical Blinn–Phong with base-color textures and vertex normals.
Normal maps, metallic/roughness maps, shadows, skinning, and animation are not used.
The loader also supports alpha cutouts and screen-door coverage for other models;
neither is used by Ferris.

## Controls and graphics defaults

OpenGL **4.6 Core** is required and checked at startup. VSync is disabled and FPS
is uncapped. A top-left counter measures presented frames per second.

- Left-drag or arrow keys: orbit.
- Mouse wheel or `+` / `-`: zoom.
- `R`: reset the camera.
- `Esc`: quit.

## Validation

```sh
cargo fmt --check
cargo build --release --locked
cargo clippy --release --locked -- -D warnings
cargo run --release --locked -- --frames 120
cargo run --release --locked -- --screenshot preview.png
```

`--frames` renders a bounded number of window frames and checks OpenGL errors.
`--screenshot` warms up rendering, saves a 1100×900 PNG, and exits. Both require
a desktop graphics session. Run `--help` for the complete option list.
