# Tiled Forward+ — many lights — SSAA 4x

Tiled Forward+, 256 point lights + directional fill, and ssaa 4×.
Uses the published `safegl = "0.1.0"` crate with the `gl46` feature.

## Run

From this directory:

```sh
cargo run --release --locked
```

From the collection directory:

```sh
cargo run --release --locked -p safegl-forward-plus-many-lights-ssaa4
```

## Pipeline

Renders geometry, visibility/attributes, and lighting at twice the output width and height: 1100×900 output means 2200×1800 internal rendering, four fully shaded subpixels per output pixel. A fullscreen pass averages each 2×2 block in **linear RGB**, then applies sRGB encoding on output. The FPS overlay remains at output resolution. This uses no multisample attachments and no temporal history.

Light tiles cover 32×32 internal pixels, equivalent to the original 16×16 output pixels. This preserves the light-culling granularity and avoids quadrupling the cluster-list allocation merely because shading resolution increased. Forward+ reduces all four subpixel depths per output pixel; clustered variants use the same 24 logarithmic depth slices. Material texture mipmaps are selected at the higher shading resolution.

The copied Forward+ intermediate color target is upgraded from sRGB8 to linear RGBA16F, so high-resolution shading is not clipped or quantized to 8-bit before the SSAA resolve.

All experiments share the same 256 point lights in an 8×8×4 grid, ambient term, weak directional fill, finite-radius attenuation, and Blinn–Phong equations. The SSAA comparison uses tiled Forward+ and the clustered Deferred/Visibility implementations, not the earlier single-directional-light baselines. G-buffer quantization and hardware versus analytical UV derivatives can cause small image differences between architectures.

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
- `--benchmark`: Measure offscreen rendering throughput at a fixed output size; excludes asset loading and presentation.
