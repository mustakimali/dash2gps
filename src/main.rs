use std::{
    io::Stderr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Instant,
};

use anyhow::{anyhow, Context};
use chrono::Utc;
use clap::Parser;
use image::ImageOutputFormat;
use tesseract::Tesseract;

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
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let workspace = Workspace::new()?;
    // extract frames
    println!(
        "Using workspace at: {}",
        workspace.path.to_str().unwrap_or("")
    );

    let input = std::env::current_dir()?.join(args.input);

    extract_frames(&input, args.interval, &workspace.path).context("extract frame using ffmpeg")?;

    Ok(())
}

fn extract_frames(input: &Path, interval_sec: u32, out_dir: &PathBuf) -> anyhow::Result<()> {
    // extract image every 10s
    // ffmpeg -i input.mp4 -vf "select=bitor(gte(t-prev_selected_t\,10)\,isnan(prev_selected_t))" -vsync 0 f%09d.jpg
    let input = input.to_str().ok_or(anyhow::anyhow!("e"))?;
    let output = out_dir
        .to_str()
        .ok_or(anyhow::anyhow!("Error determining output dir"))?
        .trim_end_matches(['/', '\\']);
    let mut i = Command::new("ffmpeg")
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
        .spawn()
        .context("start ffmpeg to extract frames")?;

    let result = i.wait()?;

    if !result.success() {
        return Err(anyhow!("ffmpeg process exited with error"));
    }

    Ok(())
}

fn test() -> anyhow::Result<()> {
    let image_name = std::env::var("SOURCE").unwrap_or_else(|_| "image.jpg".to_string());

    let out_name = format!("{}-edit.jpg", &image_name);
    {
        let mut f = std::fs::File::create(&out_name).expect("open file");
        let mut i = image::open(&image_name).expect("open image");
        let mut i = i.crop(0, i.height() - 60, i.width(), 60).grayscale();
        i.invert();

        i.adjust_contrast(-500.0)
            .brighten(50)
            .write_to(&mut f, ImageOutputFormat::Png)
            .expect("update image");
    }

    let tess = Tesseract::new(Some("."), Some("eng"))?;
    let mut tess = tess.set_image(&out_name).expect("set image");

    let text = tess.get_text().expect("get text");

    println!("Text: {}", text);

    Ok(())
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
}

#[cfg(not(debug_assertions))]
impl Drop for Workspace {
    fn drop(&mut self) {
        _ = std::fs::remove_dir_all(&self.path);
    }
}
