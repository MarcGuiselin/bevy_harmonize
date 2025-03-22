/// Indicates that the value of this Struct is tied to an address in memory
pub unsafe trait Addressable
where
    Self: Sized,
{
    const COMPONENT_ID: usize = 0;
    const PTR: *mut Self = align_of::<Self>() as _;
}
