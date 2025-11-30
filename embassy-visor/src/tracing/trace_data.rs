use crate::tracing::time::{ComputerTime, EmbassyTime, TimePair};

#[derive(Debug)]
pub enum TraceParseError {
    InvalidTimestamp,
    InvalidCoreId,
    InvalidExecutorId,
    InvalidFormat,
    InvalidTaskId,
    InvalidEventType,
    InvalidEventPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceItemType {
    ExecutorIdle { executor_id: u32 },
    ExecutorPollStart { executor_id: u32 },
    TaskNew { executor_id: u32, task_id: u32 },
    TaskEnd { executor_id: u32, task_id: u32 },
    TaskExecBegin { executor_id: u32, task_id: u32 },
    TaskExecEnd { executor_id: u32, task_id: u32 },
    TaskReadyBegin { executor_id: u32, task_id: u32 },
}

impl TraceItemType {
    pub fn get_executor_id(&self) -> u32 {
        match self {
            TraceItemType::ExecutorIdle { executor_id }
            | TraceItemType::ExecutorPollStart { executor_id }
            | TraceItemType::TaskNew { executor_id, .. }
            | TraceItemType::TaskEnd { executor_id, .. }
            | TraceItemType::TaskExecBegin { executor_id, .. }
            | TraceItemType::TaskExecEnd { executor_id, .. }
            | TraceItemType::TaskReadyBegin { executor_id, .. } => *executor_id,
        }
    }

    pub fn get_task_id(&self) -> Option<u32> {
        match self {
            TraceItemType::TaskNew { task_id, .. }
            | TraceItemType::TaskEnd { task_id, .. }
            | TraceItemType::TaskExecBegin { task_id, .. }
            | TraceItemType::TaskExecEnd { task_id, .. }
            | TraceItemType::TaskReadyBegin { task_id, .. } => Some(*task_id),
            _ => None,
        }
    }
}

impl TraceItemType {
    /// Format: <EventType>, <executor_id>, <task_id?>
    pub fn from_parts(parts: &[&str]) -> Result<Self, TraceParseError> {
        if parts.len() < 2 {
            return Err(TraceParseError::InvalidFormat);
        }

        // Destructure parts
        let event_type = parts[0].trim();
        let executor_id: u32 = parts[1]
            .trim()
            .parse()
            .map_err(|_| TraceParseError::InvalidExecutorId)?;
        let task_id = if parts.len() > 2 {
            Some(
                parts[2]
                    .trim()
                    .parse()
                    .map_err(|_| TraceParseError::InvalidTaskId)?,
            )
        } else {
            None
        };

        match event_type {
            "ExecutorIdle" => Ok(TraceItemType::ExecutorIdle { executor_id }),
            "ExecutorPollStart" => Ok(TraceItemType::ExecutorPollStart { executor_id }),
            "TaskNew" => {
                let task_id = task_id.ok_or(TraceParseError::InvalidEventPayload)?;
                Ok(TraceItemType::TaskNew {
                    executor_id,
                    task_id,
                })
            }
            "TaskEnd" => {
                let task_id = task_id.ok_or(TraceParseError::InvalidEventPayload)?;
                Ok(TraceItemType::TaskEnd {
                    executor_id,
                    task_id,
                })
            }
            "TaskExecBegin" => {
                let task_id = task_id.ok_or(TraceParseError::InvalidEventPayload)?;
                Ok(TraceItemType::TaskExecBegin {
                    executor_id,
                    task_id,
                })
            }
            "TaskExecEnd" => {
                let task_id = task_id.ok_or(TraceParseError::InvalidEventPayload)?;
                Ok(TraceItemType::TaskExecEnd {
                    executor_id,
                    task_id,
                })
            }
            "TaskReadyBegin" => {
                let task_id = task_id.ok_or(TraceParseError::InvalidEventPayload)?;
                Ok(TraceItemType::TaskReadyBegin {
                    executor_id,
                    task_id,
                })
            }
            _ => Err(TraceParseError::InvalidEventType),
        }
    }

    /// Format: "<EventType>, <executor_id>, <task_id?>"
    pub fn from_str(str: &str) -> Result<Self, TraceParseError> {
        // Split by comma
        let parts: Vec<&str> = str.split(',').collect();
        Self::from_parts(&parts)
    }
}

#[derive(Debug)]
pub struct TraceItem {
    /// Timestamp of microcontroller (event happend) and computer (event recvd)
    pub time_pair: TimePair,

    pub core_id: u32,

