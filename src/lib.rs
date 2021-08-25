#![feature(try_trait_v2)]

use log::Log;

struct MyLogger;

use core::convert::Infallible;
use core::ops::ControlFlow;
use core::ops::FromResidual;
use core::ops::Try;
use std::sync::Arc;

use miette::DiagnosticReportPrinter;

use miette::{Diagnostic, Severity, Source};
use thiserror::Error as ThisError;

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
    printer: Box<dyn DiagnosticReportPrinter>,
}

impl VecSink {
    pub fn new(printer: impl DiagnosticReportPrinter) -> Self {
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

struct LoggingSink<I> {
    inner: I,
}

impl<I: DiagnosticSink> LoggingSink<I> {
    fn new(inner: I) -> Self {
        Self { inner }
    }
}

impl<I: DiagnosticSink> DiagnosticSink for LoggingSink<I> {
    fn report_boxed(&mut self, diag: Box<dyn Diagnostic>) {
        let level = match diag.severity().unwrap_or(Severity::Error) {
            Severity::Advice => log::Level::Info,
            Severity::Warning => log::Level::Warn,
            Severity::Error => log::Level::Error,
        };
        log::log!(target: "DiagnosticSink", level, "reported {}", diag);
        self.inner.report_boxed(diag);
    }
}

use core::fmt;
impl fmt::Debug for VecSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diag in &self.inner {
            self.printer.debug(diag.as_ref(), f)?
        }
        Ok(())
    }
}
impl<I: fmt::Debug> fmt::Debug for LoggingSink<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

trait SizedDiagnostic: Diagnostic + Sized + 'static {}

impl<T> SizedDiagnostic for T where T: Diagnostic + Sized + 'static {}

trait Reportable: Sized {
    type DiagType;
    type ReportedType;
    fn report<'s, S: DiagnosticSink + ?Sized>(self, sink: &'s mut S) -> Self::ReportedType;
}

impl<T, D: SizedDiagnostic> Reportable for Result<T, D> {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Reported;

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::*;

    use miette::{GraphicalReportPrinter, NamedSource, SourceOffset, SourceSpan};

    #[derive(Debug, ThisError, Diagnostic)]
    enum Error {
        #[error("well ok")]
        #[diagnostic(severity = "warning", code(app::well_ok))]
        WellOk,
        #[error("um no")]
        #[diagnostic(code(app::um_no))]
        UmNo,
        #[error("that's not right")]
        #[diagnostic(code(app::thats_not_right))]
        ThatsNotRight,
        #[error("too large: {0}")]
        #[diagnostic(code(app::too_large), help("try doing it better next time?"))]
        TooLarge(u32),

        #[error("warning only, for {value}")]
        #[diagnostic(severity = "warning", code(app::too_large), help("try doing it better next time?"))]
        WarningOnly {
            value: u32,
            src: Arc<String>,
            #[snippet(src, message("This is the part that broke"))]
            snip: SourceSpan,
            #[highlight(snip, label("This bit here"))]
            bad_bit: SourceSpan,
        },
    }

    #[test]
    fn test() {

        fn fallible_operation(num: u32) -> Result<u32, Error> {
            if num > 5 {
                return Err(Error::TooLarge(num));
            }
            Ok(num)
        }

        fn parser_or_whatever(src: &Arc<String>, num: u32, sink: &mut dyn DiagnosticSink) -> Result<u32, Reported> {
            let val = fallible_operation(num).report(sink)?;
            if val > 3 {
                let snip = SourceSpan::new(0.into(), src.len().into());
                // within the snip
                let bad = SourceSpan::new(4.into(), 3.into());
                sink.report(Error::WarningOnly {
                    value: val,
                    src: src.clone(),
                    snip,
                    bad_bit: bad,
                });
            }
            Ok(val)
        }

        let graphical = miette::GraphicalReportPrinter::new_themed(miette::GraphicalTheme::unicode());
        let src = Arc::new(String::from("one two three\nfour five six"));
        let sink = VecSink::new(graphical);
        let mut sink = LoggingSink::new(sink);
        let failed = parser_or_whatever(&src, 4, &mut sink);
        assert_eq!(failed, Ok(4));

        println!("{:?}", sink);
        for diag in sink.inner.into_inner() {
            println!("{}", diag);
        }
    }
}
