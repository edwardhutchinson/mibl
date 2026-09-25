//! Terminal ownership. The guard exists before initialization so partial setup also restores.
use std::io;

struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

pub(crate) fn with_terminal<T>(
    run: impl FnOnce(&mut ratatui::DefaultTerminal) -> io::Result<T>,
) -> io::Result<T> {
    let _restore = Restore;
    // Ratatui also installs a panic hook that restores before printing panic diagnostics.
    let mut terminal = ratatui::try_init()?;
    run(&mut terminal)
}
