#[diagnostic::on_unimplemented(message = "`{Self}` is not a system", label = "invalid system")]
pub trait System
where
    Self: 'static,
{
    /// The system's input. See [`In`](crate::system::In) for
    /// [`FunctionSystem`](crate::system::FunctionSystem)s.
    type In;

    /// The system's output.
    type Out;

    /// Runs the system with the given input
    fn run(&mut self, input: Self::In) -> Self::Out;
}
