use super::{
    engine::{ExecuteData, ExecuteResult},
    Process,
};
use crate::error::Error;

impl Process {
    // If more than 1 start id exist then all id:s run in parallel with rayon. When all threads terminates then the
    // next id is returned to continue on. Thread terminates on a Parallel or Inclusive Join and End events.
    pub(super) fn maybe_parallelize<'a, T>(
        &'a self,
        start_ids: Vec<&'a str>,
        data: &'a ExecuteData<'a, T>,
    ) -> Result<Option<&'a str>, Error>
    where
        T: Send,
    {
        let result: ExecuteResult<'_> = {
            if start_ids.len() <= 1 {
                return Ok(start_ids.first().copied());
            } else {
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
    ($self:expr, $start_ids:expr, $data:expr) => {
        match $self.maybe_parallelize($start_ids, $data)? {
            Some(val) => val,
            None => continue,
        }
    };
}

pub(super) use parallelize_helper;
