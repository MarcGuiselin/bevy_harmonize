use std::any::type_name;

use super::{system_param::SystemParamItem, In, IntoSystem, System, SystemParam};
use common::SystemId;
use variadics_please::all_tuples;

/// The [`System`] counter part of an ordinary function.
///
/// You get this by calling [`IntoSystem::into_system`]  on a function that only accepts
/// [`SystemParam`]s. The output of the system becomes the functions return type, while the input
/// becomes the functions [`In`] tagged parameter or `()` if no such parameter exists.
///
/// [`FunctionSystem`] must be `.initialized` before they can be run.
///
/// The [`Clone`] implementation for [`FunctionSystem`] returns a new instance which
/// is NOT initialized. The cloned system must also be `.initialized` before it can be run.
pub struct FunctionSystem<Marker, F>
where
    F: SystemParamFunction<Marker>,
{
    func: F,
    state: <F::Param as SystemParam>::State,
    name: &'static str,
}

macro_rules! impl_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func: 'static, $($param: SystemParam),*> SystemParamFunction<fn($($param,)*) -> Out> for Func
        where
        for <'a> &'a mut Func:
                FnMut($($param),*) -> Out +
                FnMut($(SystemParamItem<$param>),*) -> Out, Out: 'static
        {
            type In = ();
            type Out = Out;
            type Param = ($($param,)*);
            #[inline]
            fn run(&mut self, _input: (), param_value: SystemParamItem<($($param,)*)>) -> Out {
                // Yes, this is strange, but `rustc` fails to compile this impl
                // without using this function. It fails to recognize that `func`
                // is a function, potentially because of the multiple impls of `FnMut`
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Out, $($param,)*>(
                    mut f: impl FnMut($($param,)*)->Out,
                    $($param: $param,)*
                )->Out{
                    f($($param,)*)
                }
                let ($($param,)*) = param_value;
                call_inner(self, $($param),*)
            }
        }

        #[allow(non_snake_case)]
        impl<Input, Out, Func: 'static, $($param: SystemParam),*> SystemParamFunction<fn(In<Input>, $($param,)*) -> Out> for Func
        where
        for <'a> &'a mut Func:
                FnMut(In<Input>, $($param),*) -> Out +
                FnMut(In<Input>, $(SystemParamItem<$param>),*) -> Out, Out: 'static
        {
            type In = Input;
            type Out = Out;
            type Param = ($($param,)*);
            #[inline]
            fn run(&mut self, input: Input, param_value: SystemParamItem< ($($param,)*)>) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Input, Out, $($param,)*>(
                    mut f: impl FnMut(In<Input>, $($param,)*)->Out,
                    input: In<Input>,
                    $($param: $param,)*
                )->Out{
                    f(input, $($param,)*)
                }
                let ($($param,)*) = param_value;
                call_inner(self, In(input), $($param),*)
            }
        }
    };
}

// Note that we rely on the highest impl to be <= the highest order of the tuple impls
// of `SystemParam` created.
all_tuples!(impl_system_function, 0, 16, F);

impl<Marker, F> IntoSystem<F::In, F::Out, Marker> for F
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
    <F as SystemParamFunction<Marker>>::Param: SystemParam,
{
    type System = FunctionSystem<Marker, F>;

    type State = <F::Param as SystemParam>::State;

    fn into_system(self) -> Self::System {
        let state = F::Param::init_state();
        // SAFETY: init_state always either produces a valid state or panics
        unsafe { self.into_system_with_state(state) }
    }

    unsafe fn into_system_with_state(self, state: Self::State) -> Self::System {
        let name = self.get_name();
        FunctionSystem {
            func: self,
            state,
            name,
        }
    }

    fn into_metadata() -> common::System<'static> {
        common::System {
            id: SystemId::of::<Self::System>(),
            name: extract_system_name(type_name::<Self::System>()),
            params: F::Param::get_metadata(),
        }
    }

    fn get_name(&self) -> &'static str {
        extract_system_name(type_name::<Self::System>())
    }
}

/// Takes a full quantified type name and extracts the system name from it.
fn extract_system_name(original: &'static str) -> &'static str {
    let start_pos = original
        .rfind(' ')
        .map(|start_pos| start_pos + 1usize)
        .unwrap_or(0usize);
    &original[start_pos..original.len() - 1]
}

impl<Marker, F> System for FunctionSystem<Marker, F>
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
{
    type In = F::In;
    type Out = F::Out;

    #[inline]
    fn name(&self) -> &'static str {
        self.name
    }

    #[inline]
    fn run(&mut self, input: Self::In) -> Self::Out {
        let params = F::Param::get_param(&mut self.state);
        let out = self.func.run(input, params);
        out
    }
}

/// A trait implemented for all functions that can be used as [`System`]s.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid system",
    label = "invalid system"
)]
pub trait SystemParamFunction<Marker>
where
    Self: 'static,
{
    /// The input type to this system. See [`System::In`].
    type In;

    /// The return type of this system. See [`System::Out`].
    type Out;

    /// The [`SystemParam`]/s used by this system.
    type Param: SystemParam;

    /// Executes this system once. See [`System::run`]
    fn run(&mut self, input: Self::In, param_value: SystemParamItem<Self::Param>) -> Self::Out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::any::TypeId;

    #[test]
    fn type_id_consistency() {
        fn function() {}
        fn another_function() {}

        let system_id = function.get_system_id();

        assert_eq!(
            system_id,
            function.get_system_id(),
            "System::type_id should be deterministic"
        );

        assert_eq!(
            system_id,
            SystemId::from_type(get_inner_id(function)),
            "System::type_id should be consistent with TypeId::of::<T::System>()"
        );
        fn get_inner_id<T, Marker>(_: T) -> TypeId
        where
            T: IntoSystem<(), (), Marker> + Copy,
        {
            TypeId::of::<T::System>()
        }

        assert_ne!(
            function.get_system_id(),
            another_function.get_system_id(),
            "Different systems should have different TypeIds"
        );
    }

    #[test]
    fn type_name_consistency() {
        fn function_system() {}

        assert_eq!(
            IntoSystem::get_name(&function_system),
            "bevy_harmonize_api::ecs::system::function_system::tests::type_name_consistency::function_system",
            "System::get_name should be empty for function systems"
        );
    }
}
