//! Evidence collection: one view per rendered result, which registers the definitions and
//! problems a response exposes, numbers them in display traversal order, and prints the problems
//! overview and the optional recorded-field and problem-evidence sections. Every entry point that
//! reports definitions and problems uses exactly one view, so definition ids, problem ids,
//! deduplication, contexts and evidence order stay stable across a result.

use mibl::model::*;
use std::io::{self, Write};

use super::{
    calibrations::calibration_lines,
    format::{markers, parameter_summary, quoted, reference, scalar, source, table_label, text},
};

fn alternatives<T>(a: &AtLeastTwo<T>) -> impl Iterator<Item = &T> {
    std::iter::once(a.first.as_ref())
        .chain(std::iter::once(a.second.as_ref()))
        .chain(a.rest.iter())
}

struct DefinitionEntry {
    definition: Definition,
    uses: Vec<String>,
    summary: Option<Vec<String>>,
}

struct ProblemEntry {
    problem: Problem,
    contexts: Vec<String>,
    evidence: Vec<String>,
}

/// One result's definitions and problems, registered in the order the renderer reaches them.
#[derive(Default)]
pub(super) struct View {
    definitions: Vec<DefinitionEntry>,
    problems: Vec<ProblemEntry>,
    declarations: Vec<String>,
}

impl View {
    pub(super) fn definition(&mut self, d: &Definition, context: &str) -> usize {
        let id = if let Some(index) = self
            .definitions
            .iter()
            .position(|e| e.definition.source == d.source)
        {
            if self.definitions[index].uses.iter().any(|s| s == context) {
                return index + 1;
            }
            self.definitions[index].uses.push(context.into());
            index + 1
        } else {
            let id = self.definitions.len() + 1;
            self.definitions.push(DefinitionEntry {
                definition: d.clone(),
                uses: vec![context.into()],
                summary: None,
            });
            id
        };
        for f in &d.fields {
            for m in &f.meanings {
                self.info(
                    &m.interpretation,
                    &format!("{context} {}", text(&m.schema_name)),
                );
            }
        }
        id
    }

    pub(super) fn info<T>(&mut self, i: &Info<T>, context: &str) -> Vec<usize> {
        i.problems
            .iter()
            .map(|p| self.problem(p, context))
            .collect()
    }

    pub(super) fn target(&mut self, t: &Target, context: &str) -> String {
        let id = self.definition(&t.definition, context);
        format!(
            "{}: {} [D{id}]",
            reference(&t.reference),
            source(&t.definition.source)
        )
    }

    fn runtime(&mut self, r: &RuntimeDeclaration, context: &str) -> Vec<String> {
        let mut lines = vec![format!("{context}: {}", text(&r.expression))];
        lines.extend(r.sources.iter().map(|s| format!("Source: {}", source(s))));
        for dependency in &r.dependencies {
            let context = format!("{context}, dependency {}", reference(&dependency.reference));
            let ids = self.info(&dependency.targets, &context);
            lines.push(format!("{context}{}", markers(&ids)));
            match &dependency.targets.value {
                Some(targets) if targets.is_empty() => lines.push("  Targets: none".into()),
                Some(targets) => {
                    for target in targets {
                        lines.push(format!("  Target: {}", self.target(target, &context)));
                    }
                }
                None => lines.push("  Targets: unavailable".into()),
            }
        }
        lines
    }

