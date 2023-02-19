use std::path::PathBuf;

use anyhow::Context;
use crossbeam_channel::Sender;
use notify::{
    event::{AccessKind, AccessMode},
    EventKind, RecommendedWatcher, Watcher,
};

pub struct FsWatcher(RecommendedWatcher, PathBuf);
impl FsWatcher {
    pub fn new(path: PathBuf, change: Sender<PathBuf>) -> anyhow::Result<Self> {
        let watch = RecommendedWatcher::new(
            move |res: Result<notify::Event, _>| {
                if let Ok(e) = res {
                    if let EventKind::Access(AccessKind::Close(AccessMode::Write)) = e.kind {
                        if let Some(path) = e.paths.first() {
                            _ = change.try_send(path.clone());
                        }
                    }
                }
            },
            notify::Config::default(),
        )
        .context("init file monitor")?;

        Ok(Self(watch, path))
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        self.0
            .watch(&self.1, notify::RecursiveMode::Recursive)
            .context("start file monitor")?;

        Ok(())
    }

    pub fn stop(&mut self) -> anyhow::Result<()> {
        self.0.unwatch(&self.1).context("start file monitor")?;

        Ok(())
    }
}

impl Drop for FsWatcher {
    fn drop(&mut self) {
        _ = self.stop();
    }
}
