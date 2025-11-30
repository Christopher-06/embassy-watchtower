use std::time::Duration;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::tracing::task::{TaskTraceInfo, TaskTraceState};

#[derive(Debug, Clone)]
pub struct TaskStats {
    pub name: String,
    /// CPU utilization in percent (0.0 - 100.0)
    pub cpu_utilization_percent: f32,
    /// Minimal time in State 'Ready'
    pub min_waiting_time: Duration,
    /// Maximal time in State 'Ready'
    pub max_waiting_time: Duration,
    /// Average time in State 'Ready'
    pub avg_waiting_time: Duration,
    /// Total count the task was in State 'Ready'
    pub count_waiting_time: usize,
}

impl TaskStats {
    pub fn from_task(task: &TaskTraceInfo) -> Self {
        // Calculate CPU utilization
        let total_time = task.calc_total_history_duration();
        let running_time = task.calc_total_history_state_duration(TaskTraceState::Running);
        let cpu_utilization_percent = if total_time.as_millis() > 0 {
            (running_time.as_secs_f32() / total_time.as_secs_f32()) * 100.0
        } else {
            0.0
        };

        // Calculate waiting time statistics
        let (min_waiting_time, avg_waiting_time, max_waiting_time, count_waiting_time) = task
            .calc_min_mean_max_count_waiting_time()
            .unwrap_or_default();

        Self {
            name: task.get_task_display_name(),
            cpu_utilization_percent,
            min_waiting_time,
            max_waiting_time,
            avg_waiting_time,
            count_waiting_time,
        }
    }

    pub fn from_task_list(tasks: &Vec<TaskTraceInfo>) -> Vec<Self> {
        tasks.par_iter().map(|t| Self::from_task(t)).collect()
    }
}
