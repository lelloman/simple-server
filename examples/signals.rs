//! Also used by the child-process signal integration test.
use std::io::{self, Write};

use simple_server::lifecycle::Signals;

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let signals = Signals::install()?;
    println!("ready");
    io::stdout().flush()?;
    println!("{:?}", signals.wait().await?);
    Ok(())
}
