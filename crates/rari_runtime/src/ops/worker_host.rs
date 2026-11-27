// Copyright 2018-2026 the Deno authors. MIT license.

use std::{
    cell::RefCell,
    collections::HashMap,
    future::poll_fn,
    rc::Rc,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicU32, Ordering},
    },
};

use deno_core::{
    CancelFuture, CancelHandle, DetachedBuffer, OpState, RcRef, Resource, ResourceId, op2,
};
use deno_error::JsErrorBox;
use deno_web::{
    JsMessageData, MessagePortError, Transferable, deserialize_js_transferables,
    serialize_transferables,
};
use parking_lot::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

// ============================================================
// Cross-thread messaging registry for `worker_threads.postMessageToThread`
// ============================================================
//
// Each Node-style worker thread (and the main thread) registers itself in
// this process-wide table so that any other thread can address it by id.
// The table holds the sender half of an mpsc channel plus a count of the
// destination thread's `workerMessage` event listeners; the receiver half
// lives in the thread's own resource table.

type ThreadMessage = (DetachedBuffer, Vec<Transferable>);

struct ThreadRegistryEntry {
    sender: UnboundedSender<ThreadMessage>,
    listener_count: AtomicU32,
}

static THREAD_REGISTRY: LazyLock<Mutex<HashMap<u32, Arc<ThreadRegistryEntry>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct ThreadMessageReceiver {
    thread_id: u32,
    rx: RefCell<UnboundedReceiver<ThreadMessage>>,
    cancel: CancelHandle,
}

impl Resource for ThreadMessageReceiver {
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "threadMessageReceiver".into()
    }

    fn close(self: Rc<Self>) {
        self.cancel.cancel();
    }
}

impl Drop for ThreadMessageReceiver {
    fn drop(&mut self) {
        // Worker ids never repeat (they come from a monotonic counter and
        // main is always 0), and the JS side guards against double-register,
        // so the only entry for our thread_id is the one we installed.
        THREAD_REGISTRY.lock().remove(&self.thread_id);
    }
}

deno_core::extension!(
    deno_worker_host,
    ops = [
        op_create_worker,
        op_current_thread_cpu_usage,
        op_node_worker_thread_register,
        op_node_worker_thread_set_listener_count,
        op_node_worker_thread_post_message,
        op_node_worker_thread_recv_message,
    ],
);

/// rari doesn't run workers, so the `op_host_*` ops that act on one aren't forked.
#[op2(nofast)]
#[smi]
fn op_create_worker() -> Result<u32, JsErrorBox> {
    Err(JsErrorBox::generic("Worker threads are not supported in rari"))
}

// ============================================================
// Cross-thread messaging ops (Node `worker_threads.postMessageToThread`)
// ============================================================

/// Register the current Node thread (main or worker) in the process-wide
/// thread registry so that other threads can `postMessageToThread` to it.
/// Returns the resource id of the receive-side channel, which the caller
/// is expected to keep alive for the lifetime of the thread and to poll
/// via `op_node_worker_thread_recv_message`.
#[op2(fast)]
#[smi]
fn op_node_worker_thread_register(state: &mut OpState, thread_id: u32) -> ResourceId {
    let (tx, rx) = unbounded_channel::<ThreadMessage>();
    let entry = Arc::new(ThreadRegistryEntry { sender: tx, listener_count: AtomicU32::new(0) });
    // Replace any pre-existing entry for this id (e.g. if a worker thread
    // is recycled in tests).
    THREAD_REGISTRY.lock().insert(thread_id, entry);
    state.resource_table.add(ThreadMessageReceiver {
        thread_id,
        rx: RefCell::new(rx),
        cancel: CancelHandle::default(),
    })
}

/// Update the destination's `workerMessage` listener count. Posts to a
/// thread with zero listeners synchronously fail with
/// `ERR_WORKER_MESSAGING_FAILED`, matching Node.js semantics.
#[op2(fast)]
fn op_node_worker_thread_set_listener_count(thread_id: u32, count: u32) {
    let registry = THREAD_REGISTRY.lock();
    if let Some(entry) = registry.get(&thread_id) {
        entry.listener_count.store(count, Ordering::SeqCst);
    }
}

