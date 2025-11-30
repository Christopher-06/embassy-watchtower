use std::sync::{Arc, Mutex, atomic::AtomicU64};

use crossbeam::channel::Receiver;

use crate::tracing::{executor::ExecutorTraceInfo, stats::instance_stats::InstanceStats, trace_data::TraceItem};

pub static HISTORY_MAX_TIME_S: AtomicU64 = AtomicU64::new(30); // 30seconds

#[derive(Clone)]
pub struct TracingInstance {
    executors: Arc<Mutex<Vec<ExecutorTraceInfo>>>,
}

fn update_from_trace_items(
    trace_recver: Receiver<TraceItem>,
    tracing_instance: TracingInstance,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            match trace_recver.recv() {
                Ok(trace_item) => {
                    // New Trace Item --> Update tracing instance
                    tracing_instance.update(&trace_item);
                }
                Err(_) => {
                    break; // Channel closed
                }
            }
        }
    })
}

impl TracingInstance {
    pub fn new(trace_recver: Receiver<TraceItem>) -> Self {
        let instance = Self {
            executors: Arc::new(Mutex::new(Vec::new())),
        };

        let _ = update_from_trace_items(trace_recver, instance.clone());
        instance
    }

    /// Update the tracing instance based on a new trace item
    pub fn update(&self, trace_item: &TraceItem) {
        let mut executors = self.executors.lock().unwrap();

        // Check that we have an executor for this trace item
        if Self::find_executor_by_id_locked(&executors, trace_item.data.get_executor_id()).is_none()
        {
            // Create a new executor
            let new_executor = ExecutorTraceInfo::new(
                trace_item.data.get_executor_id(),
                trace_item.core_id,
                trace_item.time_pair,
            );
            executors.push(new_executor);
        }

        // Update executors
        for executor in executors.iter_mut() {
            executor.update(trace_item);
        }

        // print count of tasks in mode RUNNING
        // let running_tasks = executors
        //     .iter()
        //     .map(|exer| {
        //         exer.iter_tasks()
        //             .filter(|t| t.get_state() == &TaskTraceState::Running)
        //     })
        //     .flatten()
        //     .count();
        // println!("Running tasks: {}", running_tasks);
    }

    /// Calculate and return instance statistics
    pub fn get_stats(&self) -> InstanceStats {
        let executors = self.executors.lock().unwrap();
        InstanceStats::from_executors(&executors)
    }

    fn find_executor_by_id_locked<'a>(
        executors: &'a Vec<ExecutorTraceInfo>,
        executor_id: u32,
    ) -> Option<&'a ExecutorTraceInfo> {
        executors.iter().find(|e| e.get_executor_id() == executor_id)
    }
}