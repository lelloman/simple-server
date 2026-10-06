#![cfg(feature = "engine-io")]
use simple_server::{
    fs,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
};
#[simple_server::test]
async fn files_preserve_position_and_durable_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data");
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .await
        .unwrap();
    file.write_all(b"abcdef").await.unwrap();
    file.sync_all().await.unwrap();
    file.seek(std::io::SeekFrom::Start(0)).await.unwrap();
    let mut first = [0; 2];
    file.read_exact(&mut first).await.unwrap();
    assert_eq!(&first, b"ab");
    assert_eq!(file.seek(std::io::SeekFrom::Current(0)).await.unwrap(), 2);
    let mut rest = Vec::new();
    file.read_to_end(&mut rest).await.unwrap();
    assert_eq!(&rest, b"cdef");
    assert_eq!(std::fs::read(path).unwrap(), b"abcdef");
}
#[cfg(feature = "process")]
#[simple_server::test]
async fn dropping_managed_child_terminates_and_reaps_it() {
    let mut child = simple_server::process::ManagedCommand::new("/bin/sh")
        .args(["-c", "printf ready; exec sleep 30"])
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut ready = [0; 5];
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_exact(&mut ready)
        .await
        .unwrap();
    assert_eq!(&ready, b"ready");
    assert!(
        simple_server::time::timeout(std::time::Duration::from_millis(20), child.wait())
            .await
            .is_err()
    );
    let pid = child.id().to_string();
    drop(child);
    simple_server::time::timeout(std::time::Duration::from_secs(2), async {
        while std::process::Command::new("kill")
            .args(["-0", &pid])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
        {
            simple_server::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("child must terminate and be reaped");
}

#[cfg(unix)]
#[simple_server::test]
async fn cancelled_pipe_read_keeps_bytes_for_the_next_reader() {
    use std::io::Write;
    let (reader, mut writer) = std::os::unix::net::UnixStream::pair().unwrap();
    let fd: std::os::fd::OwnedFd = reader.into();
    let mut reader = fs::File::from_std(fd.into());
    let mut buffer = [0; 6];
    let mut read = Box::pin(reader.read(&mut buffer));
    assert!(futures_util::poll!(read.as_mut()).is_pending());
    drop(read);
    writer.write_all(b"abcdef").unwrap();
    drop(writer);
    let mut first = [0; 2];
    reader.read_exact(&mut first).await.unwrap();
    assert_eq!(&first, b"ab");
    let mut rest = Vec::new();
    reader.read_to_end(&mut rest).await.unwrap();
    assert_eq!(&rest, b"cdef");
}
