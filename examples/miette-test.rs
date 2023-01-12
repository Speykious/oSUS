use miette::Diagnostic;
use nom::error::context;
use osus::file::beatmap::parsing::{osu_section_header, Resus};
use osus::file::beatmap::BeatmapError;

fn log_if_err<T, E>(result: Result<T, E>)
where
    E: Diagnostic + Send + Sync + 'static,
{
    if let Err(err) = result {
        let report: miette::Report = err.into();
        eprintln!("{:?}", report);
    }
}

fn parse(full_input: &'static str, f: fn(&str) -> Resus<&str>) {
    let res = f(full_input).map_err(|e| match e {
        nom::Err::Incomplete(n) => panic!("Incomplete? {n:?}"),
        nom::Err::Error(e) | nom::Err::Failure(e) => {
            BeatmapError::from_source_and_parse_error("bruh.osu", full_input, e)
        }
    });
    log_if_err(res);
}

fn parser(input: &str) -> Resus<&str> {
    context("valid section header", osu_section_header)(input)
}

fn main() -> miette::Result<()> {
    miette::set_panic_hook();

    let full_input = "[Genewrong]\njust some stuff\nhere and there\n";
    parse(full_input, parser);

    let full_input = "Genestillwrong]\njust some stuff\nhere and there\n";
    parse(full_input, parser);

    let full_input = "[Genewrongagain\njust some stuff\nhere and there\n";
    parse(full_input, parser);

    Ok(())
}
