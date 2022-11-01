use std::env::current_dir;
use std::fs::{self, File};
use std::io;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use clap::Parser;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "extract-osu-lazer-files")]
#[command(about = "Extract every .osu file from hashed osu!lazer files.")]
#[command(version = "1.0")]
#[command(author)]
struct Cli {
    #[arg(help = "Path to beatmap file or folder containing beatmap files.")]
    path: PathBuf,
    #[arg(
        short,
        long,
        help = "Output path where to copy the beatmaps (defaults to ./maps/)."
    )]
    out_path: Option<PathBuf>,
    #[arg(
        short,
        long,
        help = "Whether to recurse in the folder. (option is ignored if the path is a file)."
    )]
    recursive: bool,
}

fn main() -> io::Result<()> {
    env_logger::init();
    let args = Cli::parse();
    let dest = args.out_path.unwrap_or(current_dir()?.join("maps"));

    for entry in WalkDir::new(args.path)
        .max_depth(if args.recursive { usize::MAX } else { 0 })
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.path().is_dir())
    {
        let file = File::open(entry.path())?;

        let mut buffer = BufReader::new(file);
        let mut first_line = String::new();
        let _ = buffer.read_line(&mut first_line);

        if first_line.starts_with("osu file format v") {
            println!("Map in {:?}", entry.path());
            let entry_out_path = Path::new(entry.file_name()).with_extension("osu");
            fs::copy(entry.path(), dest.join(entry_out_path))?;
        }
    }

    Ok(())
}
