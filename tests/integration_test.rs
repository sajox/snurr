use snurr::{Data, Error, Process, Result, Symbol, TaskResult};

const COUNT_1: &str = "Count 1";
const COUNT_2: &str = "Count 2";
const COUNT_3: &str = "Count 3";
const COUNT_4: &str = "Count 4";

const A: Option<&str> = Some("A");
const B: Option<&str> = Some("B");

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
    let bpmn = Process::new("tests/files/one_task.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 1);
    Ok(())
}

#[test]
fn two_task() -> Result<()> {
    let bpmn = Process::new("tests/files/two_task.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 3);
    Ok(())
}

#[test]
fn subprocess() -> Result<()> {
    let bpmn = Process::new("tests/files/subprocess.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 3);
    Ok(())
}

#[test]
fn subprocess_nested() -> Result<()> {
    let bpmn = Process::new("tests/files/subprocess_nested.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 3);
    Ok(())
}

#[test]
fn subprocess_message_end() -> Result<()> {
    let bpmn = Process::new("tests/files/subprocess_message_end.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .exclusive("CHOOSE", |_| Default::default())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 3);
    Ok(())
}

#[test]
fn subprocess_error_message_end() -> Result<()> {
    let bpmn = Process::new("tests/files/subprocess_error_message_end.bpmn")?
        .task(COUNT_1, |_| Some(("Overflow", Symbol::Error).into()))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 5);
    Ok(())
}

#[test]
fn exclusive_gateway_default_path() -> Result<()> {
    let bpmn = Process::new("tests/files/exclusive_gateway.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .exclusive("CHOOSE", |_| Default::default())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 4);
    Ok(())
}

#[test]
fn exclusive_gateway() -> Result<()> {
    let bpmn = Process::new("tests/files/exclusive_gateway.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .exclusive("CHOOSE", |_| "YES".into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 3);
    Ok(())
}

#[test]
fn exclusive_gateway_with_id() -> Result<()> {
    let bpmn = Process::new("tests/files/exclusive_gateway.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        // Navigate by Bpmn diagram Id instead of by Name.
        .exclusive("CHOOSE", |_| "Flow_15z7fe3".into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 3);
    Ok(())
}

#[test]
fn exclusive_gateway_with_gateway_converge() -> Result<()> {
    let bpmn = Process::new("tests/files/exclusive_gateway_with_gateway_converge.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .task(COUNT_4, func_cnt(4))
        .exclusive("CHOOSE", |_| "YES".into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 7);
    Ok(())
}

#[test]
fn exclusive_gateway_with_task_converge() -> Result<()> {
    let bpmn = Process::new("tests/files/exclusive_gateway_with_task_converge.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .task(COUNT_4, func_cnt(4))
        .exclusive("CHOOSE", |_| "YES".into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 7);
    Ok(())
}

#[test]
fn inclusive_gateway_default_path() -> Result<()> {
    let bpmn = Process::new("tests/files/inclusive_gateway.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        // Empty vec run default path
        .inclusive("CHOOSE", |_| Default::default())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 5);
    Ok(())
}

#[test]
fn inclusive_gateway() -> Result<()> {
    let bpmn = Process::new("tests/files/inclusive_gateway.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .inclusive("CHOOSE", |_| vec!["YES", "NO"].into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 7);
    Ok(())
}

#[test]
fn inclusive_gateway_same_flow_used_multiple_times() -> Result<()> {
    let bpmn = Process::new("tests/files/inclusive_gateway.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .inclusive("CHOOSE", |_| vec!["YES", "YES", "NO", "NO"].into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 7);
    Ok(())
}

#[test]
fn inclusive_gateway_split_end() -> Result<()> {
    let bpmn = Process::new("tests/files/inclusive_gateway_split_end.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .inclusive("Gateway_0jgakfl", |_| vec!["YES", "NO"].into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 6);
    Ok(())
}

#[test]
fn inclusive_gateway_no_output() -> Result<()> {
    let bpmn = Process::new("tests/files/inclusive_gateway_no_output.bpmn")?
        .task("A", |_| None)
        .task("B", |_| None)
        // Empty vec run default path
        .inclusive("Gateway_0qmfmmo", |_| Default::default())
        .build()?;
    if let Err(error) = bpmn.run(Counter::default()) {
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
    let bpmn = Process::new("tests/files/inclusive_join_fork.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .inclusive("GW A", |_| vec!["A", "B"].into())
        .inclusive("GW B", |_| vec!["A", "B", "C"].into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 6);
    Ok(())
}

#[test]
fn inclusive_join_fork_gwb_one_flow() -> Result<()> {
    let bpmn = Process::new("tests/files/inclusive_join_fork.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .inclusive("GW A", |_| vec!["A", "B"].into())
        .inclusive("GW B", |_| "A".into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 4);
    Ok(())
}

#[test]
fn inclusive_join_fork_gateway_verify_sync() -> Result<()> {
    let bpmn = Process::new("tests/files/inclusive_join_fork.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .inclusive("GW A", |_| vec!["A", "B"].into())
        .inclusive("GW B", |input| {
            // Make sure that the gateway is only executed once. Do not recommend to mutate data in gateway.
            input.lock().unwrap().count += 1;
            vec!["A", "B", "C"].into()
        })
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 7);
    Ok(())
}

#[test]
fn parallel_inclusive_join_fork() -> Result<()> {
    let bpmn = Process::new("tests/files/parallel_inclusive_join_fork.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .inclusive("GW A", |_| vec!["A", "B"].into())
        .inclusive("GW AA", |_| vec!["A", "B", "C"].into())
        .inclusive("GW AAA", |_| vec!["A", "B", "C", "D", "E"].into())
        .inclusive("GW B", |_| vec!["A", "B", "C"].into())
        .inclusive("GW BB", |_| vec!["A", "B"].into())
        .inclusive("GW BBB", |_| vec!["A", "B", "C", "D"].into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 23);
    Ok(())
}

#[test]
fn inclusive_with_parallel() -> Result<()> {
    let bpmn = Process::new("tests/files/inclusive_with_parallel.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .inclusive("GW A", |_| vec!["A", "B"].into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 5);
    Ok(())
}

#[test]
fn parallell_gateway() -> Result<()> {
    let bpmn = Process::new("tests/files/parallell_gateway.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .task(COUNT_4, func_cnt(4))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 10);
    Ok(())
}

#[test]
fn error_handling() -> Result<()> {
    let bpmn = Process::new("tests/files/error_handling.bpmn")?
        .task(COUNT_1, |_| Some(Symbol::Error.into()))
        .task(COUNT_2, |_| Some(Symbol::Error.into()))
        .task(COUNT_3, func_cnt(3))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 3);
    Ok(())
}

#[test]
fn two_boundary_timer_thrown() -> Result<()> {
    let bpmn = Process::new("tests/files/two_boundary.bpmn")?
        .task(COUNT_1, |_| Some(("Timeout", Symbol::Timer).into()))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 3);
    Ok(())
}

#[test]
fn two_boundary_error_thrown() -> Result<()> {
    let bpmn = Process::new("tests/files/two_boundary.bpmn")?
        .task(COUNT_1, |_| Some(("Error", Symbol::Error).into()))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 2);
    Ok(())
}

#[test]
fn multiple_boundaries_same_symbol() -> Result<()> {
    let bpmn = Process::new("tests/files/multiple_boundaries_same_symbol.bpmn")?
        .task(COUNT_1, |_| Some(("M2", Symbol::Message).into()))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 3);
    Ok(())
}

#[test]
fn intermediate_event() -> Result<()> {
    let bpmn = Process::new("tests/files/intermediate_event.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 6);
    Ok(())
}

#[test]
fn two_process_pools() -> Result<()> {
    let bpmn = Process::new("tests/files/two_process_pools.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 3);
    Ok(())
}

#[test]
fn subprocess_external_link_fail() -> snurr::Result<()> {
    let bpmn = Process::new("tests/files/subprocess_external_link_fail.bpmn")?.build()?;
    if let Err(error) = bpmn.run(Counter::default()) {
        assert!(
            matches!(error, Error::MissingIntermediateCatchEvent(symbol, name) if symbol == "Link" && name == "Link 2"),
            "Expected Link Symbol with name Link 2"
        );
    } else {
        panic!("Expected an error");
    }
    Ok(())
}

#[test]
fn showcase() -> Result<()> {
    let bpmn = Process::new("tests/files/showcase.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task("Timeout 1", |_| Some(Symbol::Timer.into()))
        .inclusive("RUN ALL", |_| vec!["A", "B"].into())
        .inclusive("RUN A", |_| "A".into())
        .exclusive("RUN DEFAULT", |_| Default::default())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 16);
    Ok(())
}

#[test]
fn task_fork() -> Result<()> {
    let bpmn = Process::new("tests/files/task_fork.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 6);
    Ok(())
}

#[test]
fn fork_explosion() -> Result<()> {
    let bpmn = Process::new("tests/files/fork_explosion.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 33);
    Ok(())
}

#[test]
fn process_end_with_symbol() -> Result<()> {
    let bpmn = Process::new("tests/files/process_end_with_symbol.bpmn")?.build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 0);
    Ok(())
}

#[test]
fn inclusive_gateway_not_all_joined() -> Result<()> {
    let bpmn = Process::new("tests/files/inclusive_gateway_not_all_joined.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .inclusive("RUN ALL", |_| vec!["A", "B"].into())
        .exclusive("RUN C", |_| "C".into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 3);
    Ok(())
}

#[test]
fn parallel_gateway_not_all_joined() -> Result<()> {
    let bpmn = Process::new("tests/files/parallel_gateway_not_all_joined.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 4);
    Ok(())
}

#[test]
fn parallel_gateway_not_all_joined_inverse() -> Result<()> {
    let bpmn = Process::new("tests/files/parallel_gateway_not_all_joined_inverse.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 4);
    Ok(())
}

#[test]
fn parallel_multi() -> Result<()> {
    let bpmn = Process::new("tests/files/parallel_multi.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 7);
    Ok(())
}

#[test]
fn parallel_join_fork() -> Result<()> {
    let bpmn = Process::new("tests/files/parallel_join_fork.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 6);
    Ok(())
}

#[test]
fn parallel_parallel_join_fork() -> Result<()> {
    let bpmn = Process::new("tests/files/parallel_parallel_join_fork.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 23);
    Ok(())
}

#[test]
fn parallel_one_in_and_out() -> Result<()> {
    let bpmn = Process::new("tests/files/parallel_one_in_and_out.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 1);
    Ok(())
}

#[test]
fn conditional_sequence_flows() -> Result<()> {
    let failed = Process::<Counter>::new("tests/files/conditional_sequence_flows.bpmn").is_err();
    assert!(failed, "Expected an error");
    Ok(())
}

#[test]
fn exclusive_gateway_merging_branching() -> Result<()> {
    let bpmn = Process::new("tests/files/exclusive_gateway_merging_branching.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .exclusive("BRANCHING", |_| A)
        .exclusive("MERGE AND BRANCH", |_| B)
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 3);
    Ok(())
}

#[test]
fn event_gateway() -> Result<()> {
    let bpmn = Process::new("tests/files/event_gateway.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .task(COUNT_2, func_cnt(2))
        .task(COUNT_3, func_cnt(3))
        .task("Investigate", |_| None)
        .event_based("JUNIOR GATEKEEPER", |_| {
            ("Investigate", Symbol::Message).into()
        })
        .event_based("SENIOR GATEKEEPER", |_| ("Sleeping", Symbol::Timer).into())
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 2);
    Ok(())
}

#[test]
fn single_flow() -> Result<()> {
    let bpmn = Process::new("tests/files/single_flow.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .exclusive("GW A", |_| A)
        .exclusive("GW B", |_| A)
        .exclusive("GW C", |_| A)
        .exclusive("GW D", |_| A)
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 18);
    Ok(())
}

#[test]
fn terminate_event() -> Result<()> {
    let bpmn = Process::new("tests/files/terminate_event.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .exclusive("Terminate?", |_| "YES".into())
        .build()?;
    let result = bpmn.run(Counter::default())?;

    // NOTE 2 or 3 is OK result. The order in concurrent scenarios might differ.
    assert!(matches!(result.count, 2 | 3));
    Ok(())
}

#[test]
fn terminate_event_sub_process() -> Result<()> {
    let bpmn = Process::new("tests/files/terminate_event_sub_process.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .exclusive("Terminate?", |_| "YES".into())
        .build()?;
    let result = bpmn.run(Counter::default())?;

    // NOTE 4 or 5 is OK result. The order in concurrent scenarios might differ.
    assert!(matches!(result.count, 4 | 5));
    Ok(())
}

#[test]
fn startevent_not_first() -> Result<()> {
    // StartEvent out of order in XML file
    let bpmn = Process::new("tests/files/startevent_not_first.bpmn")?
        .task(COUNT_1, func_cnt(1))
        .build()?;
    let result = bpmn.run(Counter::default())?;
    assert_eq!(result.count, 4);
    Ok(())
}