/// Post a structured-clone payload to the thread identified by
/// `target_thread_id`.
///
/// Return value (kept as a small int for cheap FFI; mapped on the JS side):
///   0 — no thread with that id is currently registered.
///   1 — the destination thread has no `workerMessage` listeners.
///   2 — the message was enqueued for delivery.
///
/// If `force` is true the listener-count gate is skipped, which is used
/// for internal ack messages — the sender is, by construction, awaiting
/// the reply even though it has no public listener of its own.
#[op2]
fn op_node_worker_thread_post_message(
    state: &mut OpState,
    target_thread_id: u32,
    #[serde] data: JsMessageData,
    force: bool,
) -> Result<u8, MessagePortError> {
    let Some(entry) = THREAD_REGISTRY.lock().get(&target_thread_id).map(Arc::clone) else {
        return Ok(0);
    };
    if !force && entry.listener_count.load(Ordering::SeqCst) == 0 {
        return Ok(1);
    }
    let transferables = if data.transferables.is_empty() {
        vec![]
    } else {
        deserialize_js_transferables(state, data.transferables)?
    };
    if entry.sender.send((data.data, transferables)).is_err() {
        // Receiver was dropped between the registry lookup and the send.
        return Ok(0);
    }
    Ok(2)
}

/// Receive the next cross-thread message addressed to this thread.
/// Resolves to `None` when the channel is closed (i.e. the thread is
/// being torn down), at which point the JS-side poll loop terminates.
#[op2]
async fn op_node_worker_thread_recv_message(
    state: Rc<RefCell<OpState>>,
    #[smi] rid: ResourceId,
) -> Result<Option<JsMessageData>, MessagePortError> {
    let resource = {
        let state = state.borrow();
        match state.resource_table.get::<ThreadMessageReceiver>(rid) {
            Ok(r) => r,
            Err(_) => return Ok(None),
        }
    };
    let cancel = RcRef::map(Rc::clone(&resource), |r| &r.cancel);
    let recv = poll_fn(|cx| {
        let mut rx = resource.rx.borrow_mut();
        rx.poll_recv(cx)
    });
    let Ok(Some((data, transferables))) = recv.or_cancel(cancel).await else {
        return Ok(None);
    };
    let js_transferables = if transferables.is_empty() {
        vec![]
    } else {
        serialize_transferables(&mut state.borrow_mut(), transferables)
    };
    Ok(Some(JsMessageData { data, transferables: js_transferables }))
}

#[op2(fast)]
fn op_current_thread_cpu_usage(#[buffer] out: &mut [f64]) {
    if out.len() < 2 {
        return;
    }

    let handle = capture_current_thread_handle();
    let (user, system) = get_thread_cpu_usage_by_handle(handle);
    out[0] = user;
    out[1] = system;
}

#[cfg(target_os = "macos")]
fn capture_current_thread_handle() -> u64 {
    // SAFETY: FFI call to get the current thread's Mach port.
    unsafe { u64::from(mach_thread_self()) }
}

#[cfg(target_os = "macos")]
fn get_thread_cpu_usage_by_handle(handle: u64) -> (f64, f64) {
    let thread_port = handle as u32;
    // SAFETY: thread_info() will initialize this
    let mut info: ThreadBasicInfo = unsafe { std::mem::zeroed() };
    let mut count: u32 = THREAD_BASIC_INFO_COUNT;

    // SAFETY: FFI call to query thread CPU times
    let kr = unsafe {
        thread_info(thread_port, THREAD_BASIC_INFO, (&raw mut info).cast::<i32>(), &raw mut count)
    };

    // `mach_thread_self()` hands out a new send right on every call; release
    // it whether or not the query succeeded so repeated calls don't leak.
    // SAFETY: `thread_port` is a send right owned by this task.
    unsafe { mach_port_deallocate(mach_task_self_, thread_port) };

    if kr != 0 {
        return (0.0, 0.0);
    }

    let user = f64::from(info.user_time_seconds) * 1e6 + f64::from(info.user_time_microseconds);
    let system =
        f64::from(info.system_time_seconds) * 1e6 + f64::from(info.system_time_microseconds);
    (user, system)
}

#[cfg(target_os = "macos")]
const THREAD_BASIC_INFO: u32 = 3;
#[cfg(target_os = "macos")]
const THREAD_BASIC_INFO_COUNT: u32 = 10;

