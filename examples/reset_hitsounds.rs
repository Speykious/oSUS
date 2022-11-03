use std::io;
use std::path::PathBuf;

use clap::Parser;
use osus::file::beatmap::{BeatmapFile, TimingPoint};

#[derive(Parser)]
#[command(name = "reset-hitsounds")]
#[command(about = "Reset all timing point hitsounds of a .osu file.")]
#[command(version = "1.0")]
#[command(author)]
struct Cli {
    #[arg(help = "Path to beatmap file or folder containing beatmap files.")]
    path: PathBuf,
    #[arg(short, long, default_value_t = true, help = "Whether to use the Soft sample set as the overwriting value.")]
    soft: bool,
}

fn main() -> io::Result<()> {
    env_logger::init();
    let Cli { path, soft } = Cli::parse();

    log::warn!("Parsing {}...", path.display());
    let mut beatmap = match BeatmapFile::parse(&path) {
        Ok(beatmap) => beatmap,
        Err(err) => {
            log::error!("\n{err:?}");
            return Ok(());
        }
    };

    log::warn!("Resetting hitsounds...");
    reset_hitsounds(&mut beatmap.timing_points, soft);

    log::warn!("Adding suffix to map version...");
    if let Some(metadata) = &mut beatmap.metadata {
        metadata.version += " ||RESET";
    }

    log::warn!("Rewrite {}...", path.display());
    beatmap.deserialize(&mut io::stdout())?;

    Ok(())
}

/// Resets all hitsounds in timing points, including volume.
fn reset_hitsounds(timing_points: &mut [TimingPoint], soft: bool) {
    let sample_set = if soft { 2 } else { 0 };
    for timing_point in timing_points {
        timing_point.sample_set = sample_set;
        timing_point.sample_index = 0;
        timing_point.volume = 100;
    }
}
