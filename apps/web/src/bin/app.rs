//! Minimal browser page that delegates rendering to a dedicated worker.

#![forbid(unsafe_code)]

use js_sys::{Array, Uint8Array};
use resumark_core::{Diagnostic, PaperSize};
use resumark_web::{RenderRequest, RenderResponse};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{Blob, BlobPropertyBag, Document, Element, ErrorEvent, MessageEvent, Url, Worker};

const EXAMPLE: &str = include_str!("../../../../examples/resume.md");
const WORKER_LOADER: &str = "./resumark-worker_loader.js";

fn main() {
    if let Err(error) = start_app() {
        web_sys::console::error_1(&error);
    }
}

fn start_app() -> Result<(), JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("the browser document is unavailable"))?;
    let status = required_element(&document, "status")?;
    let worker = Worker::new(WORKER_LOADER)?;
    let request = RenderRequest {
        markdown: EXAMPLE.to_owned(),
        paper: PaperSize::Letter,
        max_pages: Some(2),
    };
    let request_message = serde_json::to_string(&request).map_err(js_error)?;

    let response_document = document.clone();
    let response_status = status.clone();
    let response_worker = worker.clone();
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let result = handle_response(
            event.data(),
            &response_document,
            &response_status,
            &response_worker,
            &request_message,
        );
        if let Err(error) = result {
            response_status
                .set_text_content(Some(&format!("Could not display the result: {error:?}")));
        }
    });
    worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    let error_status = status.clone();
    let on_error = Closure::<dyn FnMut(ErrorEvent)>::new(move |event: ErrorEvent| {
        error_status.set_text_content(Some(&format!("Render worker failed: {}", event.message())));
    });
    worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    on_error.forget();
    Ok(())
}

fn handle_response(
    value: JsValue,
    document: &Document,
    status: &Element,
    worker: &Worker,
    request_message: &str,
) -> Result<(), JsValue> {
    let json = value
        .as_string()
        .ok_or_else(|| JsValue::from_str("the worker response was not text"))?;
    let response = serde_json::from_str::<RenderResponse>(&json).map_err(js_error)?;

    match response {
        RenderResponse::Ready => {
            status.set_text_content(Some("Worker ready; compiling the example resume…"));
            worker.post_message(&JsValue::from_str(request_message))?;
        }
        RenderResponse::Rendered {
            svg_pages,
            pdf,
            diagnostics,
        } => {
            status.set_text_content(Some(&format!("Rendered {} page(s).", svg_pages.len(),)));
            show_diagnostics(document, &diagnostics)?;
            show_pages(document, &svg_pages)?;
            enable_pdf_download(document, &pdf)?;
        }
        RenderResponse::Rejected { diagnostics } => {
            status.set_text_content(Some(
                "The example resume contains errors and was not rendered.",
            ));
            show_diagnostics(document, &diagnostics)?;
        }
        RenderResponse::Failed { message } => {
            status.set_text_content(Some(&format!("Compilation failed: {message}")));
        }
    }

    Ok(())
}

fn show_pages(document: &Document, svg_pages: &[String]) -> Result<(), JsValue> {
    let preview = required_element(document, "preview")?;
    preview.set_text_content(None);

    for (index, svg) in svg_pages.iter().enumerate() {
        let image = document.create_element("img")?;
        image.set_class_name("page");
        image.set_attribute("alt", &format!("Resume preview page {}", index + 1))?;
        image.set_attribute("src", &string_blob_url(svg, "image/svg+xml")?)?;
        preview.append_child(&image)?;
    }
    Ok(())
}

fn show_diagnostics(document: &Document, diagnostics: &[Diagnostic]) -> Result<(), JsValue> {
    let container = required_element(document, "diagnostics")?;
    container.set_text_content(None);

    for diagnostic in diagnostics {
        let message = document.create_element("p")?;
        message.set_text_content(Some(&format!(
            "{}: {}",
            diagnostic.severity, diagnostic.message
        )));
        container.append_child(&message)?;
    }
    Ok(())
}

fn enable_pdf_download(document: &Document, pdf: &[u8]) -> Result<(), JsValue> {
    let download = required_element(document, "download")?;
    download.set_attribute("href", &byte_blob_url(pdf, "application/pdf")?)?;
    download.set_attribute("download", "jane-doe-resume.pdf")?;
    download.remove_attribute("hidden")?;
    Ok(())
}

fn string_blob_url(value: &str, media_type: &str) -> Result<String, JsValue> {
    let parts = Array::new();
    parts.push(&JsValue::from_str(value));
    let options = blob_options(media_type);
    let blob = Blob::new_with_str_sequence_and_options(&parts, &options)?;
    Url::create_object_url_with_blob(&blob)
}

fn byte_blob_url(value: &[u8], media_type: &str) -> Result<String, JsValue> {
    let parts = Array::new();
    parts.push(&Uint8Array::from(value));
    let options = blob_options(media_type);
    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &options)?;
    Url::create_object_url_with_blob(&blob)
}

fn blob_options(media_type: &str) -> BlobPropertyBag {
    let options = BlobPropertyBag::new();
    options.set_type(media_type);
    options
}

fn required_element(document: &Document, id: &str) -> Result<Element, JsValue> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| JsValue::from_str(&format!("missing #{id} element")))
}

fn js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
