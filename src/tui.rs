//! Keyboard navigation over an already loaded immutable MIB snapshot.
mod candidates;
mod document;
mod pus;
mod screen;
mod terminal;
#[cfg(test)]
mod tests;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use document::Document;
use mibl::{Mib, model::*};
use ratatui::widgets::TableState;
use std::io;

enum Filter {
    Inventory,
    Search(String),
    Pus { service: u16, subtype: Option<u16> },
    PusGroup(pus::Coordinates),
}

#[derive(PartialEq, Eq)]
enum View {
    Definitions,
    Pus,
}

enum InputKind {
    Search(SearchScope),
    Pus,
}

struct Input {
    kind: InputKind,
    value: String,
    error: Option<&'static str>,
}

pub(crate) struct App<'a> {
    mib: &'a Mib,
    view: View,
    pus: pus::Browser,
    scope: SearchScope,
    filter: Filter,
    input: Option<Input>,
    candidates: Vec<Candidate>,
    roots_available: bool,
    selection: TableState,
    first_column: usize,
    document: Option<Document>,
    page_height: usize,
}

impl<'a> App<'a> {
    pub(crate) fn new(mib: &'a Mib) -> Self {
        let mut app = Self {
            mib,
            view: View::Definitions,
            pus: pus::Browser::new(mib),
            scope: SearchScope::Packets,
            filter: Filter::Inventory,
            input: None,
            candidates: Vec::new(),
            roots_available: false,
            selection: TableState::default(),
            first_column: 0,
            document: None,
            page_height: 1,
        };
        app.refresh();
        app
    }

    fn browse(&mut self, scope: SearchScope) {
        self.scope = scope;
        self.filter = Filter::Inventory;
        self.refresh();
    }

    fn refresh(&mut self) {
        self.view = View::Definitions;
        self.candidates = match &self.filter {
            Filter::Inventory => self.mib.inventory(self.scope),
            Filter::Search(query) => self.mib.search(query, self.scope),
            Filter::Pus { service, subtype } => self.mib.pus(*service, *subtype),
            Filter::PusGroup(_) => self
                .pus
                .selected()
                .map(|group| group.candidates.clone())
                .unwrap_or_default(),
        };
        self.roots_available = !self.candidates.is_empty()
            || match self.filter {
                Filter::Inventory | Filter::PusGroup(_) => false,
                Filter::Search(_) => !self.mib.inventory(self.scope).is_empty(),
                Filter::Pus { .. } => {
                    !self.mib.inventory(SearchScope::Packets).is_empty()
                        || !self.mib.inventory(SearchScope::Commands).is_empty()
                }
            };
        self.selection =
            TableState::default().with_selected((!self.candidates.is_empty()).then_some(0));
        self.document = None;
        self.first_column = 0;
    }

    fn edit(&mut self, code: KeyCode) {
        let Some(input) = &mut self.input else { return };
        match code {
            KeyCode::Esc => self.input = None,
            KeyCode::Backspace => {
                input.value.pop();
                input.error = None;
            }
            KeyCode::Char(c) => {
                input.value.push(c);
                input.error = None;
            }
            KeyCode::Tab => {
                if let InputKind::Search(scope) = &mut input.kind {
                    *scope = next_scope(*scope);
                }
            }
            KeyCode::Enter => {
                match input.kind {
                    InputKind::Search(scope) => {
                        self.scope = scope;
                        self.filter = Filter::Search(input.value.clone());
                    }
                    InputKind::Pus => match parse_pus(&input.value) {
                        Some((service, subtype)) => self.filter = Filter::Pus { service, subtype },
                        None => {
                            input.error = Some(
                                "Use SERVICE or SERVICE,SUBTYPE as unsigned integers (0..65535).",
                            );
                            return;
                        }
                    },
                }
                self.input = None;
                self.refresh();
            }
            _ => {}
        }
    }

