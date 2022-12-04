use miette::Diagnostic;
use osus::file::beatmap::parsing::{parse_section, Resus};
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

fn main() -> miette::Result<()> {
    miette::set_panic_hook();

    let full_input = "[Genewrong]\njust some stuff\nhere and there\n";
    parse(full_input, parse_section);

    // TODO: This error is really not helpful. What should I do?
    let full_input = "Genestillwrong]\njust some stuff\nhere and there\n";
    parse(full_input, parse_section);

    Ok(())
}
