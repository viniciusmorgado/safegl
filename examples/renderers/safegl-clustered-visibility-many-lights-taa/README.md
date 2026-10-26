# Clustered visibility — many lights — TAA

Clustered visibility, 256 point lights + directional fill, and taa.
Uses the published `safegl = "0.1.0"` crate with the `gl46` feature.

## Run

From this directory:

```sh
cargo run --release --locked
```

From the collection directory:

```sh
cargo run --release --locked -p safegl-clustered-visibility-many-lights-taa
```

## Pipeline

- Single-sample R32UI visibility stores triangle ID and facing, with a depth24 texture for rasterization and temporal reprojection. There is no normal, albedo, position, or stored-barycentric G-buffer.
- A compute shader assigns the 256 light spheres to **16×16 pixel tiles × 24 logarithmic view-depth slices**, spanning 0.05–100.0. It tests each sphere against four side planes and each slice's near/far planes.
- Each cluster reserves 256 indices plus a count; every light fits, with no truncation. All clusters are built, including empty/unoccupied space. This simple experiment trades memory and compute work for straightforward behavior; it is not an optimized sparse cluster implementation.
- Shading reconstructs perspective-correct attributes from visible triangle IDs and geometry buffer textures. Analytical UV derivatives select texture mipmaps. The reconstructed view depth selects the cluster, and only its light list is evaluated.
- A 16-sample Halton sequence jitters both rasterization and the cluster frusta. Shading produces linear RGBA16F color. TAA reconstructs world positions transiently from the depth texture, reprojects history, rejects mismatched depths, and clamps history in YCoCg using local variance. It adds no attribute G-buffer.
- Two RGBA16F history textures store color and linear depth. Resize, large camera cuts, and `R` reset history. The FPS overlay is drawn after TAA. Screenshots accumulate 32 frames.
- This baseline supports camera motion over static geometry; animated objects would need their own motion data. Temporal filtering can soften detail.
- Colors are evaluated in linear space and encoded to sRGB on output. A fullscreen shading draw per mesh handles ordinary material texture bindings.

At 1100×900 the 69×57×24 grid reserves about 92.5 MiB of index-list storage. Resize reallocates the visibility buffer and cluster lists. Performance gains over tiled or unclustered shading are workload-dependent.

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
