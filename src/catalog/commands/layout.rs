//! The declared CDF layout: unexpanded positions, nested groups and the runtime
//! repetition each group declares.

use super::super::{Catalog, missing, reference, resolve};
use super::cdf_target;
use crate::model::{
    ArgumentValue, CommandArgument, CommandElement, Dependency, Info, Layout, Location, Problem,
    ProblemKind, Reference, Repetition, RuntimeDeclaration, Scalar, Source, Table, Target,
    ValueSource,
};
use crate::reader::{Ccf, Cdf, Row};
use std::collections::HashMap;

/// CDF_GRPSIZE is a two-digit count of 1 to 99 following elements; a larger declaration is out
/// of range and stays clamped to the elements its group retains.
const MAX_GROUP_SIZE: i64 = 99;
/// CDF groups nest and CDF_GRPSIZE counts following records, so a declaration cycle must stop
/// descending; beyond this depth the repeater stays an element with its problem.
const MAX_GROUP_DEPTH: usize = 64;

impl Catalog {
    pub(super) fn command_layout(&self, root: &Row<Ccf>) -> Info<Vec<Layout<CommandElement>>> {
        let name = root.cells.cname.value.as_ref().unwrap();
        let mut rows: Vec<_> = self
            .related(Table::Cdf, name.0.clone())
            .iter()
            .map(|id| &self.records.cdf.rows()[id.0])
            .collect();
        if rows.is_empty() && !matches!(self.records.cdf, crate::reader::TableLoad::Read { .. }) {
            return missing(
                Reference::Supporting {
                    table: Table::Cdf,
                    key: name.0.clone(),
                },
                &root.definition.source,
            );
        }
        // Declared layout order is the declared unexpanded position, then source.
        rows.sort_by_key(|r| (r.cells.bit.value, &r.definition.source));
        let duplicates = duplicate_positions(&rows);
        Info {
            value: Some(self.command_elements(&rows, &[], 0, &duplicates)),
            problems: vec![],
            sources: vec![root.definition.source.clone()],
        }
    }

    /// CDF describes the unexpanded application data, and an element declaring CDF_GRPSIZE is a
    /// repeater whose value repeats the following declared elements. Groups nest, so the tree
    /// follows the declared position order recursively, keeps every member at its declared bit
    /// and never expands a repetition.
    fn command_elements(
        &self,
        rows: &[&Row<Cdf>],
        enclosing: &[RuntimeDeclaration],
        depth: usize,
        duplicates: &HashMap<Source, Problem>,
    ) -> Vec<Layout<CommandElement>> {
        let mut layout = Vec::new();
        // Groups already repeated before the current element, in declaration order.
        let mut preceding: Vec<RuntimeDeclaration> = Vec::new();
        let mut index = 0;
        while index < rows.len() {
            let row = rows[index];
            let source = &row.definition.source;
            let size = row.cells.grpsize.value.unwrap_or(0);
            let mut element = self.command_element(row);
            if let Some(problem) = duplicates.get(source) {
                element_location(&mut element)
                    .position
                    .problems
                    .push(problem.clone());
            }
            constrain(&mut element, enclosing, &preceding);
            if size > 0 && depth < MAX_GROUP_DEPTH {
                // GRPSIZE counts following declared elements, including nested repeaters.
                let end = index
                    .saturating_add(1)
                    .saturating_add(size as usize)
                    .min(rows.len());
                let complete = end - index - 1 == size as usize;
                let group =
                    self.group_repetition(row, &element, size, &rows[index + 1..end], complete);
                let nested: Vec<RuntimeDeclaration> = enclosing
                    .iter()
                    .chain(&preceding)
                    .chain(std::iter::once(&group.position))
                    .cloned()
                    .collect();
                let children =
                    self.command_elements(&rows[index + 1..end], &nested, depth + 1, duplicates);
                preceding.push(group.position);
                layout.push(Layout::Element(element));
                layout.push(Layout::Repeat {
                    definition: row.definition.clone(),
                    repetition: group.repetition,
                    children,
                });
                index = end;
            } else {
                if size != 0 {
                    let problem = invalid_group(row, size, depth >= MAX_GROUP_DEPTH);
                    element_location(&mut element)
                        .position
                        .problems
                        .push(problem);
                }
                layout.push(Layout::Element(element));
                index += 1;
            }
        }
        layout
    }

