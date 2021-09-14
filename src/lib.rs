//! you SINK miette?
//!

use core::fmt;
use core::marker::PhantomData;
use miette::Diagnostic;
use miette::ReportHandler;

pub mod dynamic;

pub trait DiagnosticSink<D> {
    fn report(&mut self, diagnostic: D);
}

pub struct VecSink<D> {
    inner: Vec<D>,
    printer: Box<dyn ReportHandler>,
}

impl<D> VecSink<D> {
    pub fn new(printer: impl ReportHandler) -> Self {
        Self {
            inner: vec![],
            printer: Box::new(printer),
        }
    }

    pub fn clear(&mut self) {
        self.inner.clear()
    }
    pub fn diagnostics(&self) -> &[D] {
        self.inner.as_ref()
    }
    pub fn into_inner(self) -> Vec<D> {
        self.inner
    }
}

impl<S: Diagnostic, D: Into<S>> DiagnosticSink<D> for VecSink<S> {
    fn report(&mut self, diagnostic: D) {
        self.inner.push(diagnostic.into())
    }
}

impl<D: Diagnostic + 'static> fmt::Debug for VecSink<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diag in &self.inner {
            self.printer.debug(diag, f)?
        }
        Ok(())
    }
}

pub trait ResultExt<D: Diagnostic>: Sized {
    type ReportedType;
    fn report<S: DiagnosticSink<D> + ?Sized>(self, sink: &mut S) -> Self::ReportedType;
}

/// An `Err(E)` where `E` can be converted to the relevant `D: Diagnostic` is reportable.
impl<T, E: Into<D>, D: Reportable + Sized + 'static> ResultExt<D> for Result<T, E> {
    type ReportedType = Result<T, Reported<D>>;
    fn report<S: DiagnosticSink<D> + ?Sized>(self, sink: &mut S) -> Self::ReportedType {
        match self {
            Ok(x) => Ok(x),
            Err(e) => {
                let d = e.into();
                let sd = d.into();
                sink.report(sd);
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
