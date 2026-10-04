//! Untrusted Jev proposal contract. This module never authorizes orders.
//! AI output is untrusted. Only an execution gateway holding a private authorization token
//! may eventually submit, after durable reservation and a second deterministic check.
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct DecisionEnvelope {
    pub decision_id: String,
    pub snapshot_id: String,
    pub strategy_version: String,
    pub model_id: String,
    pub answer: String,
    pub probabilities: Option<Vec<Decimal>>,
    pub confidence: Option<Decimal>,
    pub received_at_ms: i64,
    pub expires_at_ms: i64,
    pub cost: Option<Decimal>,
}
pub fn validate_proposal(
    d: &DecisionEnvelope,
    snapshot: &str,
    strategy: &str,
    now: i64,
) -> Result<(), &'static str> {
    if d.model_id != "typesafe/jev-1.13" {
        return Err("MODEL_NOT_PINNED");
    }
    if d.snapshot_id != snapshot || d.strategy_version != strategy {
        return Err("STATE_CHANGED");
    }
    if d.received_at_ms > now || now >= d.expires_at_ms {
        return Err("DECISION_EXPIRED");
    }
    if !["HOLD", "CONSIDER_ENTRY", "REDUCE"].contains(&d.answer.as_str()) {
        return Err("ACTION_NOT_ALLOWED");
    }
    if d.confidence
        .is_none_or(|c| c < Decimal::ZERO || c > Decimal::ONE)
    {
        return Err("CONFIDENCE_MISSING_OR_INVALID");
    }
    if d.cost.is_none_or(|c| c < Decimal::ZERO) {
        return Err("COST_UNKNOWN");
    }
    Ok(())
}
pub fn authorize_execution() -> Result<(), &'static str> {
    Err("PROPOSAL_HAS_NO_EXECUTION_AUTHORITY")
}
#[cfg(test)]
mod tests {
    use super::*;
    fn proposal() -> DecisionEnvelope {
        DecisionEnvelope {
            decision_id: "d1".into(),
            snapshot_id: "s1".into(),
            strategy_version: "v1".into(),
            model_id: "typesafe/jev-1.13".into(),
            answer: "CONSIDER_ENTRY".into(),
            probabilities: None,
            confidence: Some(Decimal::ONE),
            received_at_ms: 100,
            expires_at_ms: 200,
            cost: Some(Decimal::ZERO),
        }
    }
    #[test]
    fn valid_proposal_is_not_permission() {
        assert!(validate_proposal(&proposal(), "s1", "v1", 150).is_ok());
        assert!(authorize_execution().is_err());
    }
    #[test]
    fn late_decision_rejected() {
        assert_eq!(
            validate_proposal(&proposal(), "s1", "v1", 200),
            Err("DECISION_EXPIRED")
        );
    }
    #[test]
    fn changed_state_rejected() {
        assert!(validate_proposal(&proposal(), "s2", "v1", 150).is_err());
    }
    #[test]
    fn ai_cannot_raise_limits() {
        let mut d = proposal();
        d.answer = "RAISE_LIMITS".into();
        assert!(validate_proposal(&d, "s1", "v1", 150).is_err());
    }
    #[test]
    fn unknown_output_fields_fail() {
        assert!(serde_json::from_str::<DecisionEnvelope>(r#"{"place_order":true}"#).is_err());
    }
    #[test]
    fn future_reply_rejected() {
        assert!(validate_proposal(&proposal(), "s1", "v1", 99).is_err());
    }
    #[test]
    fn missing_confidence_rejected() {
        let mut d = proposal();
        d.confidence = None;
        assert!(validate_proposal(&d, "s1", "v1", 150).is_err());
    }
}
