use super::*;

type BoxedDiag = Box<dyn Diagnostic + 'static>;

pub trait DynDiagnosticSink {
    fn report_boxed(&mut self, diag: BoxedDiag);
}

impl<'a> dyn DynDiagnosticSink + 'a {
    pub fn report(&mut self, diag: impl Diagnostic + 'static) {
        self.report_boxed(Box::new(diag));
    }
}

pub trait DynResultExt<D: Diagnostic>: Sized {
    type ReportedType;
    fn report(self, sink: &mut dyn DynDiagnosticSink) -> Self::ReportedType;
}

/// An `Err(E)` where `E` can be converted to the relevant `D: Diagnostic` is reportable.
impl<T, E: Into<D>, D: Reportable + Sized + 'static> DynResultExt<D> for Result<T, E> {
    type ReportedType = Result<T, Reported<D>>;
    fn report(self, sink: &mut dyn DynDiagnosticSink) -> Self::ReportedType {
        match self {
            Ok(x) => Ok(x),
            Err(e) => {
                let d = e.into();
                sink.report_boxed(Box::new(d));
                Err(Reported::new())
            }
        }
    }
}

pub struct VecSink {
    inner: Vec<BoxedDiag>,
    printer: Box<dyn ReportHandler>,
}

impl DynDiagnosticSink for VecSink {
    fn report_boxed(&mut self, diag: Box<dyn Diagnostic>) {
        self.inner.push(diag);
    }
}

impl VecSink {
    /// When you don't have a `&mut dyn DynDiagnosticSink`, you can equivalently use this method.
    pub fn report(&mut self, diag: impl Diagnostic + 'static)
    where
        Self: Sized,
    {
        self.report_boxed(Box::new(diag));
    }

    pub fn new(printer: impl ReportHandler) -> Self {
        Self {
            inner: vec![],
            printer: Box::new(printer),
        }
    }

    pub fn clear(&mut self) {
        self.inner.clear()
    }
    pub fn diagnostics(&self) -> &[BoxedDiag] {
        self.inner.as_ref()
    }
    pub fn into_inner(self) -> Vec<BoxedDiag> {
        self.inner
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