    /// Returns true when the user requests exit. Release events do nothing.
    pub(crate) fn handle(&mut self, key: KeyEvent) -> io::Result<bool> {
        if key.kind == KeyEventKind::Release {
            return Ok(false);
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(true);
        }
        if self.input.is_some() {
            self.edit(key.code);
            return Ok(false);
        }
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('/') => {
                self.input = Some(Input {
                    kind: InputKind::Search(self.scope),
                    value: String::new(),
                    error: None,
                })
            }
            KeyCode::Char('f') => {
                self.input = Some(Input {
                    kind: InputKind::Pus,
                    value: String::new(),
                    error: None,
                })
            }
            KeyCode::Char('u') => {
                self.view = View::Pus;
                self.document = None;
            }
            KeyCode::Char('?') => self.document = Some(Document::help()),
            KeyCode::Char('t') => self.document = Some(Document::tables(self.mib)?),
            KeyCode::Char('1' | 'P') => self.browse(SearchScope::Packets),
            KeyCode::Char('2' | 'p') => self.browse(SearchScope::Parameters),
            KeyCode::Char('3' | 'c' | 'C') => self.browse(SearchScope::Commands),
            KeyCode::Tab if self.view == View::Pus => self.browse(SearchScope::Packets),
            KeyCode::Tab if matches!(self.filter, Filter::Search(_)) => {
                self.scope = next_scope(self.scope);
                self.refresh();
            }
            KeyCode::Tab => self.browse(match self.scope {
                SearchScope::Packets => SearchScope::Parameters,
                SearchScope::Parameters => SearchScope::Commands,
                _ => SearchScope::Packets,
            }),
            KeyCode::Esc => {
                if self.document.take().is_none() && matches!(self.filter, Filter::PusGroup(_)) {
                    self.view = View::Pus;
                }
            }
            KeyCode::Enter if self.document.is_none() && self.view == View::Pus => {
                if let Some(group) = self.pus.selected() {
                    self.filter = Filter::PusGroup(group.coordinates);
                    self.refresh();
                }
            }
            KeyCode::Enter if self.document.is_none() => {
                if let Some(candidate) = self
                    .selection
                    .selected()
                    .and_then(|i| self.candidates.get(i))
                {
                    self.document = Some(Document::definition(self.mib, &candidate.identity)?);
                }
            }
            code => {
                if let Some(document) = &mut self.document {
                    document.navigate(code, self.page_height);
                } else if self.view == View::Pus {
                    self.pus.navigate(code, self.page_height);
                } else if let Some(index) = self.selection.selected() {
                    match code {
                        KeyCode::Right | KeyCode::Char('l') => {
                            self.first_column =
                                (self.first_column + 1).min(self.candidate_column_count() - 1)
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            self.first_column = self.first_column.saturating_sub(1)
                        }
                        KeyCode::Home => self.first_column = 0,
                        _ => {}
                    }
                    let last = self.candidates.len().saturating_sub(1);
                    let next = vertical_position(code, index, last, self.page_height);
                    self.selection.select(Some(next));
                }
            }
        }
        Ok(false)
    }
}

/// Shared keyboard movement; each view supplies its own last reachable position.
fn vertical_position(code: KeyCode, current: usize, last: usize, page_height: usize) -> usize {
    match code {
        KeyCode::Down | KeyCode::Char('j') => current.saturating_add(1).min(last),
        KeyCode::Up | KeyCode::Char('k') => current.saturating_sub(1),
        KeyCode::PageDown => current.saturating_add(page_height).min(last),
        KeyCode::PageUp => current.saturating_sub(page_height),
        KeyCode::Home => 0,
        KeyCode::End => last,
        _ => current,
    }
}

fn next_scope(scope: SearchScope) -> SearchScope {
    match scope {
        SearchScope::Packets => SearchScope::Parameters,
        SearchScope::Parameters => SearchScope::Commands,
        SearchScope::Commands => SearchScope::All,
        SearchScope::All => SearchScope::Packets,
    }
}

fn parse_pus(value: &str) -> Option<(u16, Option<u16>)> {
    let mut parts = value.split(',');
    let parse = |s: &str| {
        if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) {
            None
        } else {
            s.parse().ok()
        }
    };
    let service = parse(parts.next()?)?;
    let subtype = match parts.next() {
        Some(s) => Some(parse(s)?),
        None => None,
    };
    if parts.next().is_some() {
        return None;
    }
    Some((service, subtype))
}

/// The caller loads once, before terminal setup. Resize events simply trigger another draw.
pub(crate) fn run(mib: &Mib) -> io::Result<()> {
    let mut app = App::new(mib);
    terminal::with_terminal(|terminal| {
        loop {
            terminal.draw(|frame| app.draw(frame))?;
            if let crossterm::event::Event::Key(key) = crossterm::event::read()?
                && app.handle(key)?
            {
                return Ok(());
            }
        }
    })
}
