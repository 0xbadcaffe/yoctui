const FAILURE_ACKNOWLEDGEMENT: &[u8] = b"Press any key to continue...";

#[derive(Default)]
pub(super) struct MenuconfigFailurePrompt {
    matched: usize,
    acknowledged: bool,
}

impl MenuconfigFailurePrompt {
    pub(super) fn observe(&mut self, bytes: &[u8]) -> bool {
        if self.acknowledged {
            return false;
        }
        for byte in bytes {
            if *byte == FAILURE_ACKNOWLEDGEMENT[self.matched] {
                self.matched += 1;
            } else {
                self.matched = usize::from(*byte == FAILURE_ACKNOWLEDGEMENT[0]);
            }
            if self.matched == FAILURE_ACKNOWLEDGEMENT.len() {
                self.acknowledged = true;
                return true;
            }
        }
        false
    }
}
