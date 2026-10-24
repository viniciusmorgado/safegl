# SafeGL deferred renderer with TAA

Classical deferred, one directional light, and taa.
Uses the published `safegl = "0.1.0"` crate with the `gl46` feature.

## Run

From this directory:

```sh
cargo run --release --locked
```

From the collection directory:

```sh
cargo run --release --locked -p safegl-deferred-renderer-taa
```

## Pipeline

1. A repeating 16-sample Halton(2,3) sequence adds a subpixel offset to the projection.
2. Geometry writes single-sample world position/coverage (RGBA16F), normal/specular (RGBA16F), linear albedo (RGBA8), and depth24.
3. One directional light with classical Blinn–Phong shading produces linear RGBA16F color.
4. TAA resolves into alternating RGBA16F history textures. RGB stores accumulated color; alpha stores representative view depth for history validation.
5. A fullscreen presentation pass encodes the resolved linear color to sRGB. The FPS overlay is drawn afterward so text stays sharp.

World positions and the previous jittered view-projection matrix reproject history for this static model. A 3×3 foreground dilation supplies motion/depth near silhouettes. Bilinear history taps are individually rejected when their depth disagrees, and accepted history is constrained using the current neighborhood's YCoCg color range and variance. History weight ramps up to 90% and decreases during motion.

History resets on resize, explicit camera reset, large camera jumps, and recreation of the renderer. PNG capture resets and accumulates 32 frames at its own 1100×900 output resolution, independently of window size.

This is a basic TAA implementation for static geometry with camera motion. Independently animated objects would require previous object transforms or per-object velocity. Some softness or residual trailing around very thin hair can remain. This renderer still does not implement animation, shadows, transparent materials, or PBR/normal-map shading.

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

- `--taa-check`: Check temporal accumulation, camera motion, and resize behavior.
