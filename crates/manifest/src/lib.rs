#![no_std]
#![feature(const_trait_impl)]

extern crate alloc;
use core::any::TypeId;

use alloc::{collections::BTreeMap, vec};

use api::schema::Schema;
use common::{FeatureDescriptor, FileHash, ModManifest, ScheduleDescriptor, StableId};

mod type_signatures;
use type_signatures::TypeSignatures;

pub fn schema_to_manifest(schema: Schema) -> ModManifest<'static> {
    let mut types = TypeSignatures::new();
    for ty in schema.types() {
        types.register_type(ty);
    }

    // There can only be one default per resource
    let mut resources = BTreeMap::new();
    for (type_info, value) in schema.resources() {
        let id = StableId::from_type_info(type_info);
        resources.insert(type_info.type_id(), (id, value));
    }
    let resources = resources.into_values().collect();

    // Combine schedules with the same label together
    let mut schedules: BTreeMap<TypeId, ScheduleDescriptor<'_>> = BTreeMap::new();
    for (type_info, schedule) in schema.schedules() {
        let id = StableId::from_type_info(type_info);
        let default = ScheduleDescriptor {
            id,
            schedule: schedule.clone(),
        };
        schedules
            .entry(type_info.type_id())
            .and_modify(|descriptor| {
                let common::Schedule {
                    systems,
                    constraints,
                } = schedule;

                // TODO: dedupe systems and constraints
                descriptor.schedule.systems.extend(systems);
                descriptor.schedule.constraints.extend(constraints);
            })
            .or_insert(default);
    }
    let schedules = schedules.into_values().collect();

    ModManifest {
        wasm_hash: FileHash::empty(),
        types: types.into_vec(),
        features: vec![FeatureDescriptor {
            name: schema.name().unwrap_or("unknown"),
            resources,
            schedules,
        }],
    }
}

// Tests
#[cfg(test)]
mod tests {
    use alloc::{string::String, vec::Vec};
    use api::prelude::*;
    use common::{FieldSignature, Param, Schedule, Start, System, TypeSignature, VariantSignature};

    use super::*;

    fn make_system<Marker, F>(system: F, params: Vec<Param<'static>>) -> System<'static>
    where
        F: IntoSystem<(), (), Marker>,
    {
        System {
            id: system.get_system_id(),
            name: system.get_name(),
            params,
        }
    }

    #[test]
    fn manifest_from_schema() {
        #[derive(Reflect)]
        struct MyStruct {
            foo: u32,
            bar: MyEnum,
        }

        unsafe impl Addressable for MyStruct {}

        impl Default for MyStruct {
            fn default() -> Self {
                Self {
                    foo: 2,
                    bar: MyEnum::Left,
                }
            }
        }

        #[derive(Reflect)]
        enum MyEnum {
            Left,
            Middle(u32),
            Right { string: String },
        }

        fn system1() {}
        fn system2() {}

        const SCHEMA: Schema = Mod::new("A custom name")
            .add_resource::<MyStruct>()
            .add_systems(Start, system1)
            .add_systems(Start, system2)
            .register_type::<u32>()
            .into_schema();

        let ModManifest {
            types,
            features,
            wasm_hash: _wasm_hash,
        } = schema_to_manifest(SCHEMA);

        assert_eq!(types.len(), 4);
        // In indeterminate order
        assert!(types.contains(&TypeSignature::Struct {
            ty: StableId::from_typed::<MyStruct>(),
            size: Some(size_of::<MyStruct>()),
            align: Some(align_of::<MyStruct>()),
            generics: Vec::new(),
            fields: vec![
                FieldSignature {
                    name: "foo",
                    ty: StableId::from_typed::<u32>()
                },
                FieldSignature {
                    name: "bar",
                    ty: StableId::from_typed::<MyEnum>()
                }
            ]
        }));
        assert!(types.contains(&TypeSignature::Enum {
            ty: StableId::from_typed::<MyEnum>(),
            size: None,
            align: None,
            generics: Vec::new(),
            variants: vec![
                VariantSignature::Unit { name: "Left" },
                VariantSignature::Tuple {
                    name: "Middle",
                    fields: vec![StableId::from_typed::<u32>()]
                },
                VariantSignature::Struct {
                    name: "Right",
                    fields: vec![FieldSignature {
                        name: "string",
                        ty: StableId::from_typed::<String>(),
                    }],
                }
            ]
        }));
        assert!(types.contains(&TypeSignature::Opaque {
            ty: StableId::from_typed::<u32>(),
            // First it's registered as a dependency of MyStruct without size/alignment
            // But then it's registered again with register_type with size/alignment
            size: Some(size_of::<u32>()),
            align: Some(align_of::<u32>()),
            generics: Vec::new(),
        }));
        assert!(types.contains(&TypeSignature::Opaque {
            ty: StableId::from_typed::<String>(),
            size: None,
            align: None,
            generics: Vec::new()
        }));

        assert_eq!(
            features,
            vec![FeatureDescriptor {
                name: "A custom name",
                resources: vec![(StableId::from_typed::<MyStruct>(), vec![4, 2, 0])],
                schedules: vec![ScheduleDescriptor {
                    id: StableId::from_typed::<Start>(),
                    schedule: Schedule {
                        systems: vec![
                            make_system(system1, Vec::new()),
                            make_system(system2, Vec::new()),
                        ],
                        constraints: Vec::new()
                    }
                }],
            }]
        )
    }
}
