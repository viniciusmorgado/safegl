//! A reader of SPIR-V modules, for what OpenGL doesn't give about programs linked from SPIR-V:
//! the drivers can leave out the names of their variables, so SafeGL reads the names, the
//! locations, the bindings and the layouts of the blocks from the module itself. It also checks
//! the entry point and the specialization constants before OpenGL sees them.
#![cfg_attr(not(safegl_gl46), allow(dead_code))]

use std::collections::HashMap;

use crate::program::BlockLayout;
use crate::uniforms::UniformType;

const MAGIC: u32 = 0x0723_0203;

// opcodes
const OP_NAME: u32 = 5;
const OP_MEMBER_NAME: u32 = 6;
const OP_ENTRY_POINT: u32 = 15;
const OP_TYPE_BOOL: u32 = 20;
const OP_TYPE_INT: u32 = 21;
const OP_TYPE_FLOAT: u32 = 22;
const OP_TYPE_VECTOR: u32 = 23;
const OP_TYPE_MATRIX: u32 = 24;
const OP_TYPE_ARRAY: u32 = 28;
const OP_TYPE_RUNTIME_ARRAY: u32 = 29;
const OP_TYPE_STRUCT: u32 = 30;
const OP_TYPE_POINTER: u32 = 32;
const OP_CONSTANT: u32 = 43;
const OP_SPEC_CONSTANT_TRUE: u32 = 48;
const OP_SPEC_CONSTANT_FALSE: u32 = 49;
const OP_SPEC_CONSTANT: u32 = 50;
const OP_FUNCTION: u32 = 54;
const OP_FUNCTION_END: u32 = 56;
const OP_VARIABLE: u32 = 59;
const OP_DECORATE: u32 = 71;
const OP_MEMBER_DECORATE: u32 = 72;

// decorations
const SPEC_ID: u32 = 1;
const BLOCK: u32 = 2;
const BUFFER_BLOCK: u32 = 3;
const ARRAY_STRIDE: u32 = 6;
const BUILT_IN: u32 = 11;
const LOCATION: u32 = 30;
const COMPONENT: u32 = 31;
const INDEX: u32 = 32;
const BINDING: u32 = 33;
const OFFSET: u32 = 35;

// storage classes
const UNIFORM_CONSTANT: u32 = 0;
const INPUT: u32 = 1;
const UNIFORM: u32 = 2;
const OUTPUT: u32 = 3;
const ATOMIC_COUNTER: u32 = 10;
const STORAGE_BUFFER: u32 = 12;

/// The stage of a shader, as SPIR-V names it (`ExecutionModel`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExecutionModel {
    Vertex = 0,
    TessellationControl = 1,
    TessellationEvaluation = 2,
    Geometry = 3,
    Fragment = 4,
    GLCompute = 5,
}

impl ExecutionModel {
    /// Returns the model of a type of OpenGL shader.
    pub fn from_shader_type(shader_type: u32) -> Option<ExecutionModel> {
        use crate::gl;
        Some(match shader_type {
            gl::VERTEX_SHADER => ExecutionModel::Vertex,
            gl::TESS_CONTROL_SHADER => ExecutionModel::TessellationControl,
            gl::TESS_EVALUATION_SHADER => ExecutionModel::TessellationEvaluation,
            gl::GEOMETRY_SHADER => ExecutionModel::Geometry,
            gl::FRAGMENT_SHADER => ExecutionModel::Fragment,
            gl::COMPUTE_SHADER => ExecutionModel::GLCompute,
            _ => return None,
        })
    }
}

/// A type declared in a module.
#[derive(Clone, Debug)]
enum Type {
    Bool,
    Int {
        width: u32,
        signed: bool,
    },
    Float {
        width: u32,
    },
    Vector {
        component: u32,
        count: u32,
    },
    Matrix {
        column: u32,
        count: u32,
    },
    Array {
        element: u32,
        length: u32,
    },
    RuntimeArray {
        element: u32,
    },
    Struct {
        members: Vec<u32>,
    },
    Pointer {
        pointee: u32,
    },
    /// Images, samplers and the types that SafeGL doesn't need.
    Other,
}