#[cfg(target_os = "macos")]
#[repr(C)]
struct ThreadBasicInfo {
    user_time_seconds: i32,
    user_time_microseconds: i32,
    system_time_seconds: i32,
    system_time_microseconds: i32,
    cpu_usage: i32,
    policy: i32,
    run_state: i32,
    flags: i32,
    suspend_count: i32,
    sleep_time: i32,
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    static mach_task_self_: u32;
    fn mach_thread_self() -> u32;
    fn mach_port_deallocate(task: u32, name: u32) -> i32;
    fn thread_info(
        target_act: u32,
        flavor: u32,
        thread_info_out: *mut i32,
        thread_info_outCnt: *mut u32,
    ) -> i32;
}

#[cfg(target_os = "linux")]
fn capture_current_thread_handle() -> u64 {
    // SAFETY: syscall to get the current thread ID.
    unsafe { libc::syscall(libc::SYS_gettid) as u64 }
}

#[cfg(target_os = "linux")]
fn get_thread_cpu_usage_by_handle(handle: u64) -> (f64, f64) {
    let tid = handle as i32;
    let path = format!("/proc/self/task/{}/stat", tid);
    if let Ok(contents) = std::fs::read_to_string(&path) {
        // Parse utime and stime after pid(comm)
        if let Some(pos) = contents.rfind(')') {
            let rest = &contents[pos + 2..]; // skip ") "
            let fields: Vec<&str> = rest.split_whitespace().collect();
            // 0=state 1=ppid ... 11=utime 12=stime
            if fields.len() > 12 {
                // SAFETY: sysconf call to get clock ticks per second.
                let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
                let utime = fields[11].parse::<f64>().unwrap_or(0.0);
                let stime = fields[12].parse::<f64>().unwrap_or(0.0);
                let user_us = utime / ticks_per_sec * 1e6;
                let system_us = stime / ticks_per_sec * 1e6;
                return (user_us, system_us);
            }
        }
    }
    (0.0, 0.0)
}

#[cfg(windows)]
fn capture_current_thread_handle() -> u64 {
    // SAFETY: Returns the thread ID of the calling thread.
    unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() as u64 }
}

#[cfg(windows)]
fn get_thread_cpu_usage_by_handle(handle: u64) -> (f64, f64) {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, FALSE, FILETIME},
        System::Threading::{GetThreadTimes, OpenThread, THREAD_QUERY_INFORMATION},
    };

    let thread_id = handle as u32;

    // SAFETY: Opens a handle to the thread for querying times.
    let thread_handle = unsafe { OpenThread(THREAD_QUERY_INFORMATION, FALSE, thread_id) };
    if thread_handle.is_null() {
        return (0.0, 0.0);
    }

    let mut creation_time = std::mem::MaybeUninit::<FILETIME>::uninit();
    let mut exit_time = std::mem::MaybeUninit::<FILETIME>::uninit();
    let mut kernel_time = std::mem::MaybeUninit::<FILETIME>::uninit();
    let mut user_time = std::mem::MaybeUninit::<FILETIME>::uninit();

    // SAFETY: Queries thread CPU times.
    let ret = unsafe {
        GetThreadTimes(
            thread_handle,
            creation_time.as_mut_ptr(),
            exit_time.as_mut_ptr(),
            kernel_time.as_mut_ptr(),
            user_time.as_mut_ptr(),
        )
    };

    // SAFETY: Close the thread handle.
    unsafe { CloseHandle(thread_handle) };

    if ret == FALSE {
        return (0.0, 0.0);
    }

    // SAFETY: values are initialized.
    let user_time = unsafe { user_time.assume_init() };
    // SAFETY: values are initialized.
    let kernel_time = unsafe { kernel_time.assume_init() };

    // FILETIME is in 100-nanosecond intervals, convert to microseconds.
    let user_us =
        ((user_time.dwHighDateTime as u64) << 32 | user_time.dwLowDateTime as u64) as f64 / 10.0;
    let system_us = ((kernel_time.dwHighDateTime as u64) << 32 | kernel_time.dwLowDateTime as u64)
        as f64
        / 10.0;
    (user_us, system_us)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn capture_current_thread_handle() -> u64 {
    0
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn get_thread_cpu_usage_by_handle(_handle: u64) -> (f64, f64) {
    (0.0, 0.0)
}
