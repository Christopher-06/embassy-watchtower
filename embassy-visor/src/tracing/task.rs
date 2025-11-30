//! ## Task Tracing lifecycle
//!
//! ```text
//! ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
//!        │(1)                                            │
//! │      │
//!   ╔════▼════╗ (2) ┌─────────┐ (3) ┌─────────┐          │
//! │ ║ SPAWNED ║────▶│ WAITING │────▶│ RUNNING │
//!   ╚═════════╝     └─────────┘     └─────────┘          │
//! │                 ▲         ▲     │    │    │
//!                   │           (4)      │    │(6)       │
//! │                 │(7)      └ ─ ─ ┘    │    │
//!                   │                    │    │          │
//! │             ┌──────┐             (5) │    │  ┌─────┐
//!               │ IDLE │◀────────────────┘    └─▶│ END │ │
//! │             └──────┘                         └─────┘
//!   ┌──────────────────────┐                             │
//! └ ┤ Task Trace Lifecycle │─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
//!   └──────────────────────┘
//! ```
//!
//! 1. A task is spawned, `_embassy_trace_task_new` is called
//! 2. A task is enqueued for the first time, `_embassy_trace_task_ready_begin` is called
//! 3. A task is polled, `_embassy_trace_task_exec_begin` is called
//! 4. WHILE a task is polled, the task is re-awoken, and `_embassy_trace_task_ready_begin` is
//!      called. The task does not IMMEDIATELY move state, until polling is complete and the
//!      RUNNING state is existed. `_embassy_trace_task_exec_end` is called when polling is
//!      complete, marking the transition to WAITING
//! 5. Polling is complete, `_embassy_trace_task_exec_end` is called
//! 6. The task has completed, and `_embassy_trace_task_end` is called
//! 7. A task is awoken, `_embassy_trace_task_ready_begin` is called
//!
//! (taken from embassy-executor/src/raw/trace.rs)
//!
//! We added the Preempted state to indicate that a task was preempted by another executor task with higher priority (Interrupt context).

