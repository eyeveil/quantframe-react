pub mod desktop;
pub mod sound;

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        OnceLock,
        mpsc::{Sender, channel},
    },
    thread,
};

struct Worker<T: Send + 'static, S: Default + 'static> {
    sender: OnceLock<Option<Sender<T>>>,
    name: &'static str,
    handle: fn(&mut S, T) -> Result<(), String>,
}

impl<T: Send + 'static, S: Default + 'static> Worker<T, S> {
    const fn new(name: &'static str, handle: fn(&mut S, T) -> Result<(), String>) -> Self {
        Self {
            sender: OnceLock::new(),
            name,
            handle,
        }
    }

    fn submit(&self, job: T) {
        let sender = self.sender.get_or_init(|| self.start());
        if let Some(sender) = sender {
            let _ = sender.send(job);
        }
    }

    fn start(&self) -> Option<Sender<T>> {
        let (sender, receiver) = channel::<T>();
        let (name, handle) = (self.name, self.handle);
        let spawned = thread::Builder::new()
            .name(format!("quantframe-{name}"))
            .spawn(move || {
                let mut state = S::default();
                for job in receiver {
                    match catch_unwind(AssertUnwindSafe(|| handle(&mut state, job))) {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => eprintln!("{name}: {error}"),
                        Err(_) => {
                            eprintln!("{name}: the request panicked");
                            state = S::default();
                        }
                    }
                }
            });
        match spawned {
            Ok(_) => Some(sender),
            Err(error) => {
                eprintln!("{name}: could not start the worker thread: {error}");
                None
            }
        }
    }
}
