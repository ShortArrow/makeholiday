//! Rule trait, registry, and lint context.
//!
//! A [`Rule`] is a stateless singleton that inspects a [`LintContext`] and
//! pushes findings into a [`DiagnosticSink`]. The fixed v0.2.0 registry is
//! returned by [`all`].

use ics_core::VCalendar;

use crate::diagnostic::{Diagnostic, Severity};
use crate::walker::RawVEventScan;

pub mod rfc5545;
pub mod text;
pub mod vendor;

/// Context shared by every rule for a single lint pass.
///
/// `calendar` is `None` when the tolerant parser could not promote the
/// source to a typed `VCalendar`. Rules that depend on the typed view
/// should bail in that case; rules that work over the raw source text
/// (via `vevent_scans`) continue to run.
pub struct LintContext<'a> {
    pub source: &'a str,
    pub calendar: Option<&'a VCalendar>,
    /// One scan per `VEVENT` block in source order — preserves duplicate
    /// properties and missing required fields, which the typed parser
    /// collapses or rejects.
    pub vevent_scans: &'a [RawVEventScan],
}

/// Sink that rules push diagnostics into. Owns a `Vec<Diagnostic>` so the
/// caller can drop the sink and recover the collected findings.
pub struct DiagnosticSink {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticSink {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn from_vec(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn push(&mut self, d: Diagnostic) {
        self.diagnostics.push(d);
    }

    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

impl Default for DiagnosticSink {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Rule: Sync {
    fn id(&self) -> &'static str;
    fn default_severity(&self) -> Severity;
    fn visit(&self, ctx: &LintContext<'_>, sink: &mut DiagnosticSink);
}

/// The v0.2.0 fixed rule registry.
pub fn all() -> Vec<&'static dyn Rule> {
    vec![
        &rfc5545::RequiredUid,
        &rfc5545::RequiredDtstamp,
        &rfc5545::RequiredDtstart,
        &rfc5545::DuplicateSummary,
        &rfc5545::DuplicateDtstart,
        &rfc5545::ConflictingEndAndDuration,
        &rfc5545::EndBeforeStart,
        &rfc5545::EmptySummary,
        &vendor::MicrosoftOnly,
        &vendor::GoogleOnly,
        &vendor::IcloudOnly,
        &vendor::UnrecognizedX,
        &text::Bom,
        &text::DoubleEscape,
        &text::UnescapedCommaInSummary,
        &text::UnescapedSemicolonInSummary,
    ]
}
