use serde::{Deserialize, Serialize};

pub const MIN_PANE_WIDTH: u16 = 12;
pub const MIN_PANE_HEIGHT: u16 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneNode {
    Leaf {
        id: PaneId,
    },
    Split {
        axis: SplitAxis,
        ratio_per_mille: u16,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneLayout {
    pub root: PaneNode,
    pub focused: PaneId,
    next_id: u64,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum PaneLayoutError {
    #[error("pane ID must be non-zero")]
    InvalidPaneId,
    #[error("pane {0:?} is not present")]
    MissingPane(PaneId),
    #[error("cannot close the last pane")]
    LastPane,
    #[error("pane split ratio must stay between 10% and 90%")]
    InvalidRatio,
    #[error("pane layout ID space is exhausted")]
    IdExhausted,
    #[error("pane layout is invalid: {0}")]
    Invalid(String),
}

impl PaneLayout {
    pub fn new(id: PaneId) -> Result<Self, PaneLayoutError> {
        validate_id(id)?;
        Ok(Self {
            root: PaneNode::Leaf { id },
            focused: id,
            next_id: id.0.checked_add(1).ok_or(PaneLayoutError::IdExhausted)?,
        })
    }

    pub fn contains(&self, id: PaneId) -> bool {
        contains(&self.root, id)
    }

    pub fn pane_ids(&self) -> Vec<PaneId> {
        let mut ids = Vec::new();
        collect_ids(&self.root, &mut ids);
        ids
    }

    pub fn split(&mut self, id: PaneId, axis: SplitAxis) -> Result<PaneId, PaneLayoutError> {
        if !self.contains(id) {
            return Err(PaneLayoutError::MissingPane(id));
        }
        let new_id = PaneId(self.next_id);
        validate_id(new_id)?;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(PaneLayoutError::IdExhausted)?;
        split_leaf(&mut self.root, id, axis, new_id)?;
        self.focused = new_id;
        Ok(new_id)
    }

    pub fn close(&mut self, id: PaneId) -> Result<PaneId, PaneLayoutError> {
        if !self.contains(id) {
            return Err(PaneLayoutError::MissingPane(id));
        }
        if matches!(self.root, PaneNode::Leaf { .. }) {
            return Err(PaneLayoutError::LastPane);
        }
        let replacement = remove_leaf(&mut self.root, id)?;
        if self.focused == id {
            self.focused = replacement;
        }
        Ok(self.focused)
    }

    pub fn focus(&mut self, id: PaneId) -> Result<(), PaneLayoutError> {
        if !self.contains(id) {
            return Err(PaneLayoutError::MissingPane(id));
        }
        self.focused = id;
        Ok(())
    }

    pub fn resize(&mut self, id: PaneId, delta_per_mille: i16) -> Result<(), PaneLayoutError> {
        if !self.contains(id) {
            return Err(PaneLayoutError::MissingPane(id));
        }
        resize_for_leaf(&mut self.root, id, delta_per_mille)
            .then_some(())
            .ok_or(PaneLayoutError::Invalid(
                "pane has no split to resize".into(),
            ))
    }

    pub fn visible_panes(&self, width: u16, height: u16) -> Vec<PaneId> {
        if width < MIN_PANE_WIDTH * 2 || height < MIN_PANE_HEIGHT * 2 {
            return vec![self.focused];
        }
        self.pane_ids()
    }

    pub fn validate(&self) -> Result<(), PaneLayoutError> {
        validate_node(&self.root)?;
        if !self.contains(self.focused) {
            return Err(PaneLayoutError::Invalid("focused pane is absent".into()));
        }
        Ok(())
    }
}

fn validate_id(id: PaneId) -> Result<(), PaneLayoutError> {
    (id.0 != 0)
        .then_some(())
        .ok_or(PaneLayoutError::InvalidPaneId)
}

fn contains(node: &PaneNode, id: PaneId) -> bool {
    match node {
        PaneNode::Leaf { id: current } => *current == id,
        PaneNode::Split { first, second, .. } => contains(first, id) || contains(second, id),
    }
}

fn collect_ids(node: &PaneNode, ids: &mut Vec<PaneId>) {
    match node {
        PaneNode::Leaf { id } => ids.push(*id),
        PaneNode::Split { first, second, .. } => {
            collect_ids(first, ids);
            collect_ids(second, ids);
        }
    }
}

fn split_leaf(
    node: &mut PaneNode,
    id: PaneId,
    axis: SplitAxis,
    new_id: PaneId,
) -> Result<(), PaneLayoutError> {
    match node {
        PaneNode::Leaf { id: current } if *current == id => {
            *node = PaneNode::Split {
                axis,
                ratio_per_mille: 500,
                first: Box::new(PaneNode::Leaf { id }),
                second: Box::new(PaneNode::Leaf { id: new_id }),
            };
            Ok(())
        }
        PaneNode::Leaf { .. } => Err(PaneLayoutError::MissingPane(id)),
        PaneNode::Split { first, second, .. } => {
            if contains(first, id) {
                split_leaf(first, id, axis, new_id)
            } else {
                split_leaf(second, id, axis, new_id)
            }
        }
    }
}

fn remove_leaf(node: &mut PaneNode, id: PaneId) -> Result<PaneId, PaneLayoutError> {
    let PaneNode::Split { first, second, .. } = node else {
        return Err(PaneLayoutError::LastPane);
    };
    if matches!(first.as_ref(), PaneNode::Leaf { id: current } if *current == id) {
        let replacement = first_leaf(second);
        *node = (**second).clone();
        return Ok(replacement);
    }
    if matches!(second.as_ref(), PaneNode::Leaf { id: current } if *current == id) {
        let replacement = first_leaf(first);
        *node = (**first).clone();
        return Ok(replacement);
    }
    if contains(first, id) {
        remove_leaf(first, id)
    } else {
        remove_leaf(second, id)
    }
}

fn first_leaf(node: &PaneNode) -> PaneId {
    match node {
        PaneNode::Leaf { id } => *id,
        PaneNode::Split { first, .. } => first_leaf(first),
    }
}

fn resize_for_leaf(node: &mut PaneNode, id: PaneId, delta: i16) -> bool {
    let PaneNode::Split {
        ratio_per_mille,
        first,
        second,
        ..
    } = node
    else {
        return false;
    };
    let in_first = contains(first, id);
    let in_second = contains(second, id);
    if in_first || in_second {
        if matches!(first.as_ref(), PaneNode::Leaf { .. }) && in_first
            || matches!(second.as_ref(), PaneNode::Leaf { .. }) && in_second
        {
            let next = i32::from(*ratio_per_mille) + i32::from(delta);
            if !(100..=900).contains(&next) {
                return false;
            }
            *ratio_per_mille = next as u16;
            return true;
        }
        resize_for_leaf(if in_first { first } else { second }, id, delta)
    } else {
        false
    }
}

fn validate_node(node: &PaneNode) -> Result<(), PaneLayoutError> {
    match node {
        PaneNode::Leaf { id } => validate_id(*id),
        PaneNode::Split {
            ratio_per_mille,
            first,
            second,
            ..
        } => {
            if !(100..=900).contains(ratio_per_mille) {
                return Err(PaneLayoutError::InvalidRatio);
            }
            validate_node(first)?;
            validate_node(second)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_layout_splits_focuses_resizes_and_closes() {
        let mut layout = PaneLayout::new(PaneId(1)).unwrap();
        let second = layout.split(PaneId(1), SplitAxis::Vertical).unwrap();
        assert_eq!(layout.focused, second);
        assert!(layout.resize(second, 100).is_ok());
        layout.focus(PaneId(1)).unwrap();
        layout.close(PaneId(1)).unwrap();
        assert_eq!(layout.pane_ids(), vec![second]);
        assert!(layout.validate().is_ok());
    }

    #[test]
    fn pane_layout_collapses_to_focused_leaf_on_narrow_terminal() {
        let mut layout = PaneLayout::new(PaneId(1)).unwrap();
        layout.split(PaneId(1), SplitAxis::Horizontal).unwrap();
        assert_eq!(layout.visible_panes(20, 40), vec![PaneId(2)]);
        assert_eq!(layout.visible_panes(80, 40).len(), 2);
    }

    #[test]
    fn pane_layout_rejects_invalid_focus() {
        let layout = PaneLayout::new(PaneId(9)).unwrap();
        let mut invalid = layout;
        invalid.focused = PaneId(99);
        assert!(invalid.validate().is_err());
    }
}
