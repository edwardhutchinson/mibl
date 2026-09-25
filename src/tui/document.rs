//! Full definition text built directly from typed library results.
use crate::render;
use crossterm::event::KeyCode;
use mibl::{Mib, model::*};
use std::io::{self, Write};

pub(super) enum DocumentKind {
    Definition(SearchScope),
    Tables,
    Help,
}

pub(super) struct Document {
    pub kind: DocumentKind,
    pub lines: Vec<String>,
    pub top: usize,
    pub left: usize,
}

impl Document {
    pub fn definition(mib: &Mib, identity: &Identity) -> io::Result<Self> {
        let mut out = Vec::new();
        match identity {
            Identity::Parameter(name) => {
                definition(mib.parameter(name), render::parameter, &mut out)?
            }
            Identity::Packet(spid) => definition(mib.packet(*spid), render::packet, &mut out)?,
            Identity::Command(name) => definition(mib.command(name), render::command, &mut out)?,
        }
        let scope = match identity {
            Identity::Packet(_) => SearchScope::Packets,
            Identity::Parameter(_) => SearchScope::Parameters,
            Identity::Command(_) => SearchScope::Commands,
        };
        Ok(Self::from_bytes(out, DocumentKind::Definition(scope)))
    }

    pub fn help() -> Self {
        Self::from_bytes(b"Keyboard controls\n\n1: packets\n2: parameters\n3: commands\nTab: next list or search scope\n/: search, Tab chooses scope\np: PUS service[,subtype]\nt: supported-table load reports\nEnter: inspect selected identity\nEsc: cancel input / return to list\nUp/Down or j/k: select / scroll\nPageUp/PageDown: move one page\nHome/End: first / last page\nLeft/Right or h/l: scroll wide text\n?: this help\nq: quit outside text input\nCtrl-C: quit from any screen\n".to_vec(), DocumentKind::Help)
    }

    pub fn tables(mib: &Mib) -> io::Result<Self> {
        let mut out = Vec::new();
        render::tables(mib.tables().iter(), &mut out)?;
        Ok(Self::from_bytes(out, DocumentKind::Tables))
    }

    fn from_bytes(bytes: Vec<u8>, kind: DocumentKind) -> Self {
        Self {
            kind,
            lines: String::from_utf8(bytes)
                .expect("formatters write UTF-8")
                .lines()
                .map(str::to_owned)
                .collect(),
            top: 0,
            left: 0,
        }
    }

    pub fn navigate(&mut self, code: KeyCode, height: usize) {
        let last = self.lines.len().saturating_sub(height.max(1));
        self.top = super::vertical_position(code, self.top, last, height);
        match code {
            KeyCode::Right | KeyCode::Char('l') => self.left = self.left.saturating_add(8),
            KeyCode::Left | KeyCode::Char('h') => self.left = self.left.saturating_sub(8),
            KeyCode::Home => self.left = 0,
            _ => {}
        }
    }
}

fn definition<T>(
    lookup: Lookup<T>,
    render: fn(&T, bool, &mut dyn Write) -> io::Result<()>,
    out: &mut Vec<u8>,
) -> io::Result<()> {
    match lookup {
        Lookup::Found(value) => render(&value, true, out),
        Lookup::NotFound(NotFoundReason::NoMatchingIdentity) => {
            writeln!(out, "No matching identity.")
        }
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable) => writeln!(
            out,
            "Definitions unavailable: no usable roots of this kind."
        ),
        Lookup::Ambiguous(candidates) => {
            writeln!(out, "Ambiguous identity: no duplicate definition selected.")?;
            render::candidates(
                std::iter::once(candidates.first.as_ref())
                    .chain(std::iter::once(candidates.second.as_ref()))
                    .chain(candidates.rest.iter()),
                out,
            )
        }
    }
}