/// The kinds of values of the specialization constants.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ConstantKind {
    Bool,
    Int,
    UnsignedInt,
    Float,
    /// 64-bit constants, which OpenGL can't specialize.
    Wide,
}

/// A variable of the module.
#[derive(Clone, Debug)]
struct Variable {
    id: u32,
    /// The type of the variable, not the pointer to it.
    ty: u32,
    storage: u32,
}

/// The value of a specialization constant: its `constant_id`, and its value.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Specialization {
    pub id: u32,
    pub kind: SpecializationKind,
    pub bits: u32,
}

/// The kind of value given to a specialization constant.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum SpecializationKind {
    Bool,
    Int,
    UnsignedInt,
    Float,
}

/// A uniform block, or a shader storage block, of a module.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Block {
    /// The binding of the block, `layout(binding = N)`; the elements of an array of blocks
    /// follow it.
    pub binding: u32,
    /// The name of the block, if the module has it, with `[i]` for the elements of an array.
    pub name: Option<String>,
    /// The layout of its members.
    pub layout: BlockLayout,
}

/// A SPIR-V module.
#[derive(Clone, Debug, Default)]
pub(crate) struct Module {
    /// The execution models, names and interface variables of the entry points.
    entry_points: Vec<(u32, String, Vec<u32>)>,
    /// The interface variables of the entry point that the shader uses, once checked.
    interface: Vec<u32>,
    /// The execution model of this entry point, once checked.
    model: Option<u32>,
    names: HashMap<u32, String>,
    member_names: HashMap<(u32, u32), String>,
    /// The decorations of the ids, with their first operand.
    decorations: HashMap<u32, Vec<(u32, u32)>>,
    member_decorations: HashMap<(u32, u32), Vec<(u32, u32)>>,
    types: HashMap<u32, Type>,
    /// The values of the integer constants and of the specialization constants.
    constants: HashMap<u32, u32>,
    /// The kinds of the specialization constants, by result id.
    spec_constants: HashMap<u32, ConstantKind>,
    variables: Vec<Variable>,
}

impl Module {
    /// Reads a module. Returns why it isn't one otherwise.
    pub fn parse(bytes: &[u8]) -> Result<Module, String> {
        if bytes.len() % 4 != 0 || bytes.len() < 20 {
            return Err("the size of a SPIR-V module is a multiple of 4 bytes, at least 20".into());
        }
        // the word at `index`
        let word = |index: usize| {
            let b = &bytes[index * 4..index * 4 + 4];
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        };
        let count = bytes.len() / 4;
        let words: Vec<u32> = match word(0) {
            MAGIC => (0..count).map(word).collect(),
            magic if magic.swap_bytes() == MAGIC => {
                (0..count).map(|index| word(index).swap_bytes()).collect()
            }
            _ => return Err("the data doesn't start with the magic number of SPIR-V".into()),
        };

        let mut module = Module::default();
        let mut position = 5;
        let (mut functions, mut open) = (0, false);
        while position < words.len() {
            let count = (words[position] >> 16) as usize;
            let opcode = words[position] & 0xffff;
            if count == 0 || position + count > words.len() {
                return Err(format!("the instruction at word {} is truncated", position));
            }
            match opcode {
                OP_FUNCTION if !open => open = true,
                OP_FUNCTION_END if open => {
                    open = false;
                    functions += 1;
                }
                OP_FUNCTION | OP_FUNCTION_END => {
                    return Err(format!("the function at word {} isn't closed", position));
                }
                _ => (),
            }
            module.instruction(opcode, &words[position + 1..position + count]);
            position += count;
        }
        // entry points are functions, which the module defines last
        if open || functions == 0 {
            return Err("the module is truncated: its last function doesn't end".into());
        }
        Ok(module)
    }

