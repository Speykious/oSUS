use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use error_stack::{IntoReport, Report, Result, ResultExt};
use thiserror::Error;

use super::osu_file::OsuBeatmapFile;

#[derive(Clone, Debug, Error)]
#[error("Could not parse osu! beatmap file ({filename:?})")]
pub struct OsuBeatmapParseError {
    pub filename: OsString,
}

pub fn parse_osu_file<P>(path: P) -> Result<OsuBeatmapFile, OsuBeatmapParseError>
where
    P: AsRef<Path>,
{
    let filename = path.as_ref().file_name().unwrap().clone().to_owned();
    let file = File::open(path)
        .report()
        .change_context(OsuBeatmapParseError {
            filename: filename.clone(),
        })?;

    let mut reader = BufReader::new(file);

    let mut fformat_string: String = "".to_owned();
    reader
        .read_line(&mut fformat_string)
        .report()
        .change_context(OsuBeatmapParseError {
            filename: filename.clone(),
        })?;

    eprintln!("\n\n\nFormat string: {fformat_string:?}");

    // Remove ZERO WIDTH NO-BREAK SPACE (\u{feff}).
    // It seems to appear on v128 file formats...
    // I have no idea why.
    let format_version = fformat_string
        .trim_start_matches("\u{feff}")
        .strip_prefix("osu file format v")
        .ok_or(
            Report::new(OsuBeatmapParseError {
                filename: filename.clone(),
            })
            .attach_printable(format!(
                "File doesn't start with \"osu file format v<version>\" (Format string: {fformat_string:?})"
            ))
        )?;

    eprintln!("Format version of {:?}: {format_version}", filename.clone());

    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                log::warn!("Couldn't read line: {e}");
                continue;
            },
        };

        eprintln!("  {line:?}");
    }

    eprintln!();

    Err(Report::new(OsuBeatmapParseError {
        filename: filename.clone(),
    })
    .attach_printable("TODO"))
}
