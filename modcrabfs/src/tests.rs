// Basic test for ModcrabFS.
// This file is completely new, and was written for use with ModcrabFS.

use std::{fs, io, path::PathBuf};

use self::filesystem::ModcrabFS;
use crossterm::event::{read as read_term_event, Event};
use simple_logger::SimpleLogger;

use super::*;

/// Mounts ModcrabFS to the 'demo/mnt' directory.
#[test]
fn do_mount() -> io::Result<()> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    // Sets up some files for a test overlay.
    fs::create_dir_all("demo/mnt")?;
    fs::write(
        "demo/mnt/from_under",
        b"Hello from the shadowed mountpoint!",
    )?;

    fs::create_dir_all("demo/alpha")?;
    fs::write("demo/alpha/from_alpha", b"ALPHA")?;
    fs::write("demo/alpha/Abdgo", b"Hello from alpha!")?;

    fs::create_dir_all("demo/beta")?;
    fs::write("demo/beta/from_beta", b"BETA")?;
    fs::write("demo/beta/aBdgo", b"Hello from beta!")?;

    fs::create_dir_all("demo/delta")?;
    fs::write("demo/delta/from_delta", b"DELTA")?;
    fs::write("demo/delta/Dg", b"Hello from delta!")?;

    fs::create_dir_all("demo/gamma")?;
    fs::write("demo/gamma/from_gamma", b"GAMMA")?;
    fs::write("demo/gamma/dG", b"Hello from gamma!")?;

    fs::create_dir_all("demo/overwrite")?;
    fs::write("demo/overwrite/from_over", b"OVERWRITE")?;
    fs::write("demo/overwrite/abdgO", b"Hello from overwrite!")?;

    // We actually set up and create ModcrabFS down here.
    let overlay_one = vec!["demo/mnt", "demo/alpha", "demo/beta", "demo/overwrite"]
        .into_iter()
        .map(PathBuf::from)
        .collect();

    let overlay_two = vec!["demo/delta", "demo/gamma"]
        .into_iter()
        .map(PathBuf::from)
        .collect();

    let _mount = ModcrabFS::new("demo/mnt", "demo/cache.bin", overlay_one)?
        .attach("inner", overlay_two)?
        .spawn_mount()?;

    println!("Press any key to unmount file system...");

    // Endlessly loops until user presses ENTER.
    loop {
        if let Event::Key(_) = read_term_event().unwrap() {
            break;
        }
    }

    Ok(())
}
