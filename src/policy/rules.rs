//! Policy rule registration primitives.
//!
//! This module intentionally registers only rule metadata for now. Concrete
//! rule evaluation should be added after Remnant has package metadata fields
//! worth evaluating.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// Static metadata for a policy rule.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PolicyRule {
    id: String,
    description: String,
}

impl PolicyRule {
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

/// Errors that can occur while registering policy rules.
#[derive(Debug, PartialEq, Eq)]
pub enum PolicyRuleRegistrationError {
    /// A rule was registered without a stable identifier.
    EmptyRuleId,
    /// More than one rule used the same identifier.
    DuplicateRuleId(String),
}

impl fmt::Display for PolicyRuleRegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyRuleRegistrationError::EmptyRuleId => {
                write!(f, "policy rule id must not be empty")
            }
            PolicyRuleRegistrationError::DuplicateRuleId(rule_id) => {
                write!(f, "policy rule id is duplicated: {rule_id}")
            }
        }
    }
}

impl Error for PolicyRuleRegistrationError {}

/// A deterministic registry of policy rule metadata.
#[derive(Debug, PartialEq, Eq)]
pub struct PolicyRuleRegistry {
    rules: Vec<PolicyRule>,
}

impl PolicyRuleRegistry {
    pub fn from_rules(mut rules: Vec<PolicyRule>) -> Result<Self, PolicyRuleRegistrationError> {
        let mut seen_rule_ids = BTreeSet::new();

        for rule in &rules {
            if rule.id.is_empty() {
                return Err(PolicyRuleRegistrationError::EmptyRuleId);
            }

            if !seen_rule_ids.insert(rule.id.clone()) {
                return Err(PolicyRuleRegistrationError::DuplicateRuleId(
                    rule.id.clone(),
                ));
            }
        }

        rules.sort_by(|left, right| left.id.cmp(&right.id));

        Ok(Self { rules })
    }

    pub fn rules(&self) -> &[PolicyRule] {
        &self.rules
    }
}
