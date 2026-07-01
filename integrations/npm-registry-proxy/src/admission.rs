#[derive(Debug, PartialEq, Eq)]
pub enum ResponseCategory {
    Admitted,
    BlockedPolicy,
    BlockedParse,
    BlockedIntegrity,
    BlockedFetch,
    Error,
}

pub struct InspectionOutcome {
    pub category: ResponseCategory,
    pub finding_ids: Vec<String>,
}
