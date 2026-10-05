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
