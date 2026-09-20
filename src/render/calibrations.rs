//! The summary of one calibration definition: its numerical points, polynomial or logarithmic
//! coefficients, or textual intervals, with the recorded source of every point. Evidence
//! collection registers this summary for a definition; the parameter and command views print it
//! where a calibration appears.

use mibl::model::*;

use super::format::{default_suffix, scalar, scalar_value, source, string};

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

pub(super) fn calibration_lines(c: &Calibration) -> Vec<String> {
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