    /// Reads an instruction, given its operands.
    fn instruction(&mut self, opcode: u32, operands: &[u32]) {
        let operand = |index: usize| operands.get(index).copied().unwrap_or(0);
        match opcode {
            OP_NAME if !operands.is_empty() => {
                self.names.insert(operands[0], string(&operands[1..]));
            }
            OP_MEMBER_NAME if operands.len() >= 2 => {
                self.member_names
                    .insert((operands[0], operands[1]), string(&operands[2..]));
            }
            OP_ENTRY_POINT if operands.len() >= 2 => {
                let name = string(&operands[2..]);
                // the name takes its bytes and a nul character, in words
                let interface = operands
                    .get(2 + name.len() / 4 + 1..)
                    .unwrap_or_default()
                    .to_vec();
                self.entry_points.push((operands[0], name, interface));
            }
            OP_TYPE_BOOL => {
                self.types.insert(operand(0), Type::Bool);
            }
            OP_TYPE_INT => {
                let ty = Type::Int {
                    width: operand(1),
                    signed: operand(2) != 0,
                };
                self.types.insert(operand(0), ty);
            }
            OP_TYPE_FLOAT => {
                self.types
                    .insert(operand(0), Type::Float { width: operand(1) });
            }
            OP_TYPE_VECTOR => {
                let ty = Type::Vector {
                    component: operand(1),
                    count: operand(2),
                };
                self.types.insert(operand(0), ty);
            }
            OP_TYPE_MATRIX => {
                let ty = Type::Matrix {
                    column: operand(1),
                    count: operand(2),
                };
                self.types.insert(operand(0), ty);
            }
            OP_TYPE_ARRAY => {
                let ty = Type::Array {
                    element: operand(1),
                    length: operand(2),
                };
                self.types.insert(operand(0), ty);
            }
            OP_TYPE_RUNTIME_ARRAY => {
                let ty = Type::RuntimeArray {
                    element: operand(1),
                };
                self.types.insert(operand(0), ty);
            }
            OP_TYPE_STRUCT if !operands.is_empty() => {
                let ty = Type::Struct {
                    members: operands[1..].to_vec(),
                };
                self.types.insert(operands[0], ty);
            }
            OP_TYPE_POINTER => {
                self.types.insert(
                    operand(0),
                    Type::Pointer {
                        pointee: operand(2),
                    },
                );
            }
            // the other types are only declared, so that the types made of them are known
            19 | 25..=27 | 31 | 33..=39 => {
                self.types.insert(operand(0), Type::Other);
            }
            OP_CONSTANT => {
                self.constants.insert(operand(1), operand(2));
            }
            OP_SPEC_CONSTANT_TRUE | OP_SPEC_CONSTANT_FALSE => {
                let value = u32::from(opcode == OP_SPEC_CONSTANT_TRUE);
                self.constants.insert(operand(1), value);
                self.spec_constants.insert(operand(1), ConstantKind::Bool);
            }
            OP_SPEC_CONSTANT => {
                let kind = match self.types.get(&operand(0)) {
                    Some(Type::Int { width: 32, signed }) => {
                        if *signed {
                            ConstantKind::Int
                        } else {
                            ConstantKind::UnsignedInt
                        }
                    }
                    Some(Type::Float { width: 32 }) => ConstantKind::Float,
                    _ => ConstantKind::Wide,
                };
                self.constants.insert(operand(1), operand(2));
                self.spec_constants.insert(operand(1), kind);
            }
            OP_VARIABLE => {
                let ty = match self.types.get(&operand(0)) {
                    Some(&Type::Pointer { pointee }) => pointee,
                    _ => return,
                };
                self.variables.push(Variable {
                    id: operand(1),
                    ty,
                    storage: operand(2),
                });
            }
            OP_DECORATE if operands.len() >= 2 => {
                self.decorations
                    .entry(operands[0])
                    .or_default()
                    .push((operands[1], operand(2)));
            }
            OP_MEMBER_DECORATE if operands.len() >= 3 => {
                self.member_decorations
                    .entry((operands[0], operands[1]))
                    .or_default()
                    .push((operands[2], operand(3)));
            }
            _ => (),
        }
    }

