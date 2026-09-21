use crate::{
    BitBakeServerAdapter, BitBakeServerCapability, BitBakeServerController,
    BitBakeServerControllerError, BitBakeServerLifecycle,
};
use async_trait::async_trait;
use thiserror::Error;
use yoctui_model::{BitBakeRestartAffectedJob, BitBakeRestartConfirmation, BitBakeRestartPreview};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeRestartMetadata {
    pub bitbake_version: Option<String>,
    pub machine: Option<String>,
    pub images: Vec<String>,
}

#[async_trait]
pub trait BitBakeMetadataRefresher: Send {
    async fn refresh(&mut self) -> Result<BitBakeRestartMetadata, String>;
}

pub struct BitBakeRestartCoordinator<A, R> {
    controller: BitBakeServerController<A>,
    refresher: R,
}

impl<A: BitBakeServerAdapter, R: BitBakeMetadataRefresher> BitBakeRestartCoordinator<A, R> {
    pub fn new(controller: BitBakeServerController<A>, refresher: R) -> Self {
        Self {
            controller,
            refresher,
        }
    }

    pub fn preview(
        &self,
        affected_jobs: Vec<BitBakeRestartAffectedJob>,
    ) -> Result<BitBakeRestartPreview, BitBakeRestartError> {
        let state = self.controller.state();
        if !matches!(
            state.lifecycle,
            BitBakeServerLifecycle::Available | BitBakeServerLifecycle::Connected
        ) {
            return Err(BitBakeRestartError::UnsafeLifecycle(state.lifecycle));
        }
        let observation = state
            .observation
            .as_ref()
            .ok_or(BitBakeRestartError::MissingServer)?;
        for capability in [
            BitBakeServerCapability::ServerStop,
            BitBakeServerCapability::ServerRestart,
        ] {
            if !observation.capabilities.contains(&capability) {
                return Err(BitBakeRestartError::MissingCapability(capability));
            }
        }
        if affected_jobs.len() > yoctui_model::MAX_BITBAKE_RESTART_AFFECTED_JOBS {
            return Err(BitBakeRestartError::TooManyAffectedJobs);
        }
        Ok(BitBakeRestartPreview {
            controller_generation: state.generation,
            server_identity: observation.server_identity.clone(),
            affected_jobs,
        })
    }

    pub async fn restart(
        &mut self,
        preview: &BitBakeRestartPreview,
        current_affected_jobs: Vec<BitBakeRestartAffectedJob>,
        confirmation: Option<&BitBakeRestartConfirmation>,
    ) -> Result<BitBakeRestartMetadata, BitBakeRestartError> {
        let current = self.preview(current_affected_jobs)?;
        if &current != preview {
            return Err(BitBakeRestartError::StalePreview);
        }
        if preview.requires_confirmation()
            && !confirmation.is_some_and(|value| preview.validate_confirmation(value))
        {
            return Err(BitBakeRestartError::ConfirmationRequired);
        }
        self.controller.restart().await?;
        self.refresher
            .refresh()
            .await
            .map_err(BitBakeRestartError::MetadataRefresh)
    }

    pub fn controller(&self) -> &BitBakeServerController<A> {
        &self.controller
    }

    pub fn into_parts(self) -> (BitBakeServerController<A>, R) {
        (self.controller, self.refresher)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BitBakeRestartError {
    #[error("BitBake restart is unsafe while the controller is {0:?}")]
    UnsafeLifecycle(BitBakeServerLifecycle),
    #[error("BitBake server identity is unavailable")]
    MissingServer,
    #[error("BitBake restart requires server capability {0:?}")]
    MissingCapability(BitBakeServerCapability),
    #[error("too many active jobs to preview safely")]
    TooManyAffectedJobs,
    #[error("BitBake restart preview is stale")]
    StalePreview,
    #[error("exact confirmation is required while jobs are active")]
    ConfirmationRequired,
    #[error(transparent)]
    Controller(#[from] BitBakeServerControllerError),
    #[error("BitBake restarted but authoritative metadata refresh failed: {0}")]
    MetadataRefresh(String),
}

#[cfg(test)]
#[path = "tests/bitbake_restart/mod.rs"]
mod tests;
