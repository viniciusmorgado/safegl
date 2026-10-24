# Clustered visibility — many lights — MSAA 4x

Clustered visibility, 256 point lights + directional fill, and msaa 4×.
Uses the published `safegl = "0.1.0"` crate with the `gl46` feature.

## Run

From this directory:

```sh
cargo run --release --locked
```

From the collection directory:

```sh
cargo run --release --locked -p safegl-clustered-visibility-many-lights-msaa4
```

## Pipeline

- Four-sample R32UI visibility stores triangle ID and facing, with four-sample depth24 for rasterization. There is no normal, albedo, position, or stored-barycentric G-buffer.
- A compute shader assigns the 256 light spheres to **16×16 pixel tiles × 24 logarithmic view-depth slices**, spanning 0.05–100.0. It tests each sphere against four side planes and each slice's near/far planes.
- Each cluster reserves 256 indices plus a count; every light fits, with no truncation. All clusters are built, including empty/unoccupied space. This simple experiment trades memory and compute work for straightforward behavior; it is not an optimized sparse cluster implementation.
- Shading reconstructs perspective-correct attributes from visible triangle IDs and geometry buffer textures. Analytical UV derivatives select texture mipmaps. The reconstructed view depth selects the cluster, and only its light list is evaluated.
- Shading runs separately for every sample using `gl_SampleID` and `gl_SamplePosition`. Each sample retains its own triangle ID, reconstructed attributes, and cluster choice. Four RGBA16F shaded samples are averaged in linear space; integer IDs are never averaged. This deliberately expensive experiment does not optimize shared shading across samples.
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
