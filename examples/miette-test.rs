use osus::file::beatmap::parsing::parse_section;
use osus::file::beatmap::BeatmapError;

fn main() -> miette::Result<()> {
    let full_input = "[Genewrong]\njust some stuff\nhere and there\n";
    if let Err(e) = parse_section(full_input) {
        match e {
            nom::Err::Incomplete(n) => println!("[incomplete] {n:?}"),
            nom::Err::Error(e) | nom::Err::Failure(e) => {
                return miette::Result::Err(
                    BeatmapError::from_source_and_parse_error("bruh.osu", full_input, e).into(),
                );
            }
        }
    }

    Ok(())
}
