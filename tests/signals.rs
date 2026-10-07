#![cfg(all(feature = "lifecycle", unix))]

use std::{io, time::Duration};

use simple_server::lifecycle::Signals;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

#[tokio::test]
async fn child_signal_waiter() {
    if std::env::var_os("SIMPLE_SERVER_SIGNAL_CHILD").is_none() {
        return;
    }
    let signals = Signals::install().unwrap();
    println!("signals-ready");
    println!("received-{:?}", signals.wait().await.unwrap());
}

#[tokio::test]
async fn sigint_and_sigterm_are_received_in_child_processes() -> io::Result<()> {
    for (argument, expected) in [("-INT", "Interrupt"), ("-TERM", "Terminate")] {
        tokio::time::timeout(Duration::from_secs(10), async {
            let mut child = Command::new(std::env::current_exe()?)
                .args(["--exact", "child_signal_waiter", "--nocapture"])
                .env("SIMPLE_SERVER_SIGNAL_CHILD", "1")
                .stdout(std::process::Stdio::piped())
                .kill_on_drop(true)
                .spawn()?;
            let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
            loop {
                let line = lines
                    .next_line()
                    .await?
                    .expect("child exited before registration");
                if line.contains("signals-ready") {
                    break;
                }
            }
            assert!(
                Command::new("kill")
                    .args([argument, &child.id().unwrap().to_string()])
                    .status()
                    .await?
                    .success()
            );
            let mut output = String::new();
            while let Some(line) = lines.next_line().await? {
                output.push_str(&line);
            }
            assert!(child.wait().await?.success(), "{output}");
            assert!(
                output.contains(&format!("received-Signal({expected})")),
                "{output}"
            );
            Ok::<_, io::Error>(())
        })
        .await
        .expect("signal child timed out")?;
    }
    Ok(())
}
