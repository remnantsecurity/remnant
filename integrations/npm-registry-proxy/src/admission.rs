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
    BlockedFetch,
    Error,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Step 3 defines the inspection outcome; Step 5 will read it from tarball request handling"
    )
)]
pub struct InspectionOutcome {
    pub category: ResponseCategory,
    pub finding_ids: Vec<String>,
}
