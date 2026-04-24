#[cfg(test)]
pub(crate) mod harness;

#[cfg(test)]
mod harness_smoke;

#[cfg(test)]
mod locked_rejection;

#[cfg(test)]
mod unlock_flow;

#[cfg(test)]
mod auto_lock;

#[cfg(test)]
mod biometric_degradation;

#[cfg(test)]
mod wakeup_interception;

#[cfg(test)]
mod template_guard;

#[cfg(test)]
mod state_transitions;

#[cfg(test)]
mod unguarded_allowlist;