    /// Checks that the module has an entry point named `name` for the stage `model`, which the
    /// inputs and outputs are then read from.
    pub fn check_entry_point(&mut self, name: &str, model: ExecutionModel) -> Result<(), String> {
        let entry_point = self
            .entry_points
            .iter()
            .find(|(m, n, _)| *m == model as u32 && n == name)
            .ok_or_else(|| {
                format!(
                    "the module has no entry point `{}` for {:?} shaders",
                    name, model
                )
            })?;
        self.interface = entry_point.2.clone();
        self.model = Some(model as u32);
        Ok(())
    }

    /// Returns true if the checked entry point is a shader of the stage `model`.
    pub fn is_stage(&self, model: ExecutionModel) -> bool {
        self.model == Some(model as u32)
    }

    /// Checks the values of specialization constants against the constants of the module, and
    /// uses them for the lengths of the arrays that depend on them.
    pub fn specialize(&mut self, values: &[Specialization]) -> Result<(), String> {
        for value in values {
            let constant = self
                .decorations
                .iter()
                .find(|(_, decorations)| decorations.contains(&(SPEC_ID, value.id)))
                .map(|(&id, _)| id)
                .filter(|id| self.spec_constants.contains_key(id))
                .ok_or_else(|| format!("the module has no specialization constant {}", value.id))?;
            let expected = match value.kind {
                SpecializationKind::Bool => ConstantKind::Bool,
                SpecializationKind::Int => ConstantKind::Int,
                SpecializationKind::UnsignedInt => ConstantKind::UnsignedInt,
                SpecializationKind::Float => ConstantKind::Float,
            };
            if self.spec_constants[&constant] != expected {
                return Err(format!(
                    "the specialization constant {} is a {:?}, not a {:?}",
                    value.id, self.spec_constants[&constant], value.kind
                ));
            }
            self.constants.insert(constant, value.bits);
        }
        Ok(())
    }

    /// Returns a decoration of an id.
    fn decoration(&self, id: u32, decoration: u32) -> Option<u32> {
        self.decorations
            .get(&id)?
            .iter()
            .find(|d| d.0 == decoration)
            .map(|d| d.1)
    }

    /// Returns a decoration of a member of a struct.
    fn member_decoration(&self, ty: u32, member: u32, decoration: u32) -> Option<u32> {
        self.member_decorations
            .get(&(ty, member))?
            .iter()
            .find(|d| d.0 == decoration)
            .map(|d| d.1)
    }

    /// Returns the name of an id, if the module has it.
    fn name(&self, id: u32) -> Option<&str> {
        self.names
            .get(&id)
            .map(String::as_str)
            .filter(|name| !name.is_empty())
    }

    /// Returns the number of elements of an array type.
    fn array_length(&self, length: u32) -> usize {
        self.constants.get(&length).copied().unwrap_or(0) as usize
    }

    /// Returns the names of the uniforms that hold one value each (the basic types, the
    /// matrices and the samplers and images) by location: the members of structs are named
    /// `name.member` and the elements of arrays `name[i]`, like OpenGL names them.
    pub fn uniform_names(&self) -> HashMap<u32, String> {
        let mut names = HashMap::new();
        for variable in &self.variables {
            if variable.storage != UNIFORM_CONSTANT {
                continue;
            }
            let (Some(location), Some(name)) = (
                self.decoration(variable.id, LOCATION),
                self.name(variable.id),
            ) else {
                continue;
            };
            let mut location = location;
            self.name_locations(variable.ty, name.to_owned(), &mut location, &mut names);
        }
        names
    }

    /// Gives the names of the values of type `ty` named `name`, from `location`.
    fn name_locations(
        &self,
        ty: u32,
        name: String,
        location: &mut u32,
        names: &mut HashMap<u32, String>,
    ) {
        match self.types.get(&ty) {
            Some(Type::Array { element, length }) => {
                for index in 0..self.array_length(*length) {
                    self.name_locations(*element, format!("{name}[{index}]"), location, names);
                }
            }
            Some(Type::Struct { members }) => {
                for (index, &member) in members.iter().enumerate() {
                    let member_name = self
                        .member_names
                        .get(&(ty, index as u32))
                        .cloned()
                        .unwrap_or_else(|| index.to_string());
                    let name = format!("{name}.{member_name}");
                    self.name_locations(member, name, location, names);
                }
            }
            _ => {
                names.insert(*location, name);
                *location += 1;
            }
        }
    }

