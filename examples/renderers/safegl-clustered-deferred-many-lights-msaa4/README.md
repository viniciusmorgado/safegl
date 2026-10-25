# Clustered Deferred — many lights — MSAA 4x

Clustered deferred, 256 point lights + directional fill, and msaa 4×.
Uses the published `safegl = "0.1.0"` crate with the `gl46` feature.

## Run

From this directory:

```sh
cargo run --release --locked
```

From the collection directory:

```sh
cargo run --release --locked -p safegl-clustered-deferred-many-lights-msaa4
```

## Pipeline

Four-sample G-buffer attachments store position (RGBA16F), normal/specular (RGBA16F), and linear albedo (RGBA8), with four-sample depth24. Geometry uses sample interpolation, so attributes are evaluated at each covered sample. Fullscreen lighting fetches the matching G-buffer sample with `gl_SampleID`, selects its depth cluster, and shades into four-sample RGBA16F. A final pass averages the **shaded linear colors**, never positions/normals across surfaces. This intentionally straightforward per-sample implementation has no edge-only shading optimization.

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
