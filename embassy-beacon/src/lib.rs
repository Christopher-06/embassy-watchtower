#![no_std]

use embassy_time::Instant;

mod core_id;

/// Macro to choose which defmt level to use for publishing tracing events (e.g., info!, debug!, etc.) based on a feature flag.
macro_rules! publish {
    ($($arg:tt)*) => {
        #[cfg(feature = "defmt-trace")]
        defmt::trace!($($arg)*);

        #[cfg(feature = "defmt-debug")]
        defmt::debug!($($arg)*);

        #[cfg(feature = "defmt-info")]
        defmt::info!($($arg)*);

        #[cfg(feature = "defmt-warn")]
        defmt::warn!($($arg)*);

        #[cfg(feature = "defmt-error")]
        defmt::error!($($arg)*);

        // because defmt-debug is default active
        #[cfg(not(any(feature = "defmt-trace", feature = "defmt-debug", feature = "defmt-info", feature = "defmt-warn", feature = "defmt-error")))]
        {
            #[cfg(feature = "defmt-println")]
            defmt::println!($($arg)*);
        }
    };
}

#[unsafe(no_mangle)]
fn _embassy_trace_poll_start(executor_id: u32) {
    let now = Instant::now().as_micros();
    let core_id = core_id::core_id();
    publish!(
        "embassy executor tracer - [{}, {}, ExecutorPollStart, {}] - embassy executor tracer",
        now,
        core_id,
        executor_id
    );
}

#[unsafe(no_mangle)]
fn _embassy_trace_executor_idle(executor_id: u32) {
    let now = Instant::now().as_micros();
    let core_id = core_id::core_id();
    publish!(
        "embassy executor tracer - [{}, {}, ExecutorIdle, {}] - embassy executor tracer",
        now,
        core_id,
        executor_id
    );
}

#[unsafe(no_mangle)]
fn _embassy_trace_task_new(executor_id: u32, task_id: u32) {
    let now = Instant::now().as_micros();
    let core_id = core_id::core_id();
    publish!(
        "embassy executor tracer - [{}, {}, TaskNew, {}, {}] - embassy executor tracer",
        now,
        core_id,
        executor_id,
        task_id
    );
}

#[unsafe(no_mangle)]
fn _embassy_trace_task_end(executor_id: u32, task_id: u32) {
    let now = Instant::now().as_micros();
    let core_id = core_id::core_id();
    publish!(
        "embassy executor tracer - [{}, {}, TaskEnd, {}, {}] - embassy executor tracer",
        now,
        core_id,
        executor_id,
        task_id
    );
}

#[unsafe(no_mangle)]
fn _embassy_trace_task_exec_begin(executor_id: u32, task_id: u32) {
    let now = Instant::now().as_micros();
    let core_id = core_id::core_id();
    publish!(
        "embassy executor tracer - [{}, {}, TaskExecBegin, {}, {}] - embassy executor tracer",
        now,
        core_id,
        executor_id,
        task_id
    );
}

#[unsafe(no_mangle)]
fn _embassy_trace_task_exec_end(excutor_id: u32, task_id: u32) {
    let now = Instant::now().as_micros();
    let core_id = core_id::core_id();
    publish!(
        "embassy executor tracer - [{}, {}, TaskExecEnd, {}, {}] - embassy executor tracer",
        now,
        core_id,
        excutor_id,
        task_id
    );
}

#[unsafe(no_mangle)]
fn _embassy_trace_task_ready_begin(executor_id: u32, task_id: u32) {
    let now = Instant::now().as_micros();
    let core_id = core_id::core_id();
    publish!(
        "embassy executor tracer - [{}, {}, TaskReadyBegin, {}, {}] - embassy executor tracer",
        now,
        core_id,
        executor_id,
        task_id
    );
}
