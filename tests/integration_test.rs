use snurr::{Data, Eventhandler, Process, Symbol, TaskResult, With};

const COUNT_1: &str = "Count 1";
const COUNT_2: &str = "Count 2";
const COUNT_3: &str = "Count 3";
const COUNT_4: &str = "Count 4";

const YES: With = With::Name("YES");
const NO: With = With::Name("NO");
const A: With = With::Name("A");
const B: With = With::Name("B");
const C: With = With::Name("C");

#[derive(Debug, Default)]
struct Counter {
    count: u32,
}

fn func_cnt(value: u32) -> impl Fn(Data<Counter>) -> TaskResult {
    move |input| {
        input.lock().unwrap().count += value;
        Ok(())
    }
}

fn func_error(_: Data<Counter>) -> TaskResult {
    Err(Symbol::Error)
}

fn func_timer(_: Data<Counter>) -> TaskResult {
    Err(Symbol::Timer)
}

#[test]
fn one_task() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));

    let bpmn = Process::new("tests/files/one_task.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 1);
    Ok(())
}

#[test]
fn two_task() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));

    let bpmn = Process::new("tests/files/two_task.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));

    let bpmn = Process::new("tests/files/subprocess.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess_nested() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));

    let bpmn = Process::new("tests/files/subprocess_nested.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess_message_end() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_gateway("CHOOSE", |_| vec![]);

    let bpmn = Process::new("tests/files/subprocess_message_end.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess_error_message_end() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_error);
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    let bpmn = Process::new("tests/files/subprocess_error_message_end.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 5);
    Ok(())
}

#[test]
fn replay_process_trace() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_error);
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    let bpmn = Process::new("tests/files/subprocess_error_message_end.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    let trace_result = Process::replay_trace(&handler, Counter::default(), &pr.trace);
    assert_eq!(pr.result.count, trace_result.count);

    Ok(())
}

#[test]
fn exclusive_gateway_default_path() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    // Empty vec run default path
    handler.add_gateway("CHOOSE", |_| vec![]);

    let bpmn = Process::new("tests/files/exclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 4);
    Ok(())
}

#[test]
fn exclusive_gateway() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));
    handler.add_gateway("CHOOSE", move |_| vec![YES]);

    let bpmn = Process::new("tests/files/exclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn exclusive_gateway_with_id() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    // Navigate by Bpmn diagram Id instead of by Name.
    handler.add_gateway("CHOOSE", move |_| vec![With::Id("Flow_15z7fe3")]);

    let bpmn = Process::new("tests/files/exclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn exclusive_gateway_with_gateway_converge() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));
    handler.add_task(COUNT_4, func_cnt(4));
    handler.add_gateway("CHOOSE", |_| vec![YES]);

    let bpmn = Process::new("tests/files/exclusive_gateway_with_gateway_converge.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 7);
    Ok(())
}

#[test]
fn exclusive_gateway_with_task_converge() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));
    handler.add_task(COUNT_4, func_cnt(4));
    handler.add_gateway("CHOOSE", |_| vec![YES]);

    let bpmn = Process::new("tests/files/exclusive_gateway_with_task_converge.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 7);
    Ok(())
}

#[test]
fn inclusive_gateway_default_path() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    // Empty vec run default path
    handler.add_gateway("CHOOSE", |_| vec![]);

    let bpmn = Process::new("tests/files/inclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 4);
    Ok(())
}

#[test]
fn inclusive_gateway() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    handler.add_gateway("CHOOSE", |_| vec![YES, NO]);

    let bpmn = Process::new("tests/files/inclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 6);
    Ok(())
}

#[test]
fn inclusive_gateway_split_end() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    handler.add_gateway("Gateway_0jgakfl", |_| vec![YES, NO]);

    let bpmn = Process::new("tests/files/inclusive_gateway_split_end.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 6);
    Ok(())
}

#[test]
fn inclusive_gateway_no_output() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();

    // Empty vec run default path
    handler.add_gateway("Gateway_0qmfmmo", |_| vec![]);

    let bpmn = Process::new("tests/files/inclusive_gateway_no_output.bpmn")?;
    let failed = bpmn.run(&handler, Counter::default()).is_err();
    assert!(failed, "Expected an error");
    Ok(())
}

