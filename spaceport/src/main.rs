use nebula::spaceport;
use tokio;

#[tokio::main]
async fn main() {
    let spaceport = spaceport::Spaceport::new().await;

}