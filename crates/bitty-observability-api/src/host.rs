/// The Core-owned observation host contract.
///
/// The accepted `W-71` contract names the read-only observation seam and its
/// record ([`Observation`](crate::event::Observation)), sink
/// ([`ObservationSink`](crate::sink::ObservationSink)), and bounded, versioned
/// subscription ([`Subscription`](crate::sink::Subscription)). This trait is
/// the host side of that seam: what Core would implement to push records, and
/// what a consumer attaches to before reading through its sink. The exact
/// spelling remains parked by the accepted contract; only the shape is fixed
/// here. Nothing in this crate claims Core integration.
///
/// # Doc contract
///
/// What Core pushes: one bounded [`Observation`](crate::event::Observation)
/// per boundary event, carrying a bounded kind, a bounded attribute set, and
/// optional plugin or subsystem attribution. Records are redacted at emission
/// (sensitive by default; raw PTY bytes, clipboard content, raw environment
/// data, and raw input are never captured), never carry a secret, a raw PTY
/// byte stream, or a capability handle, and are never emitted on a hot path.
/// The seam carries observations about Core-owned mechanisms only; operating
/// system metrics sampling is not part of this trait.
///
/// What a consumer reads: a caller first negotiates a [`Subscription`] through
/// the existing [`subscribe`](crate::sink::subscribe) function (default-deny
/// gate plus fail-closed version intersection), the host admits it with
/// [`ObservationHost::attach`], and the consumer then reads through
/// [`ObservationHost::drain_to`] into its [`ObservationSink`](crate::sink::ObservationSink):
/// buffered records arrive in order via `on_observation`, an explicit loss
/// report arrives via `on_drop` only when data was dropped or truncated, and
/// teardown arrives via `on_detached` on [`ObservationHost::detach`].
///
/// Bounds: every record respects the [`Observation`](crate::event::Observation)
/// construction bounds; each observer is bounded in flight by its
/// subscription's `max_records_in_flight`; loss drops oldest first and is
/// always explicit through [`DropReport`](crate::event::DropReport); with no
/// attached observer the host is inert and stores nothing; version attachment
/// fails closed on an empty range intersection; the gate grants `debug.inspect`
/// and `debug.trace` independently and knows no control scope.
///
/// Core mapping: the current Core debug view (sanitized lifecycle snapshot)
/// and trace hub (per-owner bounded event traces) are the Core-side
/// implementations this seam would replace once the `W-71`/`W-100` removal
/// gates pass. The exact record kind strings for those rows and events remain
/// parked to `W-100`/`W-110`; no kind literal is fixed here.
use crate::event::Observation;
use crate::sink::{ContractRange, ObservationSink, Subscription};

/// What Core would push to, and what a consumer attaches to and reads from.
///
/// A host owns the advertised contract range and the per-subscription bounded
/// buffers. It exposes no mutation, control, or handle-passing path beyond
/// the subscription lifecycle: attach, bounded fan-out, ordered drain, and
/// detach. Implementations redact at emission before buffering and hold every
/// bound above; the reference buffering behavior lives in the core layer's
/// `TraceBuffer`.
pub trait ObservationHost {
    /// Returns the contract range this host advertises.
    ///
    /// An observer declares the range it implements; attachment succeeds only
    /// on a non-empty intersection, so this range is the fail-closed anchor
    /// for both [`subscribe`](crate::sink::subscribe) negotiation and
    /// [`ObservationHost::attach`] admission.
    fn contract_range(&self) -> ContractRange;

    /// Returns whether `id` is currently admitted and active.
    fn is_attached(&self, id: u64) -> bool;

    /// Admits an already-authorized subscription.
    ///
    /// The subscription is built first with
    /// [`subscribe`](crate::sink::subscribe), so the default-deny gate and
    /// the version intersection have already passed; admission re-checks the
    /// negotiated version against [`ObservationHost::contract_range`] and
    /// fails closed. Returns `false` (storing nothing) when the subscription
    /// is inactive, when its version falls outside the advertised range, or
    /// when `id` is already admitted. Returns `true` when admitted.
    fn attach(&mut self, subscription: Subscription) -> bool;

    /// Fans out one record to every attached, active subscription in order.
    ///
    /// The record is redacted at emission before buffering and bounded by
    /// each subscription's in-flight limit with drop-oldest discipline; every
    /// loss is accumulated for the next [`ObservationHost::drain_to`] report.
    /// With no attached observer this is a no-op: nothing is stored.
    fn emit(&mut self, observation: Observation);

    /// Delivers the buffered records for `id` into `sink`.
    ///
    /// Records arrive in order via `on_observation`, followed by `on_drop`
    /// only when the accumulated report is non-empty; delivery resets the
    /// report. An unknown or detached `id` makes no sink call.
    fn drain_to(&mut self, id: u64, sink: &mut dyn ObservationSink);

    /// Removes `id`, discards its buffer, and notifies `sink`.
    ///
    /// Calls `on_detached` exactly once when a subscription was removed and
    /// returns `true`; returns `false` without any sink call when `id` was
    /// not attached. Discarded records are not delivered. Later emissions
    /// reach the removed observer no more.
    fn detach(&mut self, id: u64, sink: &mut dyn ObservationSink) -> bool;
}
