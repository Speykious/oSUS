use std::env::current_dir;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use osus::algos::{
    insert_hitsound_timing_point, offset_map, remove_duplicates, remove_useless_speed_changes,
    reset_hitsounds,
};
use osus::file::beatmap::{BeatmapFile, HitObjectParams, SampleBank, TimingPoint};
use osus::{InterleavedTimestamped, Timestamped, TimestampedSlice};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

const PATH_HELP: &str = "Path to beatmap file or folder containing beatmap files.";
const OUT_PATH_HELP: &str =
    "Output path where to write the transformed beatmap (defaults to stdout).";

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
            help = OUT_PATH_HELP
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
            help = OUT_PATH_HELP
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
            help = OUT_PATH_HELP
        )]
        out_path: Option<PathBuf>,

        #[arg(help = PATH_HELP)]
        path: PathBuf,
    },

    /// Take hitsounds from a map and splat them on another.
    SplatHitsounds {
        #[arg(
            short,
            long,
            help = OUT_PATH_HELP
        )]
        out_path: Option<PathBuf>,

        #[arg(short, long, help = "Path to hitsound map file.")]
        sound_map: PathBuf,

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
        } => cli_offset(millis, out_path.as_deref(), &path)?,

        Commands::ResetSampleSets {
            soft,
            cleanup,
            out_path,
            path,
        } => cli_reset_sample_sets(soft, cleanup, out_path.as_deref(), &path)?,

        Commands::CleanupTimingPoints { out_path, path } => {
            cli_cleanup_timing_points(out_path.as_deref(), &path)?
        }

        Commands::SplatHitsounds {
            out_path,
            sound_map,
            path,
        } => cli_splat_hitsounds(out_path.as_deref(), &sound_map, &path)?,
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

fn cli_offset(millis: f64, out_path: Option<&Path>, path: &Path) -> io::Result<()> {
    log::warn!("Parsing {}...", path.display());
    let mut beatmap = match BeatmapFile::parse(path) {
        Ok(beatmap) => beatmap,
        Err(err) => {
            log::error!("\n{err:?}");
            return Ok(());
        }
    };

    log::warn!("Offsetting beatmap...");
    offset_map(&mut beatmap, millis);

    write_beatmap_out(&beatmap, out_path)
}

fn cli_reset_sample_sets(
    soft: bool,
    cleanup: bool,
    out_path: Option<&Path>,
    path: &Path,
) -> io::Result<()> {
    log::warn!("Parsing {}...", path.display());
    let mut beatmap = match BeatmapFile::parse(path) {
        Ok(beatmap) => beatmap,
        Err(err) => {
            log::error!("\n{err:?}");
            return Ok(());
        }
    };

    log::warn!("Resetting hitsounds...");
    let sample_bank = if soft {
        SampleBank::Soft
    } else {
        SampleBank::Auto
    };
    reset_hitsounds(&mut beatmap.timing_points, sample_bank);

    if cleanup {
        cleanup_timing_points(&mut beatmap);
        add_suffix_to_map_version(&mut beatmap, " ||CLEAN");
    } else {
        add_suffix_to_map_version(&mut beatmap, " ||RESET");
    }

    write_beatmap_out(&beatmap, out_path)
}

fn cli_cleanup_timing_points(out_path: Option<&Path>, path: &Path) -> io::Result<()> {
    log::warn!("Parsing {}...", path.display());
    let mut beatmap = match BeatmapFile::parse(path) {
        Ok(beatmap) => beatmap,
        Err(err) => {
            log::error!("\n{err:?}");
            return Ok(());
        }
    };

    cleanup_timing_points(&mut beatmap);
    add_suffix_to_map_version(&mut beatmap, " ||CLEAN");

    write_beatmap_out(&beatmap, out_path)
}

