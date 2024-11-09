//! Routing policy

/// Policy
#[derive(Default, Clone, Copy, Debug)]
pub enum Policy {
    #[default]
    /// Dispatch messages to endpoints in round-robin
    RoundRobin,

    /// Dispatch messages to endpoints in a random order
    Random,
}
