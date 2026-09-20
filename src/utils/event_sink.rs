use std::sync::OnceLock;

use druid::ExtEventSink;

static EVENT_SINK: OnceLock<ExtEventSink> = OnceLock::new();

pub fn set_event_sink(sink: ExtEventSink) {
    if EVENT_SINK.set(sink).is_err() {
        tracing::error!("event sink was already initialized; ignoring redundant call");
    }
}

pub fn get_event_sink() -> Option<ExtEventSink> {
    let sink = EVENT_SINK.get();

    if sink.is_none() {
        tracing::error!("event sink accessed before initialization");
    }

    sink.cloned()
}
