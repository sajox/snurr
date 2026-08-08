# Changes

## Main branch (BREAKING CHANGES)

### Updated documentation

  - Removed pictures.
  - Include documentation.md in crate doc.
  - Added changelog.

### API changes

  - Renamed Enum `With` to `Inclusive`.
  - New enum types `Exclusive`, `Task`, `IntermediateEvent`.
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

- Removed `Arc` and `Mutex` usage in Snurr and let the user choose. Callbacks now use `&T` instead of `Arc<Mutex<T>>`.
    - Change your process type to `Process::<Arc<Mutex<YourModel>>>::new` to maintain compatibility with existing code.
    - And to extract result
        ```rust
        let data = Arc::into_inner(process_result)
            .ok_or("arc has more than one strong reference")?
            .into_inner()
            .map_err(|_| "mutex is poisoned")?;
        ```
- Removed `Data<T>` type as it was `Arc<Mutex<T>>`.

### Example

- Added an example how to create a task that use an external snurr process.

### Fixes

- Subprocess now terminates the process prematurely and triggers the interrupting boundary event on end events with a symbol.
- When a task or gateways had been assigned a `name` in BPMN, it was not possible to use the BPMN `id` instead when register functions.
    - Using an BPMN `id` can be fragile as it can regenerate depending on what you do in the BPMN tool.
- Return an Error on XML errors. (did just log before)

### Dependencies

- Updated to quick-xml new API

## Version 0.14

- Make the parallel join less permissive for BPMN design errors and respect the number of tokens required before proceeding. Returns an error if gateway is stalled.
- Added support for cancel event. Used in transactions.
- Early detection if multiple none start events is found in same process.
- Removed unused errors.
- Added documentation and images in crates.io release
