use miette_sink::Reported;
use thiserror::Error;
use miette::{Diagnostic, GraphicalTheme, GraphicalReportHandler, SourceSpan};

use miette_sink::DiagnosticSink;
use miette_sink::VecSink;
use miette_sink::Reportable;

#[derive(Debug, Error, Diagnostic)]
enum Warning {
    #[error("idk, if it isn't zero i'm not sure...")]
    #[diagnostic(severity = "warning", code(app::non_zero))]
    NonZero,
    #[error("warning only, for {value}")]
    #[diagnostic(
        severity = "warning",
        code(app::warning_only),
        help("try doing it better next time?")
    )]
    WarningOnly {
        value: i32,
        src: &'static str, // for ease
        #[snippet(src, message("This is the part that broke"))]
        snip: SourceSpan,
        #[highlight(snip, label("This bit here, officer"))]
        bad_bit: SourceSpan,
    },
}

#[derive(Debug, Error, Diagnostic)]
enum Errors {
    #[error("negative numbers are utterly unacceptable")]
    #[diagnostic(code(app::negative))]
    Negative,
    #[error("that's not right. idk what's wrong but yeah")]
    #[diagnostic(code(app::thats_not_right))]
    ThatsNotRight,
    #[error("too large: {0}")]
    #[diagnostic(
        severity = "warning",
        code(app::too_large),
        help("try doing it better next time?")
    )]
    TooLarge(i32),
    #[error(transparent)]
    #[diagnostic(
        severity = "error",
        code(app::input::parse),
        help("input an integer only please")
    )]
    ParseError(<i32 as std::str::FromStr>::Err),
}

fn main() {
    type Sink<'a> = &'a mut dyn DiagnosticSink;

    fn fallible_operation(num: i32) -> Result<i32, Errors> {
        if num > 5 {
            return Err(Errors::TooLarge(num));
        }
        Ok(num)
    }

    fn validate_value_no_fluff(num: i32) -> Result<(), Errors> {
        if num < 0 {
            Err(Errors::Negative)
        } else if num < -10 {
            Err(Errors::ThatsNotRight)
        } else {
            Ok(())
        }
    }

    fn validate_generally(num: i32, sink: Sink) -> Result<i32, Reported> {
        if num != 0 {
            // you can report errors and warnings manually.
            sink.report(Warning::NonZero);
        }
        // with Result::report(sink)?, you can only use ? once
        // This might return on its own. If it does, the error is reported.
        validate_value_no_fluff(num).report(sink)?;
        // We have only so far reported a warning, so we won't return Err(Reported).
        // return Err(Reported);
        Ok(num)
    }

    fn parser_or_whatever(
        src: &'static str,
        num: &str,
        sink: &mut dyn DiagnosticSink,
    ) -> Result<i32, Reported> {

        // You get less error conversion for free, unfortunately.
        let num: i32 = num.parse().map_err(Errors::ParseError).report(sink)?;

        // see, we can report the error if desired
        // the library can't enforce that you don't have Err(x) where x is actually only a
        // warning. So you could have a top level thing to check if the sink is empty, and if
        // so, report an error like "hmm, no errors found, but unsuccessful nevertheless".
        // Or, you could separate your error type from your large warnings + errors type.
        let val = fallible_operation(num).report(sink)?;

        // validate_generally(val, sink)?;

        let (second_word, position) = src
            .split_inclusive(' ')
            .take(2)
            .fold(("", 0), |(prev_word, consumed), word| {
                (word, consumed + prev_word.len())
            });

        let bad_bit = (position, second_word.trim_end().len()).into();

        if val > 3 {
            sink.report(Warning::WarningOnly {
                value: val,
                src,
                snip: (0, src.len()).into(),
                bad_bit,
            });
        }
        Ok(val)
    }

    let graphical = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
    let src = "one two three\nfour five six";
    let mut sink = VecSink::new(graphical);
    let mut input = String::new();
    match std::io::stdin().read_line(&mut input) {
        Ok(n) => {
            println!("{} bytes read", n);
            println!("{}", input);
        }
        Err(error) => println!("error: {}", error),
    }
    let result = parser_or_whatever(&src, input.trim(), &mut sink);
    println!("result = {:?}\ndiagnostics:", result);
    println!("{:?}", sink);
    for diag in sink.into_inner() {
        println!("{}", diag);
    }
}
