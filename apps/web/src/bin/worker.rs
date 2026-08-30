//! Dedicated browser worker for the existing Resumark compilation pipeline.

#![forbid(unsafe_code)]

use resumark_core::{ParseLimits, analyze_markdown};
use resumark_render_typst::{
    RenderError, RenderOptions, Renderer, ThemeDiagnostic, ThemeSelection,
};
use resumark_theme::ThemeFile;
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
            revision: None,
            message: "the worker received a render request that was not text".to_owned(),
        };
    };
    let request = match serde_json::from_str::<RenderRequest>(&json) {
        Ok(request) => request,
        Err(error) => {
            return RenderResponse::Failed {
                revision: None,
                message: format!("the worker could not read the render request: {error}"),
            };
        }
    };

    render(request, renderer)
}

fn render(request: RenderRequest, renderer: &Renderer) -> RenderResponse {
    let revision = request.revision;
    let analysis = analyze_markdown(&request.markdown, &ParseLimits::default());
    let Some(document) = analysis.document else {
        return RenderResponse::ResumeRejected {
            revision,
            diagnostics: analysis.diagnostics,
        };
    };

    let theme = match ThemeFile::parse(request.theme_source) {
        Ok(theme) => theme,
        Err(error) => {
            return RenderResponse::ThemeRejected {
                revision,
                diagnostics: vec![ThemeDiagnostic {
                    message: error.message,
                    range: error.range,
                    hints: Vec::new(),
                }],
            };
        }
    };

    let options = RenderOptions {
        paper: request.paper,
        max_pages: request.max_pages,
        theme: ThemeSelection::Custom(theme),
    };
    let compiled = match renderer.compile(&document, &options) {
        Ok(compiled) => compiled,
        Err(RenderError::ThemeFile(error)) => {
            return RenderResponse::ThemeRejected {
                revision,
                diagnostics: vec![ThemeDiagnostic {
                    message: error.message,
                    range: error.range,
                    hints: Vec::new(),
                }],
            };
        }
        Err(RenderError::Compile(error)) => {
            return RenderResponse::ThemeRejected {
                revision,
                diagnostics: error.diagnostics,
            };
        }
        Err(error) => {
            return RenderResponse::Failed {
                revision: Some(revision),
                message: error.to_string(),
            };
        }
    };
    let pdf = match compiled.pdf() {
        Ok(pdf) => pdf,
        Err(error) => {
            return RenderResponse::Failed {
                revision: Some(revision),
                message: error.to_string(),
            };
        }
    };

    RenderResponse::Rendered {
        revision,
        svg_pages: compiled.svg_pages(),
        pdf,
        diagnostics: compiled.diagnostics().to_vec(),
    }
}

fn post_response(scope: &DedicatedWorkerGlobalScope, response: &RenderResponse) {
    let message = match serde_json::to_string(response) {
        Ok(message) => message,
        Err(error) => format!(
            r#"{{"status":"failed","revision":null,"message":"could not serialize worker response: {error}"}}"#
        ),
    };

    if let Err(error) = scope.post_message(&JsValue::from_str(&message)) {
        web_sys::console::error_1(&error);
    }
}

fn js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
