//! ## Executor Tracing lifecycle
//!
//! ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
//!       │(1)                                             │
//! │     │
//!   ╔═══▼══╗   (2)     ┌────────────┐  (3)  ┌─────────┐  │
//! │ ║ IDLE ║──────────▶│ SCHEDULING │──────▶│ POLLING │
//!   ╚══════╝           └────────────┘       └─────────┘  │
//! │     ▲              │            ▲            │
//!       │      (5)     │            │  (4)       │       │
//! │     └──────────────┘            └────────────┘
//!   ┌──────────────────────────┐                         │
//! └ ┤ Executor Trace Lifecycle │─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
//!   └──────────────────────────┘
//!
//! 1. The executor is started (no associated trace)
//! 2. A task on this executor is awoken. `_embassy_trace_task_ready_begin` is called
//!      when this occurs, and `_embassy_trace_poll_start` is called when the executor
//!      actually begins running
//! 3. The executor has decided a task to poll. `_embassy_trace_task_exec_begin` is called
//! 4. The executor finishes polling the task. `_embassy_trace_task_exec_end` is called
//! 5. The executor has finished polling tasks. `_embassy_trace_executor_idle` is called
//!
//! (taken from embassy-executor/src/raw/trace.rs)
//!

use std::{collections::VecDeque, sync::atomic::Ordering};

