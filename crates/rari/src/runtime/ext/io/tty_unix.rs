// Copyright 2018-2025 the Deno authors. MIT license.

use deno_core::OpState;
use deno_core::ResourceId;
use deno_core::op2;
use deno_error::JsErrorBox;
use deno_error::JsErrorClass;
use deno_error::builtin_classes::GENERIC_ERROR;
use nix::sys::termios;
use rustc_hash::FxHashMap;
use rustyline::Cmd;
use rustyline::Editor;
use rustyline::KeyCode;
use rustyline::KeyEvent;
use rustyline::Modifiers;
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use std::cell::RefCell;
use std::io::Error;
use std::rc::Rc;

deno_core::extension!(deno_tty, ops = [op_set_raw, op_console_size, op_read_line_prompt],);

#[derive(Default, Clone)]
pub struct TtyModeStore(Rc<RefCell<FxHashMap<ResourceId, termios::Termios>>>);

impl TtyModeStore {
    pub fn get(&self, id: ResourceId) -> Option<termios::Termios> {
        self.0.borrow().get(&id).map(ToOwned::to_owned)
    }

    pub fn take(&self, id: ResourceId) -> Option<termios::Termios> {
        self.0.borrow_mut().remove(&id)
    }

    pub fn set(&self, id: ResourceId, mode: termios::Termios) {
        self.0.borrow_mut().insert(id, mode);
    }
}

use deno_process::JsNixError;

#[derive(Debug, deno_error::JsError)]
pub enum TtyError {
    #[class(inherit)]
    Resource(deno_core::error::ResourceError),
    #[class(inherit)]
    Io(Error),
    #[class(inherit)]
    Nix(JsNixError),
    #[class(inherit)]
    #[allow(dead_code)]
    Other(JsErrorBox),
}

impl std::fmt::Display for TtyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TtyError::Resource(err) => write!(f, "{err}"),
            TtyError::Io(err) => write!(f, "{err}"),
            TtyError::Nix(err) => write!(f, "{err}"),
            TtyError::Other(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for TtyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TtyError::Resource(err) => Some(err),
            TtyError::Io(err) => Some(err),
            TtyError::Nix(err) => Some(err),
            TtyError::Other(err) => Some(err),
        }
    }
}

impl From<deno_core::error::ResourceError> for TtyError {
    fn from(err: deno_core::error::ResourceError) -> Self {
        TtyError::Resource(err)
    }
}

impl From<Error> for TtyError {
    fn from(err: Error) -> Self {
        TtyError::Io(err)
    }
}

#[op2(fast)]
fn op_set_raw(state: &mut OpState, rid: u32, is_raw: bool, cbreak: bool) -> Result<(), TtyError> {
    let handle_or_fd = state.resource_table.get_fd(rid)?;

    // From https://github.com/kkawakam/rustyline/blob/master/src/tty/windows.rs
    // and https://github.com/kkawakam/rustyline/blob/master/src/tty/unix.rs
    // and https://github.com/crossterm-rs/crossterm/blob/e35d4d2c1cc4c919e36d242e014af75f6127ab50/src/terminal/sys/windows.rs
    // Copyright (c) 2015 Katsu Kawakami & Rustyline authors. MIT license.
    // Copyright (c) 2019 Timon. MIT license.

    fn prepare_stdio() {
        // SAFETY: Save current state of stdio and restore it when we exit.
        unsafe {
            use libc::atexit;
            use libc::tcgetattr;
            use libc::tcsetattr;
            use libc::termios;
            use std::sync::OnceLock;

            // Only save original state once.
            static ORIG_TERMIOS: OnceLock<Option<termios>> = OnceLock::new();
            ORIG_TERMIOS.get_or_init(|| {
                let mut termios = std::mem::zeroed::<termios>();
                if tcgetattr(libc::STDIN_FILENO, &mut termios) == 0 {
                    extern "C" fn reset_stdio() {
                        // SAFETY: Reset the stdio state.
                        unsafe {
                            if let Some(Some(termios)) = ORIG_TERMIOS.get() {
                                tcsetattr(libc::STDIN_FILENO, 0, termios)
                            } else {
                                -1
                            }
                        };
                    }

                    atexit(reset_stdio);
                    return Some(termios);
                }

                None
            });
        }
    }

    prepare_stdio();
    let tty_mode_store = state.borrow::<TtyModeStore>().clone();
    let previous_mode = tty_mode_store.get(rid);

    // SAFETY: Nix crate requires value to implement the AsFd trait
    let raw_fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(handle_or_fd) };

    if is_raw {
        let mut raw = match previous_mode {
            Some(mode) => mode,
            None => {
                // Save original mode.
                let original_mode =
                    termios::tcgetattr(raw_fd).map_err(|e| TtyError::Nix(JsNixError(e)))?;
                tty_mode_store.set(rid, original_mode.clone());
                original_mode
            }
        };

        raw.input_flags &= !(termios::InputFlags::BRKINT
            | termios::InputFlags::ICRNL
            | termios::InputFlags::INPCK
            | termios::InputFlags::ISTRIP
            | termios::InputFlags::IXON);

        raw.control_flags |= termios::ControlFlags::CS8;

        raw.local_flags &= !(termios::LocalFlags::ECHO
            | termios::LocalFlags::ICANON
            | termios::LocalFlags::IEXTEN);
        if !cbreak {
            raw.local_flags &= !(termios::LocalFlags::ISIG);
        }
        raw.control_chars[termios::SpecialCharacterIndices::VMIN as usize] = 1;
        raw.control_chars[termios::SpecialCharacterIndices::VTIME as usize] = 0;
        termios::tcsetattr(raw_fd, termios::SetArg::TCSADRAIN, &raw)
            .map_err(|e| TtyError::Nix(JsNixError(e)))?;
    } else {
        // Try restore saved mode.
        if let Some(mode) = tty_mode_store.take(rid) {
            termios::tcsetattr(raw_fd, termios::SetArg::TCSADRAIN, &mode)
                .map_err(|e| TtyError::Nix(JsNixError(e)))?;
        }
    }

    Ok(())
}

