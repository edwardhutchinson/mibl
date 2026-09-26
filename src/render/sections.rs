//! Shared section boundaries: plain CLI headings or independently presented TUI sections.
use std::io::{self, Write};

#[derive(Clone, Copy)]
pub(crate) enum Role {
    Summary,
    Content,
    Problem,
    Reference,
    Unavailable,
}

impl Role {
    pub fn availability<T>(value: &Option<T>) -> Self {
        if value.is_some() {
            Self::Content
        } else {
            Self::Unavailable
        }
    }
}

pub(crate) trait Output: Write {
    fn section(&mut self, title: &'static str, role: Role) -> io::Result<()>;
}

pub(super) struct Plain<'a>(pub &'a mut dyn Write);

impl Write for Plain<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl Output for Plain<'_> {
    fn section(&mut self, title: &'static str, role: Role) -> io::Result<()> {
        if matches!(role, Role::Summary) {
            Ok(())
        } else {
            writeln!(self, "\n{title}")
        }
    }
}

pub(crate) struct Section {
    pub title: &'static str,
    pub role: Role,
    pub content: Vec<u8>,
}

#[derive(Default)]
pub(crate) struct Sections(pub Vec<Section>);

impl Write for Sections {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let section = self
            .0
            .last_mut()
            .ok_or_else(|| io::Error::other("section must precede content"))?;
        section.content.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Output for Sections {
    fn section(&mut self, title: &'static str, role: Role) -> io::Result<()> {
        self.0.push(Section {
            title,
            role,
            content: Vec::new(),
        });
        Ok(())
    }
}