    /// Returns the inputs of a vertex shader that have a location, by location and component,
    /// with their names if the module has them.
    pub fn input_names(&self) -> HashMap<(u32, u32), String> {
        self.interface_names(INPUT)
            .into_iter()
            .map(|(location, component, _, name)| ((location, component), name))
            .collect()
    }

    /// Returns the outputs of a fragment shader: their names, if the module has them, their
    /// locations and their indices.
    pub fn output_locations(&self) -> Vec<(Option<String>, u32, u32)> {
        self.variables
            .iter()
            .filter(|v| v.storage == OUTPUT && self.interface.contains(&v.id))
            .filter(|v| self.decoration(v.id, BUILT_IN).is_none())
            .filter_map(|v| {
                let location = self.decoration(v.id, LOCATION)?;
                let index = self.decoration(v.id, INDEX).unwrap_or(0);
                Some((self.name(v.id).map(str::to_owned), location, index))
            })
            .collect()
    }

    /// Returns the variables of a storage class with a location: their locations, components,
    /// variables and names.
    fn interface_names(&self, storage: u32) -> Vec<(u32, u32, u32, String)> {
        self.variables
            .iter()
            .filter(|v| v.storage == storage && self.interface.contains(&v.id))
            .filter_map(|v| {
                let location = self.decoration(v.id, LOCATION)?;
                let component = self.decoration(v.id, COMPONENT).unwrap_or(0);
                let name = self.name(v.id)?;
                Some((location, component, v.id, name.to_owned()))
            })
            .collect()
    }

    /// Returns the uniform blocks of the module, or its shader storage blocks.
    pub fn blocks(&self, storage_blocks: bool) -> Vec<Block> {
        let mut blocks = Vec::new();
        for variable in &self.variables {
            // arrays of blocks are blocks with consecutive bindings
            let (ty, count) = match self.types.get(&variable.ty) {
                Some(&Type::Array { element, length }) => (element, self.array_length(length)),
                _ => (variable.ty, 1),
            };
            let is_block = self.decoration(ty, BLOCK).is_some();
            let is_storage = match variable.storage {
                UNIFORM => self.decoration(ty, BUFFER_BLOCK).is_some(),
                STORAGE_BUFFER => is_block,
                _ => continue,
            };
            if !(is_block || is_storage) || is_storage != storage_blocks {
                continue;
            }
            let Some(binding) = self.decoration(variable.id, BINDING) else {
                continue;
            };
            let layout = self.layout(ty, 0);
            for index in 0..count {
                let name = self.name(ty).map(|name| match variable.ty == ty {
                    true => name.to_owned(),
                    false => format!("{name}[{index}]"),
                });
                blocks.push(Block {
                    binding: binding + index as u32,
                    name,
                    layout: layout.clone(),
                });
            }
        }
        blocks
    }

    /// Returns the atomic counters of the module: their bindings, offsets and names.
    pub fn atomic_counters(&self) -> Vec<(u32, u32, Option<String>)> {
        self.variables
            .iter()
            .filter(|v| v.storage == ATOMIC_COUNTER)
            .filter_map(|v| {
                let binding = self.decoration(v.id, BINDING)?;
                let offset = self.decoration(v.id, OFFSET).unwrap_or(0);
                Some((binding, offset, self.name(v.id).map(str::to_owned)))
            })
            .collect()
    }

