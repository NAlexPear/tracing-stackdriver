use helpers::run_with_tracing;
use mocks::MockDefaultEvent;
use std::collections::BTreeMap;

mod helpers;
mod mocks;

#[test]
fn nests_labels() {
    let mut labels = BTreeMap::new();
    labels.insert("foo", "bar".to_string());
    labels.insert("baz", "luhrmann".to_string());

    let events = run_with_tracing::<MockDefaultEvent>(|| {
        tracing::info!(
            labels.foo = labels.get("foo"),
            labels.baz = labels.get("baz"),
            "hello!"
        )
    })
    .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert!(event.labels.get("foo").is_some());
    assert_eq!(event.labels.get("foo"), labels.get("foo"));
    assert!(event.labels.get("baz").is_some());
    assert_eq!(event.labels.get("baz"), labels.get("baz"));
}

#[test]
fn stringifies_primitive_label_values() {
    let number = 2;
    let boolean = false;
    let string = "a short note";
    let events = run_with_tracing::<MockDefaultEvent>(|| {
        tracing::info!(
            labels.number = number,
            labels.boolean = boolean,
            labels.string = string,
            "hello!"
        )
    })
    .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert_eq!(event.labels.get("number"), Some(&number.to_string()));
    assert_eq!(event.labels.get("boolean"), Some(&boolean.to_string()));
    assert_eq!(event.labels.get("string"), Some(&string.to_string()));
}

#[test]
fn omits_labels_by_default() {
    let events = run_with_tracing::<MockDefaultEvent>(|| tracing::info!("hello!"))
        .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert!(event.labels.is_empty());
}
