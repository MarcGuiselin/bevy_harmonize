use std::{any::TypeId, collections::HashMap};

use common::{FeatureDescriptor, FileHash, ModManifest, ScheduleDescriptor, StableId};

use crate::schema::Schema;

use super::type_signatures::TypeSignatures;

pub fn schema_to_manifest(schema: Schema) -> ModManifest<'static> {
    let mut types = TypeSignatures::new();
    for type_info_getter in schema.types.into_slice() {
        let type_info = (type_info_getter)();
        types.register_type(type_info);
    }

    // There can only be one default per resource
    let mut resources = HashMap::new();
    for (type_info_getter, value_getter) in schema.resources.into_slice() {
        let type_info = (type_info_getter)();
        types.register_type(type_info);

        let id = StableId::from_type_info(type_info);
        let value = (value_getter)();
        resources.insert(type_info.type_id(), (id, value));
    }
    let resources = resources.into_values().collect();

    // Combine schedules with the same label together
    let mut schedules: HashMap<TypeId, ScheduleDescriptor<'_>> = HashMap::new();
    for (type_info_getter, schedule) in schema.schedules.into_slice() {
        let type_info = (type_info_getter)();
        types.register_type(type_info);

        let id = StableId::from_type_info(type_info);
        schedules
            .entry(type_info.type_id())
            .and_modify(|descriptor| {
                let common::Schedule {
                    systems,
                    constraints,
                } = schedule.build();

                // TODO: dedupe systems and constraints
                descriptor.schedule.systems.extend(systems);
                descriptor.schedule.constraints.extend(constraints);
            })
            .or_insert(ScheduleDescriptor {
                id,
                schedule: schedule.build(),
            });
    }
    let schedules = schedules.into_values().collect();

    ModManifest {
        wasm_hash: FileHash::empty(),
        types: types.into_vec(),
        features: vec![FeatureDescriptor {
            name: schema.name.unwrap_or("unknown"),
            resources,
            schedules,
        }],
    }
}

// Tests
#[cfg(test)]
mod tests {
    use bevy_reflect::Reflect;
    use common::{
        FieldSignature, Param, Schedule, Start, System, SystemId, TypeSignature, VariantSignature,
    };

    use crate::{
        ecs::{system::IntoSystem, Addressable},
        schema::Mod,
    };

    use super::*;

    fn make_system<Marker, F>(system: F, params: Vec<Param<'static>>) -> System<'static>
    where
        F: IntoSystem<(), (), Marker>,
    {
        System {
            id: SystemId::from_type(IntoSystem::get_type_id(&system)),
            name: std::any::type_name::<F::System>(),
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
            .into_schema();

        let ModManifest {
            types,
            features,
            wasm_hash: _wasm_hash,
        } = schema_to_manifest(SCHEMA);

        assert_eq!(types.len(), 5);
        // In indeterminate order
        assert!(types.contains(&TypeSignature::Struct {
            ty: StableId::from_typed::<MyStruct>(),
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
        assert!(types.contains(&TypeSignature::Struct {
            ty: StableId::from_typed::<Start>(),
            generics: Vec::new(),
            fields: Vec::new(),
        }));
        assert!(types.contains(&TypeSignature::Opaque {
            ty: StableId::from_typed::<u32>(),
            generics: Vec::new(),
        }));
        assert!(types.contains(&TypeSignature::Opaque {
            ty: StableId::from_typed::<String>(),
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
