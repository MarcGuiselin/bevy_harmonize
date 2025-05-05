use alloc::{string::String, vec::Vec};
use bincode::{Decode, Encode};

use crate::StableId;

/// A serializable version of [`TypeInfo`]
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum TypeSignature {
    Struct {
        ty: StableId,
        size: Option<usize>,
        align: Option<usize>,
        generics: Vec<GenericSignature>,
        fields: Vec<FieldSignature>,
    },
    TupleStruct {
        ty: StableId,
        size: Option<usize>,
        align: Option<usize>,
        generics: Vec<GenericSignature>,
        fields: Vec<StableId>,
    },
    Tuple {
        ty: StableId,
        size: Option<usize>,
        align: Option<usize>,
        generics: Vec<GenericSignature>,
        fields: Vec<StableId>,
    },
    List {
        ty: StableId,
        generics: Vec<GenericSignature>,
        item_ty: StableId,
    },
    Array {
        ty: StableId,
        generics: Vec<GenericSignature>,
        item_ty: StableId,
        capacity: usize,
    },
    Map {
        ty: StableId,
        generics: Vec<GenericSignature>,
        key_ty: StableId,
        value_ty: StableId,
    },
    Set {
        ty: StableId,
        generics: Vec<GenericSignature>,
        value_ty: StableId,
    },
    Enum {
        ty: StableId,
        size: Option<usize>,
        align: Option<usize>,
        generics: Vec<GenericSignature>,
        variants: Vec<VariantSignature>,
    },
    Opaque {
        ty: StableId,
        size: Option<usize>,
        align: Option<usize>,
        generics: Vec<GenericSignature>,
    },
}

impl TypeSignature {
    /// Returns the stable id of the type
    pub fn stable_id(&self) -> StableId {
        match self {
            TypeSignature::Struct { ty, .. }
            | TypeSignature::TupleStruct { ty, .. }
            | TypeSignature::Tuple { ty, .. }
            | TypeSignature::List { ty, .. }
            | TypeSignature::Array { ty, .. }
            | TypeSignature::Map { ty, .. }
            | TypeSignature::Set { ty, .. }
            | TypeSignature::Enum { ty, .. }
            | TypeSignature::Opaque { ty, .. } => ty.clone(),
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
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum GenericSignature {
    Type(StableId),
    Const(StableId),
}

/// A serializable version of [`bevy_reflect::NamedField`]
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct FieldSignature {
    pub name: String,
    pub ty: StableId,
}

/// A serializable version of [`bevy_reflect::VariantInfo`]
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum VariantSignature {
    Struct {
        name: String,
        fields: Vec<FieldSignature>,
    },
    Tuple {
        name: String,
        fields: Vec<StableId>,
    },
    Unit {
        name: String,
    },
}
