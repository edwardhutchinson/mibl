use std::{
    fs,
    path::Path,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct Fixture(PathBuf);
impl Fixture {
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "mibl-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
    pub fn write(&self, file: &str, text: &str) {
        fs::write(self.0.join(file), text).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Each test binary includes this module separately, so not every one reads every fixture.
#[allow(dead_code)]
pub const PARAMETER: &str = "TEMP\tTemperature\t\tK\t3\t4\t99\t\t\tN\tR";
