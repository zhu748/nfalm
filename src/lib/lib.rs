use rquest::{Client, ClientBuilder};
use rquest_util::Emulation;
use std::sync::LazyLock;

pub mod api;
pub mod completion;
pub mod config;
pub mod superfetch;
pub mod utils;

pub static NORMAL_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    ClientBuilder::new()
        .build()
        .expect("Failed to create client")
});

pub static SUPER_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    ClientBuilder::new()
        .emulation(Emulation::Chrome134)
        .build()
        .expect("Failed to create client")
});
