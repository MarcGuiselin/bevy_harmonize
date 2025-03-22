use std::{any::TypeId, collections::HashMap};

use bevy_reflect::{GenericInfo, Generics, Type, TypeInfo, VariantInfo};
use common::{FieldSignature, GenericSignature, StableId, TypeSignature, VariantSignature};

pub(crate) struct TypeSignatures(HashMap<TypeId, TypeSignature<'static>>);

impl TypeSignatures {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn register_type(&mut self, type_info: &TypeInfo) {
        let type_id = type_info.type_id();
        if !self.0.contains_key(&type_id) {
            let signature = match type_info {
                TypeInfo::Struct(info) => {
                    let field_count = info.field_len();
                    let mut fields = Vec::with_capacity(field_count);
                    for i in 0..field_count {
                        let field = info.field_at(i).unwrap();
                        fields.push(FieldSignature {
                            name: field.name(),
                            ty: ty(field.ty()),
                        });

                        // Recursively register fields
                        if let Some(type_id) = field.type_info() {
                            self.register_type(type_id);
                        }
                    }

                    TypeSignature::Struct {
                        ty: ty(info.ty()),
                        generics: generics(info.generics()),
                        fields,
                    }
                }
                TypeInfo::TupleStruct(info) => {
                    let field_count = info.field_len();
                    let mut fields = Vec::with_capacity(field_count);
                    for i in 0..field_count {
                        let field = info.field_at(i).unwrap();
                        fields.push(ty(field.ty()));

                        // Recursively register fields
                        if let Some(type_id) = field.type_info() {
                            self.register_type(type_id);
                        }
                    }

                    TypeSignature::TupleStruct {
                        ty: ty(info.ty()),
                        generics: generics(info.generics()),
                        fields,
                    }
                }
                TypeInfo::Tuple(info) => {
                    let field_count = info.field_len();
                    let mut fields = Vec::with_capacity(field_count);
                    for i in 0..field_count {
                        let field = info.field_at(i).unwrap();
                        fields.push(ty(field.ty()));

                        // Recursively register fields
                        if let Some(type_id) = field.type_info() {
                            self.register_type(type_id);
                        }
                    }

                    TypeSignature::Tuple {
                        ty: ty(info.ty()),
                        generics: generics(info.generics()),
                        fields,
                    }
                }
                TypeInfo::List(info) => TypeSignature::List {
                    ty: ty(info.ty()),
                    generics: generics(info.generics()),
                    item_ty: ty(&info.item_ty()),
                },
                TypeInfo::Array(info) => TypeSignature::Array {
                    ty: ty(info.ty()),
                    generics: generics(info.generics()),
                    item_ty: ty(&info.item_ty()),
                    capacity: info.capacity(),
                },
                TypeInfo::Map(info) => TypeSignature::Map {
                    ty: ty(info.ty()),
                    generics: generics(info.generics()),
                    key_ty: ty(&info.key_ty()),
                    value_ty: ty(&info.value_ty()),
                },
                TypeInfo::Set(info) => TypeSignature::Set {
                    ty: ty(info.ty()),
                    generics: generics(info.generics()),
                    value_ty: ty(&info.value_ty()),
                },
                TypeInfo::Enum(info) => {
                    let variant_count = info.variant_len();
                    let mut variants = Vec::with_capacity(variant_count);
                    for i in 0..variant_count {
                        let variant = info.variant_at(i).unwrap();
                        variants.push(match variant {
                            VariantInfo::Struct(info) => {
                                let field_count = info.field_len();
                                let mut fields = Vec::with_capacity(field_count);
                                for j in 0..field_count {
                                    let field = info.field_at(j).unwrap();
                                    fields.push(FieldSignature {
                                        name: field.name(),
                                        ty: ty(field.ty()),
                                    });

                                    // Recursively register fields
                                    if let Some(type_id) = field.type_info() {
                                        self.register_type(type_id);
                                    }
                                }

                                VariantSignature::Struct {
                                    name: info.name(),
                                    fields,
                                }
                            }
                            VariantInfo::Tuple(info) => {
                                let field_count = info.field_len();
                                let mut fields = Vec::with_capacity(field_count);
                                for j in 0..field_count {
                                    let field = info.field_at(j).unwrap();
                                    fields.push(ty(field.ty()));

                                    // Recursively register fields
                                    if let Some(type_id) = field.type_info() {
                                        self.register_type(type_id);
                                    }
                                }

                                VariantSignature::Tuple {
                                    name: info.name(),
                                    fields,
                                }
                            }
                            VariantInfo::Unit(info) => VariantSignature::Unit { name: info.name() },
                        });
                    }

                    TypeSignature::Enum {
                        ty: ty(info.ty()),
                        generics: generics(info.generics()),
                        variants,
                    }
                }
                TypeInfo::Opaque(info) => TypeSignature::Opaque {
                    ty: ty(info.ty()),
                    generics: generics(info.generics()),
                },
            };

            self.0.insert(type_id, signature);
        }
    }

    pub fn into_vec(self) -> Vec<TypeSignature<'static>> {
        self.0.into_values().collect()
    }
}

fn ty(ty: &Type) -> StableId<'static> {
    StableId::from_type_path_table(ty.type_path_table())
}

fn generics(generics: &Generics) -> Vec<GenericSignature<'static>> {
    generics
        .iter()
        .map(|generic| match generic {
            GenericInfo::Const(info) => GenericSignature::Const(ty(info.ty())),
            GenericInfo::Type(info) => GenericSignature::Type(ty(info.ty())),
        })
        .collect()
}
