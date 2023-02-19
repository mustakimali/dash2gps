use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use anyhow::{anyhow, Context};
use chrono::Utc;
use clap::Parser;
use crossbeam_channel::{unbounded, Receiver};
use image::ImageOutputFormat;
use tesseract::Tesseract;

use crate::watcher::FsWatcher;

mod parser;
mod watcher;

#[derive(Parser, Debug)]
struct Args {
    /// Path of the video file
    #[arg(short, long)]
    input: String,

    /// Find locations at interval in the video
    #[arg(long, default_value = "10")]
    interval: u32,

    /// Crop frames
    /// Format: left top right bottom
    /// Unit: px/(empty) OR %
    crop: Option<String>,

    #[arg(long, default_value = "4")]
    threads: u8,
}

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut workers = Vec::new();
    let workspace = Workspace::new()?;
    // extract frames
    println!(
        "Using workspace at: {}",
        workspace.path.to_str().unwrap_or("")
    );

    let input = std::env::current_dir()?.join(args.input);
    let (sender, receiver) = unbounded();

    let frame_path = workspace.new_folder("frames")?;
    let resize_path = workspace.new_folder("frames-resize")?;

    let mut watcher = FsWatcher::new(frame_path.clone(), sender)?;
    watcher.start()?;

    for _ in 1..args.threads {
        workers.push(process_frames_worker(receiver.clone(), resize_path.clone()));
    }

    extract_frames(&input, args.interval, &frame_path).context("extract frame using ffmpeg")?;
    SHUTDOWN_REQUESTED.store(true, Ordering::Relaxed);

    futures_util::future::join_all(workers).await;

    Ok(())
}

fn process_frames_worker(
    receiver: Receiver<PathBuf>,
    tmp_path: PathBuf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while !SHUTDOWN_REQUESTED.load(Ordering::Relaxed) {
            let Ok(source) = receiver.recv_timeout(Duration::from_millis(250)) else {
                continue;
            };

            match detect_location(&source, &tmp_path.clone()) {
                Ok(location) => println!("{}", location),
                Err(e) => eprintln!("Error: {} ({})", e, source.to_string_lossy()),
            }
        }
    })
}

fn extract_frames(input: &Path, interval_sec: u32, out_dir: &PathBuf) -> anyhow::Result<()> {
    // extract image every 10s
    // ffmpeg -i input.mp4 -vf "select=bitor(gte(t-prev_selected_t\,10)\,isnan(prev_selected_t))" -vsync 0 f%09d.jpg
    let input = input.to_str().ok_or(anyhow::anyhow!("e"))?;
    let i = Command::new("ffmpeg")
        .args(["-i", input])
        .args([
            "-vf",
            &format!(
                r#"select=bitor(gte(t-prev_selected_t\,{})\,isnan(prev_selected_t))"#,
                interval_sec
            ),
        ])
        .args(["-vsync", "0"])
        .arg("f%09d.jpg")
        .current_dir(out_dir)
        .stdout(Stdio::null())
        .output()
        .context("start ffmpeg to extract frames")?;

    if !i.status.success() {
        return Err(anyhow!("ffmpeg process exited with error"));
    }

    Ok(())
}

fn detect_location(source: &Path, tmp_path: &PathBuf) -> anyhow::Result<String> {
    let image_name = source.to_str().ok_or_else(|| anyhow::anyhow!(""))?;
    let out_name = tmp_path.join(format!(
        "{}-edit.jpg",
        source.file_name().unwrap_or_default().to_string_lossy()
    ));
    {
        let mut f = std::fs::File::create(&out_name).context("open file")?;
        let mut i = image::open(&image_name).context("open image")?;
        let mut i = i.crop(0, i.height() - 60, i.width(), 60).grayscale();
        i.invert();

        i.adjust_contrast(-500.0)
            .brighten(50)
            .write_to(&mut f, ImageOutputFormat::Png)
            .context("update image")?;
    }

    let tess = Tesseract::new(Some("."), Some("eng"))?;
    let mut tess = tess
        .set_image(&out_name.to_string_lossy().to_string())
        .context("set image")?;

    tess.get_text().map_err(anyhow::Error::from)
}

struct Workspace {
    path: PathBuf,
}

impl Workspace {
    pub fn new() -> anyhow::Result<Self> {
        // root
        let path =
            std::env::temp_dir().join(format!("dash2gps-workspace-{}", Utc::now().timestamp()));
        std::fs::create_dir(path.clone()).context("create temp folder")?;

        println!("Initialised workspace in: {}", path.to_string_lossy());

        Ok(Self { path })
    }

    pub fn new_folder(&self, name: impl Into<String>) -> anyhow::Result<PathBuf> {
        let path = self.path.join(name.into());
        std::fs::create_dir(path.clone())?;

        Ok(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        _ = std::fs::remove_dir_all(&self.path);
    }
}