#[test]
fn parallell_gateway() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));
    handler.add_task(COUNT_4, func_cnt(4));

    // All paths is taken. No need to register gateway.

    let bpmn = Process::new("tests/files/parallell_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 10);
    Ok(())
}

#[test]
fn error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_error);
    handler.add_task(COUNT_2, func_error);
    handler.add_task(COUNT_3, func_cnt(3));

    let bpmn = Process::new("tests/files/error_handling.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn two_boundary_timer_thrown() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_timer);
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    let bpmn = Process::new("tests/files/two_boundary.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn two_boundary_error_thrown() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_error);
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    let bpmn = Process::new("tests/files/two_boundary.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 2);
    Ok(())
}

#[test]
fn intermediate_event() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    let bpmn = Process::new("tests/files/intermediate_event.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 6);
    Ok(())
}

#[test]
fn two_process_pools() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));

    let bpmn = Process::new("tests/files/two_process_pools.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess_external_link_fail() -> Result<(), Box<dyn std::error::Error>> {
    let handler: Eventhandler<Counter> = Eventhandler::default();
    let bpmn = Process::new("tests/files/subprocess_external_link_fail.bpmn")?;

    let failed = bpmn.run(&handler, Counter::default()).is_err();
    assert!(failed, "Expected an error");
    Ok(())
}

#[test]
fn showcase() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task("Timeout 1", |_| Err(Symbol::Timer));
    handler.add_gateway("RUN ALL", |_| vec![A, B]);
    handler.add_gateway("RUN A", |_| vec![A]);

    // Empty vec run default path
    handler.add_gateway("RUN DEFAULT", |_| vec![]);

    let bpmn = Process::new("tests/files/showcase.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 15);
    Ok(())
}

#[test]
fn task_fork() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));

    let bpmn = Process::new("tests/files/task_fork.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 6);
    Ok(())
}

#[test]
fn fork_explosion() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));

    let bpmn = Process::new("tests/files/fork_explosion.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 33);
    Ok(())
}

#[test]
fn process_end_with_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let handler: Eventhandler<Counter> = Eventhandler::default();
    let bpmn = Process::new("tests/files/process_end_with_symbol.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 0);
    Ok(())
}

#[test]
fn inclusive_gateway_not_all_joined() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_gateway("RUN ALL", |_| vec![A, B]);
    handler.add_gateway("RUN C", |_| vec![C]);

    let bpmn = Process::new("tests/files/inclusive_gateway_not_all_joined.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn parallel_gateway_not_all_joined() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    let bpmn = Process::new("tests/files/parallel_gateway_not_all_joined.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 4);
    Ok(())
}

#[ignore]
#[test]
fn parallel_unbalanced() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    let bpmn = Process::new("tests/files/parallel_unbalanced.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 5);
    Ok(())
}

#[test]
fn join_and_fork() -> Result<(), Box<dyn std::error::Error>> {
    let failed = Process::new("tests/files/join_and_fork.bpmn").is_err();
    assert!(failed, "Expected an error");
    Ok(())
}

#[test]
fn conditional_sequence_flows() -> Result<(), Box<dyn std::error::Error>> {
    let failed = Process::new("tests/files/conditional_sequence_flows.bpmn").is_err();
    assert!(failed, "Expected an error");
    Ok(())
}

#[test]
fn exclusive_gateway_merging_branching() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_gateway("BRANCHING", |_| vec![A]);
    handler.add_gateway("MERGE AND BRANCH", |_| vec![B]);

    let bpmn = Process::new("tests/files/exclusive_gateway_merging_branching.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn event_gateway() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));

    handler.add_gateway("JUNIOR GATEKEEPER", |_| {
        vec![With::Symbol(Some("Investigate"), Symbol::Message)]
    });

    handler.add_gateway("SENIOR GATEKEEPER", |_| {
        vec![With::Symbol(Some("Sleeping"), Symbol::Timer)]
    });

    let bpmn = Process::new("tests/files/event_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 2);
    Ok(())
}

#[test]
fn event_gateway_blank_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_3, func_cnt(3));

    handler.add_gateway("JUNIOR GATEKEEPER", |_| {
        vec![With::Symbol(None, Symbol::Timer)]
    });

    let bpmn = Process::new("tests/files/event_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}