    /// Returns the layout of a value of type `ty` at `offset` in a block. The members of structs
    /// that the module doesn't name are named after their indices.
    fn layout(&self, ty: u32, offset: usize) -> BlockLayout {
        match self.types.get(&ty) {
            Some(Type::Struct { members }) => BlockLayout::Struct {
                members: members
                    .iter()
                    .enumerate()
                    .map(|(index, &member)| {
                        let index = index as u32;
                        let name = self
                            .member_names
                            .get(&(ty, index))
                            .filter(|name| !name.is_empty())
                            .cloned()
                            .unwrap_or_else(|| index.to_string());
                        let member_offset =
                            self.member_decoration(ty, index, OFFSET).unwrap_or(0) as usize;
                        (name, self.layout(member, offset + member_offset))
                    })
                    .collect(),
            },
            Some(&Type::Array { element, length }) => BlockLayout::Array {
                content: Box::new(self.layout(element, offset)),
                length: self.array_length(length),
                stride: self.decoration(ty, ARRAY_STRIDE).unwrap_or(0) as usize,
            },
            Some(&Type::RuntimeArray { element }) => BlockLayout::DynamicSizedArray {
                content: Box::new(self.layout(element, offset)),
                stride: self.decoration(ty, ARRAY_STRIDE).unwrap_or(0) as usize,
            },
            _ => BlockLayout::BasicType {
                ty: self.uniform_type(ty),
                offset_in_buffer: offset,
            },
        }
    }

    /// Returns the type of a value, `Unknown` for the types that blocks can't hold.
    fn uniform_type(&self, ty: u32) -> UniformType {
        use UniformType::*;
        let scalar = |ty: u32| match self.types.get(&ty) {
            Some(Type::Bool) => Some(0),
            Some(Type::Int {
                width: 32,
                signed: true,
            }) => Some(1),
            Some(Type::Int {
                width: 32,
                signed: false,
            }) => Some(2),
            Some(Type::Float { width: 32 }) => Some(3),
            Some(Type::Float { width: 64 }) => Some(4),
            Some(Type::Int {
                width: 64,
                signed: true,
            }) => Some(5),
            Some(Type::Int {
                width: 64,
                signed: false,
            }) => Some(6),
            _ => None,
        };
        const VECTORS: [[UniformType; 4]; 7] = [
            [Bool, BoolVec2, BoolVec3, BoolVec4],
            [Int, IntVec2, IntVec3, IntVec4],
            [
                UnsignedInt,
                UnsignedIntVec2,
                UnsignedIntVec3,
                UnsignedIntVec4,
            ],
            [Float, FloatVec2, FloatVec3, FloatVec4],
            [Double, DoubleVec2, DoubleVec3, DoubleVec4],
            [Int64, Int64Vec2, Int64Vec3, Int64Vec4],
            [
                UnsignedInt64,
                UnsignedInt64Vec2,
                UnsignedInt64Vec3,
                UnsignedInt64Vec4,
            ],
        ];
        // by columns, then rows
        const FLOAT_MATRICES: [[UniformType; 3]; 3] = [
            [FloatMat2, FloatMat2x3, FloatMat2x4],
            [FloatMat3x2, FloatMat3, FloatMat3x4],
            [FloatMat4x2, FloatMat4x3, FloatMat4],
        ];
        const DOUBLE_MATRICES: [[UniformType; 3]; 3] = [
            [DoubleMat2, DoubleMat2x3, DoubleMat2x4],
            [DoubleMat3x2, DoubleMat3, DoubleMat3x4],
            [DoubleMat4x2, DoubleMat4x3, DoubleMat4],
        ];

        let vector = |ty: u32| match self.types.get(&ty) {
            Some(&Type::Vector { component, count }) if (2..=4).contains(&count) => {
                Some((scalar(component)?, count as usize))
            }
            _ => Some((scalar(ty)?, 1)),
        };
        let found = match self.types.get(&ty) {
            Some(&Type::Matrix { column, count }) if (2..=4).contains(&count) => {
                match vector(column) {
                    Some((3, rows)) if rows >= 2 => {
                        Some(FLOAT_MATRICES[count as usize - 2][rows - 2])
                    }
                    Some((4, rows)) if rows >= 2 => {
                        Some(DOUBLE_MATRICES[count as usize - 2][rows - 2])
                    }
                    _ => None,
                }
            }
            _ => vector(ty).map(|(scalar, count)| VECTORS[scalar][count - 1]),
        };
        found.unwrap_or(Unknown(0))
    }
}

