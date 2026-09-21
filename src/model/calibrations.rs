//! Calibration definitions, their alternatives and their curve or interval points.

use super::evidence::{Definition, Info, Reference, RuntimeDeclaration, Scalar};

#[derive(Clone, Debug)]
pub struct CalibrationAlternative {
    pub condition: Option<RuntimeDeclaration>,
    pub selection: Option<Definition>,
    pub calibration: Info<Calibration>,
}

#[derive(Clone, Debug)]
pub struct Calibration {
    pub reference: Reference,
    pub definition: Definition,
    pub form: Info<CalibrationForm>,
}

#[derive(Clone, Debug)]
pub struct CalibrationPoint {
    pub raw: Info<Scalar>,
    pub engineering: Info<Scalar>,
    pub definition: Definition,
}

#[derive(Clone, Debug)]
pub struct TextInterval {
    pub low: Info<Scalar>,
    pub high: Info<Scalar>,
    pub text: Info<String>,
    pub definition: Definition,
}

#[derive(Clone, Debug)]
pub enum CalibrationForm {
    Numerical {
        points: Info<Vec<CalibrationPoint>>,
        interpolation: Info<String>,
    },
    Polynomial {
        coefficients: Vec<Info<Scalar>>,
    },
    Logarithmic {
        coefficients: Vec<Info<Scalar>>,
    },
    Textual {
        intervals: Info<Vec<TextInterval>>,
    },
    CommandConversion {
        points: Info<Vec<CalibrationPoint>>,
        interpolation: Info<String>,
    },
}
