use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use anyhow::Context;
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

    #[arg(long, default_value = "{lat},{lon}")]
    output_format: String,
}

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // find data dir
    let data_dir = find_data_dir()?;

    let mut workers = Vec::new();
    let workspace = Workspace::new()?;

    let input = std::env::current_dir()?.join(args.input);
    let (sender, receiver) = unbounded();

    let frame_path = workspace.new_folder("frames")?;
    let resize_path = workspace.new_folder("frames-resize")?;

    let mut watcher = FsWatcher::new(frame_path.clone(), sender)?;
    watcher.start()?;

    for _ in 0..args.threads {
        workers.push(process_frames_worker(
            receiver.clone(),
            resize_path.clone(),
            data_dir.clone(),
            args.output_format.clone(),
        ));
    }

    extract_frames(&input, args.interval, &frame_path).context("extract frame using ffmpeg")?;
    SHUTDOWN_REQUESTED.store(true, Ordering::Relaxed);

    futures_util::future::join_all(workers).await;

    Ok(())
}

fn find_data_dir() -> anyhow::Result<String> {
    // current dir
    fn has_train_data(input: &Path) -> anyhow::Result<bool> {
        for file in input.read_dir()? {
            if let Ok(f) = file {
                if f.file_name().to_string_lossy().ends_with(".traineddata") {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    let exe = std::env::current_exe()?;
    let exe_path = exe.parent().unwrap_or(Path::new("/")).clone();
    if has_train_data(exe_path)? {
        return Ok(exe_path.to_string_lossy().to_string());
    }

    let current_dir = std::env::current_dir()?;
    if has_train_data(&current_dir)? {
        return Ok(current_dir.to_string_lossy().to_string());
    }

    eprintln!("train data was not found. Please download training data for english language using:\ncurl -o \"{}/eng.traineddata\" https://raw.githubusercontent.com/tesseract-ocr/tessdata_best/main/eng.traineddata\n\n", exe_path.to_string_lossy());
    panic!("train data was not found")
}

fn process_frames_worker(
    receiver: Receiver<PathBuf>,
    tmp_path: PathBuf,
    data_dir: String,
    out_format: String,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while !SHUTDOWN_REQUESTED.load(Ordering::Relaxed) {
            let Ok(source) = receiver.recv_timeout(Duration::from_millis(250)) else {
                continue;
            };

            match detect_location(&source, &tmp_path.clone(), &data_dir) {
                Ok(location) => {
                    let coordinates = parser::parse_coordinate_from_lines(location)
                        .into_iter()
                        .map(|c| c.to_decimal_with_format(&out_format))
                        .collect::<Vec<_>>();

                    if !coordinates.is_empty() {
                        println!("{}", coordinates.join("\n"));
                    }
                }
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
        .args(["-vf", &format!("fps=1/{}", interval_sec)])
        .args(["-s", "1280x720"])
        //.args(["-threads", "8"])
        .arg("f%09d.jpg")
        .current_dir(out_dir)
        .stdout(Stdio::null())
        .output()
        .context("start ffmpeg to extract frames")?;

    if !i.status.success() {
        panic!(
            "ffmpeg process exited with error:\n{}",
            String::from_utf8_lossy(&i.stderr)
        );
    }

    Ok(())
}

fn detect_location(source: &Path, tmp_path: &PathBuf, data_dir: &str) -> anyhow::Result<String> {
    let image_name = source.to_str().ok_or_else(|| anyhow::anyhow!(""))?;
    let out_name = tmp_path.join(format!(
        "{}-edit.jpg",
        source.file_name().unwrap_or_default().to_string_lossy()
    ));
    {
        let mut f = std::fs::File::create(&out_name).context("open file")?;
        let mut i = image::open(&image_name).context("open image")?;
        let mut i = i.crop(0, i.height() - 50, i.width(), 50).grayscale();
        i.invert();

        i.adjust_contrast(-500.0)
            .brighten(50)
            .write_to(&mut f, ImageOutputFormat::Png)
            .context("update image")?;
    }

    let tess = Tesseract::new(Some(&data_dir), Some("eng"))?;
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
