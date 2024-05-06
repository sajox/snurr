use snurr::{Eventhandler, Process, Symbol, TaskCallback};

#[derive(Debug, Default)]
struct Counter {
    count: u32,
}

const FUNC_CNT_1: TaskCallback<Counter> = |input| {
    input.lock().unwrap().count += 1;
    Ok(())
};

const FUNC_CNT_2: TaskCallback<Counter> = |input| {
    input.lock().unwrap().count += 2;
    Ok(())
};

const FUNC_CNT_3: TaskCallback<Counter> = |input| {
    input.lock().unwrap().count += 3;
    Ok(())
};

const FUNC_CNT_4: TaskCallback<Counter> = |input| {
    input.lock().unwrap().count += 4;
    Ok(())
};

const FUNC_ERROR: TaskCallback<Counter> = |_| Err(Symbol::Error);
const FUNC_TIMER: TaskCallback<Counter> = |_| Err(Symbol::Timer);

#[test]
fn one_task() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_CNT_1);

    let bpmn = Process::new("tests/files/one_task.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 1);
    Ok(())
}

#[test]
fn two_task() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_CNT_1);
    handler.add_task("Count 2", FUNC_CNT_2);

    let bpmn = Process::new("tests/files/two_task.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_CNT_1);
    handler.add_task("Count 2", FUNC_CNT_2);

    let bpmn = Process::new("tests/files/subprocess.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess_message_end() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_CNT_1);
    handler.add_task("Count 2", FUNC_CNT_2);

    let bpmn = Process::new("tests/files/subprocess_message_end.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess_error_message_end() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_ERROR);
    handler.add_task("Count 2", FUNC_CNT_2);
    handler.add_task("Count 3", FUNC_CNT_3);

    let bpmn = Process::new("tests/files/subprocess_error_message_end.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 5);
    Ok(())
}

#[test]
fn replay_process_trace() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_ERROR);
    handler.add_task("Count 2", FUNC_CNT_2);
    handler.add_task("Count 3", FUNC_CNT_3);

    let bpmn = Process::new("tests/files/subprocess_error_message_end.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    let trace_result = Process::replay_trace(&handler, Counter::default(), &pr.trace);
    assert_eq!(pr.result.count, trace_result.count);

    Ok(())
}

#[test]
fn exclusive_gateway_default_path() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_CNT_1);
    handler.add_task("Count 2", FUNC_CNT_2);
    handler.add_task("Count 3", FUNC_CNT_3);

    // Empty response. Default path
    handler.add_gateway("CHOOSE", |_| vec![]);

    let bpmn = Process::new("tests/files/exclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 4);
    Ok(())
}

#[test]
fn exclusive_gateway() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_CNT_1);
    handler.add_task("Count 2", FUNC_CNT_2);
    handler.add_task("Count 3", FUNC_CNT_3);
    handler.add_gateway("CHOOSE", |_| vec!["YES"]);

    let bpmn = Process::new("tests/files/exclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn exclusive_gateway_with_gateway_converge() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_CNT_1);
    handler.add_task("Count 2", FUNC_CNT_2);
    handler.add_task("Count 3", FUNC_CNT_3);
    handler.add_task("Count 4", FUNC_CNT_4);
    handler.add_gateway("CHOOSE", |_| vec!["YES"]);

    let bpmn = Process::new("tests/files/exclusive_gateway_with_gateway_converge.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 7);
    Ok(())
}

#[test]
fn exclusive_gateway_with_task_converge() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_CNT_1);
    handler.add_task("Count 2", FUNC_CNT_2);
    handler.add_task("Count 3", FUNC_CNT_3);
    handler.add_task("Count 4", FUNC_CNT_4);
    handler.add_gateway("CHOOSE", |_| vec!["YES"]);

    let bpmn = Process::new("tests/files/exclusive_gateway_with_task_converge.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 7);
    Ok(())
}

#[test]
fn inclusive_gateway_default_path() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_CNT_1);
    handler.add_task("Count 2", FUNC_CNT_2);
    handler.add_task("Count 3", FUNC_CNT_3);

    // Empty response. Default path
    handler.add_gateway("CHOOSE", |_| vec![]);

    let bpmn = Process::new("tests/files/inclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 4);
    Ok(())
}

#[test]
fn inclusive_gateway() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_CNT_1);
    handler.add_task("Count 2", FUNC_CNT_2);
    handler.add_task("Count 3", FUNC_CNT_3);

    // Empty response. Default path
    handler.add_gateway("CHOOSE", |_| vec!["YES", "NO"]);

    let bpmn = Process::new("tests/files/inclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 6);
    Ok(())
}

#[test]
fn inclusive_gateway_split_end() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_CNT_1);
    handler.add_task("Count 2", FUNC_CNT_2);
    handler.add_task("Count 3", FUNC_CNT_3);

    // Empty response. Default path
    handler.add_gateway("Gateway_0jgakfl", |_| vec!["YES", "NO"]);

    let bpmn = Process::new("tests/files/inclusive_gateway_split_end.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 6);
    Ok(())
}

#[test]
fn inclusive_gateway_no_output() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();

    // Empty response. Default path
    handler.add_gateway("Gateway_0qmfmmo", |_| vec![]);

    let bpmn = Process::new("tests/files/inclusive_gateway_no_output.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default()).is_err();
    assert!(pr, "Expected an error");
    Ok(())
}

#[test]
fn parallell_gateway() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_CNT_1);
    handler.add_task("Count 2", FUNC_CNT_2);
    handler.add_task("Count 3", FUNC_CNT_3);
    handler.add_task("Count 4", FUNC_CNT_4);

    // All paths is taken. No need to register gateway.

    let bpmn = Process::new("tests/files/parallell_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 10);
    Ok(())
}

#[test]
fn error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_ERROR);
    handler.add_task("Count 2", FUNC_ERROR);
    handler.add_task("Count 3", FUNC_CNT_3);

    let bpmn = Process::new("tests/files/error_handling.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn two_boundary_timer_thrown() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_TIMER);
    handler.add_task("Count 2", FUNC_CNT_2);
    handler.add_task("Count 3", FUNC_CNT_3);

    let bpmn = Process::new("tests/files/two_boundary.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn two_boundary_error_thrown() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task("Count 1", FUNC_ERROR);
    handler.add_task("Count 2", FUNC_CNT_2);
    handler.add_task("Count 3", FUNC_CNT_3);

    let bpmn = Process::new("tests/files/two_boundary.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 2);
    Ok(())
}
