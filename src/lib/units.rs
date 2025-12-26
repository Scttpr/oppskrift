//! Unit conversion utilities for metric ↔ imperial
//!
//! Ingredients are stored in metric units and converted for display
//! based on user preference.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::fmt;

/// Measurement system preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementSystem {
    Metric,
    Imperial,
}

/// Unit categories for conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitCategory {
    Mass,
    Volume,
    Temperature,
    Length,
    Count, // pieces, eggs, etc. - no conversion
}

/// Unit conversion factor
#[derive(Debug, Clone)]
pub struct UnitConversion {
    pub from_unit: &'static str,
    pub to_unit: &'static str,
    pub factor: Decimal,
    pub category: UnitCategory,
}

/// Common unit conversions
const CONVERSIONS: &[UnitConversion] = &[
    // Mass: metric to imperial
    UnitConversion {
        from_unit: "g",
        to_unit: "oz",
        factor: dec!(0.035274),
        category: UnitCategory::Mass,
    },
    UnitConversion {
        from_unit: "kg",
        to_unit: "lb",
        factor: dec!(2.20462),
        category: UnitCategory::Mass,
    },
    // Volume: metric to imperial
    UnitConversion {
        from_unit: "ml",
        to_unit: "fl oz",
        factor: dec!(0.033814),
        category: UnitCategory::Volume,
    },
    UnitConversion {
        from_unit: "l",
        to_unit: "qt",
        factor: dec!(1.05669),
        category: UnitCategory::Volume,
    },
    // Temperature
    UnitConversion {
        from_unit: "C",
        to_unit: "F",
        factor: dec!(1.8), // special case: F = C * 1.8 + 32
        category: UnitCategory::Temperature,
    },
    // Length
    UnitConversion {
        from_unit: "cm",
        to_unit: "in",
        factor: dec!(0.393701),
        category: UnitCategory::Length,
    },
];

/// Inverse conversions (imperial to metric)
fn get_inverse_factor(conversion: &UnitConversion) -> Decimal {
    Decimal::ONE / conversion.factor
}

/// Convert a quantity between unit systems
pub fn convert_quantity(
    quantity: Decimal,
    unit: &str,
    to_system: MeasurementSystem,
) -> (Decimal, String) {
    let unit_lower = unit.to_lowercase();

    // Find applicable conversion
    for conv in CONVERSIONS {
        if to_system == MeasurementSystem::Imperial && conv.from_unit == unit_lower {
            // Metric to imperial
            if conv.category == UnitCategory::Temperature {
                // Special case: Celsius to Fahrenheit
                let fahrenheit = quantity * conv.factor + dec!(32);
                return (fahrenheit.round_dp(0), conv.to_unit.to_string());
            }
            let converted = quantity * conv.factor;
            return (converted.round_dp(2), conv.to_unit.to_string());
        } else if to_system == MeasurementSystem::Metric && conv.to_unit == unit_lower {
            // Imperial to metric
            if conv.category == UnitCategory::Temperature {
                // Special case: Fahrenheit to Celsius
                let celsius = (quantity - dec!(32)) / conv.factor;
                return (celsius.round_dp(0), conv.from_unit.to_string());
            }
            let converted = quantity * get_inverse_factor(conv);
            return (converted.round_dp(2), conv.from_unit.to_string());
        }
    }

    // No conversion found, return as-is
    (quantity, unit.to_string())
}

/// Format a quantity with its unit for display
pub fn format_quantity(quantity: Option<Decimal>, unit: Option<&str>) -> String {
    match (quantity, unit) {
        (Some(q), Some(u)) => {
            // Clean up decimal display (remove trailing zeros)
            let q_str = q.normalize().to_string();
            format!("{} {}", q_str, u)
        }
        (Some(q), None) => q.normalize().to_string(),
        (None, Some(u)) => u.to_string(),
        (None, None) => String::new(),
    }
}

/// Format a quantity converting to the user's preferred system
pub fn format_quantity_for_user(
    quantity: Option<Decimal>,
    unit: Option<&str>,
    system: MeasurementSystem,
) -> String {
    match (quantity, unit) {
        (Some(q), Some(u)) => {
            let (converted_q, converted_u) = convert_quantity(q, u, system);
            format_quantity(Some(converted_q), Some(&converted_u))
        }
        _ => format_quantity(quantity, unit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grams_to_ounces() {
        let (qty, unit) = convert_quantity(dec!(100), "g", MeasurementSystem::Imperial);
        assert_eq!(unit, "oz");
        assert!(qty > dec!(3) && qty < dec!(4)); // ~3.53 oz
    }

    #[test]
    fn test_kg_to_pounds() {
        let (qty, unit) = convert_quantity(dec!(1), "kg", MeasurementSystem::Imperial);
        assert_eq!(unit, "lb");
        assert!(qty > dec!(2.2) && qty < dec!(2.21));
    }

    #[test]
    fn test_celsius_to_fahrenheit() {
        let (qty, unit) = convert_quantity(dec!(180), "C", MeasurementSystem::Imperial);
        assert_eq!(unit, "F");
        assert_eq!(qty, dec!(356)); // 180 * 1.8 + 32 = 356
    }

    #[test]
    fn test_ml_to_fl_oz() {
        let (qty, unit) = convert_quantity(dec!(250), "ml", MeasurementSystem::Imperial);
        assert_eq!(unit, "fl oz");
        assert!(qty > dec!(8) && qty < dec!(9)); // ~8.45 fl oz
    }

    #[test]
    fn test_unknown_unit_unchanged() {
        let (qty, unit) = convert_quantity(dec!(5), "pieces", MeasurementSystem::Imperial);
        assert_eq!(qty, dec!(5));
        assert_eq!(unit, "pieces");
    }

    #[test]
    fn test_format_quantity() {
        assert_eq!(format_quantity(Some(dec!(250.00)), Some("g")), "250 g");
        assert_eq!(format_quantity(Some(dec!(1.5)), Some("kg")), "1.5 kg");
        assert_eq!(format_quantity(None, Some("pieces")), "pieces");
        assert_eq!(format_quantity(Some(dec!(3)), None), "3");
    }

    #[test]
    fn test_format_for_imperial_user() {
        let result =
            format_quantity_for_user(Some(dec!(100)), Some("g"), MeasurementSystem::Imperial);
        assert!(result.contains("oz"));
    }
}