/// Reads a nul-terminated UTF-8 string from words.
fn string(words: &[u32]) -> String {
    let bytes: Vec<u8> = words
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .take_while(|&byte| byte != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Module {
        let path = format!(
            "{}/tests/fixture/spirv/{}",
            env!("CARGO_MANIFEST_DIR"),
            name
        );
        Module::parse(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn not_modules() {
        assert!(Module::parse(&[0; 3]).is_err());
        assert!(Module::parse(&[0; 20]).is_err());
        // a truncated instruction
        let mut words = vec![MAGIC, 0x0001_0000, 0, 10, 0, 4 << 16 | OP_NAME];
        let bytes: Vec<u8> = words.drain(..).flat_map(u32::to_le_bytes).collect();
        assert!(Module::parse(&bytes).is_err());
        // a module without the end of its last function
        let path = format!(
            "{}/tests/fixture/spirv/textured.vert.spv",
            env!("CARGO_MANIFEST_DIR")
        );
        let module = std::fs::read(path).unwrap();
        assert!(Module::parse(&module).is_ok());
        assert!(Module::parse(&module[..module.len() - 4]).is_err());
    }

    #[test]
    fn entry_points() {
        let mut module = fixture("textured.frag.spv");
        assert!(module
            .check_entry_point("main", ExecutionModel::Fragment)
            .is_ok());
        assert!(module
            .check_entry_point("main", ExecutionModel::Vertex)
            .is_err());
        assert!(module
            .check_entry_point("other", ExecutionModel::Fragment)
            .is_err());
    }

    #[test]
    fn specialization() {
        let mut module = fixture("textured.frag.stripped.spv");
        let float = |id, value: f32| Specialization {
            id,
            kind: SpecializationKind::Float,
            bits: value.to_bits(),
        };
        assert!(module.specialize(&[float(7, 2.0)]).is_ok());
        assert!(module.specialize(&[float(8, 2.0)]).is_err());
        let int = Specialization {
            id: 7,
            kind: SpecializationKind::Int,
            bits: 2,
        };
        assert!(module.specialize(&[int]).is_err());
    }

    #[test]
    fn names() {
        let mut named = fixture("textured.frag.spv");
        named
            .check_entry_point("main", ExecutionModel::Fragment)
            .unwrap();
        let names = named.uniform_names();
        assert_eq!(names.get(&1).map(String::as_str), Some("tint"));
        assert_eq!(names.get(&2).map(String::as_str), Some("tex"));
        assert_eq!(
            named.blocks(false),
            vec![Block {
                binding: 1,
                name: Some("Factors".to_owned()),
                layout: BlockLayout::Struct {
                    members: vec![(
                        "scale".to_owned(),
                        BlockLayout::BasicType {
                            ty: UniformType::Float,
                            offset_in_buffer: 0,
                        }
                    )],
                },
            }]
        );
        assert_eq!(
            named.output_locations(),
            vec![(Some("color".to_owned()), 0, 0)]
        );

        // without names, the members of blocks are named after their indices
        let mut stripped = fixture("textured.frag.stripped.spv");
        stripped
            .check_entry_point("main", ExecutionModel::Fragment)
            .unwrap();
        assert!(stripped.uniform_names().is_empty());
        let blocks = stripped.blocks(false);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].name, None);
        assert!(matches!(
            &blocks[0].layout,
            BlockLayout::Struct { members } if members[0].0 == "0"
        ));
        assert_eq!(stripped.output_locations(), vec![(None, 0, 0)]);
    }

    #[test]
    fn inputs() {
        let mut module = fixture("textured.vert.spv");
        module
            .check_entry_point("main", ExecutionModel::Vertex)
            .unwrap();
        let inputs = module.input_names();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs.get(&(0, 0)).map(String::as_str), Some("position"));
    }

    #[test]
    fn storage_blocks() {
        let module = fixture("work_group.comp.spv");
        assert!(module.blocks(false).is_empty());
        let blocks = module.blocks(true);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].name.as_deref(), Some("Values"));
        assert_eq!(
            blocks[0].layout,
            BlockLayout::Struct {
                members: vec![(
                    "values".to_owned(),
                    BlockLayout::DynamicSizedArray {
                        content: Box::new(BlockLayout::BasicType {
                            ty: UniformType::UnsignedInt,
                            offset_in_buffer: 0,
                        }),
                        stride: 4,
                    }
                )],
            }
        );
    }
}
