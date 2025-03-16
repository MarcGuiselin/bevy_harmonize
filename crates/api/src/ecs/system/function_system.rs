use std::any::type_name;

use super::{system_param::SystemParamItem, In, IntoSystem, System, SystemParam};
use bevy_utils_proc_macros::all_tuples;
use common::SystemId;

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
        self.into_system_with_state(F::Param::init_state())
    }

    fn into_system_with_state(self, state: Self::State) -> Self::System {
        FunctionSystem {
            func: self,
            state,
            name: std::any::type_name::<F>(),
        }
    }

    fn into_metadata() -> common::System<'static> {
        common::System {
            id: SystemId::of::<Self::System>(),
            name: type_name::<Self::System>(),
            params: F::Param::get_metadata(),
        }
    }
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

    #[test]
    fn into_system_type_id_consistency() {
        fn test<T, Marker>(function: T)
        where
            T: IntoSystem<(), (), Marker> + Copy,
        {
            fn reference_system() {}

            use core::any::TypeId;

            let system = IntoSystem::into_system(function);

            assert_eq!(
                system.type_id(),
                function.get_type_id(),
                "System::type_id should be consistent with IntoSystem::system_type_id"
            );

            assert_eq!(
                system.type_id(),
                TypeId::of::<T::System>(),
                "System::type_id should be consistent with TypeId::of::<T::System>()"
            );

            assert_ne!(
                system.type_id(),
                IntoSystem::into_system(reference_system).type_id(),
                "Different systems should have different TypeIds"
            );
        }

        fn function_system() {}

        test(function_system);
    }
}
