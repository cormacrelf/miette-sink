//! you SINK miette?
//!

use core::fmt;
use core::marker::PhantomData;
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

/// A DiagnosticSink that
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

    /// When you don't have a `&mut dyn DiagnosticSink`, you can equivalently use this method.
    pub fn report(&mut self, diag: impl Diagnostic + 'static)
    where
        Self: Sized,
    {
        self.report_boxed(Box::new(diag));
    }

    pub fn clear(&mut self) {
        self.inner.clear()
    }
    pub fn into_inner(self) -> Vec<Box<dyn Diagnostic>> {
        self.inner
    }
    pub fn diagnostics(&self) -> &[Box<dyn Diagnostic>] {
        &self.inner
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

pub trait ResultExt<D>: Sized {
    type ReportedType;
    fn report<S: DiagnosticSink + ?Sized>(self, sink: &mut S) -> Self::ReportedType;
}

/// An `Err(E)` where `E` can be converted to the relevant `D: Diagnostic` is reportable.
impl<T, E: Into<D>, D: Reportable + Sized + 'static> ResultExt<D> for Result<T, E> {
    type ReportedType = Result<T, Reported<D>>;
    fn report<S: DiagnosticSink + ?Sized>(self, sink: &mut S) -> Self::ReportedType {
        match self {
            Ok(x) => Ok(x),
            Err(e) => {
                sink.report_boxed(Box::new(e.into()));
                Err(Reported::new())
            }
        }
    }
}

/// A marker trait for things that can be contained in a [Reported].
pub trait Reportable: Diagnostic {}

/// A zero-sized error type representing a diagnostic that has been sent to a DiagnosticSink.
///
/// We specifically do not implement Diagnostic here, because we want `Result<_,
/// Reported>::report()` to be an error. You can't report Reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reported<D: Reportable>(PhantomData<D>);

impl<D: Reportable> Reported<D> {
    fn new() -> Self {
        Reported(PhantomData)
    }
}

impl<D: Reportable> std::error::Error for Reported<D> {}
impl<D: Reportable> fmt::Display for Reported<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Errors were reported")
    }
}
