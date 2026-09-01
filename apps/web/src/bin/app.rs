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
    let starter = RwSignal::new("jakes".to_owned());
    let theme_source = RwSignal::new(BundledTheme::Jakes.source().to_owned());
    let reset_source = RwSignal::new(BundledTheme::Jakes.source().to_owned());
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
            "pirate" => BundledTheme::Pirate.source(),
            _ => BundledTheme::Jakes.source(),
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

                        <select aria-label="Theme" id="theme-select" prop:value=move || starter.get() on:change=select_starter>
                            <option value="jakes">"Jake's Resume"</option>
                            <option value="modern">"Modern"</option>
                            <option value="pirate">"Pirate"</option>
                            <option value="custom" disabled=move || starter.get() != "custom">"Custom file"</option>
                        </select>

                        <h3 class="customize-heading">"Customize"</h3>
                        <div id="theme-controls" class="theme-controls">
                            {move || customizer_view(theme_source)}
                        </div>

                        <details class="theme-files">
                            <summary>"Theme files"</summary>
                            <p>"Open a custom theme or save this one as a .typ file."</p>
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
                        </details>
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

fn customizer_view(theme_source: RwSignal<String>) -> AnyView {
    let Ok(theme) = ThemeFile::parse(theme_source.get()) else {
        return ().into_any();
    };
    let controls = &theme.manifest().controls;
    let font = controls.iter().find_map(|control| match control {
        ThemeControl::Font {
            key,
            value,
            options,
            ..
        } if key == "font_family" => Some((key.clone(), value.clone(), options.clone())),
        _ => None,
    });
    let body_size = number_bounds(controls, "body_size_pt");
    let name_size = number_bounds(controls, "title_size_pt");
    let has_spacing = ["body_leading_em", "section_gap_pt", "entry_gap_pt"]
        .iter()
        .all(|key| number_bounds(controls, key).is_some());
    let has_margins = ["page_margin_x_in", "page_margin_y_in"]
        .iter()
        .all(|key| number_bounds(controls, key).is_some());
    let main_color = color_value(controls, "text_color");
    let accent_color = color_value(controls, "accent_color")
        .map(|value| ("accent_color".to_owned(), value))
        .or_else(|| {
            color_value(controls, "rule_color").map(|value| ("rule_color".to_owned(), value))
        });

    view! {
        {font.map(|(key, value, options)| view! {
            <label class="theme-control">
                <span>"Font"</span>
                <select
                    aria-label="Font"
                    prop:value=value
                    on:change=move |event| update_control(
                        theme_source,
                        &key,
                        ThemeValue::Text(leptos::prelude::event_target_value(&event)),
                    )
                >
                    {options.into_iter().map(|option| view! {
                        <option value=option.clone()>{option.clone()}</option>
                    }).collect_view()}
                </select>
            </label>
        })}
        {body_size.map(|(value, min, max, step)| size_stepper(
            theme_source,
            "body_size_pt",
            "Text size",
            value,
            min,
            max,
            step,
        ))}
        {name_size.map(|(value, min, max, step)| size_stepper(
            theme_source,
            "title_size_pt",
            "Name size",
            value,
            min,
            max,
            step,
        ))}
        {has_spacing.then(|| scale_control(theme_source, "Spacing", "spacing"))}
        {has_margins.then(|| scale_control(theme_source, "Page margins", "margins"))}
        {main_color.map(|value| color_control(theme_source, "text_color", "Main text", value))}
        {accent_color.map(|(key, value)| color_control(theme_source, &key, "Accent", value))}
    }
    .into_any()
}

fn size_stepper(
    theme_source: RwSignal<String>,
    key: &'static str,
    label: &'static str,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
) -> AnyView {
    view! {
        <div class="customizer-control">
            <span class="control-label">{label}</span>
            <div class="stepper" role="group" aria-label=label>
                <button
                    type="button"
                    aria-label=format!("Decrease {label}")
                    disabled=move || control_number(theme_source, key).is_some_and(|value| value <= min)
                    on:click=move |_| change_number(theme_source, key, -step, min, max)
                >"−"</button>
                <output>{move || format_number(control_number(theme_source, key).unwrap_or(value))}</output>
                <button
                    type="button"
                    aria-label=format!("Increase {label}")
                    disabled=move || control_number(theme_source, key).is_some_and(|value| value >= max)
                    on:click=move |_| change_number(theme_source, key, step, min, max)
                >"+"</button>
            </div>
        </div>
    }
    .into_any()
}