#[op2(fast)]
fn op_console_size(state: &mut OpState, #[buffer] result: &mut [u32]) -> Result<(), TtyError> {
    fn check_console_size(
        state: &mut OpState,
        result: &mut [u32],
        rid: u32,
    ) -> Result<(), TtyError> {
        let fd = state.resource_table.get_fd(rid)?;
        let size = console_size_from_fd(fd)?;
        result[0] = size.cols;
        result[1] = size.rows;
        Ok(())
    }

    let mut last_result = Ok(());
    // Since stdio might be piped we try to get the size of the console for all
    // of them and return the first one that succeeds.
    for rid in [0, 1, 2] {
        last_result = check_console_size(state, result, rid);
        if last_result.is_ok() {
            return last_result;
        }
    }

    last_result
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ConsoleSize {
    pub cols: u32,
    pub rows: u32,
}

#[allow(dead_code)]
pub fn console_size(std_file: &std::fs::File) -> Result<ConsoleSize, std::io::Error> {
    use std::os::unix::io::AsRawFd;
    let fd = std_file.as_raw_fd();
    console_size_from_fd(fd)
}

fn console_size_from_fd(fd: std::os::unix::prelude::RawFd) -> Result<ConsoleSize, std::io::Error> {
    // SAFETY: libc calls
    unsafe {
        let mut size: libc::winsize = std::mem::zeroed();
        if libc::ioctl(fd, libc::TIOCGWINSZ, &mut size as *mut _) != 0 {
            return Err(Error::last_os_error());
        }
        Ok(ConsoleSize { cols: size.ws_col as u32, rows: size.ws_row as u32 })
    }
}

deno_error::js_error_wrapper!(ReadlineError, JsReadlineError, |err| {
    match err {
        ReadlineError::Io(e) => e.get_class(),
        ReadlineError::Eof => GENERIC_ERROR.into(),
        ReadlineError::Interrupted => GENERIC_ERROR.into(),
        ReadlineError::Errno(e) => JsNixError(*e).get_class(),
        ReadlineError::WindowResized => GENERIC_ERROR.into(),
        _ => GENERIC_ERROR.into(),
    }
});

#[op2]
#[string]
pub fn op_read_line_prompt(
    #[string] prompt_text: &str,
    #[string] default_value: &str,
) -> Result<Option<String>, JsReadlineError> {
    let mut editor =
        Editor::<(), rustyline::history::DefaultHistory>::new().expect("Failed to create editor.");

    editor.set_keyseq_timeout(1);
    editor.bind_sequence(KeyEvent(KeyCode::Esc, Modifiers::empty()), Cmd::Interrupt);

    let read_result = editor.readline_with_initial(prompt_text, (default_value, ""));
    match read_result {
        Ok(line) => Ok(Some(line)),
        Err(ReadlineError::Interrupted) => {
            // SAFETY: Disable raw mode and raise SIGINT.
            unsafe {
                libc::raise(libc::SIGINT);
            }
            Ok(None)
        }
        Err(ReadlineError::Eof) => Ok(None),
        Err(err) => Err(JsReadlineError(err)),
    }
}
