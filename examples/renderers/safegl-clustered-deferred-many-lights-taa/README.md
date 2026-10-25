# Clustered Deferred — many lights — TAA

Clustered deferred, 256 point lights + directional fill, and taa.
Uses the published `safegl = "0.1.0"` crate with the `gl46` feature.

## Run

From this directory:

```sh
cargo run --release --locked
```

From the collection directory:

```sh
cargo run --release --locked -p safegl-clustered-deferred-many-lights-taa
```

## Pipeline

Geometry writes world position (RGBA16F), normal/specular (RGBA16F), and linear albedo (RGBA8), plus depth24. A fullscreen lighting pass reads those attributes and evaluates the selected cluster's light list. G-buffer quantization and hardware texture derivatives can produce small differences from the visibility reconstruction path even though lighting equations match.

TAA uses the same temporal resolve as clustered visibility TAA: 16-frame Halton jitter, depth-based world-position reconstruction, camera reprojection, depth rejection per history tap, YCoCg variance clipping, and motion-dependent history blending. Both geometry and cluster frusta use the jittered projection. Current color and two history buffers use RGBA16F. Camera cuts, resize, and `R` reset history; screenshots accumulate 32 frames. This baseline handles camera motion over static geometry, not independently animated objects. Filtering can soften detail.

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
- `--taa-check`: Check temporal accumulation, camera motion, and resize behavior.
