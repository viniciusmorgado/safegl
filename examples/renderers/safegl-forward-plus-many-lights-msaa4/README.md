# SafeGL Forward+ — many lights, MSAA 4×

Tiled Forward+, 256 point lights + directional fill, and msaa 4×.
Uses the published `safegl = "0.1.0"` crate with the `gl46` feature.

## Run

From this directory:

```sh
cargo run --release --locked
```

From the collection directory:

```sh
cargo run --release --locked -p safegl-forward-plus-many-lights-msaa4
```

## Pipeline

1. **Depth prepass:** render material coverage into a four-sample depth24 texture.
2. **GPU tile culling:** one 16×16 compute workgroup per tile reduces all four depth samples per pixel to minimum/maximum view depth. It tests light spheres against the tile's four side planes and depth interval, then writes an SSBO light-index list.
3. **Forward shading:** rasterize the model again with equal-depth testing and depth writes disabled. Each fragment evaluates only its tile's light list, using textured Blinn–Phong shading and smooth finite-radius point-light attenuation.
4. **Resolve/presentation:** average the four RGBA16F color samples in linear space, encode the result to sRGB, and draw the FPS overlay afterward. The window framebuffer itself remains single-sample; MSAA happens in the offscreen scene buffers used by both window rendering and screenshots.

The depth and shading passes enable multisampling and share the same four-sample depth texture. Centroid interpolation keeps shading attributes inside covered geometry at edges. Shading runs at normal MSAA fragment frequency; this is not four full shader evaluations per pixel.

There is no material G-buffer. The prepass and shading vertex outputs are invariant, and use the same transforms. SafeGL handles SSBO write/read synchronization between compute and shading. Partial tiles at window edges and empty tiles are handled explicitly. Buffers are reallocated on resize.

Each tile has capacity for all 256 lights, so no affecting light is dropped due to list overflow. The fixed light grid is 8×8×4, with a 0.55 world-unit influence radius. Colors and positions are deterministic. A weak directional fill and ambient term retain readability between local lights. No shadows, animation, or PBR material shading are implemented.

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

- `--validate-culling`: Compare culled lighting with evaluating all 256 lights across camera and viewport cases.
