//! Scheduling and lifecycle for the browser render worker.

use std::cell::RefCell;
use std::rc::Rc;

use resumark_web::{RenderRequest, RenderResponse};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{ErrorEvent, MessageEvent, Worker};

const WORKER_LOADER: &str = "./resumark-worker_loader.js";
const DEBOUNCE_MS: i32 = 200;
const RENDER_TIMEOUT_MS: i32 = 5_000;

type ResponseHandler = Rc<dyn Fn(RenderResponse)>;

thread_local! {
    static WORKER: RefCell<Option<WorkerRuntime>> = const { RefCell::new(None) };
    static PENDING: RefCell<Option<RenderRequest>> = const { RefCell::new(None) };
    static DEBOUNCE: RefCell<Option<Timer>> = const { RefCell::new(None) };
    static RENDER_TIMEOUT: RefCell<Option<Timer>> = const { RefCell::new(None) };
    static RESPONSE_HANDLER: RefCell<Option<ResponseHandler>> = const { RefCell::new(None) };
}

struct WorkerRuntime {
    worker: Worker,
    ready: bool,
    busy: bool,
    inflight: Option<RenderRequest>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut(ErrorEvent)>,
}

struct Timer {
    id: i32,
    _callback: Closure<dyn FnMut()>,
}

pub(super) fn install_response_handler(handler: ResponseHandler) {
    RESPONSE_HANDLER.with(|stored| *stored.borrow_mut() = Some(handler));
}

pub(super) fn start_worker() {
    let Ok(worker) = Worker::new(WORKER_LOADER) else {
        deliver_failure("could not start the render worker");
        return;
    };

    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(handle_message);
    worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    let on_error = Closure::<dyn FnMut(ErrorEvent)>::new(|event: ErrorEvent| {
        restart_worker(format!("render worker error: {}", event.message()));
    });
    worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));

    WORKER.with(|stored| {
        *stored.borrow_mut() = Some(WorkerRuntime {
            worker,
            ready: false,
            busy: false,
            inflight: None,
            _on_message: on_message,
            _on_error: on_error,
        });
    });
}

pub(super) fn schedule_render(request: RenderRequest) {
    PENDING.with(|pending| *pending.borrow_mut() = Some(request));
    clear_timer(&DEBOUNCE);
    set_timer(&DEBOUNCE, DEBOUNCE_MS, send_pending);
}

fn handle_message(event: MessageEvent) {
    let Some(json) = event.data().as_string() else {
        restart_worker("render worker returned a non-text message".to_owned());
        return;
    };
    let Ok(response) = serde_json::from_str::<RenderResponse>(&json) else {
        restart_worker("render worker returned an unreadable message".to_owned());
        return;
    };

    if matches!(response, RenderResponse::Ready) {
        WORKER.with(|stored| {
            if let Some(runtime) = stored.borrow_mut().as_mut() {
                runtime.ready = true;
            }
        });
        send_pending();
        return;
    }

    clear_timer(&RENDER_TIMEOUT);
    WORKER.with(|stored| {
        if let Some(runtime) = stored.borrow_mut().as_mut() {
            runtime.busy = false;
            runtime.inflight = None;
        }
    });
    deliver(response);
    send_pending();
}

fn send_pending() {
    clear_timer(&DEBOUNCE);
    let sent = WORKER.with(|stored| {
        let mut stored = stored.borrow_mut();
        let runtime = stored.as_mut()?;
        if !runtime.ready || runtime.busy {
            return None;
        }

        let request = PENDING.with(|pending| pending.borrow_mut().take())?;
        let message = serde_json::to_string(&request).ok()?;
        runtime
            .worker
            .post_message(&JsValue::from_str(&message))
            .ok()?;
        runtime.busy = true;
        runtime.inflight = Some(request);
        Some(())
    });

    if sent.is_some() {
        clear_timer(&RENDER_TIMEOUT);
        set_timer(&RENDER_TIMEOUT, RENDER_TIMEOUT_MS, || {
            restart_worker("render timed out after 5 seconds".to_owned());
        });
    }
}

fn restart_worker(message: String) {
    clear_timer(&RENDER_TIMEOUT);
    let inflight = WORKER.with(|stored| {
        stored.borrow_mut().take().and_then(|runtime| {
            runtime.worker.terminate();
            runtime.inflight
        })
    });
    if let Some(request) = inflight {
        PENDING.with(|pending| {
            let mut pending = pending.borrow_mut();
            if pending.is_none() {
                *pending = Some(request);
            }
        });
    }
    deliver_failure(&message);
    start_worker();
}

fn deliver_failure(message: &str) {
    deliver(RenderResponse::Failed {
        revision: None,
        message: message.to_owned(),
    });
}

fn deliver(response: RenderResponse) {
    RESPONSE_HANDLER.with(|stored| {
        if let Some(handler) = stored.borrow().as_ref() {
            handler(response);
        }
    });
}

fn set_timer(
    storage: &'static std::thread::LocalKey<RefCell<Option<Timer>>>,
    delay: i32,
    callback: impl FnMut() + 'static,
) {
    let callback = Closure::<dyn FnMut()>::new(callback);
    if let Some(window) = web_sys::window()
        && let Ok(id) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            delay,
        )
    {
        storage.with(|stored| {
            *stored.borrow_mut() = Some(Timer {
                id,
                _callback: callback,
            });
        });
    }
}

fn clear_timer(storage: &'static std::thread::LocalKey<RefCell<Option<Timer>>>) {
    storage.with(|stored| {
        if let Some(timer) = stored.borrow_mut().take()
            && let Some(window) = web_sys::window()
        {
            window.clear_timeout_with_handle(timer.id);
        }
    });
}
