//! Search retained root definitions without expanding their relationships.
use super::{Catalog, commands::command_candidate, parameter_candidate};
use crate::model::*;
use nucleo_matcher::{
    Matcher, Utf32Str,
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
};

impl Catalog {
    pub(crate) fn search(&self, query: &str, scope: SearchScope) -> Vec<Candidate> {
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }
        let folded_query = query.to_lowercase();
        let pattern = Pattern::new(
            query,
            CaseMatching::Ignore,
            Normalization::Never,
            AtomKind::Fuzzy,
        );
        let mut matcher = Matcher::default();
        let mut buffer = Vec::new();
        let mut score = |text: &str| pattern.score(Utf32Str::new(text, &mut buffer), &mut matcher);
        let parameters = self
            .records
            .pcf
            .rows()
            .iter()
            .filter(|_| matches!(scope, SearchScope::Parameters | SearchScope::All))
            .map(parameter_candidate);
        let packets = self
            .records
            .pid
            .rows()
            .iter()
            .filter(|_| matches!(scope, SearchScope::Packets | SearchScope::All))
            .map(|row| self.packet_candidate(row));
        let commands = self
            .records
            .ccf
            .rows()
            .iter()
            .filter(|_| matches!(scope, SearchScope::Commands | SearchScope::All))
            .map(command_candidate);
        let mut matches: Vec<_> = parameters
            .chain(packets)
            .chain(commands)
            .filter_map(|candidate| {
                let identity = match &candidate.identity {
                    Identity::Parameter(name) => name.0.clone(),
                    Identity::Packet(spid) => spid.0.to_string(),
                    Identity::Command(name) => name.0.clone(),
                };
                let folded_identity = identity.to_lowercase();
                let names = self.searchable_names(&candidate);
                let texts =
                    std::iter::once(identity.as_str()).chain(names.iter().map(String::as_str));
                // Ranking classes are contractual; fuzzy scores within a class are tuning.
                let rank = if folded_identity == folded_query {
                    (0, std::cmp::Reverse(0))
                } else if folded_identity.starts_with(&folded_query) {
                    (1, std::cmp::Reverse(0))
                } else if let Some(best) = texts.filter_map(&mut score).max() {
                    (2, std::cmp::Reverse(best))
                } else {
                    (
                        3,
                        std::cmp::Reverse(
                            candidate
                                .description
                                .value
                                .as_deref()
                                .and_then(&mut score)?,
                        ),
                    )
                };
                Some((rank, candidate))
            })
            .collect();
        matches.sort_by(|(a_score, a), (b_score, b)| {
            a_score
                .cmp(b_score)
                .then_with(|| a.identity.cmp(&b.identity))
                .then_with(|| a.source.cmp(&b.source))
        });
        tracing::debug!(query, ?scope, matches = matches.len(), "search");
        matches
            .into_iter()
            .map(|(_, candidate)| candidate)
            .collect()
    }

    /// Every recorded name a candidate can be discovered by, in source order with duplicates
    /// kept: the name it displays, plus every TPCF name retained under a packet root. An
    /// ambiguous packet name is unavailable to display, so those retained names are what can
    /// still find the root, and none of them is selected to render a hit.
    fn searchable_names(&self, candidate: &Candidate) -> Vec<String> {
        candidate
            .name
            .value
            .iter()
            .cloned()
            .chain(match candidate.identity {
                Identity::Packet(spid) => self.packet_names(spid),
                _ => vec![],
            })
            .collect()
    }
}
