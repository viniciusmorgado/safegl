# SafeGL renderer collection

Nineteen independent Rust rendering experiments using the published SafeGL crate
and the same Ferris model, camera, and material coverage rules. Each package keeps
its renderer implementation and shaders separate for inspection and comparison.

## Run

Requires Rust/Cargo, a desktop window system, and a driver supporting OpenGL 4.6 Core.
From this directory:

```sh
cargo run --release --locked -p safegl-forward-renderer
```

Replace the package name with any entry below. You can also enter its directory
and run `cargo run --release --locked`. No local SafeGL checkout is required.
The entire directory can be copied out of the SafeGL repository as a standalone
Cargo workspace. Keep `assets/`, `ATTRIBUTION.md`, `LICENSE`, and `Cargo.lock` with it.

## Experiments

| Package | Rendering | AA | Lighting |
| --- | --- | --- | --- |
| [safegl-clustered-deferred-many-lights](safegl-clustered-deferred-many-lights/) | Clustered deferred | None | 256 point lights + directional fill |
| [safegl-clustered-deferred-many-lights-msaa4](safegl-clustered-deferred-many-lights-msaa4/) | Clustered deferred | MSAA 4× | 256 point lights + directional fill |
| [safegl-clustered-deferred-many-lights-ssaa4](safegl-clustered-deferred-many-lights-ssaa4/) | Clustered deferred | SSAA 4× | 256 point lights + directional fill |
| [safegl-clustered-deferred-many-lights-taa](safegl-clustered-deferred-many-lights-taa/) | Clustered deferred | TAA | 256 point lights + directional fill |
| [safegl-clustered-forward-many-lights](safegl-clustered-forward-many-lights/) | Clustered forward | None | 256 point lights + directional fill |
| [safegl-clustered-forward-many-lights-msaa4](safegl-clustered-forward-many-lights-msaa4/) | Clustered forward | MSAA 4× | 256 point lights + directional fill |
| [safegl-clustered-forward-many-lights-taa](safegl-clustered-forward-many-lights-taa/) | Clustered forward | TAA | 256 point lights + directional fill |
| [safegl-clustered-visibility-many-lights](safegl-clustered-visibility-many-lights/) | Clustered visibility | None | 256 point lights + directional fill |
| [safegl-clustered-visibility-many-lights-msaa4](safegl-clustered-visibility-many-lights-msaa4/) | Clustered visibility | MSAA 4× | 256 point lights + directional fill |
| [safegl-clustered-visibility-many-lights-ssaa4](safegl-clustered-visibility-many-lights-ssaa4/) | Clustered visibility | SSAA 4× | 256 point lights + directional fill |
| [safegl-clustered-visibility-many-lights-taa](safegl-clustered-visibility-many-lights-taa/) | Clustered visibility | TAA | 256 point lights + directional fill |
| [safegl-deferred-renderer](safegl-deferred-renderer/) | Classical deferred | None | One directional light |
| [safegl-deferred-renderer-taa](safegl-deferred-renderer-taa/) | Classical deferred | TAA | One directional light |
| [safegl-forward-plus-many-lights](safegl-forward-plus-many-lights/) | Tiled Forward+ | None | 256 point lights + directional fill |
| [safegl-forward-plus-many-lights-msaa4](safegl-forward-plus-many-lights-msaa4/) | Tiled Forward+ | MSAA 4× | 256 point lights + directional fill |
| [safegl-forward-plus-many-lights-ssaa4](safegl-forward-plus-many-lights-ssaa4/) | Tiled Forward+ | SSAA 4× | 256 point lights + directional fill |
| [safegl-forward-renderer](safegl-forward-renderer/) | Classical forward | None | One directional light |
| [safegl-forward-renderer-msaa8](safegl-forward-renderer-msaa8/) | Classical forward | MSAA 8× | One directional light |
| [safegl-visibility-renderer](safegl-visibility-renderer/) | Classical visibility | None | One directional light |

The many-lights variants share an 8×8×4 grid of point lights with identical
attenuation and shading. Compare those with one another; the single-light viewers
have a different lighting workload. SSAA 4× means twice the width and height,
followed by a linear-color 2×2 resolve. TAA is temporal accumulation. MSAA variants
use the sample counts shown; visibility MSAA reconstructs and shades each sample.

All renderers request OpenGL 4.6 Core, disable VSync, render without an FPS cap,
and show an FPS counter. Drag/arrows orbit, wheel/+/− zoom, R resets, Esc exits.

## Shared asset

All packages load `assets/ferris.glb`. See [ATTRIBUTION.md](ATTRIBUTION.md) for
the asset information. The example source code is licensed under Apache-2.0;
the model's rights are separate.

The GLB is included unchanged. At load time, node transforms are applied and
static primitives are grouped by material. The complete model is centered and
scaled uniformly. Ferris uses an opaque base-color texture; no transparency
approximation is needed for this asset.

These are classical lighting experiments: they use base color and vertex normals,
without normal maps, PBR, shadows, animation, or tone mapping. The loaders retain
alpha-cutout and screen-door support for other models.

## Checks

```sh
cargo fmt --all --check
cargo build --workspace --release --locked
cargo clippy --workspace --release --locked -- -D warnings
cargo run --release --locked -p safegl-forward-renderer -- --frames 120
cargo run --release --locked -p safegl-clustered-visibility-many-lights -- --validate-culling
```

Each package supports `--screenshot preview.png` and `--frames N`. See its README
and `--help` for culling, temporal, and benchmarking checks. GPU checks require a
working graphics session. Run comparisons sequentially at identical window sizes.
