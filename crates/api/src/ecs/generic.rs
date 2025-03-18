use bevy_reflect::Typed;

/// A generic trait to mark types that can be used as any of the following:
///
/// - Components
/// - Named system set (SystemSet in bevy)
/// - Schedule label (ScheduleLabel in bevy)
///
/// Note: [`Copy`] is needed to ensure these can be dropped in const contexts such as in the Schema.
pub trait Reflected
where
    Self: Typed + Copy,
{
}

impl<R> Reflected for R where R: Typed + Copy {}
