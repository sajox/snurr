# Changes

## Main branch (BREAKING CHANGES)

- Updated documentation.
- API changes
  - Renamed Enum `With` to `Inclusive`
  - New enum type `Exclusive`
  - New enum type `Task`. Slightly less verbose when a Task Boundary is used.
  - New enum type `IntermediateEvent`
  - Support for `Panic`. Enables terminating the process from task and gateways. The process returns a RunTimeError::Panic containing the specified error.
  - Using `Cow<'static, str>` instead of `&'static str` to be more flexible with different types of strings.
  - Added convenient factory methods for cases where conversions via `From` are not used.
- Error redesign. Example output below from anyhow crate.

    New
    ```text
    Error: error reading `examples/counter.bpmn`

    Caused by:
        0: error parsing
        1: error on line 16
        2: could not create bpmn type
        3: tag `sequenceFlow` missing attribute id
    ```

    Old
    ```text
    Error: BPMN type sequenceFlow missing id
    ```

- Removed Arc and Mutex usage in Snurr and let the user choose. Callbacks now use `&T` instead of `Arc<Mutex<T>>`.
    - Change your process type to `Process::<Arc<Mutex<YourModel>>>::new` to maintain compatibility with existing code.
    - And to extract result
        ```rust ignore
        let data = Arc::into_inner(process_result) // FAIL if Arc has more than one strong reference
                    .ok_or(YourError::NoProcessResult)? 
                    .into_inner() // FAIL if Mutex is poisoned
                    .map_err(|_| YourError::NoProcessResult)?;
        ```
- Removed `Data<T>` type as it was `Arc<Mutex<T>>`.

## Version 0.14

- Make the parallel join less permissive for BPMN design errors and respect the number of tokens required before proceeding. Returns an error if gateway is stalled.
- Added support for cancel event. Used in transactions.
- Early detection if multiple none start events is found in same process.
- Removed unused errors.
- Added documentation and images in crates.io release
