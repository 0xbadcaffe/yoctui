use super::*;

enum HardwareWork {
    ProjectLoad {
        root: PathBuf,
        request: HardwareLoadRequest,
    },
    Project(yoctui_model::HardwareProjectRequest),
    Browse {
        generation: u64,
        directory: PathBuf,
    },
    Load(HardwareLoadRequest),
}

#[derive(Default)]
pub(crate) struct HardwareIo {
    worker: Option<JoinHandle<Action>>,
    queued: Option<HardwareWork>,
}

impl HardwareIo {
    pub(crate) fn submit(&mut self, effect: Effect) {
        self.queued = match effect {
            Effect::Hardware(HardwareEffect::LoadProject { root, request }) => {
                Some(HardwareWork::ProjectLoad { root, request })
            }
            Effect::Hardware(HardwareEffect::Project(request)) => {
                Some(HardwareWork::Project(request))
            }
            Effect::Hardware(HardwareEffect::Browse {
                generation,
                directory,
            }) => Some(HardwareWork::Browse {
                generation,
                directory,
            }),
            Effect::Hardware(HardwareEffect::Load(request)) => Some(HardwareWork::Load(request)),
            _ => self.queued.take(),
        };
        self.start();
    }

    fn start(&mut self) {
        if self.worker.is_some() {
            return;
        }
        self.worker = self.queued.take().map(|work| {
            tokio::spawn(async move {
                match work {
                    HardwareWork::ProjectLoad { root, request } => {
                        let generation = request.generation;
                        let path = request.document.path.clone();
                        let validated = tokio::task::spawn_blocking(move || {
                            projects::validate_preview(&root, &path)
                        })
                        .await;
                        let result = match validated {
                            Ok(Ok(())) => load_document(request).await,
                            Ok(Err(error)) => Err(error),
                            Err(error) => Err(error.into()),
                        };
                        match result {
                            Ok((page_count, preview, searchable_text)) => {
                                Action::Hardware(HardwareAction::PreviewLoaded {
                                    generation,
                                    page_count,
                                    preview,
                                    searchable_text,
                                })
                            }
                            Err(error) => Action::Hardware(HardwareAction::PreviewFailed {
                                generation,
                                message: format!("{error:#}"),
                            }),
                        }
                    }
                    HardwareWork::Project(request) => {
                        let generation = request.generation;
                        tokio::task::spawn_blocking(move || projects::run(request))
                            .await
                            .unwrap_or_else(|error| {
                                Action::Hardware(HardwareAction::Project(
                                    yoctui_model::HardwareProjectAction::Finished {
                                        generation,
                                        result: Err(format!("Project worker failed: {error}")),
                                    },
                                ))
                            })
                    }
                    HardwareWork::Browse {
                        generation,
                        directory,
                    } => match tokio::task::spawn_blocking(move || browse_directory(&directory))
                        .await
                    {
                        Ok(Ok((directory, entries))) => {
                            Action::Hardware(HardwareAction::BrowserLoaded {
                                generation,
                                directory,
                                entries,
                            })
                        }
                        Ok(Err(error)) => Action::Hardware(HardwareAction::BrowserFailed {
                            generation,
                            message: error.to_string(),
                        }),
                        Err(error) => Action::Hardware(HardwareAction::BrowserFailed {
                            generation,
                            message: format!("Hardware browser worker failed: {error}"),
                        }),
                    },
                    HardwareWork::Load(request) => {
                        let generation = request.generation;
                        match load_document(request).await {
                            Ok((page_count, preview, searchable_text)) => {
                                Action::Hardware(HardwareAction::PreviewLoaded {
                                    generation,
                                    page_count,
                                    preview,
                                    searchable_text,
                                })
                            }
                            Err(error) => Action::Hardware(HardwareAction::PreviewFailed {
                                generation,
                                message: error.to_string(),
                            }),
                        }
                    }
                }
            })
        });
    }

    pub(crate) async fn poll(&mut self, app: &mut App) -> bool {
        if !self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
        {
            return false;
        }
        let action = self
            .worker
            .take()
            .expect("finished Hardware worker")
            .await
            .unwrap_or_else(|error| {
                let generation = app
                    .hardware
                    .viewer
                    .as_ref()
                    .map_or(0, |value| value.generation);
                Action::Hardware(HardwareAction::PreviewFailed {
                    generation,
                    message: format!("Hardware worker failed: {error}"),
                })
            });
        let _ = yoctui_model::update(app, action);
        self.start();
        true
    }
}
