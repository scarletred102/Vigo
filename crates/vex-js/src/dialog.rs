//! Browser-dialog request and response broker.
//!
//! The JavaScript runtime and browser chrome share this small state machine.
//! A request has a stable ID, cannot be answered twice, and is cancelled when
//! its document goes away. The renderer scheduler will use the response to
//! resume the suspended JavaScript invocation.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DialogId(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogKind {
    Alert {
        message: String,
    },
    Confirm {
        message: String,
    },
    Prompt {
        message: String,
        default: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogRequest {
    pub id: DialogId,
    pub tab_id: u32,
    pub kind: DialogKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogResponse {
    AlertDismissed,
    Confirmed(bool),
    Prompted(Option<String>),
}

#[derive(Debug, Default)]
pub struct DialogBroker {
    next_id: u64,
    pending: HashMap<DialogId, DialogRequest>,
}

impl DialogBroker {
    pub fn request(&mut self, tab_id: u32, kind: DialogKind) -> DialogRequest {
        self.next_id = self.next_id.saturating_add(1);
        let request = DialogRequest {
            id: DialogId(self.next_id),
            tab_id,
            kind,
        };
        self.pending.insert(request.id, request.clone());
        request
    }

    pub fn resolve(
        &mut self,
        id: DialogId,
        response: DialogResponse,
    ) -> Option<(DialogRequest, DialogResponse)> {
        self.pending.remove(&id).map(|request| (request, response))
    }

    pub fn cancel_tab(&mut self, tab_id: u32) -> Vec<DialogRequest> {
        let ids: Vec<_> = self
            .pending
            .values()
            .filter(|request| request.tab_id == tab_id)
            .map(|request| request.id)
            .collect();
        ids.into_iter()
            .filter_map(|id| self.pending.remove(&id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_resolves_once() {
        let mut broker = DialogBroker::default();
        let request = broker.request(
            7,
            DialogKind::Confirm {
                message: "Continue?".into(),
            },
        );
        assert_eq!(
            broker
                .resolve(request.id, DialogResponse::Confirmed(true))
                .map(|(_, answer)| answer),
            Some(DialogResponse::Confirmed(true))
        );
        assert!(broker
            .resolve(request.id, DialogResponse::Confirmed(false))
            .is_none());
    }

    #[test]
    fn navigation_cancels_only_its_tab() {
        let mut broker = DialogBroker::default();
        let first = broker.request(
            1,
            DialogKind::Alert {
                message: "one".into(),
            },
        );
        let second = broker.request(
            2,
            DialogKind::Alert {
                message: "two".into(),
            },
        );
        assert_eq!(broker.cancel_tab(1), vec![first]);
        assert!(broker
            .resolve(second.id, DialogResponse::AlertDismissed)
            .is_some());
    }
}
