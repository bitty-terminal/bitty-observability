//! Seam conformance: the read-only observation contract against a fake host.
//!
//! A fake host implements [`ObservationHost`] with per-subscription bounded
//! buffers and emission-time redaction; a fake producer emits [`Observation`]
//! records; a fake sink implements [`ObservationSink`]. The suite proves what
//! Core would push, what a consumer reads, and the bounds that hold between
//! them: fail-closed attach, redaction at emission, explicit drops, ordered
//! delivery, detach notification, and an inert host with no observer.

use bitty_observability::{
    AttributeValue, AuthorizationError, AuthorizationGate, BufferLimits, ContractRange,
    ContractVersion, DropReport, EventKind, ObservabilityCapability, Observation, ObservationHost,
    ObservationSink, ObserverOrigin, RecordStatus, Subscription, SubscriptionError, TraceBuffer,
    redact_observation, subscribe,
};
use bitty_observability_api::{DEFAULT_MAX_RECORD_BYTES, DEFAULT_MAX_TOTAL_BYTES};
use std::collections::BTreeMap;

/// Fake host: what Core would implement, backed by bounded buffers.
struct FakeHost {
    range: ContractRange,
    allow: Vec<&'static str>,
    attached: BTreeMap<u64, (Subscription, TraceBuffer)>,
}

impl FakeHost {
    fn new(range: ContractRange, allow: Vec<&'static str>) -> Self {
        Self {
            range,
            allow,
            attached: BTreeMap::new(),
        }
    }
}

impl ObservationHost for FakeHost {
    fn contract_range(&self) -> ContractRange {
        self.range
    }

    fn is_attached(&self, id: u64) -> bool {
        self.attached
            .get(&id)
            .is_some_and(|(subscription, _)| subscription.is_active())
    }

    fn attach(&mut self, subscription: Subscription) -> bool {
        if !subscription.is_active() {
            return false;
        }
        let version = subscription.contract_version();
        if self
            .range
            .intersect(&ContractRange::new(version, version))
            .is_none()
        {
            return false;
        }
        let id = subscription.id();
        if self.attached.contains_key(&id) {
            return false;
        }
        let limits = BufferLimits::new(
            subscription.max_records_in_flight(),
            DEFAULT_MAX_RECORD_BYTES,
            DEFAULT_MAX_TOTAL_BYTES,
        );
        self.attached
            .insert(id, (subscription, TraceBuffer::new(limits)));
        true
    }

    fn emit(&mut self, observation: Observation) {
        for (subscription, buffer) in self.attached.values_mut() {
            if !subscription.is_active() {
                continue;
            }
            buffer.push(redact_observation(&observation, &self.allow));
        }
    }

    fn drain_to(&mut self, id: u64, sink: &mut dyn ObservationSink) {
        let Some((_, buffer)) = self.attached.get_mut(&id) else {
            return;
        };
        let (records, report) = buffer.drain();
        for record in &records {
            sink.on_observation(record);
        }
        if !report.is_empty() {
            sink.on_drop(&report);
        }
    }

    fn detach(&mut self, id: u64, sink: &mut dyn ObservationSink) -> bool {
        if self.attached.remove(&id).is_some() {
            sink.on_detached();
            true
        } else {
            false
        }
    }
}

/// Fake consumer: what an observer reads through its sink.
#[derive(Default)]
struct FakeSink {
    records: Vec<Observation>,
    reports: Vec<DropReport>,
    detached: u32,
}

impl ObservationSink for FakeSink {
    fn on_observation(&mut self, observation: &Observation) {
        self.records.push(observation.clone());
    }

    fn on_drop(&mut self, report: &DropReport) {
        self.reports.push(*report);
    }

    fn on_detached(&mut self) {
        self.detached += 1;
    }
}

fn host_range() -> ContractRange {
    ContractRange::new(ContractVersion::new(0, 1), ContractVersion::new(0, 1))
}

fn observer_range() -> ContractRange {
    ContractRange::new(ContractVersion::new(0, 0), ContractVersion::new(0, 3))
}

fn granted_trace_gate() -> AuthorizationGate {
    let mut gate = AuthorizationGate::default_deny();
    gate.grant(ObservabilityCapability::DebugTrace);
    gate
}

