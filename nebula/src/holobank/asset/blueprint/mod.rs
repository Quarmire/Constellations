// Blueprints define how an assembly is constructed from a collection of blocks.
// Blueprints can alternatively be called templates.
// Once a blueprint is used to build an assembly, modifying the assembly does not modify the blueprint v.v.

pub trait Blueprint {
    /// Create a blueprint from another
    fn derive() -> Self;
    /// Create an empty blueprint
    fn create() -> Self;
    /// Destroy an blueprint
    fn destroy();
    /// Create an blueprint from resources outside the spaceport i.e.,
    /// documents
    fn import() -> Self;
    /// Send an blueprint to consumers outside the spaceport
    fn export(&self);
}