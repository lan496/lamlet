//! Physical dimensions and centralized conversion constants.
use serde::Serialize;

/// Exponents of mass, length, time, and temperature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Dimension(pub [i16; 4]);

impl Dimension {
    pub const SCALAR: Self = Self([0, 0, 0, 0]);
    pub const TIME: Self = Self([0, 0, 1, 0]);
    pub const LENGTH: Self = Self([0, 1, 0, 0]);
    pub const TEMPERATURE: Self = Self([0, 0, 0, 1]);
    pub const PRESSURE: Self = Self([1, -1, -2, 0]);
    pub const ENERGY: Self = Self([1, 2, -2, 0]);

    pub fn combine(self, other: Self, divide: bool) -> Option<Self> {
        let mut result = [0; 4];
        for (i, value) in result.iter_mut().enumerate() {
            *value = if divide {
                self.0[i].checked_sub(other.0[i])?
            } else {
                self.0[i].checked_add(other.0[i])?
            };
        }
        Some(Self(result))
    }

    pub fn name(self) -> String {
        match self {
            Self::SCALAR => "Dimensionless".into(),
            Self::TIME => "Time".into(),
            Self::LENGTH => "Length".into(),
            Self::TEMPERATURE => "Temperature".into(),
            Self::PRESSURE => "Pressure".into(),
            Self::ENERGY => "Energy".into(),
            Self(d) => format!("M^{} L^{} T^{} Temperature^{}", d[0], d[1], d[2], d[3]),
        }
    }
}

// Exact SI definitions: eV in joules, Avogadro constant, thermochemical kcal.
pub const JOULES_PER_EV: f64 = 1.602_176_634e-19;
pub const AVOGADRO: f64 = 6.022_140_76e23;
pub const EV_PER_KCAL_MOL: f64 = 4184.0 / AVOGADRO / JOULES_PER_EV;
const BAR_INTERNAL: f64 = 1e5 * 1e-30 / JOULES_PER_EV;
const ATM_INTERNAL: f64 = 101325.0 * 1e-30 / JOULES_PER_EV;

/// Internal numerical bases are fs, angstrom, K, and eV per particle.
pub fn unit(symbol: &str) -> Option<(Dimension, f64)> {
    Some(match symbol {
        "fs" => (Dimension::TIME, 1.0),
        "ps" => (Dimension::TIME, 1e3),
        "ns" => (Dimension::TIME, 1e6),
        "s" => (Dimension::TIME, 1e15),
        "angstrom" => (Dimension::LENGTH, 1.0),
        "nm" => (Dimension::LENGTH, 10.0),
        "m" => (Dimension::LENGTH, 1e10),
        "K" => (Dimension::TEMPERATURE, 1.0),
        "Pa" => (Dimension::PRESSURE, 1e-30 / JOULES_PER_EV),
        "bar" => (Dimension::PRESSURE, BAR_INTERNAL),
        "atm" => (Dimension::PRESSURE, ATM_INTERNAL),
        "eV" => (Dimension::ENERGY, 1.0),
        "kcal/mol" => (Dimension::ENERGY, EV_PER_KCAL_MOL),
        _ => return None,
    })
}

pub fn target_scale(style: &str, dimension: Dimension) -> Option<f64> {
    let (time, energy, pressure): (f64, f64, f64) = match style {
        "metal" => (1e3, 1.0, BAR_INTERNAL),
        "real" => (1.0, EV_PER_KCAL_MOL, ATM_INTERNAL),
        _ => return None,
    };
    if dimension == Dimension::PRESSURE {
        return Some(pressure);
    }
    let [mass, length, t, temperature] = dimension.0;
    let _ = (length, temperature); // Both target styles use angstrom and K.
    Some((energy * time.powi(2)).powi(i32::from(mass)) * time.powi(i32::from(t)))
}
