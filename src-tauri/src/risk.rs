//! Risk-plan math: risk per unit, planned reward-to-risk (R:R) and the
//! realized R-multiple.
//!
//! Definitions (plan entry = planned entry if given, else the actual entry):
//! - risk per unit   = long: entry − stop      · short: stop − entry
//! - reward per unit = long: target − entry    · short: entry − target
//! - planned R:R     = reward per unit ÷ risk per unit
//! - R-multiple      = price move in our favour (actual entry → exit) ÷ risk per unit
//!   (+2 = made twice what was risked, −1 = lost exactly the planned risk)

use rust_decimal::{Decimal, RoundingStrategy};
use thiserror::Error;

use crate::models::Direction;

/// R values are shown like "2.35R"; more digits are noise.
pub const R_DECIMALS: u32 = 2;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RiskError {
    #[error("stop loss must be below the entry for a long and above it for a short")]
    StopOnWrongSide,
    #[error("take profit must be above the entry for a long and below it for a short")]
    TargetOnWrongSide,
}

/// Distance from entry to stop, always > 0.
pub fn risk_per_unit(
    direction: Direction,
    entry: Decimal,
    stop: Decimal,
) -> Result<Decimal, RiskError> {
    // A stop on the profit side (or at entry) would mean zero or negative
    // risk, and dividing by it gives nonsense R values.
    let risk = favourable_move(direction, stop, entry);
    if risk > Decimal::ZERO {
        Ok(risk)
    } else {
        Err(RiskError::StopOnWrongSide)
    }
}

/// Distance from entry to target, always > 0.
pub fn reward_per_unit(
    direction: Direction,
    entry: Decimal,
    target: Decimal,
) -> Result<Decimal, RiskError> {
    let reward = favourable_move(direction, entry, target);
    if reward > Decimal::ZERO {
        Ok(reward)
    } else {
        Err(RiskError::TargetOnWrongSide)
    }
}

/// Planned reward-to-risk, e.g. 3.00 for "risk 1 to make 3".
pub fn planned_rr(
    direction: Direction,
    entry: Decimal,
    stop: Decimal,
    target: Decimal,
) -> Result<Decimal, RiskError> {
    let risk = risk_per_unit(direction, entry, stop)?;
    let reward = reward_per_unit(direction, entry, target)?;
    Ok(round_r(reward / risk))
}

/// Realized result in R. Risk comes from the plan (`plan_entry` → `stop`);
/// the move comes from what actually happened (`actual_entry` → `exit`), so
/// a bad fill shows up as a worse R.
pub fn r_multiple(
    direction: Direction,
    plan_entry: Decimal,
    stop: Decimal,
    actual_entry: Decimal,
    exit: Decimal,
) -> Result<Decimal, RiskError> {
    let risk = risk_per_unit(direction, plan_entry, stop)?;
    Ok(round_r(
        favourable_move(direction, actual_entry, exit) / risk,
    ))
}

/// How far price moved in the trade's favour going `from` → `to`
/// (negative = against us).
pub fn favourable_move(direction: Direction, from: Decimal, to: Decimal) -> Decimal {
    match direction {
        Direction::Long => to - from,
        Direction::Short => from - to,
    }
}

fn round_r(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(R_DECIMALS, RoundingStrategy::MidpointAwayFromZero)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Direction::{Long, Short};
    use rust_decimal_macros::dec;

    #[test]
    fn long_risk_reward_and_rr() {
        // Entry 100, stop 95 (risk 5), target 115 (reward 15) → 3R plan.
        assert_eq!(risk_per_unit(Long, dec!(100), dec!(95)), Ok(dec!(5)));
        assert_eq!(reward_per_unit(Long, dec!(100), dec!(115)), Ok(dec!(15)));
        assert_eq!(
            planned_rr(Long, dec!(100), dec!(95), dec!(115)),
            Ok(dec!(3))
        );
    }

    #[test]
    fn short_risk_reward_and_rr() {
        // Short at 2000, stop 2040 (risk 40), target 1900 (reward 100) → 2.5R.
        assert_eq!(risk_per_unit(Short, dec!(2000), dec!(2040)), Ok(dec!(40)));
        assert_eq!(
            planned_rr(Short, dec!(2000), dec!(2040), dec!(1900)),
            Ok(dec!(2.5))
        );
    }

    #[test]
    fn long_r_multiple_win_loss_and_full_stop() {
        assert_eq!(
            r_multiple(Long, dec!(100), dec!(95), dec!(100), dec!(110)),
            Ok(dec!(2))
        );
        assert_eq!(
            r_multiple(Long, dec!(100), dec!(95), dec!(100), dec!(97.5)),
            Ok(dec!(-0.5))
        );
        assert_eq!(
            r_multiple(Long, dec!(100), dec!(95), dec!(100), dec!(95)),
            Ok(dec!(-1))
        );
    }

    #[test]
    fn short_r_multiple_win_and_loss() {
        assert_eq!(
            r_multiple(Short, dec!(2000), dec!(2040), dec!(2000), dec!(1900)),
            Ok(dec!(2.5))
        );
        assert_eq!(
            r_multiple(Short, dec!(2000), dec!(2040), dec!(2000), dec!(2060)),
            Ok(dec!(-1.5))
        );
    }

    #[test]
    fn worse_actual_entry_lowers_r() {
        // Planned 100 / stop 95, but filled at 101: same exit 110 earns 1.8R, not 2R.
        assert_eq!(
            r_multiple(Long, dec!(100), dec!(95), dec!(101), dec!(110)),
            Ok(dec!(1.8))
        );
    }

    #[test]
    fn stop_on_wrong_side_or_at_entry_is_rejected() {
        assert_eq!(
            risk_per_unit(Long, dec!(100), dec!(105)),
            Err(RiskError::StopOnWrongSide)
        );
        assert_eq!(
            risk_per_unit(Short, dec!(100), dec!(95)),
            Err(RiskError::StopOnWrongSide)
        );
        assert_eq!(
            risk_per_unit(Long, dec!(100), dec!(100)),
            Err(RiskError::StopOnWrongSide)
        );
    }

    #[test]
    fn target_on_wrong_side_is_rejected() {
        assert_eq!(
            planned_rr(Long, dec!(100), dec!(95), dec!(90)),
            Err(RiskError::TargetOnWrongSide)
        );
        assert_eq!(
            planned_rr(Short, dec!(100), dec!(105), dec!(110)),
            Err(RiskError::TargetOnWrongSide)
        );
    }

    #[test]
    fn results_round_to_two_decimals_half_away_from_zero() {
        // 10 / 3 = 3.333… → 3.33
        assert_eq!(
            planned_rr(Long, dec!(100), dec!(97), dec!(110)),
            Ok(dec!(3.33))
        );
        // −1 / 8 = −0.125 → −0.13 (away from zero)
        assert_eq!(
            r_multiple(Long, dec!(100), dec!(92), dec!(100), dec!(99)),
            Ok(dec!(-0.13))
        );
    }

    #[test]
    fn tiny_crypto_prices_keep_precision() {
        // PEPE-style prices: entry 0.00001200, stop 0.00001150, target 0.00001350.
        assert_eq!(
            planned_rr(Long, dec!(0.00001200), dec!(0.00001150), dec!(0.00001350)),
            Ok(dec!(3))
        );
    }
}