fn attach_trace(host: &mut FakeHost, gate: &AuthorizationGate, id: u64, max_in_flight: usize) {
    let subscription = subscribe(
        gate,
        id,
        ObservabilityCapability::DebugTrace,
        ObserverOrigin::InProcess,
        observer_range(),
        host.contract_range(),
        max_in_flight,
    )
    .expect("compatible observer attaches");
    assert!(host.attach(subscription));
}

fn observation(kind: &str, sequence: u64) -> Observation {
    Observation::new(EventKind::new(kind), sequence)
}

#[test]
fn compatible_observer_attaches_and_reads_emitted_records_in_order() {
    let mut host = FakeHost::new(host_range(), vec!["action"]);
    let gate = granted_trace_gate();
    attach_trace(&mut host, &gate, 1, 16);
    assert!(host.is_attached(1));

    for sequence in 1..=3 {
        host.emit(
            observation("terminal.opened", sequence)
                .with_attribute("action", AttributeValue::Text("open".to_string())),
        );
    }

    let mut sink = FakeSink::default();
    host.drain_to(1, &mut sink);

    let sequences: Vec<u64> = sink.records.iter().map(|record| record.sequence).collect();
    assert_eq!(sequences, vec![1, 2, 3]);
    assert!(sink.reports.is_empty());
    assert_eq!(sink.detached, 0);

    let again = subscribe(
        &gate,
        1,
        ObservabilityCapability::DebugTrace,
        ObserverOrigin::InProcess,
        observer_range(),
        host.contract_range(),
        16,
    )
    .expect("negotiation still succeeds");
    assert!(!host.attach(again), "duplicate id is refused");
}

#[test]
fn incompatible_version_fails_closed_at_negotiation_and_admission() {
    let mut host = FakeHost::new(host_range(), vec!["action"]);
    let gate = granted_trace_gate();

    let refused = subscribe(
        &gate,
        1,
        ObservabilityCapability::DebugTrace,
        ObserverOrigin::InProcess,
        ContractRange::new(ContractVersion::new(1, 0), ContractVersion::new(2, 0)),
        host.contract_range(),
        16,
    );
    assert_eq!(refused, Err(SubscriptionError::IncompatibleVersion));

    let out_of_range = Subscription::new(
        2,
        ObservabilityCapability::DebugTrace,
        ObserverOrigin::InProcess,
        ContractVersion::new(2, 0),
        16,
    );
    assert!(!host.attach(out_of_range));
    assert!(!host.is_attached(2));

    host.emit(observation("terminal.opened", 1));
    let mut sink = FakeSink::default();
    host.drain_to(2, &mut sink);
    assert!(sink.records.is_empty());
    assert!(sink.reports.is_empty());
}

#[test]
fn ungranted_observer_is_refused_and_host_stays_inert() {
    let mut host = FakeHost::new(host_range(), vec!["action"]);
    let gate = AuthorizationGate::default_deny();

    let refused = subscribe(
        &gate,
        1,
        ObservabilityCapability::DebugTrace,
        ObserverOrigin::InProcess,
        observer_range(),
        host.contract_range(),
        16,
    );
    assert_eq!(
        refused,
        Err(SubscriptionError::Authorization(
            AuthorizationError::MissingCapability(ObservabilityCapability::DebugTrace)
        ))
    );

    host.emit(observation("terminal.opened", 1));
    assert!(!host.is_attached(1));
    let mut sink = FakeSink::default();
    host.drain_to(1, &mut sink);
    assert!(sink.records.is_empty());
}

#[test]
fn out_of_process_observer_needs_consent_before_reading() {
    let mut host = FakeHost::new(host_range(), vec!["action"]);
    let mut gate = granted_trace_gate();

    let refused = subscribe(
        &gate,
        1,
        ObservabilityCapability::DebugTrace,
        ObserverOrigin::OutOfProcess,
        observer_range(),
        host.contract_range(),
        16,
    );
    assert_eq!(
        refused,
        Err(SubscriptionError::Authorization(
            AuthorizationError::ConsentRequired
        ))
    );

    gate.grant_consent();
    let subscription = subscribe(
        &gate,
        1,
        ObservabilityCapability::DebugTrace,
        ObserverOrigin::OutOfProcess,
        observer_range(),
        host.contract_range(),
        16,
    )
    .expect("consented observer attaches");
    assert!(host.attach(subscription));

    host.emit(observation("terminal.opened", 1));
    let mut sink = FakeSink::default();
    host.drain_to(1, &mut sink);
    assert_eq!(sink.records.len(), 1);
}

