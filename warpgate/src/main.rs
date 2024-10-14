use nebula::spaceport;
use tokio;

#[tokio::main]
async fn main() {
    // open a spaceport --creating one if it does not yet exist.
    // needs commander ID and spaceport name.
    // ideally, there is a global record of commanders and their spaceports
    // however, this would make it a partially centralized system.
    // To make this work, we need an always-on device that is preferrably with you
    // i.e., a mobile phone.  This device will be the commander's personal ship.
    // This ship holds the commander's id data so that he doesn't have to remember all of it.
    // This ship will help bootstrap spaceports on other devices.
    // Eventually all devices can help a commander bootstrap new spaceports.

    // If not set up on a phone or any other devices, user may register as a commander
    // and create a new spaceport.

    // Running a query on the network: nebula/commander
    // Running a query on the network: nebula/*/spaceport

    // A spaceport provides services (data & communication) for docked service ships.
    // Docked service ships are applications that rely on a spaceport's services to operate.

    // Therefore, there is the distinction between Spaceport::open() and Spaceport::enter().
    // If a spaceport is closed, service ships must undock.  They can travel to another spaceport (their
    // state data can be transferred to resume where left off).
    // To enable this, there has to be a daemon on the device which is the essence of these service ships.
    // Registering ships and saving their state info so they can travel to other spaceports.
    // To the user, it will like like their running application just appeared on another computer.
    // If not needed at any spaceport, service ships will stay by the warpgate ready to go.

    let spaceport = spaceport::Spaceport::new().await;

}

// TODO: browser extension to trace tab history as well as act like singlefile web archiver.
// Moving the Internet to nebula piece by piece; building a distributed and decentralized Internet.