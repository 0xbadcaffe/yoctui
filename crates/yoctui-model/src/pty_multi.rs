use std::collections::{BTreeMap, BTreeSet, VecDeque};

use thiserror::Error;

use crate::{
    MAX_PTY_NAME_BYTES, PtyClientId, PtyExitStatus, PtySession, PtySessionId, PtySessionKind,
    PtySessionLifecycle,
};

pub const MAX_PTY_SESSIONS: usize = 64;
pub const MAX_PTY_SESSION_HISTORY: usize = 512;
pub const MAX_PTY_REGISTRY_CLIENTS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtySessionHistoryEntry {
    pub id: PtySessionId,
    pub name: String,
    pub kind: PtySessionKind,
    pub lifecycle: PtySessionLifecycle,
    pub exit_status: Option<PtyExitStatus>,
    pub restartable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyCloseDisposition {
    TerminationRequired,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtySessionRegistry {
    sessions: BTreeMap<PtySessionId, PtySession>,
    reserved: BTreeSet<PtySessionId>,
    selected: BTreeMap<PtyClientId, PtySessionId>,
    history: VecDeque<PtySessionHistoryEntry>,
    next_id: u64,
    max_sessions: usize,
    max_history: usize,
    max_clients: usize,
    dropped_history: u64,
}

impl PtySessionRegistry {
    pub fn new(
        max_sessions: usize,
        max_history: usize,
        max_clients: usize,
    ) -> Result<Self, PtyRegistryError> {
        if max_sessions == 0 || max_sessions > MAX_PTY_SESSIONS {
            return Err(PtyRegistryError::InvalidLimit("sessions", max_sessions));
        }
        if max_history == 0 || max_history > MAX_PTY_SESSION_HISTORY {
            return Err(PtyRegistryError::InvalidLimit("history", max_history));
        }
        if max_clients == 0 || max_clients > MAX_PTY_REGISTRY_CLIENTS {
            return Err(PtyRegistryError::InvalidLimit("clients", max_clients));
        }
        Ok(Self {
            sessions: BTreeMap::new(),
            reserved: BTreeSet::new(),
            selected: BTreeMap::new(),
            history: VecDeque::new(),
            next_id: 1,
            max_sessions,
            max_history,
            max_clients,
            dropped_history: 0,
        })
    }

    pub fn reserve_id(&mut self) -> Result<PtySessionId, PtyRegistryError> {
        if self.sessions.len() + self.reserved.len() >= self.max_sessions {
            return Err(PtyRegistryError::SessionLimit(self.max_sessions));
        }
        let id = PtySessionId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(PtyRegistryError::IdExhausted)?;
        self.reserved.insert(id);
        Ok(id)
    }

    pub fn cancel_reservation(&mut self, id: PtySessionId) -> Result<(), PtyRegistryError> {
        if !self.reserved.remove(&id) {
            return Err(PtyRegistryError::UnknownReservation(id));
        }
        Ok(())
    }

    pub fn insert(&mut self, session: PtySession) -> Result<(), PtyRegistryError> {
        if !self.reserved.remove(&session.id) {
            return Err(PtyRegistryError::UnknownReservation(session.id));
        }
        validate_registry_name(&session.name)?;
        if self
            .sessions
            .values()
            .any(|existing| existing.name == session.name)
        {
            self.reserved.insert(session.id);
            return Err(PtyRegistryError::DuplicateName(session.name));
        }
        self.sessions.insert(session.id, session);
        Ok(())
    }

    pub fn get(&self, id: PtySessionId) -> Option<&PtySession> {
        self.sessions.get(&id)
    }

    pub fn get_mut(&mut self, id: PtySessionId) -> Option<&mut PtySession> {
        self.sessions.get_mut(&id)
    }

    pub fn sessions(&self) -> impl ExactSizeIterator<Item = &PtySession> {
        self.sessions.values()
    }

    pub fn history(&self) -> &VecDeque<PtySessionHistoryEntry> {
        &self.history
    }

    pub fn dropped_history(&self) -> u64 {
        self.dropped_history
    }

    pub fn rename(&mut self, id: PtySessionId, name: String) -> Result<(), PtyRegistryError> {
        validate_registry_name(&name)?;
        if self
            .sessions
            .iter()
            .any(|(other_id, session)| *other_id != id && session.name == name)
        {
            return Err(PtyRegistryError::DuplicateName(name));
        }
        let session = self
            .sessions
            .get_mut(&id)
            .ok_or(PtyRegistryError::UnknownSession(id))?;
        session
            .apply(crate::PtySessionAction::Rename(name))
            .map_err(PtyRegistryError::Session)
    }

    pub fn switch(
        &mut self,
        client: PtyClientId,
        id: PtySessionId,
    ) -> Result<(), PtyRegistryError> {
        if !self.sessions.contains_key(&id) {
            return Err(PtyRegistryError::UnknownSession(id));
        }
        if !self.selected.contains_key(&client) && self.selected.len() >= self.max_clients {
            return Err(PtyRegistryError::ClientLimit(self.max_clients));
        }
        self.selected.insert(client, id);
        Ok(())
    }

    pub fn selected(&self, client: PtyClientId) -> Option<PtySessionId> {
        self.selected.get(&client).copied()
    }

    pub fn remove_client(&mut self, client: PtyClientId) {
        self.selected.remove(&client);
    }

    pub fn request_close(
        &mut self,
        id: PtySessionId,
    ) -> Result<PtyCloseDisposition, PtyRegistryError> {
        let lifecycle = self
            .sessions
            .get(&id)
            .ok_or(PtyRegistryError::UnknownSession(id))?
            .lifecycle;
        if !lifecycle.is_terminal() {
            return Ok(PtyCloseDisposition::TerminationRequired);
        }
        self.archive_and_remove(id)?;
        Ok(PtyCloseDisposition::Closed)
    }

    pub fn complete_close(&mut self, id: PtySessionId) -> Result<(), PtyRegistryError> {
        let lifecycle = self
            .sessions
            .get(&id)
            .ok_or(PtyRegistryError::UnknownSession(id))?
            .lifecycle;
        if !lifecycle.is_terminal() {
            return Err(PtyRegistryError::SessionStillRunning(id));
        }
        self.archive_and_remove(id)
    }

    fn archive_and_remove(&mut self, id: PtySessionId) -> Result<(), PtyRegistryError> {
        let session = self
            .sessions
            .remove(&id)
            .ok_or(PtyRegistryError::UnknownSession(id))?;
        self.selected.retain(|_, selected| *selected != id);
        while self.history.len() >= self.max_history {
            self.history.pop_front();
            self.dropped_history = self.dropped_history.saturating_add(1);
        }
        self.history.push_back(PtySessionHistoryEntry {
            id: session.id,
            name: session.name,
            kind: session.kind,
            lifecycle: session.lifecycle,
            exit_status: session.exit_status,
            restartable: session.restartable,
        });
        Ok(())
    }
}

fn validate_registry_name(name: &str) -> Result<(), PtyRegistryError> {
    if name.trim().is_empty()
        || name.len() > MAX_PTY_NAME_BYTES
        || name.chars().any(char::is_control)
    {
        return Err(PtyRegistryError::InvalidName);
    }
    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PtyRegistryError {
    #[error("invalid PTY registry {0} limit: {1}")]
    InvalidLimit(&'static str, usize),
    #[error("PTY session limit reached: {0}")]
    SessionLimit(usize),
    #[error("PTY registry client limit reached: {0}")]
    ClientLimit(usize),
    #[error("PTY session ID space is exhausted")]
    IdExhausted,
    #[error("unknown PTY reservation: {0:?}")]
    UnknownReservation(PtySessionId),
    #[error("unknown PTY session: {0:?}")]
    UnknownSession(PtySessionId),
    #[error("duplicate PTY session name: {0}")]
    DuplicateName(String),
    #[error("invalid PTY session name")]
    InvalidName,
    #[error("PTY session is still running: {0:?}")]
    SessionStillRunning(PtySessionId),
    #[error(transparent)]
    Session(crate::PtySessionError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        PtyCommandIdentity, PtyDimensions, PtySessionAction, PtySessionSpec, PtyWorkspaceContext,
    };

    fn session(id: PtySessionId, name: &str, group: i32) -> PtySession {
        let mut session = PtySession::new(
            PtySessionSpec {
                id,
                name: name.into(),
                kind: PtySessionKind::BuildShell,
                cwd: "/work/build".into(),
                command: PtyCommandIdentity {
                    executable: "/bin/sh".into(),
                    arguments: Vec::new(),
                },
                dimensions: PtyDimensions {
                    columns: 80,
                    rows: 24,
                },
                restartable: true,
                workspace: PtyWorkspaceContext {
                    source_dir: "/work".into(),
                    build_dir: "/work/build".into(),
                    authorized_context_roots: Vec::new(),
                    owner_identity: "workspace".into(),
                },
            },
            group,
        )
        .unwrap();
        session.apply(PtySessionAction::MarkRunning).unwrap();
        session
    }

    #[test]
    fn pty_multi_allocates_nonreused_ids_and_independent_client_selection() {
        let mut registry = PtySessionRegistry::new(3, 3, 2).unwrap();
        let first = registry.reserve_id().unwrap();
        let second = registry.reserve_id().unwrap();
        registry.insert(session(first, "build", 10)).unwrap();
        registry.insert(session(second, "menuconfig", 11)).unwrap();
        let client_a = PtyClientId([1; 16]);
        let client_b = PtyClientId([2; 16]);
        registry.switch(client_a, first).unwrap();
        registry.switch(client_b, second).unwrap();
        assert_eq!(registry.selected(client_a), Some(first));
        assert_eq!(registry.selected(client_b), Some(second));
        registry.rename(second, "kernel config".into()).unwrap();
        assert_eq!(registry.get(second).unwrap().name, "kernel config");
        assert_eq!(
            registry.rename(second, "build".into()),
            Err(PtyRegistryError::DuplicateName("build".into()))
        );
        let third = registry.reserve_id().unwrap();
        assert!(third.0 > second.0);
        registry.cancel_reservation(third).unwrap();
        let fourth = registry.reserve_id().unwrap();
        assert!(fourth.0 > third.0);
    }

    #[test]
    fn pty_multi_requires_runner_termination_before_close_and_bounds_history() {
        let mut registry = PtySessionRegistry::new(2, 1, 2).unwrap();
        let first = registry.reserve_id().unwrap();
        registry.insert(session(first, "one", 10)).unwrap();
        assert_eq!(
            registry.request_close(first).unwrap(),
            PtyCloseDisposition::TerminationRequired
        );
        registry
            .get_mut(first)
            .unwrap()
            .apply(PtySessionAction::Exit(PtyExitStatus::Code(0)))
            .unwrap();
        registry.complete_close(first).unwrap();
        assert_eq!(registry.history()[0].id, first);

        let second = registry.reserve_id().unwrap();
        let mut second_session = session(second, "two", 11);
        second_session.apply(PtySessionAction::MarkLost).unwrap();
        registry.insert(second_session).unwrap();
        assert_eq!(
            registry.request_close(second).unwrap(),
            PtyCloseDisposition::Closed
        );
        assert_eq!(registry.history().len(), 1);
        assert_eq!(registry.history()[0].id, second);
        assert_eq!(registry.dropped_history(), 1);
    }

    #[test]
    fn pty_multi_enforces_session_and_client_limits() {
        let mut registry = PtySessionRegistry::new(1, 1, 1).unwrap();
        let id = registry.reserve_id().unwrap();
        assert_eq!(
            registry.reserve_id(),
            Err(PtyRegistryError::SessionLimit(1))
        );
        registry.insert(session(id, "only", 10)).unwrap();
        registry.switch(PtyClientId([1; 16]), id).unwrap();
        assert_eq!(
            registry.switch(PtyClientId([2; 16]), id),
            Err(PtyRegistryError::ClientLimit(1))
        );
    }
}
