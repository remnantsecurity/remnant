#[derive(Debug, PartialEq, Eq)]
pub enum ResponseCategory {
    Admitted,
    BlockedPolicy,
    BlockedParse,
    #[expect(
        dead_code,
        reason = "Step 5 will construct integrity-blocked outcomes before response mapping"
    )]
    BlockedIntegrity,
    #[expect(
        dead_code,
        reason = "Step 4 and Step 5 will construct fetch-blocked outcomes before response mapping"
    )]
    BlockedFetch,
    Error,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Step 3 defines the inspection outcome; Step 4 will read it from request handling"
    )
)]
pub struct InspectionOutcome {
    pub category: ResponseCategory,
    pub finding_ids: Vec<String>,
}
