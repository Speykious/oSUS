use std::env;
use std::path::Path;

use error_stack::Result;
use osu::osu_file::OsuBeatmapFile;
use osu::osu_file_parsing::OsuBeatmapParseError;

#[macro_use]
mod utils;
mod osu;

fn main() -> Result<(), OsuBeatmapParseError> {
    env_logger::init();

    let mut args = env::args();
    let program = args.next().expect("Excuse me wtf");
    if args.len() == 0 {
        println!("Usage: {program} <osu! beatmap files ...>");
        return Ok(());
    }

    for path in args {
        log::warn!("Parsing {}...", &path);
        match OsuBeatmapFile::parse(&path) {
            Ok(beatmap) => {
                let file_name = Path::new(&path)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .expect("File name will aways exist, right?");

                println!("Beatmap: {file_name}\n{beatmap:#?}")
            }
            Err(err) => {
                log::error!("\n{err:?}");
            }
        }
    }

    Ok(())
}
