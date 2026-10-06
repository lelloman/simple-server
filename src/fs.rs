//! Filesystem I/O scheduled on the engine's blocking workers.
//! File operations serialize through one handle; pending reads retain unread bytes.
use crate::runtime::{JoinHandle, spawn_blocking};
use futures_util::io::{AsyncRead, AsyncSeek, AsyncWrite};
use std::{
    future::Future,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, ready},
};
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
    spawn_blocking(f)
        .await
        .map_err(|e| io::Error::other(e.to_string()))?
}
pub struct File {
    inner: Arc<Mutex<std::fs::File>>,
    read: Option<JoinHandle<io::Result<Vec<u8>>>>,
    buffered: std::io::Cursor<Vec<u8>>,
    write: Option<JoinHandle<io::Result<usize>>>,
    seek: Option<JoinHandle<io::Result<u64>>>,
}
impl File {
    pub fn from_std(f: std::fs::File) -> Self {
        Self {
            inner: Arc::new(Mutex::new(f)),
            read: None,
            buffered: std::io::Cursor::new(Vec::new()),
            write: None,
            seek: None,
        }
    }
    pub async fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let p = path.as_ref().to_owned();
        blocking(move || std::fs::File::open(p).map(Self::from_std)).await
    }
    pub async fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        let p = path.as_ref().to_owned();
        blocking(move || std::fs::File::create(p).map(Self::from_std)).await
    }
    pub async fn sync_all(&mut self) -> io::Result<()> {
        futures_util::io::AsyncWriteExt::flush(self).await?;
        let f = self.inner.clone();
        blocking(move || f.lock().unwrap().sync_all()).await
    }
    pub async fn metadata(&self) -> io::Result<std::fs::Metadata> {
        let f = self.inner.clone();
        blocking(move || f.lock().unwrap().metadata()).await
    }
    pub async fn set_permissions(&self, p: std::fs::Permissions) -> io::Result<()> {
        let f = self.inner.clone();
        blocking(move || f.lock().unwrap().set_permissions(p)).await
    }
    pub async fn set_len(&self, len: u64) -> io::Result<()> {
        let f = self.inner.clone();
        blocking(move || f.lock().unwrap().set_len(len)).await
    }
}
impl AsyncRead for File {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        ready!(self.as_mut().poll_flush(cx))?;
        if self.seek.is_some() {
            ready!(self.as_mut().poll_seek(cx, SeekFrom::Current(0)))?;
        }
        loop {
            let n = self.buffered.read(buf)?;
            if n > 0 {
                return Poll::Ready(Ok(n));
            }
            if self.read.is_none() {
                let f = self.inner.clone();
                let n = buf.len().min(65536);
                self.read = Some(spawn_blocking(move || {
                    let mut b = vec![0; n];
                    let n = f.lock().unwrap().read(&mut b)?;
                    b.truncate(n);
                    Ok(b)
                }));
            }
            let result = ready!(Pin::new(self.read.as_mut().unwrap()).poll(cx));
            self.read = None;
            let b = result.map_err(|e| io::Error::other(e.to_string()))??;
            if b.is_empty() {
                return Poll::Ready(Ok(0));
            }
            self.buffered = std::io::Cursor::new(b);
        }
    }
}
impl AsyncWrite for File {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.write.is_none()
            && (self.read.is_some()
                || self.seek.is_some()
                || self.buffered.position() < self.buffered.get_ref().len() as u64)
        {
            ready!(self.as_mut().poll_seek(cx, SeekFrom::Current(0)))?;
        }
        if self.write.is_none() {
            let f = self.inner.clone();
            let b = buf[..buf.len().min(65536)].to_vec();
            self.write = Some(spawn_blocking(move || f.lock().unwrap().write(&b)));
        }
        let r = ready!(Pin::new(self.write.as_mut().unwrap()).poll(cx));
        self.write = None;
        Poll::Ready(r.map_err(|e| io::Error::other(e.to_string()))?)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if let Some(f) = &mut self.write {
            ready!(Pin::new(f).poll(cx)).map_err(|e| io::Error::other(e.to_string()))??;
            self.write = None;
        }
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
impl AsyncSeek for File {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<io::Result<u64>> {
        ready!(self.as_mut().poll_flush(cx))?;
        if let Some(read) = &mut self.read {
            let b =
                ready!(Pin::new(read).poll(cx)).map_err(|e| io::Error::other(e.to_string()))??;
            self.read = None;
            self.buffered = std::io::Cursor::new(b);
        }
        if self.seek.is_none() {
            let remaining = self.buffered.get_ref().len() as i64 - self.buffered.position() as i64;
            let pos = match pos {
                SeekFrom::Current(n) => SeekFrom::Current(
                    n.checked_sub(remaining)
                        .ok_or_else(|| io::Error::other("seek overflow"))?,
                ),
                p => p,
            };
            self.buffered = std::io::Cursor::new(Vec::new());
            let f = self.inner.clone();
            self.seek = Some(spawn_blocking(move || f.lock().unwrap().seek(pos)));
        }
        let r = ready!(Pin::new(self.seek.as_mut().unwrap()).poll(cx));
        self.seek = None;
        Poll::Ready(r.map_err(|e| io::Error::other(e.to_string()))?)
    }
}
#[derive(Clone)]
pub struct OpenOptions(std::fs::OpenOptions);
impl OpenOptions {
    pub fn new() -> Self {
        Self(std::fs::OpenOptions::new())
    }
    #[cfg(unix)]
    pub fn mode(&mut self, v: u32) -> &mut Self {
        use std::os::unix::fs::OpenOptionsExt;
        self.0.mode(v);
        self
    }
    pub fn read(&mut self, v: bool) -> &mut Self {
        self.0.read(v);
        self
    }
    pub fn write(&mut self, v: bool) -> &mut Self {
        self.0.write(v);
        self
    }
    pub fn create(&mut self, v: bool) -> &mut Self {
        self.0.create(v);
        self
    }
    pub fn create_new(&mut self, v: bool) -> &mut Self {
        self.0.create_new(v);
        self
    }
    pub fn truncate(&mut self, v: bool) -> &mut Self {
        self.0.truncate(v);
        self
    }
    pub async fn open(&self, p: impl AsRef<Path>) -> io::Result<File> {
        let p = p.as_ref().to_owned();
        let o = self.0.clone();
        blocking(move || o.open(p).map(File::from_std)).await
    }
}
macro_rules! one {($($name:ident->$ty:ty),*)=>{$(pub async fn $name(path:impl AsRef<Path>)->io::Result<$ty>{let p=path.as_ref().to_owned();blocking(move||std::fs::$name(p)).await})*};}
one!(create_dir->(),create_dir_all->(),remove_dir_all->(),remove_file->(),metadata->std::fs::Metadata,symlink_metadata->std::fs::Metadata,canonicalize->PathBuf,read->Vec<u8>);
macro_rules! two {($($name:ident->$ty:ty),*)=>{$(pub async fn $name(a:impl AsRef<Path>,b:impl AsRef<Path>)->io::Result<$ty>{let a=a.as_ref().to_owned();let b=b.as_ref().to_owned();blocking(move||std::fs::$name(a,b)).await})*};}
two!(rename->(),hard_link->(),copy->u64);
pub async fn set_permissions(p: impl AsRef<Path>, v: std::fs::Permissions) -> io::Result<()> {
    let p = p.as_ref().to_owned();
    blocking(move || std::fs::set_permissions(p, v)).await
}
pub async fn write(p: impl AsRef<Path>, v: impl AsRef<[u8]>) -> io::Result<()> {
    let p = p.as_ref().to_owned();
    let v = v.as_ref().to_vec();
    blocking(move || std::fs::write(p, v)).await
}
pub struct ReadDir(Arc<Mutex<std::fs::ReadDir>>);
pub struct DirEntry(std::fs::DirEntry);
impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.0.path()
    }
    pub fn file_name(&self) -> std::ffi::OsString {
        self.0.file_name()
    }
    pub async fn file_type(&self) -> io::Result<std::fs::FileType> {
        let p = self.path();
        blocking(move || std::fs::symlink_metadata(p).map(|m| m.file_type())).await
    }
}
pub async fn read_dir(p: impl AsRef<Path>) -> io::Result<ReadDir> {
    let p = p.as_ref().to_owned();
    blocking(move || std::fs::read_dir(p).map(|r| ReadDir(Arc::new(Mutex::new(r))))).await
}
impl ReadDir {
    pub async fn next_entry(&mut self) -> io::Result<Option<DirEntry>> {
        let r = self.0.clone();
        blocking(move || {
            r.lock()
                .unwrap()
                .next()
                .transpose()
                .map(|e| e.map(DirEntry))
        })
        .await
    }
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self::new()
    }
}
