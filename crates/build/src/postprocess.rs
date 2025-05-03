use std::{
    fs::File,
    io::{BufReader, Seek},
    ops::Range,
    path::Path,
    u32,
};

use anyhow::*;
use common::TypeSignature;
use tracing::warn;
use wasmbin::{
    instructions::Instruction,
    sections::{payload, Import, ImportDesc, ImportPath},
    types::{Limits, MemType, PageSize},
    visit::Visit,
    Module,
};

/// Represents a type and it's unique allocated address in wasm memory
///
/// Since the address does not overlap with other types, when we parse the compiled
/// wasm we can easily determine for which type an instruction corresponds to
pub struct TypeAddress<'a> {
    pub signature: &'a TypeSignature,
    pub address: Range<u32>,
}

impl<'a> TypeAddress<'a> {
    pub fn from_type_signatures(
        types: impl Iterator<Item = &'a TypeSignature>,
    ) -> impl Iterator<Item = TypeAddress<'a>> {
        let mut address: u32 = u32::MAX;
        types
            .into_iter()
            // Take all types with known size and alignment
            .filter_map(|ty| match (ty.size(), ty.align()) {
                (Some(size), Some(align)) => Some((ty, size as u32, align as u32)),
                _ => None,
            })
            // Size should be a multiple of the alignment
            // Alignment must be a power of 2, greater than 0, less than or equal to 128
            // Size must be greater than 0
            .filter(|(_, size, align)| {
                *size > 0
                    && *align > 0
                    && size % align == 0
                    && align.is_power_of_two()
                    && *align <= 128
            })
            // Give them each their own unique address range
            .map(move |(ty, size, align)| {
                // Ensure it doesn't overlap with the previous type
                address -= size;
                // Ensure the address is aligned to the next multiple of the alignment
                address -= address % align;

                TypeAddress {
                    signature: ty,
                    address: address..address + size,
                }
            })
    }
}

pub async fn transform_wasm<P, Q>(
    src: P,
    dest: Q,
    types: impl IntoIterator<Item = TypeAddress<'_>>,
) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let file = File::open(src.as_ref())?;
    let mut reader = BufReader::new(file);
    let mut module = Module::decode_from(&mut reader).with_context(|| {
        format!(
            "Parsing error at offset 0x{:08X}",
            reader.stream_position().unwrap()
        )
    })?;

    let imports = module
        .find_or_insert_std_section(|| payload::Import::default())
        .try_contents_mut()?;

    let mut address_ranges = Vec::new();
    for ty in types {
        let id = ty.signature.stable_id();

        // Add resource memory pointers
        imports.push(Import {
            path: ImportPath {
                module: "bevy".to_string(),
                name: format!("{}::{}", id.crate_name, id.name),
            },
            desc: ImportDesc::Mem(MemType {
                page_size: Some(PageSize::MIN),
                limits: Limits { min: 0, max: None },
            }),
        });

        // Save range to get around the borrow checker
        address_ranges.push(ty.address);
    }

    // Adjust instructions to use correct memory indexes
    let mut previous_const_u32 = None;
    module.visit_mut(|instr| {
        previous_const_u32 = match instr {
            // Store a visited_const
            Instruction::I32Const(value) => Some(*value),

            // For all load/store instructions
            Instruction::I32Load(arg)
            | Instruction::I64Load(arg)
            | Instruction::F32Load(arg)
            | Instruction::F64Load(arg)
            | Instruction::I32Load8S(arg)
            | Instruction::I32Load8U(arg)
            | Instruction::I32Load16S(arg)
            | Instruction::I32Load16U(arg)
            | Instruction::I64Load8S(arg)
            | Instruction::I64Load8U(arg)
            | Instruction::I64Load16S(arg)
            | Instruction::I64Load16U(arg)
            | Instruction::I64Load32S(arg)
            | Instruction::I64Load32U(arg)
            | Instruction::I32Store(arg)
            | Instruction::I64Store(arg)
            | Instruction::F32Store(arg)
            | Instruction::F64Store(arg)
            | Instruction::I32Store8(arg)
            | Instruction::I32Store16(arg)
            | Instruction::I64Store8(arg)
            | Instruction::I64Store16(arg)
            | Instruction::I64Store32(arg) => {
                // Ideally we want know the exact range of possible addresses that will be accessed at runtime by this instruction
                // Without complex ast analysis the best we do is make an educated guess for now
                let access_min_bound = match previous_const_u32 {
                    // If the previous instruction pushes a const u32 onto the stack, we can know the address for sure
                    Some(offset) => arg.offset.wrapping_add(offset as u32),
                    // Otherwise assume that it will write to some address between the offset (inclusive) and the next allocated memory address range
                    None => arg.offset,
                };

                // Find the address range that contains the accessed address
                // Memory index corresponds to the index of the address range in the vector
                if let Some(id) = address_ranges.iter().enumerate().find_map(|(id, range)| {
                    if range.contains(&access_min_bound) {
                        Some(id)
                    } else {
                        None
                    }
                }) {
                    warn!(
                        "Found memory access at offset 0x{} for type {} {}",
                        arg.offset, id, arg.memory.index
                    );
                    arg.memory.index = id as u32 + 1;
                }

                None
            }

            // Ignore
            _ => None,
        };
    })?;

    let mut out_file = File::create(dest)?;
    module.encode_into(&mut out_file)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use common::{StableId, TypeSignature};

    use super::*;

    fn new(ty: &StableId, size: Option<usize>, align: Option<usize>) -> TypeSignature {
        TypeSignature::Struct {
            ty: ty.clone(),
            size,
            align,
            generics: vec![],
            fields: vec![],
        }
    }

    #[test]
    fn address_from_type_signatures() {
        let crate_name = "test_crate";
        let id1 = StableId::new(crate_name, "id1");
        let id2 = StableId::new(crate_name, "id2");
        let id3 = StableId::new(crate_name, "id3");
        let id4 = StableId::new(crate_name, "id4");
        let invalid = StableId::new(crate_name, "invalid");

        let types = vec![
            new(&id1, Some(256), Some(128)),
            new(&id2, Some(1), Some(1)),
            new(&id3, Some(32), Some(16)),
            new(&id4, Some(8), Some(8)),
            new(&invalid, None, None),
            new(&invalid, None, Some(4)),
            new(&invalid, Some(4), None),
            new(&invalid, Some(4), Some(0)),
            new(&invalid, Some(3), Some(3)),
            new(&invalid, Some(4), Some(8)),
            new(&invalid, Some(256), Some(256)),
        ];

        let addresses = TypeAddress::from_type_signatures(types.iter()).collect::<Vec<_>>();
        assert_eq!(addresses.len(), 4);

        let lower = u32::MAX - 127 - 256;
        assert_eq!(addresses[0].signature.stable_id(), id1);
        assert_eq!(addresses[0].address, lower..lower + 256);

        let lower = u32::MAX - 127 - 256 - 1;
        assert_eq!(addresses[1].signature.stable_id(), id2);
        assert_eq!(addresses[1].address, lower..lower + 1);

        let lower = u32::MAX - 127 - 256 - 48;
        assert_eq!(addresses[2].signature.stable_id(), id3);
        assert_eq!(addresses[2].address, lower..lower + 32);

        let lower = u32::MAX - 127 - 256 - 48 - 8;
        assert_eq!(addresses[3].signature.stable_id(), id4);
        assert_eq!(addresses[3].address, lower..lower + 8);
    }
}
