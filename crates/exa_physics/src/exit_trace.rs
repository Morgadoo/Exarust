//! A debug trace of the process's exit (`EXA_EXIT_TRACE=1`): every stage
//! of the teardown is logged with the time since the exit began, and a
//! watchdog thread reports a stage that lasts longer than
//! `EXA_EXIT_TRACE_DUMP` seconds (10 by default): the threads' names,
//! states and kernel wait channels, and every thread's stack (`eu-stack`
//! or `gdb`, here or on the host), written to the log. On Linux every
//! thread is also asked for its own stack by a signal (glibc's
//! `backtrace`), three times two seconds apart: the return addresses are
//! logged as module + address from its load base, which
//! `addr2line -f -C -i -e <binary>` resolves for a build with symbols (the
//! `profiling` profile); a debugger from another container cannot unwind
//! through the toolbox's libraries.
//!
//! Off by default: [`stage`] then costs one relaxed load.

use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

use bevy::log::{info, warn};

/// 0 unknown, 1 off, 2 on.
static ENABLED: AtomicU8 = AtomicU8::new(0);

struct Current {
    /// When the first stage was marked (the exit's start).
    start: Instant,
    /// The stage the process is in and when it entered it.
    name: String,
    since: Instant,
    /// Whether the watchdog has reported this stage as stalled.
    reported: bool,
}

static CURRENT: Mutex<Option<Current>> = Mutex::new(None);
static WATCHDOG: OnceLock<()> = OnceLock::new();

/// Whether the trace is on (`EXA_EXIT_TRACE` set to anything but `0`).
pub fn enabled() -> bool {
    match ENABLED.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => {
            let on = std::env::var("EXA_EXIT_TRACE").is_ok_and(|v| !v.is_empty() && v != "0");
            ENABLED.store(if on { 2 } else { 1 }, Ordering::Relaxed);
            on
        }
    }
}

/// The seconds a stage may last before the watchdog reports it.
fn dump_after() -> Duration {
    let secs = std::env::var("EXA_EXIT_TRACE_DUMP")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s > 0.0)
        .unwrap_or(10.0);
    Duration::from_secs_f64(secs)
}

/// Marks the exit's next stage (logged with the time since the first
/// stage). The first call starts the watchdog.
pub fn stage(name: impl Into<String>) {
    if !enabled() {
        return;
    }
    let name = name.into();
    let now = Instant::now();
    let mut current = CURRENT.lock().unwrap_or_else(|e| e.into_inner());
    let start = current.as_ref().map_or(now, |c| c.start);
    let thread = std::thread::current();
    info!(
        "exit trace: +{:.3} s {} (thread {})",
        (now - start).as_secs_f64(),
        name,
        thread.name().unwrap_or("?")
    );
    *current = Some(Current {
        start,
        name,
        since: now,
        reported: false,
    });
    drop(current);
    WATCHDOG.get_or_init(|| {
        let _ = std::thread::Builder::new()
            .name("exa-exit-watch".into())
            .spawn(watch);
    });
}

fn watch() {
    let limit = dump_after();
    loop {
        std::thread::sleep(Duration::from_millis(250));
        let stalled = {
            let mut current = CURRENT.lock().unwrap_or_else(|e| e.into_inner());
            match current.as_mut() {
                Some(c) if !c.reported && c.since.elapsed() >= limit => {
                    c.reported = true;
                    Some((c.name.clone(), c.since.elapsed(), c.start.elapsed()))
                }
                _ => None,
            }
        };
        if let Some((name, in_stage, total)) = stalled {
            warn!(
                "exit trace: stalled in \"{name}\" for {:.1} s (+{:.1} s since the exit began), pid {}",
                in_stage.as_secs_f64(),
                total.as_secs_f64(),
                std::process::id()
            );
            report_threads();
            // Three samples two seconds apart show whether a busy thread
            // moves.
            #[cfg(target_os = "linux")]
            for sample in 1..=3 {
                signal_stacks::dump(sample);
                std::thread::sleep(Duration::from_secs(2));
            }
            dump_stacks();
        }
    }
}

