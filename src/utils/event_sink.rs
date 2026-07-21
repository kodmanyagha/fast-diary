use std::sync::OnceLock;

use druid::ExtEventSink;

static EVENT_SINK: OnceLock<ExtEventSink> = OnceLock::new();

/// Must be called once, before any widget tries to use [`get_event_sink`].
pub fn set_event_sink(sink: ExtEventSink) {
    if EVENT_SINK.set(sink).is_err() {
        panic!("event sink was already initialized");
    }
}

pub fn get_event_sink() -> ExtEventSink {
    EVENT_SINK
        .get()
        .expect("event sink accessed before initialization")
        .clone()
}
