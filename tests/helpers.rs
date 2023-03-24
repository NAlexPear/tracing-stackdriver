// FIXME: make this a simple function instead
#[macro_export]
macro_rules! run_with_tracing {
    (|| $expression:expr ) => {{
        lazy_static! {
            static ref BUFFER: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let make_writer = || writer::MockWriter(&BUFFER);
        let stackdriver = tracing_stackdriver::layer().with_writer(make_writer);
        let subscriber = Registry::default().with(stackdriver);

        tracing::subscriber::with_default(subscriber, || $expression);

        &BUFFER
            .try_lock()
            .expect("Couldn't get lock on test write target")
            .to_vec()
    }};
}

#[macro_export]
macro_rules! run_with_tracing_source_location {
    (|| $expression:expr ) => {{
        lazy_static! {
            static ref BUFFER: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let make_writer = || writer::MockWriter(&BUFFER);
        let stackdriver = tracing_stackdriver::layer()
            .with_settings(tracing_stackdriver::Configuration::default().enable_source_location())
            .with_writer(make_writer);
        let subscriber = Registry::default().with(stackdriver);

        tracing::subscriber::with_default(subscriber, || $expression);

        &BUFFER
            .try_lock()
            .expect("Couldn't get lock on test write target")
            .to_vec()
    }};
}
