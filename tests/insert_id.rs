use helpers::run_with_tracing;
use mocks::MockDefaultEvent;

mod helpers;
mod mocks;

#[test]
fn includes_custom_insert_ids() {
    let insert_id = "my-new-event".to_string();
    let events =
        run_with_tracing::<MockDefaultEvent>(|| tracing::info!(insert_id = insert_id, "hello!"))
            .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert!(event.insert_id.is_some());
    assert_eq!(event.insert_id, Some(insert_id));
}

#[test]
fn stringifies_primitive_insert_id_values() {
    let insert_id = 123;
    let events =
        run_with_tracing::<MockDefaultEvent>(|| tracing::info!(insert_id = insert_id, "hello!"))
            .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert!(event.insert_id.is_some());
    assert_eq!(event.insert_id, Some(insert_id.to_string()));
}

#[test]
fn omits_insert_id_by_default() {
    let events = run_with_tracing::<MockDefaultEvent>(|| tracing::info!("hello!"))
        .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert!(event.insert_id.is_none());
}