use std::{collections::VecDeque, ops::Div, sync::atomic::Ordering, time::Duration};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    FIRMWARE_ADDR_MAP, elf_file,
    tracing::{
        instance::HISTORY_MAX_TIME_S,
        time::{ComputerTime, EmbassyTime, TimePair},
        trace_data::{TraceItem, TraceItemType},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum TaskTraceState {
    Spawned,
    Waiting,
    Running,
    /// Task was preempted by another executor (task with different executor ID on the same core)
    Preempted {
        by_executor_id: u32,
    },
    Idle,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct TaskHistoryEntry {
    state: TaskTraceState,
    start_time: TimePair,
    end_time: TimePair,
}

impl TaskHistoryEntry {
    /// Get the duration of this history entry (via UC timestamps)
    pub fn get_uc_duration(&self) -> EmbassyTime {
        let start_uc_time = self.start_time.get_uc_timestamp();
        let end_uc_time = self.end_time.get_uc_timestamp();

        end_uc_time.saturating_sub(start_uc_time)
    }
}

pub struct TaskTraceInfo {
    task_id: u32,
    task_name: Option<String>,
    executor_id: u32,
    core_id: u32,

    created_at: TimePair,

    /// Current state of the task
    state: TaskTraceState,
    /// Timestamp when the current state started
    state_start_time: TimePair,

    /// history of state changes
    state_history: VecDeque<TaskHistoryEntry>,
}

impl TaskTraceInfo {
    pub fn new(task_id: u32, executor_id: u32, core_id: u32, created_at: TimePair) -> Self {
        // try to find task name from global firmware address map
        let task_name = match FIRMWARE_ADDR_MAP.get() {
            Some(addr_map) => {
                // task id represents the address of the task's future vtable
                match addr_map.get(&(task_id as u64)) {
                    Some(name) => Some(elf_file::try_extract_short_name(name).to_string()),
                    None => None,
                }
            }
            None => {
                eprintln!(
                    "Warning: Firmware address map not initialized when creating TaskTraceInfo"
                );
                None
            }
        };

        Self {
            task_id,
            task_name,
            executor_id,
            core_id,
            created_at,
            state: TaskTraceState::Spawned,
            state_start_time: created_at,
            state_history: VecDeque::new(),
        }
    }

    /// Get the unique task ID
    pub fn get_task_id(&self) -> u32 {
        self.task_id
    }

    /// Get the task name if available
    pub fn get_task_name(&self) -> Option<&String> {
        self.task_name.as_ref()
    }

    /// Get display name for the task (either real name or "Task 0x<ID>" in hex)
    pub fn get_task_display_name(&self) -> String {
        match &self.task_name {
            Some(name) => name.clone(),
            None => format!("Task 0x{:X}", self.task_id),
        }
    }

    /// Get the executor ID this task belongs to
    pub fn get_executor_id(&self) -> u32 {
        self.executor_id
    }

    /// Get the core ID this task belongs to
    pub fn get_core_id(&self) -> u32 {
        self.core_id
    }
    /// Get the current state of the task
    pub fn get_state(&self) -> &TaskTraceState {
        &self.state
    }

    /// Get the timestamp when the task was created
    pub fn get_created_at(&self) -> TimePair {
        self.created_at
    }

    /// Get the timestamp when the current state started
    pub fn get_state_start_time(&self) -> TimePair {
        self.state_start_time
    }

    /// Set a new state for the task, sending statistics as needed
    fn set_new_state(&mut self, new_state: TaskTraceState, timestamp: TimePair) {
        if self.state != new_state {
            // println!(
            //     "Task {} changing state from {:?} to {:?} at UC time {:?}",
            //     self.get_task_display_name(),
            //     self.state,
            //     new_state,
            //     timestamp.get_uc_timestamp()
            // );

            // log history statistic
            let hist_entry = TaskHistoryEntry {
                state: self.state,
                start_time: self.state_start_time,
                end_time: timestamp,
            };
            self.state_history.push_back(hist_entry);

            // update state
            self.state = new_state;
            self.state_start_time = timestamp;
        }
    }

    /// Update the task state based on a new trace item
    pub fn update(&mut self, trace_item: &TraceItem) {
        // Check if we get preempted
        if self.state == TaskTraceState::Running {
            // check if another executor on the same core_id is beginning to poll (that would preempt us because only one executor can run on a core at a time)
            if let TraceItemType::ExecutorPollStart { executor_id, .. } = trace_item.data {
                if trace_item.core_id == self.core_id && executor_id != self.executor_id {
                    // preempted by another executor
                    self.set_new_state(
                        TaskTraceState::Preempted {
                            by_executor_id: executor_id,
                        },
                        trace_item.time_pair,
                    );
                    return;
                }
            }
        }

        // Check if we are resuming from preemption (other executor is now idle)
        if let TaskTraceState::Preempted { by_executor_id } = self.state {
            // check if the other executor goes to idle
            if let TraceItemType::ExecutorIdle { executor_id, .. } = trace_item.data {
                if executor_id == by_executor_id {
                    // resume our task to running
                    self.set_new_state(TaskTraceState::Running, trace_item.time_pair);
                    return;
                }
            }
        }

        // Check that this trace item is for this executor
        if trace_item.data.get_executor_id() != self.executor_id {
            return;
        }

        // Check that this trace item is for this task
        match trace_item.data.get_task_id() {
            Some(tid) if tid == self.task_id => {}
            _ => return,
        }

        // State machine transitions
        match self.state {
            TaskTraceState::Spawned => {
                if let TraceItemType::TaskReadyBegin { .. } = trace_item.data {
                    self.set_new_state(TaskTraceState::Waiting, trace_item.time_pair);
                }
            }
            TaskTraceState::Waiting => {
                if let TraceItemType::TaskExecBegin { .. } = trace_item.data {
                    self.set_new_state(TaskTraceState::Running, trace_item.time_pair);
                }
            }
            TaskTraceState::Running => {
                match trace_item.data {
                    TraceItemType::TaskExecEnd { .. } => {
                        self.set_new_state(TaskTraceState::Idle, trace_item.time_pair);
                    }
                    TraceItemType::TaskReadyBegin { .. } => {
                        // Normally this would transition after TaskExecEnd, but we can handle it here in this way too (maybe?)
                        // This means the task was re-awoken while running
                        self.set_new_state(TaskTraceState::Waiting, trace_item.time_pair);
                    }
                    TraceItemType::TaskEnd { .. } => {
                        self.set_new_state(TaskTraceState::Ended, trace_item.time_pair);
                    }
                    _ => {}
                }
            }
            TaskTraceState::Idle => {
                if let TraceItemType::TaskReadyBegin { .. } = trace_item.data {
                    self.set_new_state(TaskTraceState::Waiting, trace_item.time_pair);
                }
            }
            TaskTraceState::Ended => {
                // No transitions out of ended for tasks
            }
            TaskTraceState::Preempted { .. } => {} // nothing here because of other task-id
        }

        // Drain old history entries beyond max time (based on end-time)
        let max_time_s = ComputerTime::from_s(HISTORY_MAX_TIME_S.load(Ordering::Relaxed));
        let current_pc_time = trace_item.time_pair.get_pc_timestamp();
        while let Some(front) = self.state_history.front() {
            let entry_end_pc_time = front.end_time.get_pc_timestamp();

            // Check if the difference is greater than max time
            if current_pc_time.saturating_sub(entry_end_pc_time) > max_time_s {
                self.state_history.pop_front();
            } else {
                break;
            }
        }
    }

    /// Extrapolate the duration spent in the current state till now (UC time)
    fn extrapolate_current_state_duration(&self) -> EmbassyTime {
        // get pc time diff between current time and time of state start
        let pc_time_diff = self.state_start_time.get_pc_timestamp().diff_to_now();

        // estimate current uc time based time of state start and pc time diff
        self.state_start_time.get_uc_timestamp() + pc_time_diff
    }

    fn calc_current_state_duration(&self) -> EmbassyTime {
        let estimated_uc_time_now = self.extrapolate_current_state_duration();
        estimated_uc_time_now.saturating_sub(self.state_start_time.get_uc_timestamp())
    }

    /// Calculate total duration spent in all states from first history entry till now
    pub fn calc_total_history_duration(&self) -> EmbassyTime {
        // get start time from first history entry
        let start_time_uc = match self.state_history.front() {
            Some(entry) => entry.start_time.get_uc_timestamp(),
            None => EmbassyTime::ZERO,
        };

        // Add current state duration
        let estimated_uc_time_now = self.extrapolate_current_state_duration();

        estimated_uc_time_now.saturating_sub(start_time_uc)
    }

    /// Go through history and calculate total duration spent in the given state
    /// for the task. Also uses current state if matching.
    pub fn calc_total_history_state_duration(&self, state: TaskTraceState) -> EmbassyTime {
        // Retrieve total duration in the given state history
        let mut total_duration = self
            .state_history
            .par_iter()
            .filter(|e| e.state == state) // Filter by state
            .map(|e| e.get_uc_duration()) // Map to durations
            .reduce(|| EmbassyTime::ZERO, |a, b| a + b); // Sum durations

        // TODO: Check if start < MAX_TIME_S and sub from the starting element for accuracy?

        // Add current state if matching (duration till now)
        if self.state == state {
            total_duration += self.calc_current_state_duration();
        }

        total_duration
    }

    /// Calculate min, mean, max and count of waiting time durations from history. Also includes
    /// current waiting time if applicable.
    pub fn calc_min_mean_max_count_waiting_time(
        &self,
    ) -> Option<(Duration, Duration, Duration, usize)> {
        #[derive(Clone, Debug)]
        struct Stats {
            min: Duration,
            max: Duration,
            sum: Duration,
            count: usize,
        }

        let mut stats = self
            .state_history
            .par_iter()
            .filter(|e| e.state == TaskTraceState::Waiting)
            .map(|e| e.get_uc_duration().as_duration())
            .fold(
                || Stats {
                    min: Duration::MAX,
                    max: Duration::ZERO,
                    sum: Duration::ZERO,
                    count: 0,
                },
                |mut acc, duration| {
                    acc.min = acc.min.min(duration);
                    acc.max = acc.max.max(duration);
                    acc.sum += duration;
                    acc.count += 1;
                    acc
                },
            )
            .reduce(
                || Stats {
                    min: Duration::MAX,
                    max: Duration::ZERO,
                    sum: Duration::ZERO,
                    count: 0,
                },
                |a, b| Stats {
                    min: a.min.min(b.min),
                    max: a.max.max(b.max),
                    sum: a.sum + b.sum,
                    count: a.count + b.count,
                },
            );

        // Check if current state is waiting and it's duration is longer than max (min is not affected because it's minimum and we don't know how long it will last. Max is already the maximum observed so far)
        if self.state == TaskTraceState::Waiting {
            // get current duration in waiting state
            let current_duration = self.calc_current_state_duration().as_duration();

            // include when current_duration above min (glitchy short durations should be ignored)
            if current_duration > stats.min {
                stats.sum += current_duration;
                stats.count += 1;
            }

            if current_duration > stats.max {
                stats.max = current_duration;
            }
        }

        //
        if stats.count == 0 {
            None
        } else {
            Some((
                stats.min,
                stats.sum.div(stats.count as u32),
                stats.max,
                stats.count,
            ))
        }
    }
}
