#![cfg(all(feature = "runtime", unix))]
use std::{io, time::Duration};
#[simple_server::test]
async fn child() {
    if std::env::var_os("ENGINE_HUP_CHILD").is_none() {
        return;
    }
    use simple_server::signal::{UnixSignal, UnixSignalKind};
    let mut signal = UnixSignal::install(UnixSignalKind::Hangup).unwrap();
    assert!(
        simple_server::time::timeout(Duration::from_millis(5), signal.recv())
            .await
            .is_err()
    );
    for _ in 0..2 {
        println!("hup-ready");
        signal.recv().await.unwrap();
        println!("hup-received");
    }
}
#[tokio::test]
async fn repeatable_hangup_survives_a_cancelled_wait() -> io::Result<()> {
    use tokio::{
        io::{AsyncBufReadExt, BufReader},
        process::Command,
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut child = Command::new(std::env::current_exe()?)
            .args(["--exact", "child", "--nocapture"])
            .env("ENGINE_HUP_CHILD", "1")
            .stdout(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut received = 0;
        while let Some(line) = lines.next_line().await? {
            if line.contains("hup-ready") {
                assert!(
                    Command::new("kill")
                        .args(["-HUP", &child.id().unwrap().to_string()])
                        .status()
                        .await?
                        .success()
                );
            }
            if line.contains("hup-received") {
                received += 1;
            }
        }
        assert!(child.wait().await?.success());
        assert_eq!(received, 2);
        Ok::<_, io::Error>(())
    })
    .await
    .expect("signal test deadline")
}
