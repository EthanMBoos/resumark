//! Browser interface for choosing a resume, customizing a theme, and exporting PDF.

#![forbid(unsafe_code)]

use std::rc::Rc;

use js_sys::{Array, Uint8Array};
use leptos::{mount::mount_to_body, prelude::*};
use resumark_core::PaperSize;
use resumark_render_typst::BundledTheme;
use resumark_theme::{ThemeControl, ThemeFile, ThemeValue};
use resumark_web::{RenderRequest, RenderResponse};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Blob, BlobPropertyBag, Event, HtmlInputElement, Url};

mod app_worker;

use app_worker::{install_response_handler, schedule_render, start_worker};

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let resume_name = RwSignal::new("No file selected".to_owned());
    let markdown = RwSignal::new(None::<String>);
    let starter = RwSignal::new("minimal".to_owned());
    let theme_source = RwSignal::new(BundledTheme::Minimal.source().to_owned());
    let reset_source = RwSignal::new(BundledTheme::Minimal.source().to_owned());
    let show_source = RwSignal::new(false);
    let revision = RwSignal::new(0_u64);
    let status = RwSignal::new("Open a Markdown resume to begin.".to_owned());
    let diagnostics = RwSignal::new(Vec::<String>::new());
    let page_urls = RwSignal::new(Vec::<String>::new());
    let pdf_url = RwSignal::new(None::<String>);
    let stale_preview = RwSignal::new(false);

    install_response_handler(Rc::new(move |response| {
        let response_revision = match &response {
            RenderResponse::Ready => return,
            RenderResponse::Rendered { revision, .. }
            | RenderResponse::ResumeRejected { revision, .. }
            | RenderResponse::ThemeRejected { revision, .. } => *revision,
            RenderResponse::Failed {
                revision: Some(revision),
                ..
            } => *revision,
            RenderResponse::Failed { revision: None, .. } => revision.get_untracked(),
        };
        if response_revision < revision.get_untracked() {
            return;
        }

        match response {
            RenderResponse::Rendered {
                svg_pages,
                pdf,
                diagnostics: render_diagnostics,
                ..
            } => {
                replace_urls(
                    page_urls,
                    svg_pages
                        .iter()
                        .map(|svg| string_blob_url(svg, "image/svg+xml"))
                        .collect(),
                );
                replace_pdf_url(pdf_url, byte_blob_url(&pdf, "application/pdf").ok());
                diagnostics.set(
                    render_diagnostics
                        .into_iter()
                        .map(|diagnostic| {
                            format!("{}: {}", diagnostic.severity, diagnostic.message)
                        })
                        .collect(),
                );
                stale_preview.set(false);
                status.set(format!("Rendered {} page(s).", svg_pages.len()));
            }
            RenderResponse::ResumeRejected {
                diagnostics: resume_diagnostics,
                ..
            } => {
                replace_urls(page_urls, Vec::new());
                replace_pdf_url(pdf_url, None);
                diagnostics.set(
                    resume_diagnostics
                        .into_iter()
                        .map(|diagnostic| {
                            format!("{}: {}", diagnostic.severity, diagnostic.message)
                        })
                        .collect(),
                );
                stale_preview.set(false);
                status.set("The Markdown file has errors and was not rendered.".to_owned());
            }
            RenderResponse::ThemeRejected {
                diagnostics: theme_diagnostics,
                ..
            } => {
                replace_pdf_url(pdf_url, None);
                diagnostics.set(
                    theme_diagnostics
                        .into_iter()
                        .map(|diagnostic| diagnostic.message)
                        .collect(),
                );
                let has_preview = !page_urls.get_untracked().is_empty();
                stale_preview.set(has_preview);
                status.set(if has_preview {
                    "Theme error. Showing the last valid preview.".to_owned()
                } else {
                    "Theme error. No preview is available yet.".to_owned()
                });
            }
            RenderResponse::Failed { message, .. } => {
                replace_pdf_url(pdf_url, None);
                diagnostics.set(vec![message]);
                stale_preview.set(!page_urls.get_untracked().is_empty());
                status.set("The render worker failed. Restarting…".to_owned());
            }
            RenderResponse::Ready => {}
        }
    }));
    start_worker();

    Effect::new(move || {
        let Some(markdown) = markdown.get() else {
            return;
        };
        let next_revision = revision.get_untracked() + 1;
        revision.set(next_revision);
        status.set("Rendering…".to_owned());
        schedule_render(RenderRequest {
            revision: next_revision,
            markdown,
            paper: PaperSize::Letter,
            max_pages: Some(2),
            theme_source: theme_source.get(),
        });
    });

    let choose_resume = move |event: Event| {
        let input = event
            .target()
            .and_then(|target| target.dyn_into::<HtmlInputElement>().ok());
        let Some(file) = input
            .and_then(|input| input.files())
            .and_then(|files| files.get(0))
        else {
            return;
        };
        let name = file.name();
        spawn_local(async move {
            match JsFuture::from(file.text()).await.and_then(|value| {
                value
                    .as_string()
                    .ok_or_else(|| JsValue::from_str("file was not text"))
            }) {
                Ok(source) => {
                    resume_name.set(name);
                    markdown.set(Some(source));
                }
                Err(error) => status.set(format!("Could not read the Markdown file: {error:?}")),
            }
        });
    };

    let choose_custom_theme = move |event: Event| {
        let input = event
            .target()
            .and_then(|target| target.dyn_into::<HtmlInputElement>().ok());
        let Some(file) = input
            .and_then(|input| input.files())
            .and_then(|files| files.get(0))
        else {
            return;
        };
        spawn_local(async move {
            match JsFuture::from(file.text()).await.and_then(|value| {
                value
                    .as_string()
                    .ok_or_else(|| JsValue::from_str("file was not text"))
            }) {
                Ok(source) => {
                    starter.set("custom".to_owned());
                    reset_source.set(source.clone());
                    theme_source.set(source);
                }
                Err(error) => status.set(format!("Could not read the theme file: {error:?}")),
            }
        });
    };

    let select_starter = move |event: Event| {
        let id = leptos::prelude::event_target_value(&event);
        let source = match id.as_str() {
            "modern" => BundledTheme::Modern.source(),
            "compact" => BundledTheme::Compact.source(),
            "jakes" => BundledTheme::Jakes.source(),
            _ => BundledTheme::Minimal.source(),
        }
        .to_owned();
        starter.set(id);
        reset_source.set(source.clone());
        theme_source.set(source);
    };

    view! {
        <main>
            <header class="site-header">
                <div>
                    <p class="eyebrow">"Markdown to PDF"</p>
                    <h1>"Resumark"</h1>
                </div>
                <p>"Open a Markdown resume, choose a look, and download the PDF. Files stay in this browser tab."</p>
            </header>

            <section class="workspace">
                <aside class="panel controls-panel">
                    <section class="control-section">
                        <div class="section-heading">
                            <div>
                                <span class="step">"1"</span>
                                <h2>"Resume"</h2>
                            </div>
                            <span class="file-name">{move || resume_name.get()}</span>
                        </div>
                        <label class="file-button">
                            "Open resume.md"
                            <input id="resume-file" type="file" accept=".md,text/markdown,text/plain" on:change=choose_resume />
                        </label>
                    </section>

                    <section class="control-section">
                        <div class="section-heading">
                            <div>
                                <span class="step">"2"</span>
                                <h2>"Theme"</h2>
                            </div>
                            <button class="quiet-button" on:click=move |_| theme_source.set(reset_source.get_untracked())>"Reset"</button>
                        </div>

                        <label>
                            <select aria-label="Theme" id="theme-select" prop:value=move || starter.get() on:change=select_starter>
                                <option value="minimal">"Default"</option>
                                <option value="modern">"Modern"</option>
                                <option value="compact">"Compact"</option>
                                <option value="jakes">"Jake's Resume"</option>
                                <option value="custom" disabled=move || starter.get() != "custom">"Custom file"</option>
                            </select>
                        </label>

                        <div class="theme-actions">
                            <label class="file-button secondary">
                                "Open theme.typ"
                                <input id="theme-file" type="file" accept=".typ,text/plain" on:change=choose_custom_theme />
                            </label>
                            <a
                                class="quiet-button"
                                href=move || theme_data_url(&theme_source.get())
                                download=move || theme_filename(&theme_source.get())
                            >"Download theme"</a>
                        </div>

                        <div id="theme-controls" class="theme-controls">
                            {move || control_views(theme_source).collect_view()}
                        </div>

                        <label class="source-toggle">
                            <input type="checkbox" prop:checked=move || show_source.get() on:change=move |event| show_source.set(event_target_checked(&event)) />
                            <span>"Edit Typst source"</span>
                        </label>
                        <Show when=move || show_source.get()>
                            <textarea
                                id="theme-source"
                                class="theme-source"
                                aria-label="Theme Typst source"
                                spellcheck="false"
                                prop:value=move || theme_source.get()
                                on:input=move |event| theme_source.set(leptos::prelude::event_target_value(&event))
                            ></textarea>
                        </Show>
                    </section>
                </aside>

                <section class="result-column">
                    <div class="result-bar" aria-live="polite">
                        <div>
                            <h2>"Preview"</h2>
                            <p id="status">{move || status.get()}</p>
                        </div>
                        <a
                            id="download"
                            class="download"
                            class:disabled=move || pdf_url.get().is_none()
                            aria-disabled=move || pdf_url.get().is_none().to_string()
                            href=move || pdf_url.get().unwrap_or_default()
                            download=move || pdf_filename(&resume_name.get())
                        >"Download PDF"</a>
                    </div>
                    <Show when=move || stale_preview.get()>
                        <p class="stale-note">"This preview is from the last theme that compiled."</p>
                    </Show>
                    <div id="diagnostics" class="diagnostics">
                        <For
                            each=move || diagnostics.get()
                            key=|message| message.clone()
                            children=move |message| view! { <p>{message}</p> }
                        />
                    </div>
                    <div id="preview" class="preview" aria-label="Resume page previews">
                        <For
                            each=move || page_urls.get()
                            key=|url| url.clone()
                            children=move |url| view! { <img class="page" alt="Resume preview page" src=url /> }
                        />
                    </div>
                </section>
            </section>
        </main>
    }
}