#[test]
fn emission_is_redacted_at_the_host_boundary() {
    let mut host = FakeHost::new(host_range(), vec!["action", "pty.raw"]);
    let gate = granted_trace_gate();
    attach_trace(&mut host, &gate, 1, 16);

    let mut produced = observation("terminal.input", 1);
    produced.insert_attribute("action", AttributeValue::Text("paste".to_string()));
    produced.insert_attribute("token", AttributeValue::Text("secret://abc".to_string()));
    produced.insert_attribute("pty.raw", AttributeValue::Text("raw-bytes".to_string()));
    host.emit(produced);

    let mut sink = FakeSink::default();
    host.drain_to(1, &mut sink);
    assert_eq!(sink.records.len(), 1);
    let record = &sink.records[0];
    assert_eq!(
        record.attributes().get("action"),
        Some(&AttributeValue::Text("paste".to_string()))
    );
    assert_eq!(
        record.attributes().get("token"),
        Some(&AttributeValue::Redacted)
    );
    assert_eq!(
        record.attributes().get("pty.raw"),
        Some(&AttributeValue::Redacted),
        "denied fields stay redacted even when allowlisted"
    );
    assert!(!format!("{record:?}").contains("secret://abc"));
    assert!(!format!("{record:?}").contains("raw-bytes"));
}

#[test]
fn drops_are_explicit_and_oldest_first() {
    let mut host = FakeHost::new(host_range(), vec!["action"]);
    let gate = granted_trace_gate();
    attach_trace(&mut host, &gate, 1, 2);

    for sequence in 1..=5 {
        host.emit(observation("terminal.opened", sequence));
    }

    let mut sink = FakeSink::default();
    host.drain_to(1, &mut sink);
    let sequences: Vec<u64> = sink.records.iter().map(|record| record.sequence).collect();
    assert_eq!(sequences, vec![4, 5]);
    assert_eq!(sink.reports.len(), 1);
    assert_eq!(sink.reports[0].dropped, 3);
}

#[test]
fn oversized_record_is_truncated_explicitly() {
    let mut host = FakeHost::new(host_range(), vec!["field0", "field1", "field2"]);
    let gate = granted_trace_gate();
    attach_trace(&mut host, &gate, 1, 16);

    // Three maximal attributes exceed the per-record byte bound even though
    // each one respects the construction bound on its own.
    let mut produced = observation("terminal.opened", 1);
    for index in 0..3 {
        produced.insert_attribute(
            format!("field{index}"),
            AttributeValue::Text("x".repeat(4096)),
        );
    }
    assert!(produced.size_hint() > DEFAULT_MAX_RECORD_BYTES);
    host.emit(produced);

    let mut sink = FakeSink::default();
    host.drain_to(1, &mut sink);
    assert_eq!(sink.records.len(), 1);
    assert_eq!(sink.records[0].status(), RecordStatus::Truncated);
    assert_eq!(sink.reports.len(), 1);
    assert_eq!(sink.reports[0].truncated, 1);
}

#[test]
fn detach_stops_delivery_and_notifies_once() {
    let mut host = FakeHost::new(host_range(), vec!["action"]);
    let gate = granted_trace_gate();
    attach_trace(&mut host, &gate, 1, 16);

    host.emit(observation("terminal.opened", 1));
    let mut sink = FakeSink::default();
    host.drain_to(1, &mut sink);
    assert_eq!(sink.records.len(), 1);

    assert!(host.detach(1, &mut sink));
    assert_eq!(sink.detached, 1);
    assert!(!host.is_attached(1));

    host.emit(observation("terminal.opened", 2));
    host.drain_to(1, &mut sink);
    assert_eq!(
        sink.records.len(),
        1,
        "detached observer reads nothing more"
    );
    assert!(!host.detach(1, &mut sink), "second detach reports absent");
    assert_eq!(sink.detached, 1);
}

#[test]
fn inert_host_with_no_observer_stores_nothing() {
    let mut host = FakeHost::new(host_range(), vec!["action"]);
    host.emit(observation("terminal.opened", 1));

    let gate = granted_trace_gate();
    attach_trace(&mut host, &gate, 1, 16);
    host.emit(observation("terminal.opened", 2));

    let mut sink = FakeSink::default();
    host.drain_to(1, &mut sink);
    let sequences: Vec<u64> = sink.records.iter().map(|record| record.sequence).collect();
    assert_eq!(sequences, vec![2], "no retroactive delivery");
}
