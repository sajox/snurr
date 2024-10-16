use std::sync::{Arc, Mutex};

use crate::{Eventhandler, Process};

pub(super) const GATEWAY: &str = "Gateway";
pub(super) const TASK: &str = "Task";

impl Process {
    /// Replay a trace from a process run. It will be sequential. Only Tasks and gateways is traced that might mutate data.
    ///
    /// NOTE: Only with feature = "trace"
    /// ```
    /// use snurr::{Process, Eventhandler};
    ///
    /// #[derive(Debug, Default, PartialEq, Eq)]
    /// struct Counter {
    ///     count: u32,
    /// }
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn = Process::new("examples/example.bpmn")?;
    ///     let mut handler: Eventhandler<Counter> = Eventhandler::default();
    ///     handler.add_task("Count 1", |input| {
    ///         input.lock().unwrap().count += 1;
    ///         None
    ///     });
    ///    
    ///     handler.add_gateway("equal to 3", |input| {
    ///         let result = if input.lock().unwrap().count == 3 {
    ///             "YES"
    ///         } else {
    ///             "NO"
    ///         };
    ///         result.into()
    ///     });
    ///
    ///     let pr = bpmn.run(&handler, Counter::default())?;
    ///     let trace_result = Process::replay_trace(&handler, Counter::default(), &pr.trace);
    ///
    ///     #[cfg(feature = "trace")]
    ///     assert_eq!(pr.result, trace_result);
    ///     Ok(())
    /// }
    /// ```
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
