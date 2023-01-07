use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use osus::file::beatmap::BeatmapFile;
use osus::offset_map;

#[derive(Parser)]
#[command(name = "offset")]
#[command(about = "Offset the whole beatmap.")]
#[command(version = "1.0")]
#[command(author)]
struct Cli {
    #[arg(help = "Path to beatmap file or folder containing beatmap files.")]
    path: PathBuf,
    #[arg(long)]
    millis: u64,
}

fn main() -> io::Result<()> {
    env_logger::init();
    let Cli { path, millis } = Cli::parse();

    log::warn!("Parsing {}...", path.display());
    let mut beatmap = match BeatmapFile::parse(&path) {
        Ok(beatmap) => beatmap,
        Err(err) => {
            log::error!("\n{err:?}");
            return Ok(());
        }
    };

    log::warn!("Offsetting beatmap...");
    offset_map(&mut beatmap, Duration::from_millis(millis));

    log::warn!("Rewrite {}...", path.display());
    beatmap.deserialize(&mut io::stdout())?;

    Ok(())
}
