use ulid::Ulid;

pub struct System {
    id: Ulid,
    name: Option<Name>,
}

pub enum Name {
    Star(String), // claimed system
    Fleet(String), // pervasive system
    Alias(String), // free system
}

pub enum Host {
    CapitalShip, // mobile devices
    Ship, // mobile embedded devices
    Planet, // system-bound stationary devices
    Asteroid, // specialized devices with certain resources
    Comet, // system-bound embedded devices
}