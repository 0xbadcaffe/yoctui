pub struct BitBakeServerController<A> {
    adapter: A,
    context: BitBakeServerContext,
    operation_timeout: Duration,
    state: BitBakeServerControllerState,
    session: Option<BitBakeServerSession>,
}

impl<A: BitBakeServerAdapter> BitBakeServerController<A> {
    pub fn new(
        adapter: A,
        context: BitBakeServerContext,
        operation_timeout: Duration,
    ) -> Result<Self, BitBakeServerControllerError> {
        context.validate()?;
        if operation_timeout.is_zero() {
            return Err(BitBakeServerControllerError::InvalidTimeout);
        }
        Ok(Self {
            adapter,
            context,
            operation_timeout,
            state: BitBakeServerControllerState {
                lifecycle: BitBakeServerLifecycle::Unknown,
                generation: 0,
                observation: None,
                connection_identity: None,
                diagnostic: None,
            },
            session: None,
        })
    }

    pub fn state(&self) -> &BitBakeServerControllerState {
        &self.state
    }

    pub fn into_adapter(self) -> A {
        self.adapter
    }

    pub async fn detect(&mut self) -> Result<BitBakeDetection, BitBakeServerControllerError> {
        self.require_not_busy(BitBakeServerOperation::Detect)?;
        self.transition(BitBakeServerLifecycle::Detecting, None)?;
        let result =
            tokio::time::timeout(self.operation_timeout, self.adapter.detect(&self.context)).await;
        match result {
            Err(_) => self.fail_timeout(BitBakeServerOperation::Detect),
            Ok(Err(error)) => self.fail_adapter(BitBakeServerOperation::Detect, error),
            Ok(Ok(Some(mut observation))) => {
                observation.capabilities.sort();
                observation.capabilities.dedup();
                observation.validate()?;
                self.state.observation = Some(observation);
                self.session = None;
                self.transition(BitBakeServerLifecycle::Available, None)?;
                Ok(BitBakeDetection::Available)
            }
            Ok(Ok(None)) => {
                self.state.observation = None;
                self.session = None;
                self.transition(BitBakeServerLifecycle::Unavailable, None)?;
                Ok(BitBakeDetection::Unavailable)
            }
        }
    }

    pub async fn start(&mut self) -> Result<(), BitBakeServerControllerError> {
        self.require_lifecycle(
            BitBakeServerOperation::Start,
            &[
                BitBakeServerLifecycle::Unknown,
                BitBakeServerLifecycle::Unavailable,
                BitBakeServerLifecycle::Failed,
            ],
        )?;
        self.transition(BitBakeServerLifecycle::Starting, None)?;
        let result =
            tokio::time::timeout(self.operation_timeout, self.adapter.start(&self.context)).await;
        match result {
            Err(_) => self.fail_timeout(BitBakeServerOperation::Start),
            Ok(Err(error)) => self.fail_adapter(BitBakeServerOperation::Start, error),
            Ok(Ok(mut observation)) => {
                observation.capabilities.sort();
                observation.capabilities.dedup();
                observation.validate()?;
                self.state.observation = Some(observation);
                self.session = None;
                self.transition(BitBakeServerLifecycle::Available, None)
            }
        }
    }

    pub async fn connect(&mut self) -> Result<(), BitBakeServerControllerError> {
        self.require_lifecycle(
            BitBakeServerOperation::Connect,
            &[BitBakeServerLifecycle::Available],
        )?;
        let observation = self
            .state
            .observation
            .clone()
            .ok_or(BitBakeServerControllerError::MissingObservation)?;
        self.transition(BitBakeServerLifecycle::Connecting, None)?;
        let result = tokio::time::timeout(
            self.operation_timeout,
            self.adapter.connect(&self.context, &observation),
        )
        .await;
        match result {
            Err(_) => self.fail_timeout(BitBakeServerOperation::Connect),
            Ok(Err(error)) => self.fail_adapter(BitBakeServerOperation::Connect, error),
            Ok(Ok(session)) => {
                session.validate(&observation)?;
                self.state.connection_identity = Some(session.connection_identity.clone());
                self.session = Some(session);
                self.transition(BitBakeServerLifecycle::Connected, None)
            }
        }
    }

