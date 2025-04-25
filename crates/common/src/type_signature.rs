use bitcode::{Decode, Encode};

use crate::StableId;

/// A serializable version of [`TypeInfo`]
#[derive(Encode, Decode, PartialEq, Debug)]
pub enum TypeSignature<'a> {
    Struct {
        ty: StableId<'a>,
        size: Option<usize>,
        align: Option<usize>,
        generics: Vec<GenericSignature<'a>>,
        fields: Vec<FieldSignature<'a>>,
    },
    TupleStruct {
        ty: StableId<'a>,
        size: Option<usize>,
        align: Option<usize>,
        generics: Vec<GenericSignature<'a>>,
        fields: Vec<StableId<'a>>,
    },
    Tuple {
        ty: StableId<'a>,
        size: Option<usize>,
        align: Option<usize>,
        generics: Vec<GenericSignature<'a>>,
        fields: Vec<StableId<'a>>,
    },
    List {
        ty: StableId<'a>,
        generics: Vec<GenericSignature<'a>>,
        item_ty: StableId<'a>,
    },
    Array {
        ty: StableId<'a>,
        generics: Vec<GenericSignature<'a>>,
        item_ty: StableId<'a>,
        capacity: usize,
    },
    Map {
        ty: StableId<'a>,
        generics: Vec<GenericSignature<'a>>,
        key_ty: StableId<'a>,
        value_ty: StableId<'a>,
    },
    Set {
        ty: StableId<'a>,
        generics: Vec<GenericSignature<'a>>,
        value_ty: StableId<'a>,
    },
    Enum {
        ty: StableId<'a>,
        size: Option<usize>,
        align: Option<usize>,
        generics: Vec<GenericSignature<'a>>,
        variants: Vec<VariantSignature<'a>>,
    },
    Opaque {
        ty: StableId<'a>,
        size: Option<usize>,
        align: Option<usize>,
        generics: Vec<GenericSignature<'a>>,
    },
}

impl TypeSignature<'_> {
    /// Returns the stable id of the type
    pub fn stable_id(&self) -> StableId<'_> {
        match self {
            TypeSignature::Struct { ty, .. }
            | TypeSignature::TupleStruct { ty, .. }
            | TypeSignature::Tuple { ty, .. }
            | TypeSignature::List { ty, .. }
            | TypeSignature::Array { ty, .. }
            | TypeSignature::Map { ty, .. }
            | TypeSignature::Set { ty, .. }
            | TypeSignature::Enum { ty, .. }
            | TypeSignature::Opaque { ty, .. } => *ty,
        }
    }

    /// Returns the size of the type
    pub fn size(&self) -> Option<usize> {
        match self {
            TypeSignature::Struct { size, .. }
            | TypeSignature::TupleStruct { size, .. }
            | TypeSignature::Tuple { size, .. }
            | TypeSignature::Enum { size, .. }
            | TypeSignature::Opaque { size, .. } => *size,
            TypeSignature::List { .. }
            | TypeSignature::Array { .. }
            | TypeSignature::Map { .. }
            | TypeSignature::Set { .. } => None,
        }
    }

    /// Returns the alignment of the type
    pub fn align(&self) -> Option<usize> {
        match self {
            TypeSignature::Struct { align, .. }
            | TypeSignature::TupleStruct { align, .. }
            | TypeSignature::Tuple { align, .. }
            | TypeSignature::Enum { align, .. }
            | TypeSignature::Opaque { align, .. } => *align,
            TypeSignature::List { .. }
            | TypeSignature::Array { .. }
            | TypeSignature::Map { .. }
            | TypeSignature::Set { .. } => None,
        }
    }
}

/// A serializable version of [`bevy_reflect::GenericInfo`]
#[derive(Encode, Decode, PartialEq, Debug)]
pub enum GenericSignature<'a> {
    Type(StableId<'a>),
    Const(StableId<'a>),
}

/// A serializable version of [`bevy_reflect::NamedField`]
#[derive(Encode, Decode, PartialEq, Debug)]
pub struct FieldSignature<'a> {
    pub name: &'a str,
    pub ty: StableId<'a>,
}

/// A serializable version of [`bevy_reflect::VariantInfo`]
#[derive(Encode, Decode, PartialEq, Debug)]
pub enum VariantSignature<'a> {
    Struct {
        name: &'a str,
        fields: Vec<FieldSignature<'a>>,
    },
    Tuple {
        name: &'a str,
        fields: Vec<StableId<'a>>,
    },
    Unit {
        name: &'a str,
    },
}
