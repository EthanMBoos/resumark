//! Dedicated browser worker for the existing Resumark compilation pipeline.

#![forbid(unsafe_code)]

use resumark_core::{ParseLimits, analyze_markdown};
use resumark_render_typst::{RenderOptions, Renderer};
use resumark_web::{RenderRequest, RenderResponse};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

fn main() {
    if let Err(error) = start_worker() {
        web_sys::console::error_1(&error);
    }
}

fn start_worker() -> Result<(), JsValue> {
    let scope = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    let reply_scope = scope.clone();
    let renderer = Renderer::new().map_err(js_error)?;

    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let response = handle_message(event.data(), &renderer);
        post_response(&reply_scope, &response);
    });

    scope.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
    post_response(&scope, &RenderResponse::Ready);
    Ok(())
}

fn handle_message(value: JsValue, renderer: &Renderer) -> RenderResponse {
    let Some(json) = value.as_string() else {
        return RenderResponse::Failed {
            message: "the worker received a render request that was not text".to_owned(),
        };
    };
    let request = match serde_json::from_str::<RenderRequest>(&json) {
        Ok(request) => request,
        Err(error) => {
            return RenderResponse::Failed {
                message: format!("the worker could not read the render request: {error}"),
            };
        }
    };

    render(request, renderer)
}

fn render(request: RenderRequest, renderer: &Renderer) -> RenderResponse {
    let analysis = analyze_markdown(&request.markdown, &ParseLimits::default());
    let Some(document) = analysis.document else {
        return RenderResponse::Rejected {
            diagnostics: analysis.diagnostics,
        };
    };

    let options = RenderOptions {
        paper: request.paper,
        max_pages: request.max_pages,
    };
    let compiled = match renderer.compile(&document, &options) {
        Ok(compiled) => compiled,
        Err(error) => {
            return RenderResponse::Failed {
                message: error.to_string(),
            };
        }
    };
    let pdf = match compiled.pdf() {
        Ok(pdf) => pdf,
        Err(error) => {
            return RenderResponse::Failed {
                message: error.to_string(),
            };
        }
    };

    RenderResponse::Rendered {
        svg_pages: compiled.svg_pages(),
        pdf,
        diagnostics: compiled.diagnostics().to_vec(),
    }
}

fn post_response(scope: &DedicatedWorkerGlobalScope, response: &RenderResponse) {
    let message = match serde_json::to_string(response) {
        Ok(message) => message,
        Err(error) => format!(
            r#"{{"status":"failed","message":"could not serialize worker response: {error}"}}"#
        ),
    };

    if let Err(error) = scope.post_message(&JsValue::from_str(&message)) {
        web_sys::console::error_1(&error);
    }
}

fn js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
