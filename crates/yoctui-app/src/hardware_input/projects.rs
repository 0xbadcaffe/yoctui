use super::*;
use yoctui_model::{HardwareProjectAction as A, HardwareProjectForm};

pub(super) fn action(app: &App, input: Input) -> Option<Action> {
    let state = &app.hardware.projects;
    if state.loading {
        return None;
    }
    let mapped = if let Some(form) = &state.form {
        match input {
            Input::Esc => A::Cancel,
            Input::Enter => A::Confirm,
            Input::Backspace => A::Backspace,
            _ => match form {
                HardwareProjectForm::Name { .. } => match input {
                    Input::Char(character) => A::EditName(character),
                    _ => return None,
                },
                HardwareProjectForm::Progress { .. } => match input {
                    Input::Up | Input::BackTab => A::SelectStage { delta: -1 },
                    Input::Down | Input::Tab => A::SelectStage { delta: 1 },
                    Input::Left => A::ChangeProgress { delta: -5 },
                    Input::Right => A::ChangeProgress { delta: 5 },
                    Input::Char(' ') => A::ToggleProgress,
                    Input::Char(character) if character.is_ascii_digit() => {
                        A::ProgressDigit(character)
                    }
                    _ => return None,
                },
            },
        }
    } else if state.import_browser.is_some() {
        match input {
            Input::Esc => A::Cancel,
            Input::Up | Input::Char('k') => A::Select { delta: -1 },
            Input::Down | Input::Char('j') => A::Select { delta: 1 },
            Input::PageUp => A::Select { delta: -10 },
            Input::PageDown => A::Select { delta: 10 },
            Input::Enter => A::ImportOpen,
            Input::Backspace => A::ImportParent,
            _ => return None,
        }
    } else {
        match input {
            Input::Char('p') => A::Toggle,
            Input::Char('n') => A::NewName,
            Input::Char('s') if state.project.is_some() => A::BeginProgress,
            Input::Char('a') if state.project.is_some() => A::BeginImport {
                directory: app
                    .hardware
                    .last_directory
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("/")),
            },
            Input::Up | Input::Char('k') => A::Select { delta: -1 },
            Input::Down | Input::Char('j') => A::Select { delta: 1 },
            Input::PageUp => A::Select { delta: -10 },
            Input::PageDown => A::Select { delta: 10 },
            Input::Home => A::Select { delta: isize::MIN },
            Input::End => A::Select { delta: isize::MAX },
            Input::Enter => A::Open,
            Input::Backspace | Input::Esc => A::Parent,
            Input::Char('r') => A::Reload,
            _ => return None,
        }
    };
    Some(Action::Hardware(HardwareAction::Project(mapped)))
}
