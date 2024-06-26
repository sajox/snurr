use super::ExecuteResult;

#[cfg(feature = "parallel")]
use std::thread;

// If more than 1 output exist then all outputs run in a thread with registered Fn. When all threads terminates
// then the next Id is returned to continue on. Thread terminates on a Parallel or Inclusive Join and End events.
pub(super) fn maybe_parallelize<'a>(
    outputs: Vec<&'a str>,
    func: impl Fn(Vec<&'a str>) -> ExecuteResult<'a> + Sync,
) -> ExecuteResult<'a> {
    if outputs.len() <= 1 {
        return Ok(outputs);
    }

    #[cfg(feature = "parallel")]
    {
        // Diverging gateway
        let (oks, mut errors): (Vec<_>, Vec<_>) = thread::scope(|s| {
            //Start everything first
            let children: Vec<_> = outputs
                .iter()
                .map(|output| s.spawn(|| (func)(vec![output])))
                .collect();

            // Collect results
            children
                .into_iter()
                .filter_map(|handle| handle.join().ok())
                .partition(Result::is_ok)
        });

        if let Some(result) = errors.pop() {
            return result;
        }

        Ok(oks
            .into_iter()
            .filter_map(Result::ok)
            .flatten()
            .collect::<Vec<_>>())
    }

    #[cfg(not(feature = "parallel"))]
    {
        func(outputs)
    }
}

// Early return if no id is found.
macro_rules! parallelize_helper {
    ($outputs:expr, $func:expr) => {{
        match crate::process::parallel::maybe_parallelize($outputs, $func)?.as_slice() {
            &[id, ..] => id,
            _ => continue,
        }
    }};
}

pub(super) use parallelize_helper;