    fn problem(&mut self, p: &Problem, context: &str) -> usize {
        let id = if let Some(index) = self.problems.iter().position(|e| e.problem == *p) {
            if self.problems[index].contexts.iter().any(|s| s == context) {
                return index + 1;
            }
            self.problems[index].contexts.push(context.into());
            index + 1
        } else {
            let id = self.problems.len() + 1;
            self.problems.push(ProblemEntry {
                problem: p.clone(),
                contexts: vec![context.into()],
                evidence: vec![],
            });
            id
        };
        let mut evidence = vec![format!("Explanation: {}", text(&p.explanation))];
        for s in &p.sources {
            evidence.push(format!("Source: {}", source(s)));
        }
        match &p.kind {
            ProblemKind::MissingReference { reference: r } => {
                evidence.push(format!("Reference: {}; no target.", reference(r)))
            }
            ProblemKind::AmbiguousReference {
                reference: r,
                alternatives: a,
            } => {
                evidence.push(format!("Reference: {}", reference(r)));
                for t in alternatives(a) {
                    evidence.push(format!(
                        "Candidate: {}",
                        self.target(t, &format!("[P{id}] {context} candidate"))
                    ));
                }
            }
            ProblemKind::InconsistentDefinition {
                fields,
                values,
                available,
            } => {
                evidence.push(format!(
                    "Fields: {}",
                    fields
                        .iter()
                        .map(|s| text(s))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                evidence.push(format!(
                    "Values: {}",
                    values.iter().map(scalar).collect::<Vec<_>>().join(", ")
                ));
                for t in available {
                    evidence.push(format!(
                        "Declaration: {}",
                        self.target(t, &format!("[P{id}] {context} declaration"))
                    ));
                }
            }
            ProblemKind::UnsupportedInterpretation { column, meanings } => {
                if let Some(column) = column {
                    evidence.push(format!("Column: {column}"));
                }
                for m in meanings {
                    let ids = self.info(
                        &m.interpretation,
                        &format!("[P{id}] {context} {}", text(&m.schema_name)),
                    );
                    evidence.push(format!("{}{}", text(&m.schema_name), markers(&ids)));
                    evidence.extend(meaning_lines(m).into_iter().map(|s| format!("  {s}")));
                }
            }
            ProblemKind::RuntimeDependent { declaration } => {
                evidence.extend(self.runtime(declaration, &format!("[P{id}] {context} runtime")))
            }
        }
        for line in evidence {
            if !self.problems[id - 1].evidence.contains(&line) {
                self.problems[id - 1].evidence.push(line);
            }
        }
        id
    }

    pub(super) fn parameter(&mut self, p: &ParameterSummary, context: &str) {
        let id = self.definition(&p.definition, context);
        self.definitions[id - 1].summary = Some(parameter_summary(p));
        self.info(&p.description, &format!("{context} description"));
        self.info(&p.encoding.ptc, &format!("{context} PTC"));
        self.info(&p.encoding.pfc, &format!("{context} PFC"));
        self.info(&p.encoding.encoded_bits, &format!("{context} width"));
        self.info(&p.encoding.endian, &format!("{context} endian"));
        self.info(&p.units, &format!("{context} units"));
        self.info(&p.calibrations, &format!("{context} calibration"));
        if let Some(alts) = &p.calibrations.value {
            for (index, a) in alts.iter().enumerate() {
                let context = format!("{context} calibration alternative {}", index + 1);
                if let Some(d) = &a.selection {
                    self.definition(d, &context);
                }
                if let Some(r) = &a.condition {
                    self.declaration(r, &context);
                }
                self.info(&a.calibration, &context);
                if let Some(c) = &a.calibration.value {
                    self.calibration(c, &context);
                }
            }
        }
    }

    pub(super) fn calibration(&mut self, c: &Calibration, context: &str) {
        let id = self.definition(
            &c.definition,
            &format!("{context}, {}", reference(&c.reference)),
        );
        self.definitions[id - 1].summary = Some(calibration_lines(c));
        self.info(&c.form, context);
        match &c.form.value {
            Some(
                CalibrationForm::Numerical {
                    points,
                    interpolation,
                }
                | CalibrationForm::CommandConversion {
                    points,
                    interpolation,
                },
            ) => {
                self.info(interpolation, context);
                self.info(points, context);
                if let Some(points) = &points.value {
                    for p in points {
                        self.definition(&p.definition, context);
                        self.info(&p.raw, context);
                        self.info(&p.engineering, context);
                    }
                }
            }
            Some(
                CalibrationForm::Polynomial { coefficients }
                | CalibrationForm::Logarithmic { coefficients },
            ) => {
                for c in coefficients {
                    self.info(c, context);
                }
            }
            Some(CalibrationForm::Textual { intervals }) => {
                self.info(intervals, context);
                if let Some(intervals) = &intervals.value {
                    for i in intervals {
                        self.definition(&i.definition, context);
                        self.info(&i.low, context);
                        self.info(&i.high, context);
                        self.info(&i.text, context);
                    }
                }
            }
            None => {}
        }
    }

    pub(super) fn packet(&mut self, p: &PacketSummary, context: &str) {
        self.definition(&p.definition, context);
        self.info(&p.name, &format!("{context} name"));
        self.info(&p.description, &format!("{context} description"));
        self.info(&p.characteristics, &format!("{context} characteristics"));
        if let Some(ds) = &p.characteristics.value {
            for d in ds {
                self.definition(d, &format!("{context} characteristics"));
            }
        }
    }

    pub(super) fn declaration(&mut self, r: &RuntimeDeclaration, context: &str) {
        let lines = self.runtime(r, context);
        self.declarations.extend(lines);
    }

    pub(super) fn location(&mut self, l: &Location, context: &str) -> Vec<usize> {
        let mut ids = self.info(&l.position, &format!("{context} location"));
        ids.extend(self.info(&l.encoded_bits, &format!("{context} width")));
        if let Some(Position::Runtime(r)) = &l.position.value {
            self.declaration(r, context);
        }
        for r in &l.constraints {
            self.declaration(r, context);
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    pub(super) fn occurrence(&mut self, o: &ParameterOccurrence, context: &str) -> Vec<usize> {
        if let Some(p) = &o.parameter.value {
            let id = self.definition(&p.definition, context);
            self.definitions[id - 1].summary = Some(parameter_summary(p));
        }
        self.definition(&o.definition, context);
        let mut ids = self.info(&o.parameter, context);
        ids.extend(self.location(&o.location, context));
        for definition in &o.enclosing_definitions {
            self.definition(definition, &format!("{context} enclosure"));
        }
        for e in &o.enclosing {
            match e {
                Enclosure::Repetition(r) => self.repetition(r, context),
                Enclosure::Condition(r) => self.declaration(r, context),
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    pub(super) fn repetition(&mut self, r: &Info<Repetition>, context: &str) {
        self.info(r, &format!("{context} repetition"));
        match &r.value {
            Some(Repetition::Fixed { stride_bits, .. }) => {
                self.info(stride_bits, &format!("{context} stride"));
            }
            Some(Repetition::Runtime(r)) => self.declaration(r, context),
            None => {}
        }
    }

    pub(super) fn finish(&self, details: bool, out: &mut dyn Write) -> io::Result<()> {
        writeln!(out, "\nProblems")?;
        if self.problems.is_empty() {
            writeln!(out, "none")?;
        }
        for (i, p) in self.problems.iter().enumerate() {
            writeln!(
                out,
                "[P{}] {}: {}",
                i + 1,
                p.contexts
                    .iter()
                    .map(|s| if let Some(s) = s.strip_prefix("Parameter ") {
                        let mut chars = s.chars();
                        chars.next().map_or_else(String::new, |c| {
                            c.to_uppercase().to_string() + chars.as_str()
                        })
                    } else {
                        s.clone()
                    })
                    .collect::<Vec<_>>()
                    .join("; "),
                problem_summary(&p.problem)
            )?;
        }
        if !details {
            return writeln!(
                out,
                "Use --details for recorded fields and problem evidence."
            );
        }
        writeln!(out, "\nProblem evidence")?;
        if self.problems.is_empty() {
            writeln!(out, "none")?;
        }
        for (i, p) in self.problems.iter().enumerate() {
            writeln!(out, "[P{}] {}", i + 1, p.contexts.join("; "))?;
            for line in &p.evidence {
                writeln!(out, "  {line}")?;
            }
            for s in &p.problem.sources {
                if let Some(index) = self
                    .definitions
                    .iter()
                    .position(|d| d.definition.source == *s)
                {
                    writeln!(out, "  Definition: {} [D{}]", source(s), index + 1)?;
                }
            }
        }
        writeln!(out, "\nDefinitions")?;
        for line in &self.declarations {
            writeln!(out, "  {line}")?;
        }
        for (index, entry) in self.definitions.iter().enumerate() {
            writeln!(out, "[D{}] {}", index + 1, source(&entry.definition.source))?;
            for context in &entry.uses {
                writeln!(out, "  Used by: {context}")?;
            }
            if let Some(summary) = &entry.summary {
                for line in summary {
                    writeln!(out, "  {line}")?;
                }
            }
            for f in &entry.definition.fields {
                let names = f
                    .meanings
                    .iter()
                    .map(|m| text(&m.schema_name))
                    .collect::<Vec<_>>()
                    .join(" / ");
                writeln!(out, "  Column {}  {names}", f.column)?;
                let recorded = match &f.presence {
                    Presence::Omitted => "omitted".into(),
                    Presence::Empty => "empty".into(),
                    Presence::Text(s) => format!("text {}", quoted(s)),
                };
                writeln!(out, "    Recorded: {recorded}")?;
                if f.meanings.is_empty() {
                    writeln!(out, "    Interpreted: unavailable")?;
                }
                for m in &f.meanings {
                    if f.meanings.len() > 1 {
                        writeln!(out, "    Meaning: {}", text(&m.schema_name))?;
                    }
                    for line in meaning_lines(m) {
                        writeln!(out, "    {line}")?;
                    }
                    for p in &m.interpretation.problems {
                        if let Some(id) = self.problems.iter().position(|e| e.problem == *p) {
                            writeln!(out, "    Problem: [P{}]", id + 1)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn meaning_lines(m: &FieldMeaning) -> Vec<String> {
    let mut lines = Vec::new();
    match &m.interpretation.value {
        None => lines.push("Interpreted: unavailable".into()),
        Some(i) => match &i.origin {
            InterpretationOrigin::Recorded => {
                lines.push(format!("Interpreted: {}, recorded", scalar(&i.value)))
            }
            InterpretationOrigin::DocumentedDefault { rule } => {
                lines.push(format!(
                    "Interpreted: {}, documented default",
                    scalar(&i.value)
                ));
                lines.push(format!("Rule: {}", text(rule)));
            }
        },
    }
    for s in &m.interpretation.sources {
        lines.push(format!("Source: {}", source(s)));
    }
    lines
}

fn problem_summary(p: &Problem) -> String {
    match &p.kind {
        ProblemKind::MissingReference { reference: r } => format!(
            "missing reference; no matching {} definition.",
            reference(r).split_whitespace().next().unwrap_or("target")
        ),
        ProblemKind::AmbiguousReference {
            reference: r,
            alternatives: a,
        } => {
            // One key can resolve in several supporting tables, so name every table holding a candidate.
            let mut tables: Vec<String> = Vec::new();
            for target in alternatives(a) {
                if let Reference::Supporting { table, .. } = &target.reference {
                    let label = table_label(table);
                    if !tables.contains(&label) {
                        tables.push(label);
                    }
                }
            }
            let target = if tables.is_empty() {
                reference(r)
                    .split_whitespace()
                    .next()
                    .unwrap_or("target")
                    .to_owned()
            } else {
                tables.join(", ")
            };
            format!(
                "ambiguous reference; {} {target} candidates.",
                a.rest.len() + 2,
            )
        }
        ProblemKind::InconsistentDefinition { fields, values, .. }
            if fields
                .iter()
                .any(|s| s == "PIC_PI1_WID" || s == "PIC_PI2_WID") =>
        {
            format!(
                "inconsistent definition; {} bits.",
                values.iter().map(scalar).collect::<Vec<_>>().join(" and ")
            )
        }
        ProblemKind::InconsistentDefinition { .. } => {
            format!("inconsistent definition; {}", text(&p.explanation))
        }
        ProblemKind::UnsupportedInterpretation { .. } => {
            format!("unsupported interpretation; {}", text(&p.explanation))
        }
        ProblemKind::RuntimeDependent { .. } => {
            format!("runtime dependent; {}", text(&p.explanation))
        }
    }
}
