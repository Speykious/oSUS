use std::env;

use error_stack::Result;
use osu::osu_file::OsuBeatmapFile;
use osu::osu_file_parsing::OsuBeatmapParseError;

#[macro_use]
mod utils;
mod osu;

fn main() -> Result<(), OsuBeatmapParseError> {
    env_logger::init();

    let mut args = env::args().into_iter();
    let program = args.next().expect("Excuse me wtf");
    if args.len() == 0 {
        println!("Usage: {program} <osu! beatmap files ...>");
        return Ok(());
    }

    for path in args {
        match OsuBeatmapFile::parse(&path) {
            Ok(beatmap) => println!("{beatmap:#?}"),
            Err(err) => {
                log::error!("\n{err:?}");
            },
        }
    }

    Ok(())
}
