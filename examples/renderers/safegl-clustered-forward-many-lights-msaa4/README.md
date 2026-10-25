# Clustered Forward — many lights — MSAA 4x

Clustered forward, 256 point lights + directional fill, and msaa 4×.
Uses the published `safegl = "0.1.0"` crate with the `gl46` feature.

## Run

From this directory:

```sh
cargo run --release --locked
```

From the collection directory:

```sh
cargo run --release --locked -p safegl-clustered-forward-many-lights-msaa4
```

## Pipeline

A depth prepass followed by depth-equal forward material shading. Attributes are interpolated by rasterization; there is no G-buffer or visibility buffer. The depth prepass is retained from the Forward+ source project, allowing a direct tiled-versus-clustered comparison.

Depth and linear RGBA16F color have four samples per pixel. Materials use centroid interpolation and conventional fragment shading, without forcing four shader invocations per pixel. The final pass averages the four linear color samples before sRGB encoding. This differs intentionally from the visibility MSAA experiment, which reconstructs and shades each sample separately.

Clustering matches the visibility experiments: 16×16 pixel tiles, 24 logarithmic depth slices from 0.05 to 100, and sphere/frustum light assignment in a compute shader. Each cluster reserves a count and 256 indices, so every light fits without truncation. All clusters are built, including unoccupied space. At 1100×900, cluster lists occupy approximately 92.5 MiB; storage grows with resolution. This is a straightforward baseline rather than a compact cluster allocator.

All variants share the same deterministic 8×8×4 grid of 256 point lights, finite-radius attenuation, ambient term, weak directional fill, and Blinn–Phong equations. Cluster selection uses the shaded world position's view depth. No shadows, animation, transparency, or PBR/normal-map shading.

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
