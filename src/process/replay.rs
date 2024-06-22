use std::sync::{Arc, Mutex};

use crate::{Eventhandler, Process};

pub(super) const GATEWAY: &str = "Gateway";
pub(super) const TASK: &str = "Task";

impl Process {
    /// Replay a trace from a process run. It will be sequential. Only Tasks and gateways is traced that might mutate data.
    pub fn replay_trace<T>(
        handler: &Eventhandler<T>,
        data: T,
        trace: &[(impl AsRef<str>, impl AsRef<str>)],
    ) -> T
    where
        T: std::fmt::Debug,
    {
        let data = Arc::new(Mutex::new(data));
        for (ty, id) in trace.iter().map(|(ty, id)| (ty.as_ref(), id.as_ref())) {
            match ty {
                TASK => {
                    let _ = handler.run_task(id, Arc::clone(&data));
                }
                GATEWAY => {
                    let _ = handler.run_gateway(id, Arc::clone(&data));
                }
                _ => {}
            }
        }
        Arc::try_unwrap(data).unwrap().into_inner().unwrap()
    }
}