    pub async fn disconnect(&mut self) -> Result<(), BitBakeServerControllerError> {
        self.require_lifecycle(
            BitBakeServerOperation::Disconnect,
            &[BitBakeServerLifecycle::Connected],
        )?;
        let session = self
            .session
            .clone()
            .ok_or(BitBakeServerControllerError::MissingSession)?;
        self.transition(BitBakeServerLifecycle::Disconnecting, None)?;
        let result =
            tokio::time::timeout(self.operation_timeout, self.adapter.disconnect(&session)).await;
        match result {
            Err(_) => self.fail_timeout(BitBakeServerOperation::Disconnect),
            Ok(Err(error)) => self.fail_adapter(BitBakeServerOperation::Disconnect, error),
            Ok(Ok(())) => {
                self.session = None;
                self.state.connection_identity = None;
                self.transition(BitBakeServerLifecycle::Available, None)
            }
        }
    }

    pub async fn stop(&mut self) -> Result<(), BitBakeServerControllerError> {
        self.require_lifecycle(
            BitBakeServerOperation::Stop,
            &[
                BitBakeServerLifecycle::Available,
                BitBakeServerLifecycle::Connected,
            ],
        )?;
        if self.session.is_some() {
            self.disconnect().await?;
        }
        let observation = self
            .state
            .observation
            .clone()
            .ok_or(BitBakeServerControllerError::MissingObservation)?;
        self.transition(BitBakeServerLifecycle::Stopping, None)?;
        let result = tokio::time::timeout(
            self.operation_timeout,
            self.adapter.stop(&self.context, &observation),
        )
        .await;
        match result {
            Err(_) => self.fail_timeout(BitBakeServerOperation::Stop),
            Ok(Err(error)) => self.fail_adapter(BitBakeServerOperation::Stop, error),
            Ok(Ok(())) => {
                self.state.observation = None;
                self.transition(BitBakeServerLifecycle::Unavailable, None)
            }
        }
    }

    pub async fn restart(&mut self) -> Result<(), BitBakeServerControllerError> {
        self.require_lifecycle(
            BitBakeServerOperation::Restart,
            &[
                BitBakeServerLifecycle::Available,
                BitBakeServerLifecycle::Connected,
            ],
        )?;
        let reconnect = self.session.is_some();
        self.transition(BitBakeServerLifecycle::Restarting, None)?;
        if let Some(session) = self.session.take() {
            self.timed_disconnect(BitBakeServerOperation::Restart, &session)
                .await?;
            self.state.connection_identity = None;
        }
        let observation = self
            .state
            .observation
            .clone()
            .ok_or(BitBakeServerControllerError::MissingObservation)?;
        self.timed_stop(BitBakeServerOperation::Restart, &observation)
            .await?;
        let observation = self.timed_start(BitBakeServerOperation::Restart).await?;
        self.state.observation = Some(observation.clone());
        if reconnect {
            let session = self
                .timed_connect(BitBakeServerOperation::Restart, &observation)
                .await?;
            self.state.connection_identity = Some(session.connection_identity.clone());
            self.session = Some(session);
            self.transition(BitBakeServerLifecycle::Connected, None)
        } else {
            self.transition(BitBakeServerLifecycle::Available, None)
        }
    }

    pub async fn reconnect(&mut self) -> Result<(), BitBakeServerControllerError> {
        self.require_lifecycle(
            BitBakeServerOperation::Reconnect,
            &[
                BitBakeServerLifecycle::Available,
                BitBakeServerLifecycle::Connected,
                BitBakeServerLifecycle::Failed,
            ],
        )?;
        let observation = self
            .state
            .observation
            .clone()
            .ok_or(BitBakeServerControllerError::MissingObservation)?;
        self.transition(BitBakeServerLifecycle::Recovering, None)?;
        if let Some(session) = self.session.take() {
            self.timed_disconnect(BitBakeServerOperation::Reconnect, &session)
                .await?;
            self.state.connection_identity = None;
        }
        let session = self
            .timed_connect(BitBakeServerOperation::Reconnect, &observation)
            .await?;
        self.state.connection_identity = Some(session.connection_identity.clone());
        self.session = Some(session);
        self.transition(BitBakeServerLifecycle::Connected, None)
    }


}
