use snurr::{Data, Error, Eventhandler, Process, Result, Symbol, TaskResult, With};

const COUNT_1: &str = "Count 1";
const COUNT_2: &str = "Count 2";
const COUNT_3: &str = "Count 3";
const COUNT_4: &str = "Count 4";

const YES: With = With::Flow("YES");
const A: With = With::Flow("A");
const B: With = With::Flow("B");
const C: With = With::Flow("C");

#[derive(Debug, Default)]
struct Counter {
    count: u32,
}

fn func_cnt(value: u32) -> impl Fn(Data<Counter>) -> TaskResult {
    move |input| {
        input.lock().unwrap().count += value;
        None
    }
}

#[test]
fn one_task() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));

    let bpmn = Process::new("tests/files/one_task.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 1);
    Ok(())
}

#[test]
fn two_task() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));

    let bpmn = Process::new("tests/files/two_task.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));

    let bpmn = Process::new("tests/files/subprocess.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess_nested() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));

    let bpmn = Process::new("tests/files/subprocess_nested.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess_message_end() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_gateway("CHOOSE", |_| Default::default());

    let bpmn = Process::new("tests/files/subprocess_message_end.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess_error_message_end() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, |_| Some(("Overflow", Symbol::Error).into()));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    let bpmn = Process::new("tests/files/subprocess_error_message_end.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 5);
    Ok(())
}

#[cfg(feature = "trace")]
#[test]
fn replay_process_trace() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, |_| Some(("Overflow", Symbol::Error).into()));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    let bpmn = Process::new("tests/files/subprocess_error_message_end.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    let trace_result = Process::replay_trace(&handler, Counter::default(), &pr.trace);
    assert_eq!(pr.result.count, trace_result.count);

    Ok(())
}

#[test]
fn exclusive_gateway_default_path() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    // Empty vec run default path
    handler.add_gateway("CHOOSE", |_| With::Default);

    let bpmn = Process::new("tests/files/exclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 4);
    Ok(())
}

#[test]
fn exclusive_gateway() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));
    handler.add_gateway("CHOOSE", move |_| YES);

    let bpmn = Process::new("tests/files/exclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn exclusive_gateway_with_id() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    // Navigate by Bpmn diagram Id instead of by Name.
    handler.add_gateway("CHOOSE", move |_| With::Flow("Flow_15z7fe3"));

    let bpmn = Process::new("tests/files/exclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn exclusive_gateway_with_gateway_converge() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));
    handler.add_task(COUNT_4, func_cnt(4));
    handler.add_gateway("CHOOSE", |_| YES);

    let bpmn = Process::new("tests/files/exclusive_gateway_with_gateway_converge.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 7);
    Ok(())
}

#[test]
fn exclusive_gateway_with_task_converge() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));
    handler.add_task(COUNT_4, func_cnt(4));
    handler.add_gateway("CHOOSE", |_| YES);

    let bpmn = Process::new("tests/files/exclusive_gateway_with_task_converge.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 7);
    Ok(())
}

#[test]
fn inclusive_gateway_default_path() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    // Empty vec run default path
    handler.add_gateway("CHOOSE", |_| With::Default);

    let bpmn = Process::new("tests/files/inclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 4);
    Ok(())
}

#[test]
fn inclusive_gateway() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    handler.add_gateway("CHOOSE", |_| vec!["YES", "NO"].into());

    let bpmn = Process::new("tests/files/inclusive_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 6);
    Ok(())
}

#[test]
fn inclusive_gateway_split_end() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    handler.add_gateway("Gateway_0jgakfl", |_| With::Fork(vec!["YES", "NO"]));

    let bpmn = Process::new("tests/files/inclusive_gateway_split_end.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 6);
    Ok(())
}

#[test]
fn inclusive_gateway_no_output() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();

    // Empty vec run default path
    handler.add_gateway("Gateway_0qmfmmo", |_| With::Default);

    let bpmn = Process::new("tests/files/inclusive_gateway_no_output.bpmn")?;
    if let Err(error) = bpmn.run(&handler, Counter::default()) {
        assert!(
            matches!(error, Error::MissingOutput(_, _)),
            "Expected missing output"
        );
    } else {
        panic!("Expected an error");
    }

    Ok(())
}

#[test]
fn inclusive_join_fork() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_gateway("GW A", |_| vec!["A", "B"].into());
    handler.add_gateway("GW B", |_| vec!["A", "B", "C"].into());
    let bpmn = Process::new("tests/files/inclusive_join_fork.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 6);
    Ok(())
}

#[test]
fn inclusive_join_fork_gwb_one_flow() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_gateway("GW A", |_| vec!["A", "B"].into());
    handler.add_gateway("GW B", |_| vec!["A"].into());
    let bpmn = Process::new("tests/files/inclusive_join_fork.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 4);
    Ok(())
}

#[test]
fn inclusive_join_fork_gateway_verify_sync() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_gateway("GW A", |_| vec!["A", "B"].into());
    handler.add_gateway("GW B", |input| {
        // Make sure that the gateway is only executed once. Do not recommend to mutate data in gateway.
        input.lock().unwrap().count += 1;
        vec!["A", "B", "C"].into()
    });
    let bpmn = Process::new("tests/files/inclusive_join_fork.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 7);
    Ok(())
}

