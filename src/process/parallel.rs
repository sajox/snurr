use super::{
    engine::{ExecuteData, ExecuteResult},
    Process,
};
use crate::error::Error;

impl Process {
    // When all threads terminates then the next id is returned to continue on.
    // Thread terminates on a Parallel or Inclusive Join and End events.
    pub(super) fn maybe_parallelize<'a, T>(
        &'a self,
        start_ids: Vec<&'a usize>,
        data: &ExecuteData<'a, T>,
    ) -> Result<Option<&'a usize>, Error>
    where
        T: Send,
    {
        let result: ExecuteResult<'_> = {
            #[cfg(feature = "parallel")]
            {
                use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
                // Fork and collect result
                let (oks, mut errors): (Vec<_>, Vec<_>) = start_ids
                    .par_iter()
                    .map(|output| self.execute(vec![output], data))
                    .partition(Result::is_ok);

                if let Some(result) = errors.pop() {
                    result
                } else {
                    Ok(oks
                        .into_iter()
                        .filter_map(Result::ok)
                        .flatten()
                        .collect::<Vec<_>>())
                }
            }

            #[cfg(not(feature = "parallel"))]
            {
                self.execute(start_ids, data)
            }
        };

        Ok(match result?.as_slice() {
            // Test if diagram is balanced in debug mode.
            // Check if all result ids is the same. If not. Match on row below.
            #[cfg(debug_assertions)]
            arr @ [id, ..] if arr.iter().all(|item| item == id) => Some(id),
            _arr @ [id, ..] => {
                #[cfg(debug_assertions)]
                log::error!("unbalanced BPMN diagram detected! {:?}", _arr);
                Some(id)
            }
            // process finished
            _ => None,
        })
    }
}

macro_rules! parallelize_helper {
    ($self:expr, $outputs:expr, $data:expr, $ty:expr, $noi:expr) => {
        if $outputs.len() <= 1 {
            $outputs
                .first()
                .ok_or_else(|| Error::MissingOutput($ty.to_string(), $noi.to_string()))?
        } else {
            match $self.maybe_parallelize($outputs.ids(), $data)? {
                Some(val) => val,
                None => continue,
            }
        }
    };
}

pub(super) use parallelize_helper;
