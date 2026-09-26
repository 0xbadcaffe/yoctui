//! Bounded client-local Hardware browsing and document conversion.
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Output,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use tokio::{process::Command, task::JoinHandle};
use yoctui_model::{
    Action, App, Effect, HardwareAction, HardwareBrowserEntry, HardwareDocumentKind,
    HardwareEffect, HardwareLoadRequest, HardwarePreview, HardwareRaster, HardwareRgb,
    MAX_HARDWARE_BROWSER_ENTRIES, MAX_HARDWARE_RASTER_PIXELS, MAX_HARDWARE_TEXT_BYTES,
};

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(1);
const DOCUMENT_TOOL_TIMEOUT: Duration = Duration::from_secs(30);

enum HardwareWork {
    Browse { generation: u64, directory: PathBuf },
    Load(HardwareLoadRequest),
}

#[derive(Default)]
pub(crate) struct HardwareIo {
    worker: Option<JoinHandle<Action>>,
    queued: Option<HardwareWork>,
}

impl HardwareIo {
    pub(crate) fn submit(&mut self, effect: Effect) {
        self.queued = match effect {
            Effect::Hardware(HardwareEffect::Browse {
                generation,
                directory,
            }) => Some(HardwareWork::Browse {
                generation,
                directory,
            }),
            Effect::Hardware(HardwareEffect::Load(request)) => Some(HardwareWork::Load(request)),
            _ => self.queued.take(),
        };
        self.start();
    }

    fn start(&mut self) {
        if self.worker.is_some() {
            return;
        }
        self.worker = self.queued.take().map(|work| {
            tokio::spawn(async move {
                match work {
                    HardwareWork::Browse {
                        generation,
                        directory,
                    } => match tokio::task::spawn_blocking(move || browse_directory(&directory))
                        .await
                    {
                        Ok(Ok((directory, entries))) => {
                            Action::Hardware(HardwareAction::BrowserLoaded {
                                generation,
                                directory,
                                entries,
                            })
                        }
                        Ok(Err(error)) => Action::Hardware(HardwareAction::BrowserFailed {
                            generation,
                            message: error.to_string(),
                        }),
                        Err(error) => Action::Hardware(HardwareAction::BrowserFailed {
                            generation,
                            message: format!("Hardware browser worker failed: {error}"),
                        }),
                    },
                    HardwareWork::Load(request) => {
                        let generation = request.generation;
                        match load_document(request).await {
                            Ok((page_count, preview, searchable_text)) => {
                                Action::Hardware(HardwareAction::PreviewLoaded {
                                    generation,
                                    page_count,
                                    preview,
                                    searchable_text,
                                })
                            }
                            Err(error) => Action::Hardware(HardwareAction::PreviewFailed {
                                generation,
                                message: error.to_string(),
                            }),
                        }
                    }
                }
            })
        });
    }

    pub(crate) async fn poll(&mut self, app: &mut App) -> bool {
        if !self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
        {
            return false;
        }
        let action = self
            .worker
            .take()
            .expect("finished Hardware worker")
            .await
            .unwrap_or_else(|error| {
                let generation = app
                    .hardware
                    .viewer
                    .as_ref()
                    .map_or(0, |value| value.generation);
                Action::Hardware(HardwareAction::PreviewFailed {
                    generation,
                    message: format!("Hardware worker failed: {error}"),
                })
            });
        let _ = yoctui_model::update(app, action);
        self.start();
        true
    }
}

pub(crate) fn browse_directory(directory: &Path) -> Result<(PathBuf, Vec<HardwareBrowserEntry>)> {
    let directory = fs::canonicalize(directory)
        .with_context(|| format!("Could not open Hardware directory {}", directory.display()))?;
    if !fs::symlink_metadata(&directory)?.is_dir() {
        bail!(
            "Hardware browser path is not a directory: {}",
            directory.display()
        );
    }
    let mut entries = Vec::new();
    for child in fs::read_dir(&directory)? {
        let child = child?;
        let metadata = fs::symlink_metadata(child.path())?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        let path = child.path();
        let is_directory = metadata.is_dir();
        let kind = (!is_directory)
            .then(|| HardwareDocumentKind::from_path(&path))
            .flatten();
        if !is_directory && kind.is_none() {
            continue;
        }
        entries.push(HardwareBrowserEntry {
            name: child.file_name().to_string_lossy().into_owned(),
            path,
            is_directory,
            kind,
        });
        if entries.len() >= MAX_HARDWARE_BROWSER_ENTRIES {
            break;
        }
    }
    entries.sort_by_key(|entry| (!entry.is_directory, entry.name.to_lowercase()));
    Ok((directory, entries))
}

