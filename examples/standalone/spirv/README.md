# Loading SPIR-V shaders with SafeGL

In this example, we start with a vertex and fragment shader in separate GLSL files, `shader.vert` and `shader.frag`.
Since we want to load SPIR-V shaders, we first compile the GLSL shaders to SPIR-V.
For that, we use `glslangValidator` which you can download [here](https://github.com/google/shaderc/blob/main/downloads.md).

We compile them to SPIR-V modules (`.spv` files) as follows:
```sh
glslangValidator -G shader.vert -o vert.spv
glslangValidator -G shader.frag -o frag.spv
```

Then we can load them in SafeGL using:
```rust
SpirvProgram::from_vs_and_fs(
    SpirvEntryPoint::new(include_bytes!("vert.spv"), "main"),
    SpirvEntryPoint::new(include_bytes!("frag.spv"), "main"),
)
```

But SPIR-V also allows having multiple entry points in the same module.
For this example we link fragment and vertex shaders together:
```sh
spirv-link vert.spv frag.spv -o shader.spv
```
And then we load them from the same `.spv` file:
```rust
let spirv = SpirvEntryPoint::new(include_bytes!("shader.spv"), "main");
let program = safegl::Program::new(&display, SpirvProgram::from_vs_and_fs(spirv, spirv)).unwrap();
```

Note: It's not a problem that both entry points are named `main`, since they are distinguished by their shader type:
```sh
$ spirv-cross shader.spv --reflect | jq .entryPoints
[
  {
    "name": "main",
    "mode": "vert"
  },
  {
    "name": "main",
    "mode": "frag"
  }
]
```

But we could also rename the entry points:
```sh
glslangValidator -G shader.vert --source-entrypoint main -e main_vs -o vert.spv
glslangValidator -G shader.frag --source-entrypoint main -e main_fs -o frag.spv
spirv-link vert.spv frag.spv -o shader.spv
```

Then we would load it like this:
```rust
let data = include_bytes!("shader.spv");
SpirvProgram::from_vs_and_fs(
    SpirvEntryPoint::new(data, "main_vs"),
    SpirvEntryPoint::new(data, "main_fs"),
)
```

Specialization constants (`layout(constant_id = N)`) get their values with `specialized`:
```rust
let fragment = SpirvEntryPoint::new(data, "main_fs")
    .specialized(&[(0, SpecializationConstant::Float(2.0))]);
```

Drivers can leave out the names that SPIR-V modules hold, and modules compiled without debug
information have none, so SafeGL reads the names from the modules, and also names every
resource after its location or binding: `location=N` for uniforms, attributes and fragment
outputs, `binding=N` for blocks. See the documentation of `SpirvEntryPoint`.
