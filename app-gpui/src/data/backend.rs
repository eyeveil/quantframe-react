use futures::{StreamExt, channel::mpsc::UnboundedReceiver};
use gpui::{App, AppContext, Context, Entity, EventEmitter};

use super::{BackendEvent, DataSource};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendStatus {
    Starting(Option<String>),
    Ready,
    Failed(String),
}

pub struct Backend {
    pub source: DataSource,
    pub status: BackendStatus,
}

impl EventEmitter<BackendEvent> for Backend {}

impl Backend {
    pub fn new(
        source: DataSource,
        events: UnboundedReceiver<BackendEvent>,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx: &mut Context<Self>| {
            cx.spawn(async move |this, cx| {
                let mut events = events;
                while let Some(event) = events.next().await {
                    let delivered = this.update(cx, |backend, cx| {
                        backend.apply(&event);
                        cx.emit(event);
                        cx.notify();
                    });
                    if delivered.is_err() {
                        break;
                    }
                }
            })
            .detach();
            Self {
                source,
                status: BackendStatus::Starting(None),
            }
        })
    }

    fn apply(&mut self, event: &BackendEvent) {
        match event {
            BackendEvent::StartingUp(step) if self.status != BackendStatus::Ready => {
                self.status = BackendStatus::Starting(Some(step.clone()));
            }
            BackendEvent::Ready => self.status = BackendStatus::Ready,
            BackendEvent::StartupFailed(error) => {
                self.status = BackendStatus::Failed(error.clone())
            }
            _ => {}
        }
    }
}