fn control_views(theme_source: RwSignal<String>) -> impl Iterator<Item = AnyView> {
    let controls = ThemeFile::parse(theme_source.get())
        .map(|theme| theme.manifest().controls.clone())
        .unwrap_or_default();

    controls.into_iter().map(move |control| match control {
        ThemeControl::Number {
            key,
            label,
            value,
            min,
            max,
            step,
            unit,
            ..
        } => {
            let control_key = key.clone();
            view! {
                <label class="theme-control">
                    <span>{label} <small>{unit}</small></span>
                    <div class="number-control">
                        <input
                            type="range"
                            min=min
                            max=max
                            step=step
                            prop:value=value
                            on:input=move |event| {
                                if let Ok(value) = leptos::prelude::event_target_value(&event).parse() {
                                    update_control(theme_source, &control_key, ThemeValue::Number(value));
                                }
                            }
                        />
                        <output>{move || control_number(theme_source, &key).unwrap_or(value)}</output>
                    </div>
                </label>
            }
            .into_any()
        }
        ThemeControl::Color { key, label, value, .. } => {
            let control_key = key.clone();
            view! {
                <label class="theme-control color-control">
                    <span>{label}</span>
                    <input
                        type="color"
                        prop:value=value
                        on:input=move |event| update_control(
                            theme_source,
                            &control_key,
                            ThemeValue::Text(leptos::prelude::event_target_value(&event)),
                        )
                    />
                </label>
            }
            .into_any()
        }
        ThemeControl::Font { key, label, value, options, .. } => {
            let control_key = key;
            view! {
                <label class="theme-control">
                    <span>{label}</span>
                    <select
                        prop:value=value
                        on:change=move |event| update_control(
                            theme_source,
                            &control_key,
                            ThemeValue::Text(leptos::prelude::event_target_value(&event)),
                        )
                    >
                        {options.into_iter().map(|option| view! { <option value=option.clone()>{option.clone()}</option> }).collect_view()}
                    </select>
                </label>
            }
            .into_any()
        }
    })
}

