//! Cell declarations, recorded-field reading and typed cell access shared by every family.

use crate::model::{
    Definition, FieldMeaning, Info, Interpretation, InterpretationOrigin, Presence, RecordedField,
    Scalar, Source,
};

use CellType::{Code, Integer, Text};

#[derive(Clone, Copy)]
pub(super) enum CellType {
    Text,
    Integer,
    Code(&'static str),
}

pub(super) struct Column {
    pub(super) name: &'static str,
    pub(super) kind: CellType,
    pub(super) required: bool,
    pub(super) default: Option<&'static str>,
}

pub(super) fn parse_definition(
    text: &str,
    source: Source,
    schema: &[Column],
) -> Result<Definition, String> {
    let cells: Vec<_> = text.split('\t').collect();
    if cells.len() > schema.len() {
        tracing::debug!(file = %source.file.display(), line = source.line.get(), extra = cells.len() - schema.len(), "ignored extra columns");
    }
    let mut fields = Vec::with_capacity(schema.len());
    for (index, column) in schema.iter().enumerate() {
        let recorded = cells.get(index).copied();
        let presence = match recorded {
            None => Presence::Omitted,
            Some("") => Presence::Empty,
            Some(s) => Presence::Text(s.to_owned()),
        };
        let value = recorded.filter(|s| !s.is_empty()).or(column.default);
        if column.required && value.is_none() {
            return Err(format!("{} is required", column.name));
        }
        let interpretation = value
            .map(|s| {
                let value = match column.kind {
                    Text => Scalar::Text(s.to_owned()),
                    Integer => Scalar::Integer(
                        s.parse()
                            .map_err(|_| format!("{}: invalid integer {s:?}", column.name))?,
                    ),
                    Code(allowed) if s.len() == 1 && allowed.contains(s) => {
                        Scalar::Code(s.to_owned())
                    }
                    Code(_) => return Err(format!("{}: invalid code {s:?}", column.name)),
                };
                let origin = if recorded.is_some_and(|s| !s.is_empty()) {
                    InterpretationOrigin::Recorded
                } else {
                    InterpretationOrigin::DocumentedDefault {
                        rule: format!("{} defaults to {s:?} when omitted or empty", column.name),
                    }
                };
                Ok(Interpretation { value, origin })
            })
            .transpose()?;
        fields.push(RecordedField {
            column: (index + 1).try_into().unwrap(),
            presence,
            meanings: vec![FieldMeaning {
                schema_name: column.name.into(),
                interpretation: Info {
                    value: interpretation,
                    problems: vec![],
                    sources: vec![source.clone()],
                },
            }],
        });
    }
    Ok(Definition { source, fields })
}

pub(super) fn cell<T>(
    definition: &Definition,
    index: usize,
    convert: impl FnOnce(&Scalar) -> Option<T>,
) -> Info<T> {
    let interpretation = &definition.fields[index].meanings[0].interpretation;
    Info {
        value: interpretation
            .value
            .as_ref()
            .and_then(|i| convert(&i.value)),
        problems: interpretation.problems.clone(),
        sources: interpretation.sources.clone(),
    }
}

pub(super) fn text_cell(definition: &Definition, index: usize) -> Info<String> {
    cell(definition, index, |s| match s {
        Scalar::Text(s) | Scalar::Code(s) => Some(s.clone()),
        _ => None,
    })
}

pub(super) fn integer_cell(definition: &Definition, index: usize) -> Info<i64> {
    cell(definition, index, |s| match s {
        Scalar::Integer(n) => Some(*n),
        _ => None,
    })
}
