//! GPUYield: a menu bar app for Intel MacBooks with two graphics cards that switches the discrete
//! GPU off (Intel only), on (NVIDIA only) or back to automatic switching, through `pmset`.

mod autostart;
mod dispatch;
mod gpu;
mod i18n;
mod icon;
mod log;
mod menu;
mod model;
mod privileged;

use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--write-icons") {
        let dir = args.get(i + 1).map_or(".", String::as_str);
        if let Err(e) = icon::write_iconset(Path::new(dir)) {
            eprintln!("icons not written: {e}");
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a| a == "--version") {
        println!("GPUYield {}", version());
        return;
    }

    log::init();
    let Some(mtm) = MainThreadMarker::new() else {
        log!("not started on the main thread");
        return;
    };
    if !single_instance() {
        log!("GPUYield runs already, this start ends");
        return;
    }
    log!("GPUYield {} starts", version());
    // The mode stays as it is when GPUYield ends; it belongs to macOS, not to the app
    for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
        dispatch::on_signal(signal, quit);
    }
    autostart::refresh();
    gpu::watch(menu::refresh_soon);

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    dispatch::later(menu::start);
    app.run();
}

fn data_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join("Library/Application Support/GPUYield")
}

/// Only one GPUYield at a time: a lock on a file in the data folder, held until the end.
fn single_instance() -> bool {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(dir.join("lock"));
    let Ok(file) = file else {
        // Without a lock file better run twice than not at all
        return true;
    };
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return false;
    }
    std::mem::forget(file);
    true
}

pub fn quit() {
    log!("GPUYield quits");
    std::process::exit(0);
}
