impl<A: BitBakeServerAdapter> BitBakeServerController<A> {
    async fn timed_start(
        &mut self,
        operation: BitBakeServerOperation,
    ) -> Result<BitBakeServerObservation, BitBakeServerControllerError> {
        match tokio::time::timeout(self.operation_timeout, self.adapter.start(&self.context)).await
        {
            Err(_) => self.fail_timeout(operation),
            Ok(Err(error)) => self.fail_adapter(operation, error),
            Ok(Ok(mut observation)) => {
                observation.capabilities.sort();
                observation.capabilities.dedup();
                observation.validate()?;
                Ok(observation)
            }
        }
    }

    async fn timed_connect(
        &mut self,
        operation: BitBakeServerOperation,
        observation: &BitBakeServerObservation,
    ) -> Result<BitBakeServerSession, BitBakeServerControllerError> {
        match tokio::time::timeout(
            self.operation_timeout,
            self.adapter.connect(&self.context, observation),
        )
        .await
        {
            Err(_) => self.fail_timeout(operation),
            Ok(Err(error)) => self.fail_adapter(operation, error),
            Ok(Ok(session)) => {
                session.validate(observation)?;
                Ok(session)
            }
        }
    }

    async fn timed_disconnect(
        &mut self,
        operation: BitBakeServerOperation,
        session: &BitBakeServerSession,
    ) -> Result<(), BitBakeServerControllerError> {
        match tokio::time::timeout(self.operation_timeout, self.adapter.disconnect(session)).await {
            Err(_) => self.fail_timeout(operation),
            Ok(Err(error)) => self.fail_adapter(operation, error),
            Ok(Ok(())) => Ok(()),
        }
    }

    async fn timed_stop(
        &mut self,
        operation: BitBakeServerOperation,
        observation: &BitBakeServerObservation,
    ) -> Result<(), BitBakeServerControllerError> {
        match tokio::time::timeout(
            self.operation_timeout,
            self.adapter.stop(&self.context, observation),
        )
        .await
        {
            Err(_) => self.fail_timeout(operation),
            Ok(Err(error)) => self.fail_adapter(operation, error),
            Ok(Ok(())) => Ok(()),
        }
    }

    fn require_not_busy(
        &self,
        operation: BitBakeServerOperation,
    ) -> Result<(), BitBakeServerControllerError> {
        if matches!(
            self.state.lifecycle,
            BitBakeServerLifecycle::Detecting
                | BitBakeServerLifecycle::Starting
                | BitBakeServerLifecycle::Connecting
                | BitBakeServerLifecycle::Disconnecting
                | BitBakeServerLifecycle::Stopping
                | BitBakeServerLifecycle::Restarting
                | BitBakeServerLifecycle::Recovering
        ) {
            return Err(BitBakeServerControllerError::InvalidTransition {
                operation,
                lifecycle: self.state.lifecycle,
            });
        }
        if self.state.lifecycle == BitBakeServerLifecycle::Connected {
            return Err(BitBakeServerControllerError::InvalidTransition {
                operation,
                lifecycle: self.state.lifecycle,
            });
        }
        Ok(())
    }

    fn require_lifecycle(
        &self,
        operation: BitBakeServerOperation,
        allowed: &[BitBakeServerLifecycle],
    ) -> Result<(), BitBakeServerControllerError> {
        if !allowed.contains(&self.state.lifecycle) {
            return Err(BitBakeServerControllerError::InvalidTransition {
                operation,
                lifecycle: self.state.lifecycle,
            });
        }
        Ok(())
    }

    fn transition(
        &mut self,
        lifecycle: BitBakeServerLifecycle,
        diagnostic: Option<String>,
    ) -> Result<(), BitBakeServerControllerError> {
        self.state.generation = self
            .state
            .generation
            .checked_add(1)
            .ok_or(BitBakeServerControllerError::GenerationExhausted)?;
        self.state.lifecycle = lifecycle;
        self.state.diagnostic = diagnostic;
        Ok(())
    }

    fn fail_timeout<T>(
        &mut self,
        operation: BitBakeServerOperation,
    ) -> Result<T, BitBakeServerControllerError> {
        self.transition(
            BitBakeServerLifecycle::Failed,
            Some(format!("{operation:?} timed out")),
        )?;
        Err(BitBakeServerControllerError::Timeout(operation))
    }

    fn fail_adapter<T>(
        &mut self,
        operation: BitBakeServerOperation,
        error: BitBakeServerAdapterError,
    ) -> Result<T, BitBakeServerControllerError> {
        self.transition(BitBakeServerLifecycle::Failed, Some(error.message.clone()))?;
        Err(BitBakeServerControllerError::Adapter {
            operation,
            message: error.message,
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BitBakeServerControllerError {
    #[error("invalid BitBake server context: {0}")]
    InvalidContext(String),
    #[error("BitBake server timeout must be greater than zero")]
    InvalidTimeout,
    #[error("invalid BitBake server adapter data: {0}")]
    InvalidAdapterData(String),
    #[error("cannot {operation:?} while controller is {lifecycle:?}")]
    InvalidTransition {
        operation: BitBakeServerOperation,
        lifecycle: BitBakeServerLifecycle,
    },
    #[error("BitBake server observation is unavailable")]
    MissingObservation,
    #[error("BitBake server session is unavailable")]
    MissingSession,
    #[error("BitBake server {0:?} timed out")]
    Timeout(BitBakeServerOperation),
    #[error("BitBake server {operation:?} failed: {message}")]
    Adapter {
        operation: BitBakeServerOperation,
        message: String,
    },
    #[error("BitBake server controller generation is exhausted")]
    GenerationExhausted,
}