    /// The actual trace data
    pub data: TraceItemType,
}

impl TraceItem {
    pub fn new(time_pair: TimePair, core_id: u32, data: TraceItemType) -> Self {
        TraceItem {
            time_pair,
            core_id,
            data,
        }
    }

    /// Format: [<timestamp>, <core_id>, <EventType>, <executor_id>, <task_id?>]
    pub fn parse_from_line(line: &str, pc_timestamp: ComputerTime) -> Result<Self, TraceParseError> {
        // remove anything before and after the brackets (including brackets)
        let start = line.find('[').ok_or(TraceParseError::InvalidFormat)? + 1;
        let end = line.find(']').ok_or(TraceParseError::InvalidFormat)?;
        let content = &line[start..end];

        // Split by comma
        let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();
        if parts.len() < 4 {
            return Err(TraceParseError::InvalidFormat);
        }

        // Parse timestamp
        let timestamp_micros: u64 = parts[0]
            .parse()
            .map_err(|_| TraceParseError::InvalidTimestamp)?;
        let uc_timestamp = EmbassyTime::from_micros(timestamp_micros);
        let time_pair = TimePair::new(uc_timestamp, pc_timestamp);

        // Parse core_id
        let core_id: u32 = parts[1]
            .parse()
            .map_err(|_| TraceParseError::InvalidCoreId)?;

        // Parse trace item type
        let data = TraceItemType::from_parts(&parts[2..])?;
        Ok(TraceItem::new(time_pair, core_id, data))
    }
}

#[cfg(test)]
mod tests {
    use crate::tracing::time::get_app_base_instant;

    use super::*;

    #[test]
    fn test_trace_item_parsing() {
        let _ = get_app_base_instant(); // init app base instant
        std::thread::sleep(std::time::Duration::from_millis(22));
        let pc_timestamp = ComputerTime::now();

        let line = "[123456, 17, TaskNew, 1, 42]";
        let trace_item = TraceItem::parse_from_line(line, pc_timestamp.clone()).unwrap();

        assert_eq!(
            trace_item.time_pair.get_uc_timestamp(),
            EmbassyTime::from_micros(123456)
        );
        assert_eq!(trace_item.time_pair.get_pc_timestamp(), pc_timestamp);
        assert_eq!(trace_item.core_id, 17);
        match trace_item.data {
            TraceItemType::TaskNew {
                executor_id,
                task_id,
            } => {
                assert_eq!(executor_id, 1);
                assert_eq!(task_id, 42);
            }
            _ => panic!("Expected TaskNew variant"),
        }
    }

    #[test]
    fn test_invalid_trace_item_parsing() {
        let _ = get_app_base_instant(); // init app base instant
        std::thread::sleep(std::time::Duration::from_millis(22));
        let pc_timestamp = ComputerTime::now();

        let line = "[invalid_timestamp, 17, TaskNew, 1, 42]";
        let result = TraceItem::parse_from_line(line, pc_timestamp);
        assert!(matches!(result, Err(TraceParseError::InvalidTimestamp)));

        let line = "[12457, invalid_core_id, TaskNew, 1, 42]";
        let result = TraceItem::parse_from_line(line, pc_timestamp);
        assert!(matches!(result, Err(TraceParseError::InvalidCoreId)));

        let line = "[123456, 17, UnknownEvent, 1, 42]";
        let result = TraceItem::parse_from_line(line, pc_timestamp);
        assert!(matches!(result, Err(TraceParseError::InvalidEventType)));
        
        let line = "[123456, 17, TaskNew, invalid_executor_id, 42]";
        let result = TraceItem::parse_from_line(line, pc_timestamp);
        assert!(matches!(result, Err(TraceParseError::InvalidExecutorId)));

        let line = "[123456, 17, TaskNew, 1, invalid_task_id]";
        let result = TraceItem::parse_from_line(line, pc_timestamp);
        assert!(matches!(result, Err(TraceParseError::InvalidTaskId)));

        let line = "[123456, 17, TaskNew, 1]"; // missing task_id
        let result = TraceItem::parse_from_line(line, pc_timestamp);
        assert!(matches!(result, Err(TraceParseError::InvalidEventPayload)));
    }

    #[test]
    fn test_trace_item_type_from_str() {
        let trace_type =
            TraceItemType::from_str("TaskExecBegin, 2, 99").expect("Failed to parse trace type");

        match trace_type {
            TraceItemType::TaskExecBegin {
                executor_id,
                task_id,
            } => {
                assert_eq!(executor_id, 2);
                assert_eq!(task_id, 99);
            }
            _ => panic!("Expected TaskExecBegin variant"),
        }
    }
}
