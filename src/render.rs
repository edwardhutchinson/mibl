//! Plain, deterministic views of the returned description. Never queries the MIB.
mod commands;
pub(super) use commands::command;
use mibl::model::*;
use std::{
    collections::BTreeMap,
    io::{self, Write},
};
use unicode_width::UnicodeWidthStr;

fn text(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
fn quoted(s: &str) -> String {
    let escaped: String = s
        .chars()
        .map(|c| match c {
            '\\' => "\\\\".into(),
            '"' => "\\\"".into(),
            c if c.is_control() => c.escape_default().to_string(),
            c => c.to_string(),
        })
        .collect();
    format!("\"{escaped}\"")
}

fn source(s: &Source) -> String {
    format!("{}:{}", text(&s.file.to_string_lossy()), s.line)
}
fn number<T: std::fmt::Display>(i: &Info<T>) -> String {
    i.value
        .as_ref()
        .map_or_else(|| "unavailable".into(), ToString::to_string)
}
fn string(i: &Info<String>) -> String {
    i.value
        .as_deref()
        .map_or_else(|| "unavailable".into(), text)
}
fn width(i: &Info<u64>) -> String {
    i.value
        .map_or_else(|| "unavailable".into(), |n| format!("{n} bits"))
}
fn scalar_value(i: &Info<Scalar>) -> String {
    i.value
        .as_ref()
        .map_or_else(|| "unavailable".into(), scalar)
}
fn scalar(s: &Scalar) -> String {
    match s {
        Scalar::Text(s) | Scalar::Code(s) | Scalar::Decimal(s) => quoted(s),
        Scalar::Integer(n) => n.to_string(),
        Scalar::Unsigned(n) => n.to_string(),
        Scalar::Boolean(b) => b.to_string(),
    }
}
fn table(rows: &[Vec<String>], out: &mut dyn Write) -> io::Result<()> {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<_> = (0..columns)
        .map(|col| {
            rows.iter()
                .filter_map(|r| r.get(col))
                .map(|s| s.width())
                .max()
                .unwrap_or(0)
        })
        .collect();
    for row in rows {
        for (col, cell) in row.iter().enumerate() {
            write!(out, "{cell}")?;
            if col + 1 < row.len() {
                write!(out, "{}", " ".repeat(widths[col] - cell.width() + 2))?;
            }
        }
        writeln!(out)?;
    }
    Ok(())
}
fn default_suffix(d: &Definition, name: &str) -> &'static str {
    if d.fields.iter().flat_map(|f| &f.meanings).any(|m| {
        m.schema_name == name
            && m.interpretation
                .value
                .as_ref()
                .is_some_and(|i| matches!(i.origin, InterpretationOrigin::DocumentedDefault { .. }))
    }) {
        " [default]"
    } else {
        ""
    }
}
fn field_value<'a>(d: &'a Definition, name: &str) -> Option<&'a Scalar> {
    d.fields
        .iter()
        .flat_map(|f| &f.meanings)
        .find(|m| m.schema_name == name)?
        .interpretation
        .value
        .as_ref()
        .map(|i| &i.value)
}
/// A recorded textual field, printed without the quoting that marks an interpreted value.
fn recorded_text<'a>(d: &'a Definition, name: &str) -> Option<&'a str> {
    match field_value(d, name)? {
        Scalar::Text(s) | Scalar::Code(s) | Scalar::Decimal(s) => Some(s),
        _ => None,
    }
}
fn parameter_summary(p: &ParameterSummary) -> Vec<String> {
    let e = &p.encoding;
    let kind = encoding_kind(e);
    let endian = match e.endian.value.as_deref() {
        Some("B") => "big endian".into(),
        Some("L") => "little endian".into(),
        _ => string(&e.endian),
    };
    vec![
        format!(
            "Description: {}{}",
            string(&p.description),
            default_suffix(&p.definition, "PCF_DESCR")
        ),
        format!(
            "Encoding: {kind}, {}, {endian}{} (PTC {} / PFC {})",
            width(&e.encoded_bits),
            default_suffix(&p.definition, "PCF_ENDIAN"),
            number(&e.ptc),
            number(&e.pfc)
        ),
        format!(
            "Units: {}{}",
            string(&p.units),
            default_suffix(&p.definition, "PCF_UNIT")
        ),
    ]
}
fn position(p: &Info<Position>) -> String {
    match &p.value {
        Some(Position::PacketAbsolute { byte, bit }) => format!("byte {byte} bit {bit}"),
        Some(Position::ApplicationDeclaredBit(n)) => format!("application-declared bit {n}"),
        Some(Position::HeaderBit(n)) => format!("header bit {n}"),
        Some(Position::RelativeBits(n)) => format!("relative {n} bits"),
        Some(Position::Runtime(r)) => format!("runtime dependent: {}", text(&r.expression)),
        None => "unavailable".into(),
    }
}
fn table_label(table: &Table) -> String {
    format!("{table:?}").to_uppercase()
}
fn reference(r: &Reference) -> String {
    match r {
        Reference::Root(Identity::Parameter(n)) => format!("PCF NAME {}", text(&n.0)),
        Reference::Root(Identity::Packet(n)) => format!("PID SPID {}", n.0),
        Reference::Root(Identity::Command(n)) => format!("CCF NAME {}", text(&n.0)),
        Reference::Supporting { table, key } => {
            format!("{} {}", table_label(table), text(key))
        }
        Reference::Deferred { concept, key } => format!("{} {}", text(concept), text(key)),
    }
}
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
#[derive(Default)]
struct View {
    definitions: Vec<DefinitionEntry>,
    problems: Vec<ProblemEntry>,
    declarations: Vec<String>,
}
impl View {
    fn definition(&mut self, d: &Definition, context: &str) -> usize {
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
    fn info<T>(&mut self, i: &Info<T>, context: &str) -> Vec<usize> {
        i.problems
            .iter()
            .map(|p| self.problem(p, context))
            .collect()
    }
    fn target(&mut self, t: &Target, context: &str) -> String {
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
    fn parameter(&mut self, p: &ParameterSummary, context: &str) {
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
    fn calibration(&mut self, c: &Calibration, context: &str) {
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
    fn packet(&mut self, p: &PacketSummary, context: &str) {
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
    fn declaration(&mut self, r: &RuntimeDeclaration, context: &str) {
        let lines = self.runtime(r, context);
        self.declarations.extend(lines);
    }
    fn location(&mut self, l: &Location, context: &str) -> Vec<usize> {
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
    fn occurrence(&mut self, o: &ParameterOccurrence, context: &str) -> Vec<usize> {
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
    fn repetition(&mut self, r: &Info<Repetition>, context: &str) {
        self.info(r, &format!("{context} repetition"));
        match &r.value {
            Some(Repetition::Fixed { stride_bits, .. }) => {
                self.info(stride_bits, &format!("{context} stride"));
            }
            Some(Repetition::Runtime(r)) => self.declaration(r, context),
            None => {}
        }
    }
    fn finish(&self, details: bool, out: &mut dyn Write) -> io::Result<()> {
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
fn markers(ids: &[usize]) -> String {
    ids.iter().map(|id| format!(" [P{id}]")).collect()
}
fn repeat(o: &ParameterOccurrence, seen: &mut BTreeMap<Source, u64>) -> String {
    let instance = seen.entry(o.definition.source.clone()).or_default();
    *instance += 1;
    let mut parts = Vec::new();
    for e in &o.enclosing {
        parts.push(match e {
            Enclosure::Repetition(r) => match &r.value {
                Some(Repetition::Fixed { count, stride_bits })
                    if field_value(&o.definition, "VPD_TPSD").is_some() =>
                {
                    format!("fixed count {count}, stride {}", width(stride_bits))
                }
                Some(Repetition::Fixed { count, stride_bits }) => format!(
                    "{instance}/{count}, stride {}{}",
                    width(stride_bits),
                    default_suffix(&o.definition, "PLF_LGOCC")
                ),
                Some(Repetition::Runtime(r)) => format!("runtime {}", text(&r.expression)),
                None => "unavailable".into(),
            },
            Enclosure::Condition(r) => format!("condition {}", text(&r.expression)),
        });
    }
    if parts.is_empty() {
        format!("once{}", default_suffix(&o.definition, "PLF_NBOCC"))
    } else {
        parts.join("; ")
    }
}
fn location_text(o: &ParameterOccurrence) -> String {
    format!(
        "{}{}",
        position(&o.location.position),
        if default_suffix(&o.definition, "PLF_OFFBY").is_empty() {
            default_suffix(&o.definition, "PLF_OFFBI")
        } else {
            " [default]"
        }
    )
}
pub(super) fn parameter(
    p: &ParameterDescription,
    details: bool,
    out: &mut dyn Write,
) -> io::Result<()> {
    let mut view = View::default();
    view.parameter(&p.parameter, "Parameter");
    writeln!(out, "Parameter {}", text(&p.parameter.name.0))?;
    for line in parameter_summary(&p.parameter) {
        writeln!(out, "{line}")?;
    }
    writeln!(
        out,
        "Source: {}\n\nPackets",
        source(&p.parameter.definition.source)
    )?;
    view.info(&p.occurrences, "Packets");
    let mut rows = vec![vec![
        "SPID".into(),
        "Name".into(),
        "Location".into(),
        "Width".into(),
        "Repeat".into(),
    ]];
    let mut seen = BTreeMap::new();
    if let Some(packets) = &p.occurrences.value {
        for packet in packets {
            view.info(&packet.packet, "Containing packet");
            if let Some(p) = &packet.packet.value {
                view.packet(p, &format!("Packet {}", p.spid.0));
            }
            for o in &packet.occurrences {
                let spid = packet
                    .packet
                    .value
                    .as_ref()
                    .map(|p| p.spid.0.to_string())
                    .or_else(|| field_value(&o.definition, "PLF_SPID").map(scalar))
                    .unwrap_or_else(|| "unavailable".into());
                let context = format!(
                    "Packet {spid} {} at {}",
                    text(&o.reference.0),
                    location_text(o)
                );
                let ids = view.occurrence(o, &context);
                if let Some(parameter) = &o.parameter.value {
                    view.parameter(parameter, "Parameter");
                }
                rows.push(vec![
                    spid,
                    packet
                        .packet
                        .value
                        .as_ref()
                        .map_or_else(|| "unavailable".into(), |p| string(&p.name)),
                    location_text(o),
                    width(&o.location.encoded_bits),
                    format!("{}{}", repeat(o, &mut seen), markers(&ids)),
                ]);
            }
        }
        if rows.len() == 1 {
            writeln!(out, "none")?;
        } else {
            table(&rows, out)?;
        }
    } else {
        writeln!(out, "unavailable")?;
    }
    writeln!(out, "\nCalibrations")?;
    match &p.parameter.calibrations.value {
        Some(alternatives) if alternatives.is_empty() => writeln!(out, "none declared")?,
        Some(alternatives) => {
            for (i, a) in alternatives.iter().enumerate() {
                writeln!(out, "Alternative {}", i + 1)?;
                if let Some(condition) = &a.condition {
                    writeln!(
                        out,
                        "  Condition: {} [runtime dependent]",
                        text(&condition.expression)
                    )?;
                }
                if let Some(c) = &a.calibration.value {
                    writeln!(
                        out,
                        "  {} at {}",
                        reference(&c.reference),
                        source(&c.definition.source)
                    )?;
                    for line in calibration_lines(c) {
                        writeln!(out, "  {line}")?;
                    }
                } else {
                    writeln!(out, "  unavailable")?;
                }
            }
        }
        None => writeln!(out, "unavailable")?,
    }
    view.finish(details, out)
}
pub(super) fn packet(p: &PacketDescription, details: bool, out: &mut dyn Write) -> io::Result<()> {
    let mut view = View::default();
    view.packet(&p.packet, "Packet");
    writeln!(
        out,
        "Packet {}  {}\nDescription: {}\nSource: {}",
        p.packet.spid.0,
        string(&p.packet.name),
        string(&p.packet.description),
        source(&p.packet.definition.source)
    )?;
    let i = &p.identification;
    for d in &i.definitions {
        view.definition(d, "Identification");
    }
    view.info(&i.apid, "APID");
    view.info(&i.service_type, "Service type");
    view.info(&i.service_subtype, "Service subtype");
    writeln!(
        out,
        "\nIdentification\nAPID: {}{}\nService: type {}{}, subtype {}{}",
        number(&i.apid),
        default_suffix(&p.packet.definition, "PID_APID"),
        number(&i.service_type),
        default_suffix(&p.packet.definition, "PID_TYPE"),
        number(&i.service_subtype),
        default_suffix(&p.packet.definition, "PID_STYPE")
    )?;
    match &i.criteria.value {
        Some(cs) if cs.is_empty() => writeln!(out, "Additional criteria: none declared")?,
        Some(cs) => {
            for (index, c) in cs.iter().enumerate() {
                // The result omits disabled criteria; identify PI2 by its retained PID field sources/values and PIC declarations.
                let pi = criterion_number(i, index);
                let context = format!("PI{pi} extraction");
                let ids = view.location(&c.extraction, &context);
                view.info(&c.expected, &format!("PI{pi} expected"));
                for d in &c.definitions {
                    view.definition(d, &format!("PI{pi}"));
                }
                writeln!(
                    out,
                    "PI{pi} expected: {}{}; location: {}; width: {}{}",
                    number(&c.expected),
                    default_suffix(&p.packet.definition, &format!("PID_PI{pi}_VAL")),
                    position(&c.extraction.position),
                    width(&c.extraction.encoded_bits),
                    markers(&ids)
                )?;
            }
        }
        None => writeln!(out, "Additional criteria: unavailable")?,
    }
    writeln!(out, "\nLayout")?;
    view.info(&p.layout, "Layout");
    let mut occurrences = Vec::new();
    let mut rows = vec![vec![
        "Parameter".into(),
        "Location".into(),
        "Width".into(),
        "Repeat".into(),
    ]];
    if let Some(layout) = &p.layout.value {
        collect_layout(
            layout,
            &mut occurrences,
            &mut view,
            &mut rows,
            &mut BTreeMap::new(),
            0,
        );
    }
    if rows.len() > 1 {
        if p.layout.value.as_ref().is_some_and(|layout| {
            layout
                .iter()
                .any(|node| !matches!(node, Layout::Element(_)))
        }) {
            for row in rows.iter().skip(1) {
                writeln!(out, "{}", row.join("  "))?;
            }
        } else {
            table(&rows, out)?;
        }
    } else {
        writeln!(
            out,
            "{}",
            if p.layout.value.is_some() {
                "none"
            } else {
                "unavailable"
            }
        )?;
    }
    for o in occurrences {
        if let Some(parameter) = &o.parameter.value {
            view.parameter(parameter, &text(&o.reference.0));
        }
    }
    view.info(&i.criteria, "Identification criteria");
    view.finish(details, out)
}
fn criterion_number(i: &PacketIdentification, index: usize) -> usize {
    if index == 0
        && !i.definitions.is_empty()
        && i.definitions
            .iter()
            .all(|d| matches!(field_value(d, "PIC_PI1_OFF"), Some(Scalar::Integer(-1))))
    {
        2
    } else {
        index + 1
    }
}
fn collect_layout<'a>(
    layout: &'a [Layout<ParameterOccurrence>],
    occurrences: &mut Vec<&'a ParameterOccurrence>,
    view: &mut View,
    rows: &mut Vec<Vec<String>>,
    seen: &mut BTreeMap<Source, u64>,
    depth: usize,
) {
    for item in layout {
        match item {
            Layout::Element(o) => {
                occurrences.push(o);
                let context = format!("{} at {}", text(&o.reference.0), location_text(o));
                let ids = view.occurrence(o, &context);
                rows.push(vec![
                    format!("{}{}", "  ".repeat(depth), text(&o.reference.0)),
                    location_text(o),
                    width(&o.location.encoded_bits),
                    format!("{}{}", repeat(o, seen), markers(&ids)),
                ]);
            }
            Layout::Repeat {
                definition,
                repetition,
                children,
            } => {
                view.definition(definition, "Layout repeat");
                view.repetition(repetition, "Layout repeat");
                let label = match &repetition.value {
                    Some(Repetition::Fixed { count, stride_bits }) => {
                        format!("fixed count {count}, stride {}", width(stride_bits))
                    }
                    Some(Repetition::Runtime(r)) => format!("runtime {}", text(&r.expression)),
                    None => "unavailable".into(),
                };
                rows.push(vec![
                    format!("{}Repeat group", "  ".repeat(depth)),
                    source(&definition.source),
                    String::new(),
                    label,
                ]);
                collect_layout(children, occurrences, view, rows, seen, depth + 1);
                rows.push(vec![format!("{}End repeat", "  ".repeat(depth))]);
            }
            Layout::Conditional {
                definition,
                condition,
                children,
            } => {
                view.definition(definition, "Layout condition");
                view.declaration(condition, "Layout condition");
                rows.push(vec![
                    format!("{}Conditional structure", "  ".repeat(depth)),
                    source(&definition.source),
                    String::new(),
                    text(&condition.expression),
                ]);
                collect_layout(children, occurrences, view, rows, seen, depth + 1);
            }
        }
    }
}
pub(super) fn candidates<'a>(
    candidates: impl Iterator<Item = &'a Candidate>,
    out: &mut dyn Write,
) -> io::Result<()> {
    let mut rows = vec![vec![
        "Kind".into(),
        "Identity".into(),
        "PUS".into(),
        "Name".into(),
        "Description".into(),
        "Source".into(),
    ]];
    for c in candidates {
        let (kind, identity) = match &c.identity {
            Identity::Parameter(n) => ("parameter", text(&n.0)),
            Identity::Packet(n) => ("packet", n.0.to_string()),
            Identity::Command(n) => ("command", text(&n.0)),
        };
        rows.push(vec![
            kind.into(),
            identity,
            match (&c.identity, c.service_type) {
                (Identity::Packet(_) | Identity::Command(_), Some(service)) => {
                    let prefix = if matches!(c.identity, Identity::Packet(_)) {
                        "TM"
                    } else {
                        "TC"
                    };
                    let subtype = c
                        .service_subtype
                        .map_or_else(|| "-".into(), |s| s.to_string());
                    format!("{prefix}({service},{subtype})")
                }
                _ => "-".into(),
            },
            string(&c.name),
            string(&c.description),
            source(&c.source),
        ]);
    }
    table(&rows, out)
}

/// Every supported table, with the file name it comes from, what its rows declare, the
/// lookup that addresses them directly, and what the loaded directory provided. The
/// listing never depends on `--details`, which adds nothing here.
pub(super) fn tables<'a>(
    reports: impl Iterator<Item = &'a TableReport>,
    out: &mut dyn Write,
) -> io::Result<()> {
    let mut rows = vec![vec![
        "Code".into(),
        "File".into(),
        "Defines".into(),
        "Lookup".into(),
        "Rows".into(),
    ]];
    for report in reports {
        rows.push(vec![
            report.table.code().into(),
            report.table.file().into(),
            report.table.meaning().into(),
            match report.table.root() {
                Some(TableRoot::Parameter) => "parameter".into(),
                Some(TableRoot::Packet) => "packet".into(),
                Some(TableRoot::Command) => "command".into(),
                None => "-".into(),
            },
            match report.rows {
                TableRows::Missing => "missing".into(),
                TableRows::Unreadable => "unreadable".into(),
                TableRows::Read { rows } => rows.to_string(),
            },
        ]);
    }
    table(&rows, out)
}

#[cfg(test)]
mod tests;

fn encoding_kind(e: &Encoding) -> &'static str {
    match e.ptc.value {
        Some(1) => "boolean",
        Some(2) => "enumerated",
        Some(3) => "unsigned integer",
        Some(4) => "signed integer",
        Some(5) => "real",
        Some(6) => "bit string",
        Some(7) => "octet string",
        Some(8) => "character string",
        Some(9) => "absolute time",
        Some(10) => "relative time",
        Some(11) => "deduced",
        Some(13) => "saved synthetic",
        _ => "unavailable",
    }
}

/// The documented-default marker of the first of `names` that carries one.
fn default_suffix_any(d: &Definition, names: &[&str]) -> &'static str {
    names
        .iter()
        .map(|name| default_suffix(d, name))
        .find(|suffix| !suffix.is_empty())
        .unwrap_or("")
}
fn coefficient_line(
    label: &str,
    prefix: &str,
    coefficients: &[Info<Scalar>],
    d: &Definition,
) -> Vec<String> {
    vec![format!(
        "{label} coefficients A0..A4: {}",
        coefficients
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let value = v
                    .value
                    .as_ref()
                    .map_or_else(|| "unavailable".to_owned(), scalar);
                format!(
                    "{value}{}",
                    default_suffix(d, &format!("{prefix}{}", i + 1))
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )]
}
fn calibration_lines(c: &Calibration) -> Vec<String> {
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
            let mut lines = vec![format!(
                "Numerical curve; extrapolation: {}{}",
                string(interpolation),
                default_suffix_any(&c.definition, &["CAF_INTER", "CCA_INTER"])
            )];
            if let Some(points) = &points.value {
                lines.extend(points.iter().map(|p| {
                    format!(
                        "{} -> {} [{}]",
                        scalar_value(&p.raw),
                        scalar_value(&p.engineering),
                        source(&p.definition.source)
                    )
                }));
            } else {
                lines.push("Points: unavailable".into());
            }
            lines
        }
        Some(CalibrationForm::Polynomial { coefficients }) => {
            coefficient_line("Polynomial", "MCF_POL", coefficients, &c.definition)
        }
        Some(CalibrationForm::Logarithmic { coefficients }) => {
            coefficient_line("Logarithmic", "LGF_POL", coefficients, &c.definition)
        }
        Some(CalibrationForm::Textual { intervals }) => {
            let mut lines = vec!["Textual intervals".into()];
            if let Some(intervals) = &intervals.value {
                lines.extend(intervals.iter().map(|i| {
                    format!(
                        "{}..{} -> {} [{}]",
                        scalar_value(&i.low),
                        scalar_value(&i.high),
                        string(&i.text),
                        source(&i.definition.source)
                    )
                }));
            } else {
                lines.push("Intervals: unavailable".into());
            }
            lines
        }
        None => vec!["Calibration form: unavailable".into()],
    }
}
