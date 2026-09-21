use crate::{
    App, ImageArtifactIdentity, PackageDetailState, PackageField, RootfsComposition,
    RootfsCompositionState, SecurityReport, SignatureComparisonState, SignatureDifferenceCategory,
    TaskInfo, TaskState,
};
use std::{collections::BTreeMap, time::SystemTime};

include!("overview/types.rs");
include!("overview/projections.rs");
include!("overview/critical_path.rs");

#[cfg(test)]
#[path = "tests/overview/mod.rs"]
mod tests;
