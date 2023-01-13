use std::error::Error;
use std::fs::read_to_string;

use miette::Diagnostic;
use osus::file::beatmap::parsing::{osu_beatmap, Resus};
use osus::file::beatmap::{BeatmapError, BeatmapFile};
use walkdir::WalkDir;

fn log_if_err<T, E>(result: Result<T, E>)
where
    E: Diagnostic + Send + Sync + 'static,
{
    if let Err(err) = result {
        let report: miette::Report = err.into();
        eprintln!("{:?}", report);
    }
}

fn parse(full_input: &str, f: fn(&str) -> Resus<BeatmapFile>) {
    let res = f(full_input).map_err(|e| match e {
        nom::Err::Incomplete(n) => panic!("Incomplete? {n:?}"),
        nom::Err::Error(e) | nom::Err::Failure(e) => {
            BeatmapError::from_src_and_parse_error("bruh.osu", full_input, e)
        }
    });
    log_if_err(res);
}

fn parser(input: &str) -> Resus<BeatmapFile> {
    osu_beatmap(input)
}

fn main() -> Result<(), Box<dyn Error>> {
    miette::set_panic_hook();

    for entry in WalkDir::new("map-samples")
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.extension().unwrap_or_default() == "osu" {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            println!("Parsing {}", file_name);
            let full_input = read_to_string(path)?;
            parse(&full_input, parser);
        }
    }

    Ok(())
}