fn scale_control(
    theme_source: RwSignal<String>,
    label: &'static str,
    kind: &'static str,
) -> AnyView {
    let choices = preset_choices(kind);
    view! {
        <label class="preset-control">
            <span class="preset-heading">{label}</span>
            <input
                type="range"
                aria-label=label
                min="0"
                max="100"
                step="1"
                prop:value=move || scale_position(theme_source, kind)
                on:input=move |event| {
                    let position = leptos::prelude::event_target_value(&event)
                        .parse::<f64>()
                        .unwrap_or(50.0);
                    apply_scale(theme_source, kind, position);
                }
            />
            <div class="range-labels" aria-hidden="true">
                {choices.into_iter().map(|choice| view! { <span>{choice}</span> }).collect_view()}
            </div>
        </label>
    }
    .into_any()
}

fn color_control(
    theme_source: RwSignal<String>,
    key: &str,
    label: &'static str,
    value: String,
) -> AnyView {
    let input_key = key.to_owned();
    let update_key = key.to_owned();
    let output_key = key.to_owned();
    view! {
        <label class="theme-control color-control">
            <span>{label}</span>
            <span class="color-value">
                <output>{move || control_text(theme_source, &output_key).unwrap_or_else(|| value.clone())}</output>
                <input
                    aria-label=label
                    type="color"
                    prop:value=move || control_text(theme_source, &input_key).unwrap_or_default()
                    on:input=move |event| update_control(
                        theme_source,
                        &update_key,
                        ThemeValue::Text(leptos::prelude::event_target_value(&event)),
                    )
                />
            </span>
        </label>
    }
    .into_any()
}

fn update_control(theme_source: RwSignal<String>, key: &str, value: ThemeValue) {
    let Ok(mut theme) = ThemeFile::parse(theme_source.get_untracked()) else {
        return;
    };
    if theme.set_control_value(key, value).is_ok() {
        theme_source.set(theme.source().to_owned());
    }
}

fn update_numbers(theme_source: RwSignal<String>, values: &[(&str, f64)]) {
    let Ok(mut theme) = ThemeFile::parse(theme_source.get_untracked()) else {
        return;
    };
    for (key, value) in values {
        if theme
            .set_control_value(key, ThemeValue::Number(*value))
            .is_err()
        {
            return;
        }
    }
    theme_source.set(theme.source().to_owned());
}

fn change_number(theme_source: RwSignal<String>, key: &str, delta: f64, min: f64, max: f64) {
    let Some(current) = control_number(theme_source, key) else {
        return;
    };
    let next = ((current + delta).clamp(min, max) * 100.0).round() / 100.0;
    update_control(theme_source, key, ThemeValue::Number(next));
}

fn preset_choices(kind: &str) -> [&'static str; 3] {
    if kind == "spacing" {
        ["Compact", "Balanced", "Open"]
    } else {
        ["Narrow", "Standard", "Wide"]
    }
}

fn apply_scale(theme_source: RwSignal<String>, kind: &str, position: f64) {
    let source = theme_source.get_untracked();
    let choices = preset_choices(kind);
    let (lower, upper, progress) = if position <= 50.0 {
        (choices[0], choices[1], position / 50.0)
    } else {
        (choices[1], choices[2], (position - 50.0) / 50.0)
    };
    let lower = preset_values(&source, kind, &lower.to_ascii_lowercase());
    let upper = preset_values(&source, kind, &upper.to_ascii_lowercase());
    let values = lower
        .iter()
        .zip(upper.iter())
        .map(|((key, start), (_, end))| {
            let value = start + ((end - start) * progress);
            (*key, (value * 10_000.0).round() / 10_000.0)
        })
        .collect::<Vec<_>>();
    update_numbers(theme_source, &values);
}

