# Fern 02

"Fern 02" from [Poly Haven](https://polyhaven.com/a/fern_02), released under
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) (public domain): no attribution is
required, but Poly Haven appreciates it.

Used by the `alpha_to_coverage` example. The files are the 1k glTF download, unmodified, minus
the normal map and the packed ambient occlusion/roughness/metalness map that the example doesn't
use (the glTF still refers to them). The diffuse texture of the glTF download is a JPEG without
alpha, so the cut-out mask is added from the separate alpha map of the same asset.

| File | Source | SHA-256 |
| --- | --- | --- |
| `fern_02_1k.gltf` | `Models/gltf/1k/fern_02/fern_02_1k.gltf` | `33974295ee19670af4877705fc7d91f33f42d6302f0d44a625e072271223031f` |
| `fern_02.bin` | `Models/gltf/8k/fern_02/fern_02.bin` (the same for every resolution) | `fe1abb762bcbbc7ad9e86925c8080437a62add257d9280e9142fdc85c226e64c` |
| `textures/fern_02_diff_1k.jpg` | `Models/jpg/1k/fern_02/fern_02_diff_1k.jpg` | `a828a6e7b4c01c826911e2eac4b3afc7457dab7b3b392a31c681ed88b7d50029` |
| `textures/fern_02_alpha_1k.png` | `Models/png/1k/fern_02/fern_02_alpha_1k.png` | `ea3c64284c7ebdeedf484d2d816eae92825ec526140c3bb83859ddc7a64001f9` |

Sources are relative to `https://dl.polyhaven.org/file/ph-assets/`.

This folder is excluded from the published crate.
