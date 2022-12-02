use std::path::{Path, PathBuf};

use clap::Parser;
use error_stack::Result;
use osus::file::beatmap::{BeatmapFile, BeatmapFileParseError};

use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "parse-osus")]
#[command(about = "Parse every .osu file and dump a debug string of their code structure.")]
#[command(version = "1.0")]
#[command(author)]
struct Cli {
    #[arg(help = "Path to beatmap file or folder containing beatmap files.")]
    path: PathBuf,
    #[arg(
        short,
        long,
        help = "Whether to recurse in the folder. (option is ignored if the path is a file)."
    )]
    recursive: bool,
}

fn main() -> Result<(), BeatmapFileParseError> {
    env_logger::init();
    let args = Cli::parse();

    for entry in WalkDir::new(args.path)
        .max_depth(if args.recursive { usize::MAX } else { 0 })
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.path().is_dir())
    {
        log::warn!("Parsing {}...", entry.path().display());

        match BeatmapFile::parse(entry.path()) {
            Ok(beatmap) => {
                let file_name = Path::new(entry.path())
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
