#![cfg(unix)]
mod common;
#[path = "../src/tui/terminal.rs"]
mod terminal;

use common::{Fixture, PARAMETER};
use std::{
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::CommandExt,
    },
    process::{Command, Stdio},
    time::{Duration, Instant},
};

struct Pty {
    master: File,
    slave: File,
    original: libc::termios,
}

fn attributes(file: &File) -> libc::termios {
    let mut value = std::mem::MaybeUninit::uninit();
    // SAFETY: tcgetattr writes one termios to valid storage for an open PTY.
    assert_eq!(
        unsafe { libc::tcgetattr(file.as_raw_fd(), value.as_mut_ptr()) },
        0
    );
    unsafe { value.assume_init() }
}

impl Pty {
    fn new() -> Self {
        let mut master = -1;
        let mut slave = -1;
        let size = libc::winsize {
            ws_row: 30,
            ws_col: 120,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // SAFETY: output pointers and size are valid, null requests default name/termios.
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    &size,
                )
            },
            0
        );
        let master = unsafe { File::from_raw_fd(master) };
        let slave = unsafe { File::from_raw_fd(slave) };
        // SAFETY: master is an open file descriptor; enable nonblocking reads for a bounded test.
        assert_ne!(
            unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) },
            -1
        );
        let original = attributes(&slave);
        Self {
            master,
            slave,
            original,
        }
    }

    fn run(
        &mut self,
        mut command: Command,
        input: &[u8],
        ready: &str,
    ) -> (std::process::ExitStatus, String) {
        command
            .stdin(Stdio::from(self.slave.try_clone().unwrap()))
            .stdout(Stdio::from(self.slave.try_clone().unwrap()))
            .stderr(Stdio::from(self.slave.try_clone().unwrap()))
            .env("TERM", "xterm-256color");
        // SAFETY: only async-signal-safe libc calls run after fork; fd 0 is the PTY slave.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 || libc::ioctl(0, libc::TIOCSCTTY, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut output = Vec::new();
        let mut sent = false;
        let status = loop {
            let mut buf = [0; 16384];
            while let Ok(count) = self.master.read(&mut buf) {
                if count == 0 {
                    break;
                }
                output.extend_from_slice(&buf[..count]);
            }
            if !sent && String::from_utf8_lossy(&output).contains(ready) {
                self.master.write_all(input).unwrap();
                sent = true;
            }
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "terminal child timed out: {}",
                    String::from_utf8_lossy(&output)
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let mut buf = [0; 16384];
        while let Ok(count) = self.master.read(&mut buf) {
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buf[..count]);
        }
        let restored = attributes(&self.slave);
        assert_eq!(
            restored.c_lflag, self.original.c_lflag,
            "local terminal flags"
        );
        assert_eq!(
            restored.c_iflag, self.original.c_iflag,
            "input terminal flags"
        );
        assert_eq!(
            restored.c_oflag, self.original.c_oflag,
            "output terminal flags"
        );
        assert_eq!(
            restored.c_cflag, self.original.c_cflag,
            "control terminal flags"
        );
        assert_eq!(
            restored.c_cc, self.original.c_cc,
            "terminal control characters"
        );
        (status, String::from_utf8_lossy(&output).into_owned())
    }
}

#[test]
fn bare_browser_quits_and_restores_terminal() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    for input in [b"q".as_slice(), b"\x03".as_slice()] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mibl"));
        command.env("MIB_DIR", dir.path());
        let (status, output) = Pty::new().run(command, input, "definitions");
        assert!(status.success(), "{output}");
        assert!(
            output.contains("\x1b[?1049h") && output.contains("\x1b[?1049l"),
            "{output}"
        );
    }
}

#[test]
fn load_failure_never_enters_alternate_screen() {
    let dir = Fixture::new();
    let mut command = Command::new(env!("CARGO_BIN_EXE_mibl"));
    command.env("MIB_DIR", dir.path());
    let (status, output) = Pty::new().run(command, b"", "");
    assert_eq!(status.code(), Some(2));
    assert!(output.contains("no usable supported rows"), "{output}");
    assert!(!output.contains("\x1b[?1049h"));
}

#[test]
fn terminal_restores_after_returned_error_and_panic() {
    for failure in ["error", "panic", "init"] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "terminal_failure_child", "--nocapture"])
            .env("MIBL_TEST_TERMINAL_FAILURE", failure);
        let (status, output) = Pty::new().run(command, b"", "");
        assert!(!status.success(), "{output}");
        if failure != "init" {
            assert!(
                output.contains("\x1b[?1049h") && output.contains("\x1b[?1049l"),
                "{output}"
            );
        }
    }
}

#[test]
fn terminal_failure_child() {
    let Ok(failure) = std::env::var("MIBL_TEST_TERMINAL_FAILURE") else {
        return;
    };
    if failure == "init" {
        // SAFETY: this disposable child closes its own stdout to fail after enabling raw mode.
        assert_eq!(unsafe { libc::close(1) }, 0);
    }
    terminal::with_terminal(|_| -> std::io::Result<()> {
        if failure == "panic" {
            panic!("synthetic terminal panic");
        }
        Err(std::io::Error::other("synthetic terminal failure"))
    })
    .unwrap();
}
