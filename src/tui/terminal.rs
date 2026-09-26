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

/// Leave raw mode and the alternate screen for an external program, then redraw.
/// Resuming directly avoids stacking a new panic hook for every editor invocation.
pub(crate) fn suspend<T>(
    terminal: &mut ratatui::DefaultTerminal,
    action: impl FnOnce() -> T,
) -> io::Result<T> {
    terminal.show_cursor()?;
    ratatui::try_restore()?;
    let result = action();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    // Recreate the fullscreen viewport and buffers after the editor, including any resize.
    terminal.resize(terminal.size()?.into())?;
    Ok(result)
}
