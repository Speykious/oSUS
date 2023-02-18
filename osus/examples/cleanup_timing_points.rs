use std::io;
use std::path::PathBuf;

use clap::Parser;
use osus::file::beatmap::BeatmapFile;
use osus::algos::{remove_duplicates, remove_useless_speed_changes};

#[derive(Parser)]
#[command(name = "cleanup-timing-points")]
#[command(about = "Remove duplicate and useless timing points from a .osu file.")]
#[command(version = "1.0")]
#[command(author)]
struct Cli {
    #[arg(help = "Path to beatmap file or folder containing beatmap files.")]
    path: PathBuf,
}

fn main() -> io::Result<()> {
    env_logger::init();
    let Cli { path } = Cli::parse();

    log::warn!("Parsing {}...", path.display());
    let mut beatmap = match BeatmapFile::parse(&path) {
        Ok(beatmap) => beatmap,
        Err(err) => {
            log::error!("\n{err:?}");
            return Ok(());
        }
    };

    log::warn!("Removing duplicates...");
    beatmap.timing_points = remove_duplicates(&beatmap.timing_points);

    log::warn!("Removing useless speed changes...");
    beatmap.timing_points = remove_useless_speed_changes(&beatmap.timing_points, &beatmap.hit_objects);

    log::warn!("Removing duplicates again...");
    beatmap.timing_points = remove_duplicates(&beatmap.timing_points);

    log::warn!("Adding suffix to map version...");
    if let Some(metadata) = &mut beatmap.metadata {
        metadata.version += " ||CLEAN";
    }

    log::warn!("Rewrite {}...", path.display());
    beatmap.deserialize(&mut io::stdout())?;

    Ok(())
}
