//! The engine's floating-point mode: every thread of the original sets it
//! before it runs anything.
//!
//! The original's system unit has one routine for it, which ORs `0x8FC0`
//! into the SSE control and status register (MXCSR): flush-to-zero (bit
//! 15), denormals-are-zero (bit 6) and the invalid, denormal, divide-by-zero,
//! overflow and underflow exception masks (bits 7–11, already set by
//! default). The rounding mode (bits 13–14) is left at round-to-nearest and
//! the precision mask (bit 12) at its default. The program's start calls it
//! on the main thread, after the x87 control word is set, and both of the
//! engine's thread entries call it first: the system unit's own threads
//! (terrain, voxel lighting, content streaming, kd-tree builds) and the
//! operating-system unit's thread that runs the simulation.
//!
//! So a denormal result is written as zero, and a denormal operand is read
//! as zero, everywhere in the original's SSE arithmetic. Exarust sets the
//! same bits with [`enter`] on the main thread, on the simulation thread and
//! on every task-pool thread ([`task_pool_options`]), so a step computes the
//! same bits whichever thread runs a part of it.
//!
//! The x87 control word the start also sets is the runtime library's (the
//! original's long-double maths); Exarust does no x87 arithmetic.
//!
//! [guess: Rust's compiler assumes the default floating-point environment
//! when it folds constants, so a constant expression whose exact value is
//! denormal is folded without the flush; at run time the flush applies.]

/// The bits the original ORs into MXCSR.
pub const ENGINE_MXCSR_BITS: u32 = 0x8fc0;
/// Flush-to-zero: a denormal result becomes a signed zero.
pub const FLUSH_TO_ZERO: u32 = 0x8000;
/// Denormals-are-zero: a denormal operand is read as a signed zero.
pub const DENORMALS_ARE_ZERO: u32 = 0x0040;

/// Sets the engine's floating-point mode on the calling thread: reads MXCSR,
/// ORs in [`ENGINE_MXCSR_BITS`] and writes it back, as the original's
/// routine does. A no-op off x86-64.
pub fn enter() {
    #[cfg(target_arch = "x86_64")]
    {
        let csr = mxcsr().unwrap_or(0) | ENGINE_MXCSR_BITS;
        // SAFETY: LDMXCSR only loads the control word; the bits set are
        // the mode and exception masks, none reserved, so it cannot fault.
        unsafe {
            std::arch::asm!(
                "ldmxcsr [{}]",
                in(reg) &csr,
                options(nostack, readonly, preserves_flags),
            );
        }
    }
}

/// [`enter`], then logs the thread's MXCSR under `thread`'s name.
pub fn enter_logged(thread: &str) {
    enter();
    if let Some(csr) = mxcsr() {
        bevy::log::info!("{thread}: engine float mode, MXCSR {csr:#06x}");
    }
}

/// The calling thread's MXCSR, or `None` off x86-64.
pub fn mxcsr() -> Option<u32> {
    #[cfg(target_arch = "x86_64")]
    {
        let mut csr: u32 = 0;
        // SAFETY: STMXCSR stores the 32-bit register to the local word.
        unsafe {
            std::arch::asm!(
                "stmxcsr [{}]",
                in(reg) &mut csr,
                options(nostack, preserves_flags),
            );
        }
        Some(csr)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        None
    }
}

/// Whether the calling thread is in the engine's mode (always on a target
/// without MXCSR).
pub fn engaged() -> bool {
    mxcsr().is_none_or(|csr| csr & ENGINE_MXCSR_BITS == ENGINE_MXCSR_BITS)
}

/// Task-pool options whose every thread enters the engine's mode as it
/// starts, as the original's thread entries do.
pub fn task_pool_options() -> bevy::app::TaskPoolOptions {
    use std::sync::Arc;
    let mut options = bevy::app::TaskPoolOptions::default();
    let hook: Arc<dyn Fn() + Send + Sync> = Arc::new(enter);
    options.io.on_thread_spawn = Some(hook.clone());
    options.async_compute.on_thread_spawn = Some(hook.clone());
    options.compute.on_thread_spawn = Some(hook);
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_flushes_denormals_on_its_thread_only() {
        std::thread::spawn(|| {
            let tiny = std::hint::black_box(f32::MIN_POSITIVE);
            // Without the mode a halved smallest normal is a denormal.
            assert!(!engaged() || mxcsr().is_none());
            assert!((tiny * 0.5).is_subnormal());
            enter();
            assert!(engaged());
            #[cfg(target_arch = "x86_64")]
            {
                // The result is flushed to zero, and a denormal operand
                // reads as zero.
                assert_eq!(std::hint::black_box(tiny) * 0.5, 0.0);
                let denormal = std::hint::black_box(f32::from_bits(1));
                assert_eq!(denormal * 2.0f32.powi(30), 0.0);
                // The rounding mode stays round-to-nearest.
                assert_eq!(mxcsr().unwrap() & 0x6000, 0);
                assert_eq!(mxcsr().unwrap() & 0x8fc0, 0x8fc0);
            }
        })
        .join()
        .unwrap();
        // The test's own thread is untouched.
        let tiny = std::hint::black_box(f32::MIN_POSITIVE);
        assert!((tiny * 0.5).is_subnormal());
    }

    #[test]
    fn the_task_pool_options_hook_every_pool() {
        let options = task_pool_options();
        assert!(options.io.on_thread_spawn.is_some());
        assert!(options.async_compute.on_thread_spawn.is_some());
        assert!(options.compute.on_thread_spawn.is_some());
    }
}
