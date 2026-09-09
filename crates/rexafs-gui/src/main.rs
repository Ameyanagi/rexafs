//! rexafs-gui: GPUI desktop shell for rexafs XAS analysis.
//!
//! Opens a single window whose root view is [`app::StudioApp`].
//! UX reference: doc/gui-ux-design.md.

// Explorer/Start menu launches must not allocate a console. Keep the test
// harness as a console program so Cargo can collect its output normally.
#![cfg_attr(all(target_os = "windows", not(test)), windows_subsystem = "windows")]

mod accessibility;
mod app;
mod catalog;
mod codex_client;
mod debug_stats;
mod feffgen;
mod fit_details;
mod fit_report;
mod fitting;
mod group_identity;
mod icons;
mod import_mapping;
mod import_recipes;
mod joint_fitting;
mod licenses;
mod params;
mod plotting;
mod project;
mod publication;
mod settings;
mod source_evidence;
mod spectrum_colors;
mod spectrum_interest;
mod structure;
mod theme;
mod updates;
mod widgets;

use std::path::PathBuf;

use gpui::{App, AppContext, Bounds, Size, WindowBounds, WindowOptions, px};

use crate::app::StudioApp;

fn main() {
    #[cfg(target_os = "windows")]
    attach_parent_console();

    // Handle re-executed FEFF stage workers before argument or GUI setup.
    #[cfg(feature = "feff10-runner")]
    feff10::worker::init();

    match std::env::args().nth(1).as_deref() {
        Some("--version") => {
            println!("rexafs {}", updates::installed_label());
            return;
        }
        Some("--build-info") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&updates::build_info()).unwrap()
            );
            return;
        }
        Some("--self-check") => {
            if let Err(error) = check_package() {
                eprintln!("rexafs package check failed: {error}");
                std::process::exit(1);
            }
            return;
        }
        Some("--self-check-feff") => {
            if let Err(error) = feffgen::check_package_backends() {
                eprintln!("rexafs FEFF package check failed: {error}");
                std::process::exit(1);
            }
            return;
        }
        _ => {}
    }

    // Optional positional arg: a spectrum, folder or .rxs to open. Enables
    // "open with", scripted launches, and screenshot testing without driving
    // the native file dialog.
    let initial_open: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from);

    // gpui 0.2.2 has no zero-arg `Application::new()`; the public entry point
    // is `gpui_platform::application()`.
    gpui_platform::application()
        .with_assets(icons::IconAssets)
        .run(move |cx: &mut App| {
            cx.bind_keys(widgets::text_input::text_input_keybindings());
            cx.bind_keys(app::studio_keybindings());
            cx.on_action(|_: &app::Quit, cx| cx.quit());
            let window_size = Size {
                width: px(1440.0),
                height: px(900.0),
            };
            let bounds = Bounds::centered(None, window_size, cx);
            let main_window = cx
                .open_window(
                    WindowOptions {
                        show: false,
                        focus: false,
                        titlebar: Some(gpui::TitlebarOptions {
                            title: Some(updates::application_name().into()),
                            ..Default::default()
                        }),
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        ..Default::default()
                    },
                    |window, cx| {
                        if debug_stats::enabled() {
                            eprintln!("[renderer] {:?}", window.gpu_specs());
                        }
                        accessibility::install(window, cx);
                        cx.new(|cx| StudioApp::new_with_open(initial_open.clone(), window, cx))
                    },
                )
                .expect("failed to open window");
            let _ = main_window.update(cx, |_, window, _| accessibility::show(window));
            cx.activate(true);
        });
}