async fn load_document(
    request: HardwareLoadRequest,
) -> Result<(usize, HardwarePreview, Vec<String>)> {
    validate_source(&request.document.path, request.document.kind)?;
    match request.document.kind {
        HardwareDocumentKind::Pdf => load_pdf(&request.document.path, request.page).await,
        HardwareDocumentKind::Raster => {
            let raster = decode_raster(request.document.path.clone()).await?;
            Ok((1, HardwarePreview::Raster(raster), Vec::new()))
        }
        HardwareDocumentKind::Kicad => load_kicad(&request.document.path).await,
        HardwareDocumentKind::Svg => load_svg(&request.document.path).await,
    }
}

fn validate_source(path: &Path, kind: HardwareDocumentKind) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("Hardware document is missing: {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("Hardware document must be a regular non-symlink file.");
    }
    if HardwareDocumentKind::from_path(path) != Some(kind) {
        bail!("Hardware document extension changed or is unsupported.");
    }
    Ok(())
}

async fn load_pdf(path: &Path, page: usize) -> Result<(usize, HardwarePreview, Vec<String>)> {
    let page_count = pdf_page_count(path).await.unwrap_or(1).max(1);
    let page = page.clamp(1, page_count);
    let text = pdf_text(path, page).await.unwrap_or_default();
    let searchable = bounded_lines(&text);
    if !program_exists("pdftoppm") {
        return Ok((
            page_count,
            HardwarePreview::Text {
                lines: searchable.clone(),
                limitation: Some("Install Poppler pdftoppm for graphical PDF rendering.".into()),
            },
            searchable,
        ));
    }
    match render_pdf_page(path, page).await {
        Ok(raster) => Ok((page_count, HardwarePreview::Raster(raster), searchable)),
        Err(error) if !searchable.is_empty() => Ok((
            page_count,
            HardwarePreview::Text {
                lines: searchable.clone(),
                limitation: Some(format!("Graphical PDF rendering failed: {error}")),
            },
            searchable,
        )),
        Err(error) => Err(error),
    }
}

async fn pdf_page_count(path: &Path) -> Result<usize> {
    if !program_exists("pdfinfo") {
        return Ok(1);
    }
    let output = run_command("pdfinfo", [path.as_os_str()]).await?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find_map(|line| line.strip_prefix("Pages:").map(str::trim))
        .ok_or_else(|| anyhow!("pdfinfo did not report a page count"))?
        .parse()
        .context("invalid PDF page count")
}

