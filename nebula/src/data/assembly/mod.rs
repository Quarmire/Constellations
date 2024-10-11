use super::blueprint::Blueprint;

pub trait Assembly {
    fn construct<B>(blueprint: B) 
        where B: Blueprint {

    }
    /// Create an assembly from another
    fn derive() -> Self;
    /// Create an assembly
    fn create() -> Self;
    /// Destroy an assembly
    fn destroy();
    /// Create an assembly from resources outside the spaceport i.e.,
    /// documents
    fn import() -> Self;
    /// Send an assembly to consumers outside the spaceport
    fn export(&self);
}