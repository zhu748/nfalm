use anyhow::Result;
use clewdr::{self, config::Config, utils::BANNER};

fn main() -> Result<()> {
    let config = Config::load()?;
    println!("{}", *BANNER);
    Ok(())
}
