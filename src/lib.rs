//! you SINK miette?
//!

use core::fmt;
use miette::Diagnostic;
use miette::ReportHandler;

pub trait DiagnosticSink {
    fn report_boxed(&mut self, diag: Box<dyn Diagnostic>);
}

impl<'a> dyn DiagnosticSink + 'a {
    pub fn report(&mut self, diag: impl Diagnostic + 'static) {
        self.report_boxed(Box::new(diag));
    }
}

pub struct VecSink {
    inner: Vec<Box<dyn Diagnostic>>,
    printer: Box<dyn ReportHandler>,
}

impl VecSink {
    pub fn new(printer: impl ReportHandler) -> Self {
        Self {
            inner: Vec::new(),
            printer: Box::new(printer),
        }
    }
    pub fn into_inner(self) -> Vec<Box<dyn Diagnostic>> {
        self.inner
    }
}

impl DiagnosticSink for VecSink {
    fn report_boxed(&mut self, diag: Box<dyn Diagnostic>) {
        self.inner.push(diag);
    }
}

impl fmt::Debug for VecSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diag in &self.inner {
            self.printer.debug(diag.as_ref(), f)?
        }
        Ok(())
    }
}

trait SizedDiagnostic: Diagnostic + Sized + 'static {}

impl<T> SizedDiagnostic for T where T: Diagnostic + Sized + 'static {}

pub trait Reportable: Sized {
    type DiagType;
    type ReportedType;
    fn report<'s, S: DiagnosticSink + ?Sized>(self, sink: &'s mut S) -> Self::ReportedType;
}

impl<T, D: Diagnostic + Sized + 'static> Reportable for Result<T, D> {
    type DiagType = D;
    type ReportedType = Result<T, Reported>;
    fn report<'s, S: DiagnosticSink + ?Sized>(self, sink: &'s mut S) -> Self::ReportedType {
        match self {
            Ok(x) => Ok(x),
            Err(d) => {
                sink.report_boxed(Box::new(d));
                Err(Reported)
            }
        }
    }
}

/// A zero-sized error type representing a diagnostic that has been sent to a DiagnosticSink.
///
/// We specifically do not implement Diagnostic here, because we want `Result<_,
/// Reported>::report()` to be an error. You can't report Reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reported;
impl std::error::Error for Reported {}
impl fmt::Display for Reported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Errors were reported")
    }
}