fn update_control(theme_source: RwSignal<String>, key: &str, value: ThemeValue) {
    let Ok(mut theme) = ThemeFile::parse(theme_source.get_untracked()) else {
        return;
    };
    if theme.set_control_value(key, value).is_ok() {
        theme_source.set(theme.source().to_owned());
    }
}

fn control_number(theme_source: RwSignal<String>, key: &str) -> Option<f64> {
    ThemeFile::parse(theme_source.get())
        .ok()?
        .manifest()
        .controls
        .iter()
        .find_map(|control| match control {
            ThemeControl::Number {
                key: found, value, ..
            } if found == key => Some(*value),
            _ => None,
        })
}

fn event_target_checked(event: &Event) -> bool {
    event
        .target()
        .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
        .is_some_and(|input| input.checked())
}

fn replace_urls(signal: RwSignal<Vec<String>>, next: Vec<Result<String, JsValue>>) {
    let next = next.into_iter().filter_map(Result::ok).collect::<Vec<_>>();
    for url in signal.get_untracked() {
        let _ = Url::revoke_object_url(&url);
    }
    signal.set(next);
}

fn replace_pdf_url(signal: RwSignal<Option<String>>, next: Option<String>) {
    if let Some(url) = signal.get_untracked() {
        let _ = Url::revoke_object_url(&url);
    }
    signal.set(next);
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

fn theme_data_url(source: &str) -> String {
    format!(
        "data:text/plain;charset=utf-8,{}",
        js_sys::encode_uri_component(source)
    )
}

fn theme_filename(source: &str) -> String {
    ThemeFile::parse(source)
        .map(|theme| format!("{}.typ", slug(theme.manifest().name.as_str())))
        .unwrap_or_else(|_| "theme.typ".to_owned())
}

fn pdf_filename(resume_name: &str) -> String {
    let stem = resume_name
        .rsplit('/')
        .next()
        .unwrap_or("resume")
        .strip_suffix(".md")
        .unwrap_or("resume");
    format!("{stem}.pdf")
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