fn cli_splat_hitsounds(
    out_path: Option<&Path>,
    soundmap_path: &Path,
    beatmap_path: &Path,
) -> io::Result<()> {
    log::warn!("Parsing {}...", beatmap_path.display());
    let mut beatmap = match BeatmapFile::parse(beatmap_path) {
        Ok(beatmap) => beatmap,
        Err(err) => {
            log::error!("\n{err:?}");
            return Ok(());
        }
    };

    log::warn!("Parsing {}...", soundmap_path.display());
    let soundmap = match BeatmapFile::parse(soundmap_path) {
        Ok(beatmap) => beatmap,
        Err(err) => {
            log::error!("\n{err:?}");
            return Ok(());
        }
    };

    // insert soundmap's hitsound information from timing points
    log::warn!("Inserting soundmap's timing points...");
    let mut new_timing_points: Vec<TimingPoint> = Vec::new();
    let mut last_sound_point = &soundmap.timing_points[0];
    for smtp_bmtp in (soundmap.timing_points).interleave_timestamped(&beatmap.timing_points) {
        match smtp_bmtp {
            Ok(soundmap_tp) => {
                last_sound_point = soundmap_tp;

                if let Some(new_tp) = new_timing_points.last_mut() {
                    if soundmap_tp.basically_eq(new_tp) {
                        new_tp.sample_set = soundmap_tp.sample_set;
                        new_tp.sample_index = soundmap_tp.sample_index;
                        new_tp.volume = soundmap_tp.volume;
                    } else {
                        let mut new_tp = new_tp.clone();
                        new_tp.time = soundmap_tp.time;
                        new_tp.sample_set = soundmap_tp.sample_set;
                        new_tp.sample_index = soundmap_tp.sample_index;
                        new_tp.volume = soundmap_tp.volume;
                        new_timing_points.push(new_tp.clone());
                    }
                }
            }
            Err(beatmap_tp) => {
                let mut new_tp = beatmap_tp.clone();
                new_tp.sample_set = last_sound_point.sample_set;
                new_tp.sample_index = last_sound_point.sample_index;
                new_tp.volume = last_sound_point.volume;
                new_timing_points.push(new_tp);
            }
        }
    }
    beatmap.timing_points = new_timing_points;

    log::warn!("Inserting soundmap's hitsounds...");
    let slider_multiplier = beatmap.difficulty.as_ref().unwrap().slider_multiplier as f64;

    let mut modified_hit_objects = Vec::new();

    // TODO: improve performance by somehow walking along both maps
    //       (instead of binary-searching the soundmap every time)

    let mut beat_length = 0.0;
    let mut slider_velocity = 1.0;
    for ho_tp in beatmap.iter_hit_objects_and_timing_points() {
        match ho_tp {
            Ok(hit_object) => {
                let new_hit_object = match &hit_object.object_params {
                    HitObjectParams::HitCircle => {
                        // affect hitsound properties of the hitcircle

                        let mut hit_object = hit_object.clone();

                        let start_hitsound =
                            soundmap.hit_objects.at_timestamp(hit_object.timestamp());

                        if let Some(sound_object) = start_hitsound {
                            log::info!("affecting hitcircle at {}", hit_object.timestamp());

                            hit_object.hit_sample = sound_object.hit_sample.clone();
                            hit_object.hit_sound = sound_object.hit_sound;
                        }

                        hit_object
                    }
                    HitObjectParams::Slider { length, .. } => {
                        // affect all edge hitsound properties of the slider

                        let mut hit_object = hit_object.clone();

                        let timestamp = hit_object.timestamp();
                        let dur =
                            *length * beat_length / (slider_multiplier * 100.0 * slider_velocity);

                        if let HitObjectParams::Slider {
                            edge_hitsounds,
                            edge_samplesets,
                            ..
                        } = &mut hit_object.object_params
                        {
                            for (i, (edge_hs, edge_ss)) in (edge_hitsounds.iter_mut())
                                .zip(edge_samplesets.iter_mut())
                                .enumerate()
                            {
                                let start_hitsound = soundmap
                                    .hit_objects
                                    .at_timestamp(timestamp + i as f64 * dur);

                                if let Some(sound_object) = start_hitsound {
                                    log::info!(
                                        "affecting slider at {}",
                                        timestamp + i as f64 * dur
                                    );

                                    *edge_ss = sound_object.hit_sample.to_hit_sample_set();
                                    *edge_hs = sound_object.hit_sound;
                                }
                            }
                        }

                        hit_object
                    }
                    HitObjectParams::Spinner { end_time } => {
                        // affect hitsound properties of the spinner

                        let mut hit_object = hit_object.clone();

                        let end_hitsound = soundmap.hit_objects.at_timestamp(*end_time);

                        if let Some(sound_object) = end_hitsound {
                            log::info!("affecting spinner at {}", end_time);

                            hit_object.hit_sample = sound_object.hit_sample.clone();
                            hit_object.hit_sound = sound_object.hit_sound;
                        }

                        hit_object
                    }
                    HitObjectParams::Hold { .. } => {
                        // affect hitsound properties of the mania hold

                        let mut hit_object = hit_object.clone();

                        let start_hitsound =
                            soundmap.hit_objects.at_timestamp(hit_object.timestamp());

                        if let Some(sound_object) = start_hitsound {
                            log::info!("affecting mania hold at {}", hit_object.timestamp());

                            hit_object.hit_sample = sound_object.hit_sample.clone();
                            hit_object.hit_sound = sound_object.hit_sound;
                        }

                        hit_object
                    }
                };

                modified_hit_objects.push(new_hit_object);

                // // old prints
                // if let HitObjectParams::Slider {
                //     edge_hitsounds,
                //     length,
                //     ..
                // } = &hit_object.object_params
                // {
                //     let dur = *length * beat_length / (slider_multiplier * 100.0 * slider_velocity);

                //     print!(
                //         "[{}] {:>6}:{:<6} {} | {} (dur={})",
                //         hit_object.time,
                //         format!("{:?}", hit_object.hit_sample.normal_set),
                //         format!("{:?}", hit_object.hit_sample.addition_set),
                //         hit_object.hit_sound.fixed_flags_string(),
                //         hit_object.object_type,
                //         dur.round(),
                //     );

                //     for eh in edge_hitsounds {
                //         print!(" -> {}", eh.fixed_flags_string());
                //     }
                //     println!();
                // } else {
                //     println!(
                //         "[{}] {:>6}:{:<6} {} | {}",
                //         hit_object.time,
                //         format!("{:?}", hit_object.hit_sample.normal_set),
                //         format!("{:?}", hit_object.hit_sample.addition_set),
                //         hit_object.hit_sound.fixed_flags_string(),
                //         hit_object.object_type,
                //     );
                // }
            }
            Err(timing_point) if timing_point.uninherited => {
                beat_length = timing_point.beat_length;
                // println!(
                //     "[{}] Timing Point ~ BeatLength = {}",
                //     timing_point.time, beat_length
                // );
            }
            Err(timing_point) => {
                slider_velocity = -100.0 / timing_point.beat_length;
                // println!(
                //     "[{}] Timing Point (inherited) ~ SV = {:.2}",
                //     timing_point.time, slider_velocity
                // );
            }
        }
    }

    beatmap.hit_objects = modified_hit_objects;
    if let Some(ref mut metadata) = beatmap.metadata {
        metadata.version += "~ HITSOUNDED";
    }

    write_beatmap_out(&beatmap, out_path)
}