    /// The declared repetition of one group. CDF has no fixed-count column, so the count is the
    /// repeater element's value at command invocation and stays a declaration with dependencies.
    /// A fixed area cannot supply that value, and a group that outlives its declared elements
    /// keeps them with the disagreement recorded.
    fn group_repetition(
        &self,
        row: &Row<Cdf>,
        element: &CommandElement,
        size: i64,
        members: &[&Row<Cdf>],
        complete: bool,
    ) -> DeclaredGroup {
        let source = &row.definition.source;
        let mut problems = Vec::new();
        if size > MAX_GROUP_SIZE {
            problems.push(group_problem(
                row,
                size,
                members,
                "CDF_GRPSIZE is outside the declared 1 to 99 range and is clamped to the retained elements",
            ));
        } else if !complete {
            problems.push(group_problem(
                row,
                size,
                members,
                "CDF_GRPSIZE declares more elements than its group retains",
            ));
        }
        // Every member and following element carries the unexpanded-position rule. Its typed
        // count source stays on the group itself, so no location repeats that dependency.
        let position = RuntimeDeclaration {
            expression: format!(
                "Declared CDF_BIT is the unexpanded application-data position and shifts with the {size} declared element(s) this group repeats"
            ),
            dependencies: vec![],
            sources: vec![source.clone()],
        };
        let repetition = match element {
            // A fixed area carries no value, so it cannot declare how often the group repeats.
            CommandElement::Fixed(_) => {
                problems.push(group_problem(
                    row,
                    size,
                    members,
                    "A fixed area declares a group size and cannot supply a repetition count",
                ));
                Info {
                    value: None,
                    problems,
                    sources: vec![source.clone()],
                }
            }
            CommandElement::Argument(argument) => {
                let count = repetition_source(row, argument);
                let dependencies = self.repetition_dependencies(row, argument);
                // A source whose declared target never resolved keeps its problem beside the group.
                problems.extend(
                    dependencies
                        .iter()
                        .flat_map(|dependency| dependency.targets.problems.clone()),
                );
                let declaration = RuntimeDeclaration {
                    expression: format!(
                        "CDF_GRPSIZE={size} repeats the following {size} declared element(s) using {count}; declared CDF_BIT positions assume one repetition"
                    ),
                    dependencies,
                    sources: vec![source.clone()],
                };
                problems.push(Problem {
                    kind: ProblemKind::RuntimeDependent {
                        declaration: declaration.clone(),
                    },
                    sources: vec![source.clone()],
                    explanation: format!(
                        "Group of {size} declared element(s) repeats using {count}"
                    ),
                });
                Info {
                    value: Some(Repetition::Runtime(declaration)),
                    problems,
                    sources: vec![source.clone()],
                }
            }
        };
        DeclaredGroup {
            repetition,
            position,
        }
    }

    /// The runtime value that supplies a group's repetition count, typed as the declaration names
    /// it: a telemetry parameter, or the command parameter the operator or the MIB supplies.
    fn repetition_dependencies(
        &self,
        row: &Row<Cdf>,
        argument: &CommandArgument,
    ) -> Vec<Dependency> {
        if let Some(ArgumentValue {
            source: ValueSource::Telemetry { declaration, .. },
            ..
        }) = &argument.rules.element_value.value
        {
            return declaration.dependencies.clone();
        }
        let key = row.cells.pname.value.clone().unwrap_or_default();
        let reference = reference(Table::Cpc, &key);
        let rows: Vec<_> = self
            .related(Table::Cpc, key)
            .iter()
            .map(|id| &self.records.cpc.rows()[id.0])
            .collect();
        let targets = resolve(&rows, reference.clone(), &row.definition.source, |r| {
            Some(vec![Target {
                reference: reference.clone(),
                definition: r.definition.clone(),
            }])
        });
        vec![Dependency { reference, targets }]
    }
}

/// One declared CDF group: how it repeats, and the position rule its members follow.
struct DeclaredGroup {
    repetition: Info<Repetition>,
    /// The unexpanded-position rule shared by every member and following element.
    position: RuntimeDeclaration,
}

/// Every element inside or after a repeated group depends on that group's runtime expansion,
/// so each one keeps the group's position rule in the location's constraints.
fn constrain(
    element: &mut CommandElement,
    enclosing: &[RuntimeDeclaration],
    preceding: &[RuntimeDeclaration],
) {
    let location = element_location(element);
    for declaration in enclosing.iter().chain(preceding) {
        if !location.constraints.contains(declaration) {
            location.constraints.push(declaration.clone());
        }
    }
}

