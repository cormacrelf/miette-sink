use miette::{Diagnostic, GraphicalReportHandler, GraphicalTheme, Severity};
use std::num::ParseIntError;
use thiserror::Error;

use miette_sink::{Reportable, Reported};

use miette_sink::{DiagnosticSink, ResultExt, VecSink};
/// A little shorthand, as a treat. This sink holds Diag, an enum of warning/error.
type Sink<'a> = &'a mut VecSink<Diag>;

// // use this instead to use BoxedDiag (= Box<dyn Diagnostic + 'static>) everywhere.
// use miette_sink::dynamic::{DynDiagnosticSink, VecSink, DynResultExt};
// type Sink<'a> = &'a mut dyn DynDiagnosticSink;

#[derive(Debug, Error, Diagnostic)]
enum Warning {
    #[error("not the end of the world, but nonzero values scare me")]
    #[diagnostic(severity = "warning", code(app::non_zero))]
    NonZero,
    #[error("the number is not the optimal size")]
    #[diagnostic(transparent)]
    IntSize(#[from] IntSize),
}

#[derive(Debug, Error, Diagnostic)]
enum Error {
    #[error("negative numbers are not allowed")]
    #[diagnostic(
        help("we only accept positive numbers. try something that isn't negative"),
        code(app::negative)
    )]
    Negative,

    #[error(transparent)]
    #[diagnostic(code(app::input::parse), help("input an integer only please"))]
    Parse(#[from] ParseIntError),

    #[error(transparent)]
    #[diagnostic(code(app::input::io))]
    Io(#[from] std::io::Error),
}

/// Make a nice enum to wrap both the warnings and the errors
#[derive(Debug, Error, Diagnostic)]
enum Diag {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Warning(#[from] Warning),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Error(#[from] Error),
}

/// Error is reportable, Warning is not.
/// This lets us use `.report(sink)?` but not on Warning, where it would mean
/// we end up returning Err but only having warnings in the sink.
impl Reportable for Error {}

#[derive(Debug, Error, Diagnostic)]
enum IntSize {
    #[diagnostic(
        severity = "warning",
        code(app::int_size::too_large),
        help("i don't like numbers > 5")
    )]
    #[error("{0} is a bit big")]
    TooLarge(i32),
    #[diagnostic(
        severity = "warning",
        code(app::int_size::way_too_large),
        help("i REALLY don't like numbers > 10")
    )]
    #[error("{0} is WAY too big")]
    WayTooLarge(i32),
}

/// Returns an error if negative.
fn check_non_negative(num: i32) -> Result<(), Error> {
    if num < 0 {
        Err(Error::Negative)
    } else {
        Ok(())
    }
}

/// Checks an integer is an appropriate size (this will end up as a warning, not an error)
fn validate_integer_size(num: i32) -> Result<(), IntSize> {
    if num > 10 {
        return Err(IntSize::WayTooLarge(num));
    } else if num > 5 {
        return Err(IntSize::TooLarge(num));
    }
    Ok(())
}

fn validate_generally(value: i32, sink: Sink) -> Result<(), Reported<Error>> {
    // Note that check_non_negative does not need to know about the error handling mechanism.
    // It just needs to create any Err type that's convertible to an Error.
    //
    // With Result::report(sink)?, obviously the ? might return from the function.
    // If it does, the error is reported to the sink.
    check_non_negative(value).report(sink)?;

    // ... and if not, we continue, and emit some warnings if relevant.
    if value != 0 {
        // you can report errors and warnings manually.
        sink.report(Warning::NonZero);
    }
    if let Err(size_warning) = validate_integer_size(value).map_err(Warning::from) {
        sink.report(size_warning);
    }

    Ok(())
}

fn parser_or_whatever(num: &str, sink: Sink) -> Result<i32, Reported<Error>> {
    // Reported<Error> up the top means anything that can be Into::into()'d into an Error can
    // be reported.
    let value: i32 = num.parse().report(sink)?;

    // Obviously that includes an actual Error instance itself, as it is trivially
    // convertible.
    fn does_not_error() -> Result<(), Error> {
        Ok(())
    }
    does_not_error().report(sink)?;

    // // Note that if we do exit via ?, the library can't enforce that you have actually reported
    // // a diagnostic that's an actual Error to phone home about, not just a warning.
    // // So you could have a top level thing to check if the sink is empty, and if so, report an
    // // error like "hmm, no errors found, but unsuccessful nevertheless". Or, you could separate
    // // your error type from your large warnings + errors type, as we have done with Error vs
    // // Warning.

    // Functions that have the same `-> Result<_, Reported<D>>` structure can be embedded with
    // plain ? as usual: Reported<D> is a promise that the error has been reported already. No need
    // to .report(sink)? it again. If you try, rustc will yell at you; Reported<D> does not
    // implement From<Anything>, nor can it be reported itself, as it does not implement
    // Diagnostic. This ensures you do not report already-reported errors.
    validate_generally(value, sink)?;

    Ok(value)
}

fn value_reader(sink: Sink) -> Result<String, Reported<Error>> {
    let mut input = String::new();
    println!();
    println!("=> please enter an integer below, non-negative please");
    std::io::stdin().read_line(&mut input).report(sink)?;
    Ok(input)
}

fn full_program(sink: Sink) -> Result<i32, Reported<Error>> {
    let input = value_reader(sink)?;
    let result = parser_or_whatever(input.trim(), sink)?;
    Ok(result)
}

fn main() {
    let graphical = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
    let mut sink = VecSink::new(graphical);
    loop {
        sink.clear();
        let result = full_program(&mut sink);
        println!("the returned result = {:?}", result);
        println!("{:?}", sink);
        println!("here are the diagnostics we just printed:");
        for diag in sink.diagnostics() {
            println!(
                "- [{:?}] {:?}",
                diag.severity().unwrap_or(Severity::Error),
                diag
            );
        }
    }
}