use crate::{
    FIRMWARE_ADDR_MAP,
    tracing::{
        instance::HISTORY_MAX_TIME_S,
        task::TaskTraceInfo,
        time::{ComputerTime, EmbassyTime, TimePair},
        trace_data::{TraceItem, TraceItemType},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum PreemptedPrevState {
    Scheduling,
    Polling,
}

impl Into<ExecutorState> for PreemptedPrevState {
    fn into(self) -> ExecutorState {
        match self {
            PreemptedPrevState::Scheduling => ExecutorState::Scheduling,
            PreemptedPrevState::Polling => ExecutorState::Polling,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum ExecutorState {
    Idle,
    Scheduling,
    /// Executor was preempted by another higher priority executor on the same core
    Preempted {
        by_executor_id: u32,
        prev_state: PreemptedPrevState,
    },
    Polling,
}

pub struct ExecutorHistoryEntry {
    state: ExecutorState,
    start_time: TimePair,
    end_time: TimePair,
}

// TODO: Executor CPU usage statistics via the state history and not sum of tasks

pub struct ExecutorTraceInfo {
    executor_id: u32,
    executor_name: Option<String>,
    core_id: u32,

    #[allow(dead_code)]
    created_at: TimePair,

    /// Current state of the executor
    state: ExecutorState,
    /// Timestamp when the current state started
    state_start_time: TimePair,

    state_history: VecDeque<ExecutorHistoryEntry>,

    tasks: Vec<TaskTraceInfo>,
}

impl ExecutorTraceInfo {
    pub fn new(executor_id: u32, core_id: u32, created_at: TimePair) -> Self {
        // try to find task name from global firmware address map
        let executor_name = match FIRMWARE_ADDR_MAP.get() {
            Some(addr_map) => {
                // task id represents the address of the task's future vtable
                match addr_map.get(&(executor_id as u64)) {
                    Some(name) => Some(name.to_string()),
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
            executor_id,
            executor_name,
            core_id,
            state: ExecutorState::Idle,
            state_start_time: created_at,
            tasks: Vec::new(),
            created_at,
            state_history: VecDeque::new(),
        }
    }

    /// Set a new state for the executor, sending statistics as needed
    fn set_new_state(&mut self, new_state: ExecutorState, timestamp: TimePair) {
        if self.state != new_state {
            // log history stats
            let hist_entry = ExecutorHistoryEntry {
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

    /// Get the unique executor ID
    pub fn get_executor_id(&self) -> u32 {
        self.executor_id
    }

    /// Get the executor name if available
    pub fn get_executor_name(&self) -> Option<&String> {
        self.executor_name.as_ref()
    }

    /// Get a display name for the executor (either the name or "Executor 0x<id>")
    pub fn get_executor_display_name(&self) -> String {
        match &self.executor_name {
            Some(name) => name.clone(),
            None => format!("Executor 0x{:X}", self.executor_id),
        }
    }

    /// Get the core ID this executor belongs to
    pub fn get_core_id(&self) -> u32 {
        self.core_id
    }

    /// Get the current state of the executor
    pub fn get_state(&self) -> &ExecutorState {
        &self.state
    }

    /// Get the timestamp when the current state started
    pub fn get_state_start_time(&self) -> TimePair {
        self.state_start_time
    }

    pub fn get_tasks(&self) -> &Vec<TaskTraceInfo> {
        &self.tasks
    }

    /// Get an iterator over all tasks associated with this executor
    pub fn iter_tasks(&self) -> impl Iterator<Item = &TaskTraceInfo> {
        self.tasks.iter()
    }

    /// Get a mutable iterator over all tasks associated with this executor
    pub fn iter_tasks_mut(&mut self) -> impl Iterator<Item = &mut TaskTraceInfo> {
        self.tasks.iter_mut()
    }

    /// Find a task by its ID
    pub fn find_task_by_id(&self, task_id: u32) -> Option<&TaskTraceInfo> {
        self.tasks.iter().find(|t| t.get_task_id() == task_id)
    }

    pub fn count_tasks(&self) -> usize {
        self.tasks.len()
    }

    /// Find a task by its ID (mutable)
    pub fn find_task_by_id_mut(&mut self, task_id: u32) -> Option<&mut TaskTraceInfo> {
        self.tasks.iter_mut().find(|t| t.get_task_id() == task_id)
    }

    /// Update belonging tasks based on a trace item
    fn update_tasks(&mut self, trace_item: &TraceItem) {
        // Check preemption state
        match self.state {
            ExecutorState::Polling | ExecutorState::Scheduling => {
                // Check if we are beeing preempted
                if let TraceItemType::ExecutorPollStart { executor_id } = trace_item.data {
                    if executor_id != self.executor_id && trace_item.core_id == self.core_id {
                        // preempt
                        let prev_state = match self.state {
                            ExecutorState::Scheduling => PreemptedPrevState::Scheduling,
                            ExecutorState::Polling => PreemptedPrevState::Polling,
                            _ => unreachable!(),
                        };

                        self.set_new_state(
                            ExecutorState::Preempted {
                                by_executor_id: executor_id,
                                prev_state,
                            },
                            trace_item.time_pair,
                        );
                    }
                }
            }
            ExecutorState::Preempted {
                by_executor_id,
                prev_state,
            } => {
                // Check if we can resume (the higher prio executor goes back to idle)
                if let TraceItemType::ExecutorIdle { .. } = trace_item.data {
                    if trace_item.data.get_executor_id() == by_executor_id {
                        // resume
                        self.set_new_state(prev_state.into(), trace_item.time_pair);
                    }
                }
            }
            _ => {}
        }

        // Check if the task is for this executor and we list it
        if trace_item.data.get_executor_id() == self.executor_id {
            // this is our executor ==> get task or create it
            if let Some(task_id) = trace_item.data.get_task_id() {
                if self.find_task_by_id(task_id).is_none() {
                    // If the task does not exist, create it (probably a TaskNew event)
                    let new_task = TaskTraceInfo::new(
                        task_id,
                        self.executor_id,
                        self.core_id,
                        trace_item.time_pair,
                    );
                    self.tasks.push(new_task);
                }
            }
        }

        // publish updates to existing tasks
        for task in self.tasks.iter_mut() {
            task.update(trace_item);
        }
    }

    /// Run State Machine transition based on trace item
    pub fn update(&mut self, trace_item: &TraceItem) {
        // Update tasks first
        self.update_tasks(trace_item);

        // Check that the trace item is for this executor
        if trace_item.data.get_executor_id() == self.executor_id {
            // Executor State machine transitions

            match self.state {
                ExecutorState::Idle => {
                    if let TraceItemType::ExecutorPollStart { .. } = trace_item.data {
                        self.set_new_state(ExecutorState::Scheduling, trace_item.time_pair);
                    }
                }
                ExecutorState::Scheduling => {
                    if let TraceItemType::TaskExecBegin { .. } = trace_item.data {
                        self.set_new_state(ExecutorState::Polling, trace_item.time_pair);
                    }

                    if let TraceItemType::ExecutorIdle { .. } = trace_item.data {
                        self.set_new_state(ExecutorState::Idle, trace_item.time_pair);
                    }
                }
                ExecutorState::Polling => {
                    if let TraceItemType::TaskExecEnd { .. } = trace_item.data {
                        self.set_new_state(ExecutorState::Scheduling, trace_item.time_pair);
                    }
                }
                _ => {}
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

        // calculate idle percentage by summing all statistics
        // let total_idle_time = self
        //     .statistics
        //     .iter()
        //     .filter_map(|stat| {
        //         if let StatisticMetrics::IdleTime { start, end } = stat {
        //             Some(end.checked_sub(*start).unwrap_or(Duration::from_secs(0)))
        //         } else {
        //             None
        //         }
        //     })
        //     .fold(Duration::from_secs(0), |acc, x| acc + x);
        // let total_time =
        //     self.statistics
        //         .iter()
        //         .fold(Duration::from_secs(0), |acc, stat| match stat {
        //             StatisticMetrics::IdleTime { start, end }
        //             | StatisticMetrics::SchedulingTime { start, end }
        //             | StatisticMetrics::PollingTime { start, end } => {
        //                 acc + end.checked_sub(*start).unwrap_or(Duration::from_secs(0))
        //             }
        //         });

        // if total_time.as_millis() > 0 {
        //     let idle_percentage =
        //         (total_idle_time.as_millis() as f64 / total_time.as_millis() as f64) * 100.0;
        //     println!(
        //         "Executor {} idle percentage: {:.2}%",
        //         self.executor_name
        //             .as_ref()
        //             .unwrap_or(&self.executor_id.to_string()),
        //         idle_percentage
        //     );
        // }

        // // remove old data (timestamps older than 1 minute) from statistics
        // let one_minute_ago = trace_item
        //     .timestamp
        //     .checked_sub(Duration::from_secs(60))
        //     .unwrap_or(Duration::from_secs(0));
        // self.statistics.retain(|stat| match stat {
        //     StatisticMetrics::IdleTime { end, .. } => *end >= one_minute_ago,
        //     StatisticMetrics::SchedulingTime { end, .. } => *end >= one_minute_ago,
        //     StatisticMetrics::PollingTime { end, .. } => *end >= one_minute_ago,
        // });
    }

    /// Extrapolate the duration spent in the current state till now (UC time)
    fn extrapolate_current_state_duration(&self) -> EmbassyTime {
        // get pc time diff between current time and time of state start
        let pc_time_diff = self.state_start_time.get_pc_timestamp().diff_to_now();

        // estimate current uc time based time of state start and pc time diff
        self.state_start_time.get_uc_timestamp() + pc_time_diff
    }

    /// Calculate CPU utilization based on state history using time spent in POLLING and SCHEDULING states over total time
    pub fn calculate_cpu_utilization(&self) -> f32 {
        let mut total_time_s = 0.0;
        let mut active_time_s = 0.0;

        // add up all history entries
        for entry in self.state_history.iter() {
            let start_pc_time = entry.start_time.get_pc_timestamp();
            let end_pc_time = entry.end_time.get_pc_timestamp();

            let duration_s = end_pc_time.saturating_sub(start_pc_time).as_secs_f32();
            total_time_s += duration_s;

            match entry.state {
                ExecutorState::Scheduling | ExecutorState::Polling => {
                    active_time_s += duration_s;
                }
                _ => {}
            }
        }

        // add current state time
        let estimated_uc_time = self.extrapolate_current_state_duration();
        let estimated_duration =
            estimated_uc_time.saturating_sub(self.state_start_time.get_uc_timestamp());
        total_time_s += estimated_duration.as_secs_f32();

        match self.state {
            ExecutorState::Scheduling | ExecutorState::Polling => {
                active_time_s += estimated_duration.as_secs_f32();
            }
            _ => {}
        }

        if total_time_s > 0.0 {
            (active_time_s / total_time_s) * 100.0
        } else {
            0.0
        }
    }
}