/// Every thread's name, state and kernel wait channel (from `/proc`).
fn report_threads() {
    let Ok(tasks) = std::fs::read_dir("/proc/self/task") else {
        return;
    };
    let mut lines = Vec::new();
    for task in tasks.flatten() {
        let path = task.path();
        let read = |f: &str| {
            std::fs::read_to_string(path.join(f))
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        let stat = read("stat");
        // The state is the field after the parenthesised name.
        let state = stat
            .rsplit_once(") ")
            .and_then(|(_, rest)| rest.split(' ').next())
            .unwrap_or("?")
            .to_string();
        lines.push(format!(
            "{} {:16} {} wchan {} syscall {}",
            task.file_name().to_string_lossy(),
            read("comm"),
            state,
            read("wchan"),
            read("syscall").split(' ').next().unwrap_or("?")
        ));
    }
    lines.sort();
    info!("exit trace: threads:\n{}", lines.join("\n"));
}

/// Every thread's stack: `eu-stack`, else `gdb -batch`, else the host's
/// `eu-stack` through `flatpak-spawn --host` (a toolbox without either).
fn dump_stacks() {
    let pid = std::process::id().to_string();
    let gdb = [
        "gdb",
        "-batch",
        "-nx",
        "-ex",
        "set pagination off",
        "-ex",
        "thread apply all bt 40",
        "-p",
        &pid,
    ];
    let tries: [&[&str]; 3] = [
        &["eu-stack", "-i", "-p", &pid],
        &gdb,
        &["flatpak-spawn", "--host", "eu-stack", "-i", "-p", &pid],
    ];
    for argv in tries {
        match std::process::Command::new(argv[0])
            .args(&argv[1..])
            .output()
        {
            Ok(out) if out.status.success() || !out.stdout.is_empty() => {
                info!(
                    "exit trace: stacks from {}:\n{}{}",
                    argv[..argv.len() - 1].join(" "),
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                return;
            }
            Ok(out) => info!(
                "exit trace: {} failed: {}",
                argv[0],
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            Err(e) => info!("exit trace: no {} ({e})", argv[0]),
        }
    }
}

/// Marks a stage when a value drops (a resource whose drop shows where
/// the world's teardown has got to).
pub struct DropStage(pub &'static str);

impl Drop for DropStage {
    fn drop(&mut self) {
        stage(format!("dropping {}", self.0));
    }
}

/// A resource that marks a stage when its world drops it (resources drop
/// in the order they were inserted, so a marker inserted first and one
/// inserted last bracket the world's teardown). `N` tells the markers of
/// one world apart (a resource type is held once).
#[derive(bevy::prelude::Resource)]
pub struct ExitTraceMarker<const N: u8>(pub DropStage);

/// Marks the frame reading the exit message (the last frame).
pub fn mark_exit_read(mut exits: bevy::prelude::MessageReader<bevy::app::AppExit>) {
    if let Some(exit) = exits.read().last() {
        stage(format!("frame: {exit:?} read at the frame's end"));
    }
}

/// Every thread's stack, taken by the thread itself in a signal handler.
#[cfg(target_os = "linux")]
mod signal_stacks {
    use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

    const SLOTS: usize = 128;
    const DEPTH: usize = 48;

    struct Slot {
        tid: AtomicI32,
        depth: AtomicUsize,
        frames: [AtomicUsize; DEPTH],
    }

    #[allow(clippy::declare_interior_mutable_const)]
    const EMPTY: Slot = Slot {
        tid: AtomicI32::new(0),
        depth: AtomicUsize::new(0),
        frames: [const { AtomicUsize::new(0) }; DEPTH],
    };
    static SLOT: [Slot; SLOTS] = [EMPTY; SLOTS];
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    extern "C" fn on_signal(_: libc::c_int) {
        let i = NEXT.fetch_add(1, Ordering::SeqCst);
        if i >= SLOTS {
            return;
        }
        let mut buf = [std::ptr::null_mut::<libc::c_void>(); DEPTH];
        // SAFETY: glibc's backtrace writes at most DEPTH pointers into buf;
        // it was called once outside the handler, so its unwinder is
        // loaded and it does not allocate here.
        let n = unsafe { backtrace(buf.as_mut_ptr(), DEPTH as libc::c_int) }.max(0) as usize;
        let slot = &SLOT[i];
        for (f, p) in slot.frames.iter().zip(&buf[..n]) {
            f.store(*p as usize, Ordering::Relaxed);
        }
        slot.depth.store(n, Ordering::Relaxed);
        // SAFETY: gettid has no preconditions.
        let tid = unsafe { libc::syscall(libc::SYS_gettid) } as i32;
        slot.tid.store(tid, Ordering::SeqCst);
    }

    unsafe extern "C" {
        fn backtrace(buffer: *mut *mut libc::c_void, size: libc::c_int) -> libc::c_int;
    }

    /// The module (file name) of an address and its address in that file
    /// (the address less the module's load base: what `addr2line -e`
    /// takes).
    fn locate(maps: &[(usize, usize, usize, String)], addr: usize) -> String {
        let Some((_, _, _, path)) = maps.iter().find(|(lo, hi, ..)| (*lo..*hi).contains(&addr))
        else {
            return format!("{addr:#x}");
        };
        // The load base: the module's mapping of file offset 0.
        let base = maps
            .iter()
            .filter(|(_, _, off, p)| p == path && *off == 0)
            .map(|(lo, ..)| *lo)
            .min()
            .unwrap_or(0);
        let name = path.rsplit('/').next().unwrap_or(path);
        format!("{name}+{:#x}", addr - base)
    }

    /// The process's file mappings: start, end, file offset and path.
    fn mappings() -> Vec<(usize, usize, usize, String)> {
        let text = std::fs::read_to_string("/proc/self/maps").unwrap_or_default();
        text.lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let range = parts.next()?;
                let _perms = parts.next()?;
                let off = usize::from_str_radix(parts.next()?, 16).ok()?;
                let path = parts.nth(2)?.to_string();
                let (lo, hi) = range.split_once('-')?;
                Some((
                    usize::from_str_radix(lo, 16).ok()?,
                    usize::from_str_radix(hi, 16).ok()?,
                    off,
                    path,
                ))
            })
            .collect()
    }

    pub(super) fn dump(sample: u32) {
        let mut warm = [std::ptr::null_mut::<libc::c_void>(); 4];
        // SAFETY: as in the handler; this first call loads the unwinder.
        unsafe { backtrace(warm.as_mut_ptr(), 4) };
        for slot in &SLOT {
            slot.tid.store(0, Ordering::SeqCst);
        }
        NEXT.store(0, Ordering::SeqCst);
        // SAFETY: a plain handler for a signal nothing else uses, installed
        // with a zeroed sigaction (no flags but SA_RESTART).
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = on_signal as extern "C" fn(libc::c_int) as usize;
            action.sa_flags = libc::SA_RESTART;
            libc::sigemptyset(&mut action.sa_mask);
            libc::sigaction(libc::SIGURG, &action, std::ptr::null_mut());
        }
        // SAFETY: getpid/gettid have no preconditions.
        let pid = unsafe { libc::getpid() };
        let me = unsafe { libc::syscall(libc::SYS_gettid) } as i32;
        let tids: Vec<i32> = std::fs::read_dir("/proc/self/task")
            .map(|d| {
                d.flatten()
                    .filter_map(|e| e.file_name().to_str()?.parse().ok())
                    .filter(|t| *t != me)
                    .collect()
            })
            .unwrap_or_default();
        for tid in &tids {
            // SAFETY: tgkill to a thread of this process with a handled signal.
            unsafe { libc::syscall(libc::SYS_tgkill, pid, *tid, libc::SIGURG) };
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
        let maps = mappings();
        let mut out = String::new();
        for slot in SLOT.iter().take(NEXT.load(Ordering::SeqCst).min(SLOTS)) {
            let tid = slot.tid.load(Ordering::SeqCst);
            if tid == 0 {
                continue;
            }
            let name =
                std::fs::read_to_string(format!("/proc/self/task/{tid}/comm")).unwrap_or_default();
            out.push_str(&format!("thread {tid} {}:\n", name.trim()));
            // The first frame is the handler itself.
            let depth = slot.depth.load(Ordering::Relaxed);
            for f in slot.frames.iter().take(depth).skip(1) {
                out.push_str(&format!("  {}\n", locate(&maps, f.load(Ordering::Relaxed))));
            }
        }
        bevy::log::info!(
            "exit trace: signal stacks, sample {sample}, of {} threads (resolve with addr2line -f -C -i -e <binary> <address - 1>):\n{out}",
            tids.len()
        );
    }
}
