#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonDevtoolStatusData {
    pub recipe: String,
    pub recipe_file: String,
    pub capability: DaemonDevtoolCapabilityData,
    pub workspace: DaemonDevtoolWorkspaceData,
    pub git: DaemonDevtoolGitData,
    pub error: Option<DaemonDevtoolStatusErrorData>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DaemonDevtoolCapabilityData {
    Available,
    MissingExecutable,
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DaemonDevtoolWorkspaceData {
    NotMember,
    MissingDirectory {
        source_path: String,
    },
    Present {
        source_path: String,
        recipe_file: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DaemonDevtoolGitData {
    NotApplicable,
    MissingExecutable,
    NotRepository,
    Available {
        branch: Option<String>,
        head: Option<String>,
        modified: u64,
        untracked: u64,
        conflicted: u64,
    },
    Failed {
        exit_code: Option<i32>,
        message: String,
    },
    Malformed {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonDevtoolStatusErrorData {
    InvalidRecipeIdentity,
    DevtoolFailed {
        exit_code: Option<i32>,
        message: String,
    },
    MalformedOutput {
        line: String,
    },
}
