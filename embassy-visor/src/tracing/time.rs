use std::{
    ops::{Add, AddAssign},
    sync::OnceLock,
    time::{Duration, Instant},
};

static APP_BASE_INSTANT: OnceLock<Instant> = OnceLock::new();

pub fn get_app_base_instant() -> &'static Instant {
    APP_BASE_INSTANT.get_or_init(Instant::now)
}

pub fn duration_since_app_start() -> Duration {
    Instant::now().saturating_duration_since(*get_app_base_instant())
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, PartialOrd, Ord)]
pub struct ComputerTime(Duration);

impl ComputerTime {
    pub fn now() -> Self {
        Self(duration_since_app_start())
    }

    pub fn new_from(value: Instant) -> Self {
        Self(value.saturating_duration_since(*get_app_base_instant()))
    }

    pub fn new_from_duration(duration: Duration) -> Self {
        Self(duration)
    }

    pub fn from_s(secs: u64) -> Self {
        let duration = Duration::from_secs(secs);
        Self(duration)
    }

    pub fn as_secs_f32(&self) -> f32 {
        self.0.as_secs_f32()
    }

    pub fn as_millis(&self) -> u128 {
        self.0.as_millis()
    }

    pub fn saturating_sub(&self, other: ComputerTime) -> ComputerTime {
        ComputerTime(self.0.saturating_sub(other.0))
    }

    pub fn diff_to_now(&self) -> Duration {
        let now = duration_since_app_start();
        now.saturating_sub(self.0)
    }
}

impl From<Instant> for ComputerTime {
    fn from(value: Instant) -> Self {
        Self::new_from(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct EmbassyTime(Duration);

impl EmbassyTime {
    pub const ZERO: EmbassyTime = EmbassyTime(Duration::from_secs(0));

    pub const fn from_micros(us: u64) -> Self {
        Self(Duration::from_micros(us))
    }

    pub const fn from_millis(ms: u64) -> Self {
        Self(Duration::from_millis(ms))
    }

    pub fn as_secs_f32(&self) -> f32 {
        self.0.as_secs_f32()
    }

    pub fn as_millis(&self) -> u128 {
        self.0.as_millis()
    }

    pub fn saturating_sub(&self, other: EmbassyTime) -> EmbassyTime {
        EmbassyTime(self.0.saturating_sub(other.0))
    }

    pub fn as_duration(&self) -> Duration {
        self.0
    }
}

impl Add<Duration> for EmbassyTime {
    type Output = EmbassyTime;

    fn add(self, other: Duration) -> EmbassyTime {
        EmbassyTime(self.0 + other)
    }
}

impl Add for EmbassyTime {
    type Output = EmbassyTime;

    fn add(self, other: EmbassyTime) -> EmbassyTime {
        EmbassyTime(self.0 + other.0)
    }
}

impl AddAssign for EmbassyTime {
    fn add_assign(&mut self, other: EmbassyTime) {
        self.0 += other.0;
    }
}

/// Pair of two timings taken nearly at the same time
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimePair {
    /// Time the event happend (uC Clock)
    uc: EmbassyTime,
    /// Time the event recvd at the pc (Windows Clock e.q.)
    pc: ComputerTime,
}

impl TimePair {
    pub fn new(uc: EmbassyTime, pc: ComputerTime) -> Self {
        TimePair { uc, pc }
    }

    pub fn get_uc_timestamp(&self) -> EmbassyTime {
        self.uc
    }

    pub fn get_pc_timestamp(&self) -> ComputerTime {
        self.pc
    }

    /// Calculate the time between the pc time and uc time in seconds as f32
    /// Negative ---> uc time is greater than pc time
    /// Positive ---> uc time is less than pc time
    pub fn diff_s(&self) -> f32 {
        self.pc.as_secs_f32() - self.uc.as_secs_f32()
    }

    /// Combine a recvd Embassy Time with the current computer clock time
    pub fn now_with_uc_time(uc: EmbassyTime) -> Self {
        let pc = ComputerTime::now();
        TimePair { uc, pc }
    }
}
