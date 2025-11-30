use std::collections::HashMap;

use itertools::Itertools;

use crate::tracing::stats::executor_stats::ExecutorStats;

#[derive(Debug, Clone)]
pub struct CoreStats {
    pub core_id: u32,
    pub executors: Vec<ExecutorStats>,

    /// CPU utilization in percent (0.0 - 100.0)
    pub cpu_utilization_percent: f32,
}

impl CoreStats {
    /// Same core_id means same core
    pub fn from_executor_list_on_core(
        executors: &Vec<&crate::tracing::executor::ExecutorTraceInfo>,
    ) -> Self {
        let core_id = executors.first().map_or(0, |e| e.get_core_id());
        let executors = ExecutorStats::from_executor_list(executors);
        let cpu_utilization_percent = executors.iter().map(|e| e.cpu_utilization_percent).sum();

        Self {
            core_id,
            executors,
            cpu_utilization_percent,
        }
    }

    /// Group by core_id and create CoreStats for each core
    pub fn from_executor_list(
        executors: &Vec<crate::tracing::executor::ExecutorTraceInfo>,
    ) -> Vec<Self> {
        let mut executors_by_core: HashMap<u32, Vec<_>> =
            HashMap::new();

        for executor in executors {
            executors_by_core
                .entry(executor.get_core_id())
                .or_default()
                .push(executor);
        }

        executors_by_core
            .into_iter()
            .map(|(_, execs)| Self::from_executor_list_on_core(&execs))
            .sorted_by(|a, b| a.core_id.cmp(&b.core_id))
            .collect()
    }
}
