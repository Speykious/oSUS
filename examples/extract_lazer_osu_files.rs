use std::env::current_dir;
use std::fs::{File, self};
use std::io::{BufReader, BufRead};
use std::{env, io};

use walkdir::WalkDir;

fn main() -> io::Result<()> {
    env_logger::init();

    let mut args = env::args();
    let program = args.next().expect("Excuse me wtf");
    if args.len() == 0 {
        println!("Usage: {program} <osu!lazer 'files' folder or other ...>");
        return Ok(());
    }

    let dest = current_dir()?.join("maps");

    for path in args {
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file = File::open(entry.path())?;

            let mut buffer = BufReader::new(file);
            let mut first_line = String::new();
            let _ = buffer.read_line(&mut first_line);

            if first_line.starts_with("osu file format v") {
                println!("Map in {:?}", entry.path());
                fs::copy(entry.path(), dest.join(entry.file_name()))?;
            }
        }
    }

    Ok(())
}
