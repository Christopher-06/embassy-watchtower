use crate::tracing::stats::core_stats::CoreStats;

#[derive(Debug, Clone, Default)]
pub struct InstanceStats {
    pub core_stats: Vec<CoreStats>,

    pub tasks_count: usize,
    pub executor_count: usize,
}

impl InstanceStats {
    pub fn from_executors(executors: &Vec<crate::tracing::executor::ExecutorTraceInfo>) -> Self {
        let core_stats = CoreStats::from_executor_list(executors);
        let tasks_count = executors.iter().map(|e| e.get_tasks().len()).sum();
        let executor_count = executors.len();

        Self {
            core_stats,
            tasks_count,
            executor_count,
        }
    }
}

// Dummy Data
// InstanceStats {
//     core_stats: vec![
//         CoreStats {
//             core_id: 0,
//             cpu_utilization_percent: 75.0,
//             executors: vec![
//                 ExecutorStats {
//                     name: "Executor 1".into(),
//                     cpu_utilization_percent: 50.0,
//                     tasks: vec![
//                         TaskStats {
//                             name: "Task A".into(),
//                             cpu_utilization_percent: 30.0,
//                             min_waiting_time: Duration::from_millis(5),
//                             max_waiting_time: Duration::from_millis(20),
//                             avg_waiting_time: Duration::from_millis(10),
//                             count_waiting_time: 3,
//                         },
//                         TaskStats {
//                             name: "Task B".into(),
//                             cpu_utilization_percent: 20.0,
//                             min_waiting_time: Duration::from_millis(10),
//                             max_waiting_time: Duration::from_millis(30),
//                             avg_waiting_time: Duration::from_millis(15),
//                             count_waiting_time: 4,
//                         },
//                     ],
//                 },
//                 ExecutorStats {
//                     name: "Executor 2".into(),
//                     cpu_utilization_percent: 25.0,
//                     tasks: vec![
//                         TaskStats {
//                             name: "Task C".into(),
//                             cpu_utilization_percent: 25.0,
//                             min_waiting_time: Duration::from_millis(8),
//                             max_waiting_time: Duration::from_millis(25),
//                             avg_waiting_time: Duration::from_millis(12),
//                             count_waiting_time: 2,
//                         },
//                     ],
//                 },
//             ]
//         },
//         CoreStats {
//             core_id: 1,
//             cpu_utilization_percent: 60.0,
//             executors: vec![
//                 ExecutorStats {
//                     name: "Executor 3".into(),
//                     cpu_utilization_percent: 60.0,
//                     tasks: vec![
//                         TaskStats {
//                             name: "Task D".into(),
//                             cpu_utilization_percent: 40.0,
//                             min_waiting_time: Duration::from_millis(7),
//                             max_waiting_time: Duration::from_millis(22),
//                             avg_waiting_time: Duration::from_millis(14),
//                             count_waiting_time: 3,
//                         },
//                         TaskStats {
//                             name: "Task E".into(),
//                             cpu_utilization_percent: 20.0,
//                             min_waiting_time: Duration::from_millis(12),
//                             max_waiting_time: Duration::from_millis(28),
//                             avg_waiting_time: Duration::from_millis(16),
//                             count_waiting_time: 5,
//                         },
//                     ],
//                 },
//             ]
//         },
//     ],
//     tasks_count: 5,
//     executor_count: 3,
// }
