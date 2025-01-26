use const_vec::ConstVec;

use super::system::ConstParams;
use bevy_utils_proc_macros::all_tuples;

#[const_trait]
pub trait SystemParam: Sized {
    /// Used to store data which persists across invocations of a system.
    type State: Send + Sync + 'static;

    /// The item type returned when constructing this system param.
    /// The value of this associated type should be `Self`, instantiated with new lifetimes.
    ///
    /// You could think of `SystemParam::Item<'s>` as being an *operation* that changes the lifetimes bound to `Self`.
    type Item<'state>: SystemParam<State = Self::State>;

    /// Creates a new instance of this param's [`State`](Self::State).
    fn init_state() -> Self::State;

    /// Creates a parameter to be passed into a [`SystemParamFunction`].
    fn get_param<'state>(state: &'state mut Self::State) -> Self::Item<'state>;

    /// Returns a descriptor for this param
    fn get_descriptors() -> ConstParams;
}

/// Shorthand way of accessing the associated type [`SystemParam::Item`] for a given [`SystemParam`].
pub type SystemParamItem<'s, P> = <P as SystemParam>::Item<'s>;

macro_rules! impl_system_param_tuple {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<$($param: ~const SystemParam),*> const SystemParam for ($($param,)*) {
            type State = ($($param::State,)*);
            type Item<'s> = ($($param::Item::<'s>,)*);

            #[inline]
            fn init_state() -> Self::State {
                (($($param::init_state(),)*))
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            fn get_param<'s>(
                state: &'s mut Self::State,
            ) -> Self::Item<'s> {
                let ($($param,)*) = state;
                ($($param::get_param($param),)*)
            }

            #[inline]
            fn get_descriptors() -> ConstParams {
                #[allow(unused_mut)]
                let mut vec = ConstVec::new();
                $(
                    vec.append($param::get_descriptors());
                )*
                vec
            }
        }
    };
}

all_tuples!(impl_system_param_tuple, 0, 16, P);
