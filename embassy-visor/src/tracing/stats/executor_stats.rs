use crate::tracing::{executor::ExecutorTraceInfo, stats::task_stats::TaskStats};

#[derive(Debug, Clone)]
pub struct ExecutorStats {
    pub name : String,
    pub tasks : Vec<TaskStats>,

    /// CPU utilization in percent (0.0 - 100.0) [Scheduling + Polling]
    pub cpu_utilization_percent : f32,
}

impl ExecutorStats {
    pub fn from_executor(executor: &ExecutorTraceInfo) -> Self {
        let tasks = TaskStats::from_task_list(&executor.get_tasks());

        // Sum up CPU utilization from tasks
        let cpu_utilization_percent = executor.calculate_cpu_utilization();

        Self {
            name: executor.get_executor_display_name(),
            tasks,
            cpu_utilization_percent,
        }
    }

    pub fn from_executor_list(executors: &Vec<&ExecutorTraceInfo>) -> Vec<Self> {
        executors.iter().map(|e| Self::from_executor(e)).collect()
    }
}