/// Duplicate declared positions keep every element and name every competing definition.
fn duplicate_positions(rows: &[&Row<Cdf>]) -> HashMap<Source, Problem> {
    let mut duplicates = HashMap::new();
    for group in rows.chunk_by(|a, b| a.cells.bit.value == b.cells.bit.value) {
        if group.len() < 2 {
            continue;
        }
        let problem = Problem {
            kind: ProblemKind::InconsistentDefinition {
                fields: vec!["CDF_BIT".into()],
                values: group
                    .iter()
                    .filter_map(|r| r.cells.bit.value.map(Scalar::Integer))
                    .collect(),
                available: group.iter().map(|r| cdf_target(r)).collect(),
            },
            sources: group.iter().map(|r| r.definition.source.clone()).collect(),
            explanation: "Duplicate declared command element positions".into(),
        };
        for row in group {
            duplicates.insert(row.definition.source.clone(), problem.clone());
        }
    }
    duplicates
}

/// The declaration a group and its members report: the repeater, its declared group size and
/// every element the group covers.
fn group_problem(row: &Row<Cdf>, size: i64, members: &[&Row<Cdf>], explanation: &str) -> Problem {
    Problem {
        kind: ProblemKind::InconsistentDefinition {
            fields: vec!["CDF_GRPSIZE".into()],
            values: vec![Scalar::Integer(size)],
            available: std::iter::once(cdf_target(row))
                .chain(members.iter().map(|member| cdf_target(member)))
                .collect(),
        },
        sources: vec![row.definition.source.clone()],
        explanation: explanation.into(),
    }
}

/// A declared group size that cannot declare any group: fewer than one element, or nested deeper
/// than any usable declaration. The element and its position stay available.
fn invalid_group(row: &Row<Cdf>, size: i64, too_deep: bool) -> Problem {
    Problem {
        kind: ProblemKind::InconsistentDefinition {
            fields: vec!["CDF_GRPSIZE".into()],
            values: vec![Scalar::Integer(size)],
            available: vec![cdf_target(row)],
        },
        sources: vec![row.definition.source.clone()],
        explanation: if too_deep {
            format!(
                "CDF_GRPSIZE={size} nests groups deeper than the supported depth; the retained elements stay declared at this level"
            )
        } else {
            format!("CDF_GRPSIZE={size} cannot declare a group of fewer than one element")
        },
    }
}

/// How a repeater element's value arrives at command invocation, as CDF_INTER and CDF_ELTYPE
/// declare it. The element's own value rules already interpret the recorded default.
fn repetition_source(row: &Row<Cdf>, argument: &CommandArgument) -> String {
    let key = row.cells.pname.value.clone().unwrap_or_default();
    match argument
        .rules
        .element_value
        .value
        .as_ref()
        .map(|v| &v.source)
    {
        Some(ValueSource::Telemetry { parameter, .. }) => {
            format!("the value of monitoring parameter {}", parameter.0)
        }
        Some(ValueSource::Runtime(_)) => {
            format!("the value an operator supplies for {key} at command invocation")
        }
        Some(ValueSource::Literal(value)) if row.cells.eltype.value.as_deref() == Some("E") => {
            format!(
                "the recorded value {} of {key}, which an operator may replace at command invocation",
                scalar_text(value)
            )
        }
        Some(ValueSource::Literal(value)) => {
            format!("the recorded value {} of {key}", scalar_text(value))
        }
        None => format!("the declared value of {key}"),
    }
}

/// A recorded value named inside a declaration. CLI rendering quotes values, but a declaration
/// sentence keeps the recorded text and lets the renderer escape it.
fn scalar_text(value: &Scalar) -> String {
    match value {
        Scalar::Text(text) | Scalar::Decimal(text) | Scalar::Code(text) => text.clone(),
        Scalar::Integer(number) => number.to_string(),
        Scalar::Unsigned(number) => number.to_string(),
        Scalar::Boolean(value) => value.to_string(),
    }
}

fn element_location(element: &mut CommandElement) -> &mut Location {
    match element {
        CommandElement::Argument(a) => &mut a.location,
        CommandElement::Fixed(f) => &mut f.location,
    }
}
