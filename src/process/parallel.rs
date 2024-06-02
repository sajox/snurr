use std::thread;

use crate::error::Error;

use super::ExecuteResult;

// If more than 1 output exist then all outputs run in a thread with registered Fn. When all threads terminates
// then the next Id is returned to continue on. Thread terminates on a Parallel or Inclusive Join and End events.
pub(super) fn maybe_parallelize<'a>(
    outputs: &'a [String],
    message: &str,
    func: impl Fn(&'a str) -> ExecuteResult<'a> + Sync,
) -> Result<&'a str, Error> {
    if outputs.len() <= 1 {
        return outputs
            .first()
            .map(|s| s.as_str())
            .ok_or_else(|| Error::MissingOutput(message.into()));
    }

    // Diverging gateway
    let (oks, mut errors): (Vec<_>, Vec<_>) = thread::scope(|s| {
        //Start everything first
        let children: Vec<_> = outputs
            .iter()
            .map(|output| s.spawn(|| (func)(output)))
            .collect();

        // Collect results
        children
            .into_iter()
            .filter_map(|handle| handle.join().ok())
            .partition(Result::is_ok)
    });
    if !errors.is_empty() {
        if let Some(result) = errors.pop() {
            return result;
        }
    }
    oks.into_iter()
        .filter_map(Result::ok)
        .next()
        .ok_or(Error::NoGatewayOutput)
}