fn scale_position(theme_source: RwSignal<String>, kind: &str) -> f64 {
    let source = theme_source.get();
    let choices = preset_choices(kind);
    let stops = choices.map(|choice| {
        preset_values(&source, kind, &choice.to_ascii_lowercase())
            .first()
            .copied()
    });
    let Some((key, compact)) = stops[0] else {
        return 50.0;
    };
    let Some((_, balanced)) = stops[1] else {
        return 50.0;
    };
    let Some((_, open)) = stops[2] else {
        return 50.0;
    };
    let Some(actual) = control_number(theme_source, key) else {
        return 50.0;
    };
    let position = if actual <= balanced {
        50.0 * (actual - compact) / (balanced - compact)
    } else {
        50.0 + (50.0 * (actual - balanced) / (open - balanced))
    };
    position.clamp(0.0, 100.0).round()
}

fn preset_values(source: &str, kind: &str, preset: &str) -> Vec<(&'static str, f64)> {
    let Ok(theme) = ThemeFile::parse(source) else {
        return Vec::new();
    };
    let theme_name = theme.manifest().name.as_str();
    let preset_index = match preset {
        "compact" | "narrow" => 0,
        "balanced" | "standard" => 1,
        "open" | "wide" => 2,
        _ => return Vec::new(),
    };
    let (keys, values): (&[&str], Vec<f64>) = match (kind, theme_name) {
        ("spacing", "Modern") => (
            &["body_leading_em", "section_gap_pt", "entry_gap_pt"],
            [[0.34, 8.0, 6.0], [0.5, 12.0, 9.0], [0.68, 17.0, 13.0]][preset_index].to_vec(),
        ),
        ("spacing", "Pirate") => (
            &["body_leading_em", "section_gap_pt", "entry_gap_pt"],
            [[0.48, 8.0, 9.0], [0.66, 10.5, 13.0], [0.8, 14.0, 17.0]][preset_index].to_vec(),
        ),
        ("spacing", _) => (
            &["body_leading_em", "section_gap_pt", "entry_gap_pt"],
            [[0.28, 7.0, 5.0], [0.4, 10.0, 8.0], [0.55, 14.0, 11.0]][preset_index].to_vec(),
        ),
        ("margins", "Modern") => (
            &["page_margin_x_in", "page_margin_y_in"],
            [[0.5, 0.5], [0.72, 0.68], [0.95, 0.9]][preset_index].to_vec(),
        ),
        ("margins", "Pirate") => (
            &["page_margin_x_in", "page_margin_y_in"],
            [[0.25, 0.42], [0.3125, 0.58], [0.55, 0.75]][preset_index].to_vec(),
        ),
        ("margins", _) => (
            &["page_margin_x_in", "page_margin_y_in"],
            [[0.38, 0.38], [0.5, 0.5], [0.75, 0.75]][preset_index].to_vec(),
        ),
        _ => return Vec::new(),
    };
    keys.iter()
        .zip(values)
        .map(|(key, value)| (*key, value))
        .collect()
}

fn number_bounds(controls: &[ThemeControl], key: &str) -> Option<(f64, f64, f64, f64)> {
    controls.iter().find_map(|control| match control {
        ThemeControl::Number {
            key: found,
            value,
            min,
            max,
            step,
            ..
        } if found == key => Some((*value, *min, *max, *step)),
        _ => None,
    })
}

fn color_value(controls: &[ThemeControl], key: &str) -> Option<String> {
    controls.iter().find_map(|control| match control {
        ThemeControl::Color {
            key: found, value, ..
        } if found == key => Some(value.clone()),
        _ => None,
    })
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

fn control_text(theme_source: RwSignal<String>, key: &str) -> Option<String> {
    ThemeFile::parse(theme_source.get())
        .ok()?
        .manifest()
        .controls
        .iter()
        .find_map(|control| match control {
            ThemeControl::Color {
                key: found, value, ..
            }
            | ThemeControl::Font {
                key: found, value, ..
            } if found == key => Some(value.clone()),
            _ => None,
        })
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
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
