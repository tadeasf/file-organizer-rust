mod cli;
mod modules;
mod utils;

use anyhow::Result;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::new()?;
    cli.run().await?;
    Ok(())
}
