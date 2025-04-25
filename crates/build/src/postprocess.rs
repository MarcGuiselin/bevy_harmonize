use std::u32;

use common::TypeSignature;

/// Represents a type and it's unique allocated address in wasm memory
///
/// Since the address does not overlap with other types, when we parse the compiled
/// wasm we can easily determine for which type an instruction corresponds to
pub struct TypeAddress<'a> {
    pub ty: &'a TypeSignature<'a>,
    pub address: u32,
}

impl<'a> TypeAddress<'a> {
    pub fn from_type_signatures(
        types: impl Iterator<Item = &'a TypeSignature<'a>>,
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

                TypeAddress { ty, address }
            })
    }
}

#[cfg(test)]
mod tests {
    use common::{StableId, TypeSignature};

    use super::*;

    fn new<'a>(ty: StableId<'a>, size: Option<usize>, align: Option<usize>) -> TypeSignature<'a> {
        TypeSignature::Struct {
            ty,
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
            new(id1, Some(256), Some(128)),
            new(id2, Some(1), Some(1)),
            new(id3, Some(32), Some(16)),
            new(id4, Some(8), Some(8)),
            new(invalid, None, None),
            new(invalid, None, Some(4)),
            new(invalid, Some(4), None),
            new(invalid, Some(4), Some(0)),
            new(invalid, Some(3), Some(3)),
            new(invalid, Some(4), Some(8)),
            new(invalid, Some(256), Some(256)),
        ];

        let addresses = TypeAddress::from_type_signatures(types.iter()).collect::<Vec<_>>();
        assert_eq!(addresses.len(), 4);

        assert_eq!(addresses[0].ty.stable_id(), id1);
        assert_eq!(addresses[0].address, u32::MAX - 127 - 256);

        assert_eq!(addresses[1].ty.stable_id(), id2);
        assert_eq!(addresses[1].address, u32::MAX - 127 - 256 - 1);

        assert_eq!(addresses[2].ty.stable_id(), id3);
        assert_eq!(addresses[2].address, u32::MAX - 127 - 256 - 48);

        assert_eq!(addresses[3].ty.stable_id(), id4);
        assert_eq!(addresses[3].address, u32::MAX - 127 - 256 - 48 - 8);
    }
}