/// Keep terminal diagnostics available when launched from a shell, while
/// preserving redirected output used by packaging and installer checks.
#[cfg(target_os = "windows")]
fn attach_parent_console() {
    use windows::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
    };

    // SAFETY: these APIs inspect this process's standard handles and attach
    // only to an existing parent console; they never create a console window.
    unsafe {
        let has_output = [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE]
            .into_iter()
            .any(|kind| GetStdHandle(kind).is_ok_and(|handle| !handle.is_invalid()));
        if !has_output {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

/// Exercise the distributed example and numerical pipeline without a display.
/// This deliberately requires packaged resources, with no checkout fallback.
fn check_package() -> Result<(), String> {
    let path = app::packaged_data_file().ok_or("Packaged example is missing")?;
    let (energy, mu) = params::load_raw(&path, &params::PipelineParams::default())?;
    let mut spectrum = rexafs::Spectrum::from_arrays(&energy, &mu).map_err(|e| e.to_string())?;
    spectrum.fft().map_err(|e| e.to_string())?;
    let e0 = spectrum.e0().ok_or("Missing E0")?;
    let k = spectrum.k().ok_or("Missing k")?;
    let r = spectrum.r().ok_or("Missing R")?;
    let chi = spectrum.chi().ok_or("Missing chi")?;
    let magnitude = spectrum.chir_mag().ok_or("Missing Fourier magnitude")?;
    if !e0.is_finite()
        || k.is_empty()
        || r.is_empty()
        || chi.iter().chain(magnitude.iter()).any(|v| !v.is_finite())
    {
        return Err("Packaged example produced invalid numerical output".into());
    }
    println!(
        "rexafs {}: package check passed (E0={}, k={}, R={})",
        env!("CARGO_PKG_VERSION"),
        e0,
        k.len(),
        r.len()
    );
    Ok(())
}

fn native_menus(blocked: bool, cx: &mut App) {
    use gpui::{Menu, MenuItem};
    cx.set_menus([
        Menu::new("rexafs").items([
            MenuItem::action("About rexafs", app::ShowHelp).disabled(blocked),
            MenuItem::separator(),
            MenuItem::action("Quit rexafs", app::Quit),
        ]),
        Menu::new("File").items([
            MenuItem::action("Import…", app::ImportPaths).disabled(blocked),
            MenuItem::action("Open project…", app::OpenProject).disabled(blocked),
            MenuItem::action("Save project…", app::SaveProject).disabled(blocked),
        ]),
        Menu::new("Edit").items([
            MenuItem::action("Undo analysis change", app::Undo).disabled(blocked),
            MenuItem::action("Redo analysis change", app::Redo).disabled(blocked),
            MenuItem::separator(),
            MenuItem::action("Cut", widgets::text_input::Cut),
            MenuItem::action("Copy", widgets::text_input::Copy),
            MenuItem::action("Paste", widgets::text_input::Paste),
            MenuItem::action("Select all", widgets::text_input::SelectAll),
        ]),
        Menu::new("View").items([
            MenuItem::action("Data", app::StageData).disabled(blocked),
            MenuItem::action("Normalize", app::StageNormalize).disabled(blocked),
            MenuItem::action("Background", app::StageBackground).disabled(blocked),
            MenuItem::action("Transform", app::StageTransform).disabled(blocked),
            MenuItem::action("Fit", app::StageFit).disabled(blocked),
            MenuItem::action("Series", app::StageSeries).disabled(blocked),
            MenuItem::action("Publish", app::StagePublish).disabled(blocked),
            MenuItem::separator(),
            MenuItem::action("Groups", app::ToggleDataPanel).disabled(blocked),
            MenuItem::action("Parameters", app::ToggleContextPanel).disabled(blocked),
            MenuItem::action("Switch theme", app::SwitchTheme).disabled(blocked),
            MenuItem::action("Command palette…", app::PaletteOpen).disabled(blocked),
        ]),
        Menu::new("Help").items([
            MenuItem::action("Open Cu example", app::ShowExample).disabled(blocked),
            MenuItem::action("Licenses", app::ShowLicenses).disabled(blocked),
            MenuItem::action("Updates…", app::ShowUpdates).disabled(blocked),
        ]),
    ]);
}
