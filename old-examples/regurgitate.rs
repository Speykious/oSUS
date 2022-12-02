use std::io;
use std::path::PathBuf;

use clap::Parser;
use osus::file::beatmap::BeatmapFile;

#[derive(Parser)]
#[command(name = "regurgitate")]
#[command(about = "Parse a .osu file and deserialize it immediately into stdout.")]
#[command(version = "1.0")]
#[command(author)]
struct Cli {
    #[arg(help = "Path to beatmap file.")]
    path: PathBuf,
}

fn main() -> io::Result<()> {
    env_logger::init();
    let Cli { path } = Cli::parse();

    log::warn!("Parsing {}...", path.display());
    let beatmap = match BeatmapFile::parse(&path) {
        Ok(beatmap) => beatmap,
        Err(err) => {
            log::error!("\n{err:?}");
            return Ok(());
        }
    };

    log::warn!("Rewrite {}...", path.display());
    beatmap.deserialize(&mut io::stdout())?;

    Ok(())
}