async fn pdf_text(path: &Path, page: usize) -> Result<String> {
    if !program_exists("pdftotext") {
        return Ok(String::new());
    }
    let page = page.to_string();
    let output = run_command(
        "pdftotext",
        [
            OsString::from("-f"),
            OsString::from(&page),
            OsString::from("-l"),
            OsString::from(page),
            path.as_os_str().to_owned(),
            OsString::from("-"),
        ],
    )
    .await?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

async fn render_pdf_page(path: &Path, page: usize) -> Result<HardwareRaster> {
    let prefix = temporary_path("pdf-page");
    let output_path = prefix.with_extension("ppm");
    let page = page.to_string();
    let result = run_command(
        "pdftoppm",
        [
            OsString::from("-f"),
            OsString::from(&page),
            OsString::from("-l"),
            OsString::from(page),
            OsString::from("-singlefile"),
            OsString::from("-scale-to-x"),
            OsString::from("320"),
            OsString::from("-scale-to-y"),
            OsString::from("-1"),
            path.as_os_str().to_owned(),
            prefix.as_os_str().to_owned(),
        ],
    )
    .await;
    if let Err(error) = result {
        let _ = fs::remove_file(&output_path);
        return Err(error);
    }
    let decoded = decode_raster(output_path.clone()).await;
    let _ = fs::remove_file(output_path);
    decoded
}

async fn load_kicad(path: &Path) -> Result<(usize, HardwarePreview, Vec<String>)> {
    let text = bounded_source_text(path)?;
    if program_exists("kicad-cli") {
        let pdf = temporary_path("kicad").with_extension("pdf");
        let result = run_command(
            "kicad-cli",
            [
                OsString::from("sch"),
                OsString::from("export"),
                OsString::from("pdf"),
                OsString::from("--output"),
                pdf.as_os_str().to_owned(),
                path.as_os_str().to_owned(),
            ],
        )
        .await;
        if result.is_ok() {
            let rendered = load_pdf(&pdf, 1).await;
            let _ = fs::remove_file(pdf);
            if let Ok((pages, preview, _)) = rendered {
                return Ok((pages, preview, text));
            }
        }
    }
    Ok((
        1,
        HardwarePreview::Text {
            lines: text.clone(),
            limitation: Some("Install kicad-cli for graphical schematic rendering.".into()),
        },
        text,
    ))
}

async fn load_svg(path: &Path) -> Result<(usize, HardwarePreview, Vec<String>)> {
    let text = bounded_source_text(path)?;
    Ok((
        1,
        HardwarePreview::Text {
            lines: text.clone(),
            limitation: Some(
                "SVG source preview; install a supported SVG raster converter for graphics.".into(),
            ),
        },
        text,
    ))
}

fn bounded_source_text(path: &Path) -> Result<Vec<String>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_HARDWARE_TEXT_BYTES as u64 {
        bail!("Hardware source text exceeds the 2 MiB preview limit.");
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("Hardware source is not valid UTF-8: {}", path.display()))?;
    Ok(bounded_lines(&text))
}

fn bounded_lines(text: &str) -> Vec<String> {
    let mut bytes = 0usize;
    text.lines()
        .take_while(|line| {
            bytes = bytes.saturating_add(line.len() + 1);
            bytes <= MAX_HARDWARE_TEXT_BYTES
        })
        .map(str::to_owned)
        .collect()
}

async fn decode_raster(path: PathBuf) -> Result<HardwareRaster> {
    tokio::task::spawn_blocking(move || {
        let image = image::ImageReader::open(&path)
            .with_context(|| format!("Could not open image {}", path.display()))?
            .with_guessed_format()?
            .decode()
            .with_context(|| format!("Could not decode image {}", path.display()))?;
        let image = if image.width() > 320 || image.height() > 240 {
            image.thumbnail(320, 240)
        } else {
            image
        }
        .to_rgb8();
        let (width, height) = image.dimensions();
        let pixels = image
            .pixels()
            .map(|pixel| HardwareRgb {
                red: pixel[0],
                green: pixel[1],
                blue: pixel[2],
            })
            .collect::<Vec<_>>();
        if pixels.len() > MAX_HARDWARE_RASTER_PIXELS {
            bail!("Decoded image exceeds the Hardware raster limit.");
        }
        Ok(HardwareRaster {
            width: width as usize,
            height: height as usize,
            pixels,
        })
    })
    .await
    .context("image decoder worker failed")?
}

async fn run_command<I, S>(program: &str, arguments: I) -> Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut command = Command::new(program);
    command.args(arguments).kill_on_drop(true);
    let output = tokio::time::timeout(DOCUMENT_TOOL_TIMEOUT, command.output())
        .await
        .with_context(|| format!("{program} timed out after 30 seconds"))??;
    if !output.status.success() {
        bail!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output)
}

fn program_exists(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|directory| directory.join(program).is_file())
    })
}

fn temporary_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "yoctui-{label}-{}-{}",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ))
}

#[cfg(test)]
#[path = "tests/hardware_io.rs"]
mod tests;
