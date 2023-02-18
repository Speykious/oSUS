use std::env::current_dir;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use osus::algos::{offset_map, remove_duplicates, remove_useless_speed_changes, reset_hitsounds};
use osus::file::beatmap::BeatmapFile;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

const PATH_HELP: &str = "Path to beatmap file or folder containing beatmap files.";

#[derive(Subcommand)]
enum Commands {
    /// Extract every .osu file from hashed osu!lazer files.
    ExtractOsuLazerFiles {
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

        #[arg(help = PATH_HELP)]
        path: PathBuf,
    },

    /// Offset the whole beatmap by some amount of milliseconds.
    Offset {
        #[arg(help = "Amount of milliseconds to offset the beatmap (can be a decimal number).")]
        millis: f64,

        #[arg(
            short,
            long,
            help = "Output path where to write the transformed beatmap (defaults to stdout)."
        )]
        out_path: Option<PathBuf>,

        #[arg(help = PATH_HELP)]
        path: PathBuf,
    },

    /// Reset all hitsounds to the same sample set (not touching actual samples on hit objects).
    ResetSampleSets {
        #[arg(
            long,
            default_value_t = true,
            help = "Whether to use the Soft sample set as the overwriting value."
        )]
        soft: bool,

        #[arg(
            long,
            default_value_t = true,
            help = "Whether to cleanup timing points after resetting hitsounds."
        )]
        cleanup: bool,

        #[arg(
            short,
            long,
            help = "Output path where to write the transformed beatmap (defaults to stdout)."
        )]
        out_path: Option<PathBuf>,

        #[arg(help = PATH_HELP)]
        path: PathBuf,
    },

    /// Cleanup timing points by removing all the ones that are useless/duplicates.
    CleanupTimingPoints {
        #[arg(
            short,
            long,
            help = "Output path where to write the transformed beatmap (defaults to stdout)."
        )]
        out_path: Option<PathBuf>,

        #[arg(help = PATH_HELP)]
        path: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let Cli { command } = Cli::parse();

    match command {
        Commands::ExtractOsuLazerFiles {
            out_path,
            recursive,
            path,
        } => {
            let out_path = out_path.unwrap_or(current_dir()?.join("maps"));
            cli_extract_osu_lazer_files(&out_path, recursive, &path)?;
        }

        Commands::Offset {
            millis,
            out_path,
            path,
        } => cli_offset(millis, out_path, path)?,

        Commands::ResetSampleSets {
            soft,
            cleanup,
            out_path,
            path,
        } => cli_reset_sample_sets(soft, cleanup, out_path, path)?,

        Commands::CleanupTimingPoints { out_path, path } => {
            cli_cleanup_timing_points(out_path, path)?
        }
    }

    Ok(())
}

fn add_suffix_to_map_version(beatmap: &mut BeatmapFile, suffix: &str) {
    log::warn!("Adding suffix to map version...");
    if let Some(metadata) = &mut beatmap.metadata {
        metadata.version += suffix;
    }
}

fn write_beatmap_out(beatmap: &BeatmapFile, path: Option<&Path>) -> io::Result<()> {
    if let Some(path) = path {
        log::warn!("Write beatmap to {}...", path.display());
        let mut out_file = File::create(path)?;
        beatmap.deserialize(&mut out_file)?;
    } else {
        beatmap.deserialize(&mut io::stdout())?;
    }

    Ok(())
}

fn cleanup_timing_points(beatmap: &mut BeatmapFile) {
    log::warn!("Removing duplicates...");
    beatmap.timing_points = remove_duplicates(&beatmap.timing_points);

    log::warn!("Removing useless speed changes...");
    beatmap.timing_points =
        remove_useless_speed_changes(&beatmap.timing_points, &beatmap.hit_objects);

    log::warn!("Removing duplicates again...");
    beatmap.timing_points = remove_duplicates(&beatmap.timing_points);
}

fn cli_extract_osu_lazer_files(out_path: &Path, recursive: bool, path: &Path) -> io::Result<()> {
    for entry in WalkDir::new(path)
        .max_depth(if recursive { usize::MAX } else { 0 })
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
            fs::copy(entry.path(), out_path.join(entry_out_path))?;
        }
    }

    Ok(())
}

fn cli_offset(millis: f64, out_path: Option<PathBuf>, path: PathBuf) -> io::Result<()> {
    log::warn!("Parsing {}...", path.display());
    let mut beatmap = match BeatmapFile::parse(&path) {
        Ok(beatmap) => beatmap,
        Err(err) => {
            log::error!("\n{err:?}");
            return Ok(());
        }
    };

    log::warn!("Offsetting beatmap...");
    offset_map(&mut beatmap, millis);

    write_beatmap_out(&beatmap, out_path.as_deref())
}

fn cli_reset_sample_sets(
    soft: bool,
    cleanup: bool,
    out_path: Option<PathBuf>,
    path: PathBuf,
) -> io::Result<()> {
    log::warn!("Parsing {}...", path.display());
    let mut beatmap = match BeatmapFile::parse(&path) {
        Ok(beatmap) => beatmap,
        Err(err) => {
            log::error!("\n{err:?}");
            return Ok(());
        }
    };

    log::warn!("Resetting hitsounds...");
    reset_hitsounds(&mut beatmap.timing_points, if soft { 2 } else { 0 });

    if cleanup {
        cleanup_timing_points(&mut beatmap);
        add_suffix_to_map_version(&mut beatmap, " ||CLEAN");
    } else {
        add_suffix_to_map_version(&mut beatmap, " ||RESET");
    }

    write_beatmap_out(&beatmap, out_path.as_deref())
}

fn cli_cleanup_timing_points(out_path: Option<PathBuf>, path: PathBuf) -> io::Result<()> {
    log::warn!("Parsing {}...", path.display());
    let mut beatmap = match BeatmapFile::parse(&path) {
        Ok(beatmap) => beatmap,
        Err(err) => {
            log::error!("\n{err:?}");
            return Ok(());
        }
    };

    cleanup_timing_points(&mut beatmap);
    add_suffix_to_map_version(&mut beatmap, " ||CLEAN");

    write_beatmap_out(&beatmap, out_path.as_deref())
}