#[test]
fn parallell_gateway() -> Result<()> {
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
fn error_handling() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, |_| Some(Symbol::Error.into()));
    handler.add_task(COUNT_2, |_| Some(Symbol::Error.into()));
    handler.add_task(COUNT_3, func_cnt(3));

    let bpmn = Process::new("tests/files/error_handling.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn two_boundary_timer_thrown() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, |_| Some(("Timeout", Symbol::Timer).into()));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    let bpmn = Process::new("tests/files/two_boundary.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn two_boundary_error_thrown() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, |_| Some(("Error", Symbol::Error).into()));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    let bpmn = Process::new("tests/files/two_boundary.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 2);
    Ok(())
}

#[test]
fn multiple_boundaries_same_symbol() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, |_| Some(("M2", Symbol::Message).into()));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_task(COUNT_3, func_cnt(3));

    let bpmn = Process::new("tests/files/multiple_boundaries_same_symbol.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn intermediate_event() -> Result<()> {
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
fn two_process_pools() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));

    let bpmn = Process::new("tests/files/two_process_pools.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn subprocess_external_link_fail() -> snurr::Result<()> {
    let handler: Eventhandler<Counter> = Eventhandler::default();
    let bpmn = Process::new("tests/files/subprocess_external_link_fail.bpmn")?;

    if let Err(error) = bpmn.run(&handler, Counter::default()) {
        let result = matches!(error, Error::MissingIntermediateCatchEvent(symbol, name) if symbol == "Link" && name == "Link 2");
        assert!(result, "Expected Link Symbol with name Link 2");
    } else {
        panic!("Expected an error");
    }
    Ok(())
}

#[test]
fn showcase() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task("Timeout 1", |_| Some(Symbol::Timer.into()));
    handler.add_gateway("RUN ALL", |_| With::Fork(vec!["A", "B"]));
    handler.add_gateway("RUN A", |_| A);

    handler.add_gateway("RUN DEFAULT", |_| With::Default);

    let bpmn = Process::new("tests/files/showcase.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 15);
    Ok(())
}

#[test]
fn task_fork() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));

    let bpmn = Process::new("tests/files/task_fork.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 6);
    Ok(())
}

#[test]
fn fork_explosion() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));

    let bpmn = Process::new("tests/files/fork_explosion.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 33);
    Ok(())
}

#[test]
fn process_end_with_symbol() -> Result<()> {
    let handler: Eventhandler<Counter> = Eventhandler::default();
    let bpmn = Process::new("tests/files/process_end_with_symbol.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 0);
    Ok(())
}

#[test]
fn inclusive_gateway_not_all_joined() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_gateway("RUN ALL", |_| With::Fork(vec!["A", "B"]));
    handler.add_gateway("RUN C", |_| C);

    let bpmn = Process::new("tests/files/inclusive_gateway_not_all_joined.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn parallel_gateway_not_all_joined() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    let bpmn = Process::new("tests/files/parallel_gateway_not_all_joined.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 4);
    Ok(())
}

#[test]
fn parallel_gateway_not_all_joined_inverse() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    let bpmn = Process::new("tests/files/parallel_gateway_not_all_joined_inverse.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 4);
    Ok(())
}

#[test]
fn parallel_multi() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    let bpmn = Process::new("tests/files/parallel_multi.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 5);
    Ok(())
}

#[test]
fn parallel_join_fork() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    let bpmn = Process::new("tests/files/parallel_join_fork.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 6);
    Ok(())
}

#[ignore]
#[test]
fn parallel_unbalanced() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    let bpmn = Process::new("tests/files/parallel_unbalanced.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 5);
    Ok(())
}

#[test]
fn conditional_sequence_flows() -> Result<()> {
    let failed = Process::new("tests/files/conditional_sequence_flows.bpmn").is_err();
    assert!(failed, "Expected an error");
    Ok(())
}

#[test]
fn exclusive_gateway_merging_branching() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));
    handler.add_gateway("BRANCHING", |_| A);
    handler.add_gateway("MERGE AND BRANCH", |_| B);

    let bpmn = Process::new("tests/files/exclusive_gateway_merging_branching.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn event_gateway() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));
    handler.add_task(COUNT_2, func_cnt(2));

    handler.add_gateway("JUNIOR GATEKEEPER", |_| {
        ("Investigate", Symbol::Message).into()
    });

    handler.add_gateway("SENIOR GATEKEEPER", |_| ("Sleeping", Symbol::Timer).into());

    let bpmn = Process::new("tests/files/event_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 2);
    Ok(())
}

#[test]
fn event_gateway_blank_symbol() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_3, func_cnt(3));

    handler.add_gateway("JUNIOR GATEKEEPER", |_| Symbol::Timer.into());

    let bpmn = Process::new("tests/files/event_gateway.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 3);
    Ok(())
}

#[test]
fn single_flow() -> Result<()> {
    let mut handler: Eventhandler<Counter> = Eventhandler::default();
    handler.add_task(COUNT_1, func_cnt(1));

    handler.add_gateway("GW A", |_| A);
    handler.add_gateway("GW B", |_| A);
    handler.add_gateway("GW C", |_| A);
    handler.add_gateway("GW D", |_| A);

    let bpmn = Process::new("tests/files/single_flow.bpmn")?;
    let pr = bpmn.run(&handler, Counter::default())?;
    assert_eq!(pr.result.count, 18);
    Ok(())
}
