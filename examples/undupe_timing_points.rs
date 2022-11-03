use std::io;
use std::path::PathBuf;

use clap::Parser;
use osus::file::beatmap::{BeatmapFile, TimingPoint};

#[derive(Parser)]
#[command(name = "undupe-timing-points")]
#[command(about = "Remove duplicate timing points of a .osu file.")]
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

    // remove duplicates
    beatmap.timing_points = remove_duplicates(&beatmap.timing_points);

    if let Some(metadata) = &mut beatmap.metadata {
        metadata.version += " ||UNDUPED";
    }

    log::warn!("Rewrite {}...", path.display());
    beatmap.deserialize(&mut io::stdout())?;

    Ok(())
}

/// Removes all duplicate timing points. It will keep every 
fn remove_duplicates(timing_points: &[TimingPoint]) -> Vec<TimingPoint> {
    if timing_points.is_empty() {
        return Vec::new();
    }

    let mut unduped_points = vec![timing_points[0].clone()];
    let mut prev_timing_point = &timing_points[0];

    for timing_point in &timing_points[1..] {
        if timing_point.uninherited || !timing_point.is_duplicate(prev_timing_point) {
            unduped_points.push(timing_point.clone());
            prev_timing_point = timing_point;
        }
    }

    unduped_points
}
