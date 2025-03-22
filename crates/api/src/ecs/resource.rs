use bevy_reflect::{
    serde::TypedReflectSerializer, FromReflect, GetTypeRegistration, TypeRegistry, Typed,
};

use super::Addressable;

pub trait Resource
where
    Self: Sized + Typed + FromReflect + GetTypeRegistration + Addressable,
{
    fn default_value() -> Self;

    fn default_value_as_buffer() -> Vec<u8> {
        let value = Self::default_value();

        let registry = TypeRegistry::new();
        let serializer = TypedReflectSerializer::new(&value, &registry);
        bitcode::serialize(&serializer).unwrap()
    }
}

impl<R> Resource for R
where
    R: Sized + Typed + FromReflect + GetTypeRegistration + Addressable + Default,
{
    fn default_value() -> Self {
        Self::default()
    }
}
