use notify_rust::Notification;

use super::Worker;

pub const APP_NAME: &str = "Quantframe";

struct Request {
    title: String,
    body: String,
}

static WORKER: Worker<Request, ()> = Worker::new("notification", deliver);

pub fn show(title: &str, body: &str) {
    WORKER.submit(Request {
        title: title.to_owned(),
        body: body.to_owned(),
    });
}

pub fn show_now(title: &str, body: &str) -> Result<(), String> {
    deliver(
        &mut (),
        Request {
            title: title.to_owned(),
            body: body.to_owned(),
        },
    )
}

fn deliver(_: &mut (), request: Request) -> Result<(), String> {
    Notification::new()
        .appname(APP_NAME)
        .summary(&request.title)
        .body(&request.body)
        .show()
        .map(|_| ())
        .map_err(|error| format!("could not show \"{}\": {error}", request.title))
}
