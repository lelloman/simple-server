//! Engine-backed child process execution.
use serde_json::json;
use std::{
    ffi::{OsStr, OsString},
    io,
    os::unix::{ffi::OsStrExt, process::ExitStatusExt},
    process::{ExitStatus, Output},
};

#[derive(Debug)]
pub struct Command {
    program: OsString,
    args: Vec<OsString>,
}
impl Command {
    pub fn new(program: impl AsRef<OsStr>) -> Self {
        Self {
            program: program.as_ref().to_owned(),
            args: Vec::new(),
        }
    }
    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|arg| arg.as_ref().to_owned()));
        self
    }
    pub async fn output(&mut self) -> io::Result<Output> {
        let args: Vec<_> = self.args.iter().map(|s| s.as_bytes()).collect();
        let (header, mut body) = crate::engine_wire::command(json!({"op":"process_output","program_bytes":self.program.as_bytes(),"args_bytes":args}), &[]).await?;
        if header["ok"] != true {
            return Err(if let Some(code) = header["os_error"].as_i64() {
                io::Error::from_raw_os_error(code as i32)
            } else {
                io::Error::other(
                    header["message"]
                        .as_str()
                        .unwrap_or("engine process failure")
                        .to_owned(),
                )
            });
        }
        let length = header["stdout_len"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .filter(|v| *v <= body.len())
            .ok_or_else(|| io::Error::other("invalid process output"))?;
        let status = header["status"]
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .ok_or_else(|| io::Error::other("invalid exit status"))?;
        let stderr = body.split_off(length);
        Ok(Output {
            status: ExitStatus::from_raw(status),
            stdout: body,
            stderr,
        })
    }
}

/// Child process with explicit stdio and kill-on-drop ownership.
/// Waiting uses engine timers and never occupies an executor thread with waitpid.
pub struct ManagedCommand {
    command: std::process::Command,
    kill_on_drop: bool,
    explicit_stdio: [bool; 3],
}
impl ManagedCommand {
    pub fn new(program: impl AsRef<OsStr>) -> Self {
        Self {
            command: std::process::Command::new(program),
            kill_on_drop: false,
            explicit_stdio: [false; 3],
        }
    }
    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.command.arg(arg);
        self
    }
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }
    pub fn stdin(&mut self, v: std::process::Stdio) -> &mut Self {
        self.explicit_stdio[0] = true;
        self.command.stdin(v);
        self
    }
    pub fn stdout(&mut self, v: std::process::Stdio) -> &mut Self {
        self.explicit_stdio[1] = true;
        self.command.stdout(v);
        self
    }
    pub fn stderr(&mut self, v: std::process::Stdio) -> &mut Self {
        self.explicit_stdio[2] = true;
        self.command.stderr(v);
        self
    }
    pub fn kill_on_drop(&mut self, v: bool) -> &mut Self {
        self.kill_on_drop = v;
        self
    }
    /// Capture output, preserving explicit stdio and kill-on-drop behavior.
    #[cfg(feature = "engine-io")]
    pub async fn output(&mut self) -> io::Result<Output> {
        use crate::io::AsyncReadExt;
        if !self.explicit_stdio[0] {
            self.command.stdin(std::process::Stdio::null());
        }
        if !self.explicit_stdio[1] {
            self.command.stdout(std::process::Stdio::piped());
        }
        if !self.explicit_stdio[2] {
            self.command.stderr(std::process::Stdio::piped());
        }
        let child = self.spawn();
        // Preserve default inherited stdio for subsequent spawn/status calls.
        if !self.explicit_stdio[0] {
            self.command.stdin(std::process::Stdio::inherit());
        }
        if !self.explicit_stdio[1] {
            self.command.stdout(std::process::Stdio::inherit());
        }
        if !self.explicit_stdio[2] {
            self.command.stderr(std::process::Stdio::inherit());
        }
        let mut child = child?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        async fn read(pipe: Option<crate::fs::File>) -> io::Result<Vec<u8>> {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                pipe.read_to_end(&mut bytes).await?;
            }
            Ok(bytes)
        }
        let (status, stdout, stderr) =
            futures_util::future::try_join3(child.wait(), read(stdout), read(stderr)).await?;
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
    /// Wait with the configured stdio (inherited by default).
    #[cfg(feature = "engine-io")]
    pub async fn status(&mut self) -> io::Result<ExitStatus> {
        self.spawn()?.wait().await
    }
    #[cfg(feature = "engine-io")]
    pub fn spawn(&mut self) -> io::Result<Child> {
        let runtime = crate::runtime::Runtime::try_current()?;
        let mut child = self.command.spawn()?;
        let stdout = child.stdout.take().map(|s| {
            let fd: std::os::fd::OwnedFd = s.into();
            crate::fs::File::from_std(fd.into())
        });
        let stderr = child.stderr.take().map(|s| {
            let fd: std::os::fd::OwnedFd = s.into();
            crate::fs::File::from_std(fd.into())
        });
        Ok(Child {
            child: Some(child),
            stdout,
            stderr,
            kill_on_drop: self.kill_on_drop,
            runtime,
        })
    }
}
#[cfg(feature = "engine-io")]
pub struct Child {
    runtime: crate::runtime::Runtime,
    child: Option<std::process::Child>,
    pub stdout: Option<crate::fs::File>,
    pub stderr: Option<crate::fs::File>,
    kill_on_drop: bool,
}
#[cfg(feature = "engine-io")]
impl Child {
    pub fn id(&self) -> u32 {
        self.child.as_ref().unwrap().id()
    }
    pub async fn wait(&mut self) -> io::Result<ExitStatus> {
        self.child.as_mut().unwrap().stdin.take();
        loop {
            if let Some(status) = self.child.as_mut().unwrap().try_wait()? {
                return Ok(status);
            }
            crate::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
}
#[cfg(feature = "engine-io")]
impl Drop for Child {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return;
            }
            if self.kill_on_drop {
                let _ = child.kill();
            }
            self.runtime.handle().spawn_blocking(move || {
                let _ = child.wait();
            });
        }
    }
}
