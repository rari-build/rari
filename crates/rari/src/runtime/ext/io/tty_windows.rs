// Copyright 2018-2025 the Deno authors. MIT license.

use deno_core::OpState;
use deno_core::op2;
use deno_core::parking_lot::Mutex;
use deno_error::JsErrorBox;
use deno_error::JsErrorClass;
use deno_error::builtin_classes::GENERIC_ERROR;
use deno_io::WinTtyState;
use rustyline::Cmd;
use rustyline::Editor;
use rustyline::KeyCode;
use rustyline::KeyEvent;
use rustyline::Modifiers;
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use std::io::Error;
use std::sync::Arc;
use winapi::shared::minwindef::DWORD;
use winapi::shared::minwindef::FALSE;
use winapi::um::consoleapi;
use winapi::um::wincon;

deno_core::extension!(deno_tty, ops = [op_set_raw, op_console_size, op_read_line_prompt],);

#[derive(Debug, deno_error::JsError)]
pub enum TtyError {
    #[class(inherit)]
    Resource(deno_core::error::ResourceError),
    #[class(inherit)]
    Io(Error),
    #[class(inherit)]
    Other(JsErrorBox),
}

impl std::fmt::Display for TtyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TtyError::Resource(err) => write!(f, "{}", err),
            TtyError::Io(err) => write!(f, "{}", err),
            TtyError::Other(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for TtyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TtyError::Resource(err) => Some(err),
            TtyError::Io(err) => Some(err),
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

// ref: <https://learn.microsoft.com/en-us/windows/console/setconsolemode>
const COOKED_MODE: DWORD =
    // enable line-by-line input (returns input only after CR is read)
    wincon::ENABLE_LINE_INPUT
  // enables real-time character echo to console display (requires ENABLE_LINE_INPUT)
  | wincon::ENABLE_ECHO_INPUT
  // system handles CTRL-C (with ENABLE_LINE_INPUT, also handles BS, CR, and LF) and other control keys (when using `ReadFile` or `ReadConsole`)
  | wincon::ENABLE_PROCESSED_INPUT;

fn mode_raw_input_on(original_mode: DWORD) -> DWORD {
    original_mode & !COOKED_MODE | wincon::ENABLE_VIRTUAL_TERMINAL_INPUT
}

fn mode_raw_input_off(original_mode: DWORD) -> DWORD {
    original_mode & !wincon::ENABLE_VIRTUAL_TERMINAL_INPUT | COOKED_MODE
}

#[op2(fast)]
fn op_set_raw(state: &mut OpState, rid: u32, is_raw: bool, cbreak: bool) -> Result<(), TtyError> {
    let handle_or_fd = state.resource_table.get_fd(rid)?;

    // From https://github.com/kkawakam/rustyline/blob/master/src/tty/windows.rs
    // and https://github.com/kkawakam/rustyline/blob/master/src/tty/unix.rs
    // and https://github.com/crossterm-rs/crossterm/blob/e35d4d2c1cc4c919e36d242e014af75f6127ab50/src/terminal/sys/windows.rs
    // Copyright (c) 2015 Katsu Kawakami & Rustyline authors. MIT license.
    // Copyright (c) 2019 Timon. MIT license.

    let handle = handle_or_fd;

    if cbreak {
        return Err(TtyError::Other(JsErrorBox::not_supported()));
    }

    let mut original_mode: DWORD = 0;
    // SAFETY: winapi call
    if unsafe { consoleapi::GetConsoleMode(handle, &mut original_mode) } == FALSE {
        return Err(TtyError::Io(Error::last_os_error()));
    }

    let new_mode =
        if is_raw { mode_raw_input_on(original_mode) } else { mode_raw_input_off(original_mode) };

    let stdin_state = state.borrow::<Arc<Mutex<WinTtyState>>>();
    let mut stdin_state = stdin_state.lock();

    if stdin_state.reading {
        let cvar = stdin_state.cvar.clone();

        /* Trick to unblock an ongoing line-buffered read operation if not already pending.
        See https://github.com/libuv/libuv/pull/866 for prior art */
        if original_mode & COOKED_MODE != 0 && !stdin_state.cancelled {
            // SAFETY: Write enter key event to force the console wait to return.
            let record = unsafe {
                let mut record: wincon::INPUT_RECORD = std::mem::zeroed();
                record.EventType = wincon::KEY_EVENT;
                record.Event.KeyEvent_mut().wVirtualKeyCode = winapi::um::winuser::VK_RETURN as u16;
                record.Event.KeyEvent_mut().bKeyDown = 1;
                record.Event.KeyEvent_mut().wRepeatCount = 1;
                *record.Event.KeyEvent_mut().uChar.UnicodeChar_mut() = '\r' as u16;
                record.Event.KeyEvent_mut().dwControlKeyState = 0;
                record.Event.KeyEvent_mut().wVirtualScanCode = winapi::um::winuser::MapVirtualKeyW(
                    winapi::um::winuser::VK_RETURN as u32,
                    winapi::um::winuser::MAPVK_VK_TO_VSC,
                ) as u16;
                record
            };
            stdin_state.cancelled = true;

            // SAFETY: winapi call to open conout$ and save screen state.
            let active_screen_buffer = unsafe {
                /* Save screen state before sending the VK_RETURN event */
                let handle = winapi::um::fileapi::CreateFileW(
                    "conout$".encode_utf16().chain(Some(0)).collect::<Vec<_>>().as_ptr(),
                    winapi::um::winnt::GENERIC_READ | winapi::um::winnt::GENERIC_WRITE,
                    winapi::um::winnt::FILE_SHARE_READ | winapi::um::winnt::FILE_SHARE_WRITE,
                    std::ptr::null_mut(),
                    winapi::um::fileapi::OPEN_EXISTING,
                    0,
                    std::ptr::null_mut(),
                );

                let mut active_screen_buffer = std::mem::zeroed();
                winapi::um::wincon::GetConsoleScreenBufferInfo(handle, &mut active_screen_buffer);
                winapi::um::handleapi::CloseHandle(handle);
                active_screen_buffer
            };
            stdin_state.screen_buffer_info = Some(active_screen_buffer);

            // SAFETY: winapi call to write the VK_RETURN event.
            if unsafe { winapi::um::wincon::WriteConsoleInputW(handle, &record, 1, &mut 0) }
                == FALSE
            {
                return Err(TtyError::Io(Error::last_os_error()));
            }

            /* Wait for read thread to acknowledge the cancellation to ensure that nothing
            interferes with the screen state.
            NOTE: `wait_while` automatically unlocks stdin_state */
            cvar.wait_while(&mut stdin_state, |state: &mut WinTtyState| state.cancelled);
        }
    }

    // SAFETY: winapi call
    if unsafe { consoleapi::SetConsoleMode(handle, new_mode) } == FALSE {
        return Err(TtyError::Io(Error::last_os_error()));
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

fn console_size_from_fd(
    handle: std::os::windows::io::RawHandle,
) -> Result<ConsoleSize, std::io::Error> {
    // SAFETY: winapi calls
    unsafe {
        let mut bufinfo: winapi::um::wincon::CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();

        if winapi::um::wincon::GetConsoleScreenBufferInfo(handle, &mut bufinfo) == 0 {
            return Err(Error::last_os_error());
        }

        // calculate the size of the visible window
        // * use over/under-flow protections b/c MSDN docs only imply that srWindow components are all non-negative
        // * ref: <https://docs.microsoft.com/en-us/windows/console/console-screen-buffer-info-str> @@ <https://archive.is/sfjnm>
        let cols = std::cmp::max(
            i32::from(bufinfo.srWindow.Right) - i32::from(bufinfo.srWindow.Left) + 1,
            0,
        ) as u32;
        let rows = std::cmp::max(
            i32::from(bufinfo.srWindow.Bottom) - i32::from(bufinfo.srWindow.Top) + 1,
            0,
        ) as u32;

        Ok(ConsoleSize { cols, rows })
    }
}

deno_error::js_error_wrapper!(ReadlineError, JsReadlineError, |err| {
    match err {
        ReadlineError::Io(e) => e.get_class(),
        ReadlineError::Eof
        | ReadlineError::Interrupted
        | ReadlineError::WindowResized
        | ReadlineError::Decode(_)
        | ReadlineError::SystemError(_)
        | _ => GENERIC_ERROR.into(),
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
