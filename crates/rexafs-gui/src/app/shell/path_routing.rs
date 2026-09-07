//! Shared startup, picker and external-drop routing. Directory contents are intake's concern.
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use gpui::{
    Context, ExternalPaths, IntoElement, ParentElement, Styled, Window, div, prelude::*, px,
};

use super::button;
use crate::app::{DismissPathRoute, StudioApp};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DropRoute {
    OpenProject(PathBuf),
    ChooseProject(Vec<PathBuf>),
    ImportFolder(PathBuf),
    ImportData(Vec<PathBuf>),
    Mixed {
        projects: Vec<PathBuf>,
        data: Vec<PathBuf>,
    },
    Nothing,
}

/// Classify only explicitly supplied paths; even a directory named *.rxs is data.
pub(crate) fn route_paths(paths: Vec<PathBuf>, is_dir: impl Fn(&Path) -> bool) -> DropRoute {
    let (projects, data): (Vec<_>, Vec<_>) = paths
        .into_iter()
        .partition(|path| !is_dir(path) && crate::project::is_project(path));
    match (projects.as_slice(), data.is_empty()) {
        ([], true) => DropRoute::Nothing,
        ([], false) if data.len() == 1 && is_dir(&data[0]) => {
            DropRoute::ImportFolder(data[0].clone())
        }
        ([], false) => DropRoute::ImportData(data),
        ([path], true) => DropRoute::OpenProject(path.clone()),
        (_, true) => DropRoute::ChooseProject(projects),
        _ => DropRoute::Mixed { projects, data },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoutedImport {
    pub paths: Vec<PathBuf>,
    pub remember: bool,
}

/// Consume the continuation once, after both catalog discovery and index verification.
fn take_ready_import(
    pending: &mut VecDeque<RoutedImport>,
    scanning: bool,
    verifying: bool,
) -> Option<RoutedImport> {
    if scanning || verifying {
        None
    } else {
        pending.pop_front()
    }
}

pub(crate) struct RoutingCard {
    route: DropRoute,
    data: Option<RoutedImport>,
}

impl RoutingCard {
    pub(crate) fn after_failed_project(data: Option<RoutedImport>) -> Option<Self> {
        data.filter(|data| !data.paths.is_empty()).map(|data| Self {
            route: DropRoute::ImportData(data.paths.clone()),
            data: Some(data),
        })
    }
}

/// Pinned gpui (3060e41) PathPromptOptions has no starting-directory field, so the
/// picker opens at the platform default; recent folders import directly via the palette.
fn import_location_status(folder: Option<&Path>) -> String {
    folder.map_or_else(
        || "Import files or folders".into(),
        |folder| {
            format!(
                "Import files or folders · Last import folder: {}",
                folder.display()
            )
        },
    )
}

impl StudioApp {
    pub(crate) fn route_paths(
        &mut self,
        paths: Vec<PathBuf>,
        remember: bool,
        cx: &mut Context<Self>,
    ) {
        let route = route_paths(paths, Path::is_dir);
        match route {
            DropRoute::Nothing => return,
            DropRoute::OpenProject(path) => self.load_project_path(path, cx),
            DropRoute::ImportFolder(path) => {
                // Preserve indexed folder startup until slice 1.7 makes intake index-aware.
                self.scan_folder(path.clone(), false, cx);
                if remember {
                    let path = path.canonicalize().unwrap_or(path);
                    crate::settings::push_recent(
                        &mut self.structure.settings.recent_import_folders,
                        path,
                        8,
                    );
                    self.persist_recent_locations();
                }
            }
            DropRoute::ImportData(paths) => {
                self.import_routed_data(RoutedImport { paths, remember }, cx)
            }
            route => {
                self.path_route = Some(RoutingCard {
                    route,
                    data: Some(RoutedImport {
                        paths: vec![],
                        remember,
                    }),
                });
                // Startup and native picker callbacks don't have a Window. Focus after rendering,
                // so list/editor bindings cannot operate behind the routing card.
                let view = cx.weak_entity();
                let handle = self.main_window;
                cx.defer(move |cx| {
                    let _ = handle.update(cx, |_, window, cx| {
                        let _ = view.update(cx, |app, cx| {
                            if app.path_route.is_some() {
                                app.root_focus.focus(window, cx);
                            }
                        });
                    });
                });
            }
        }
        cx.notify();
    }

    pub(crate) fn dismiss_path_route(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.path_route = None;
        self.root_focus.focus(window, cx);
        cx.notify();
    }

    pub(crate) fn import_routed_data(&mut self, data: RoutedImport, cx: &mut Context<Self>) {
        self.pending_routed_import.push_back(data);
        if self.catalog.scanning || self.verify_running {
            self.status = "Import queued until catalog scanning and verification finish.".into();
            cx.notify();
        }
        self.finish_routed_import(cx);
    }

    pub(crate) fn finish_routed_import(&mut self, cx: &mut Context<Self>) {
        let Some(data) = take_ready_import(
            &mut self.pending_routed_import,
            self.catalog.scanning,
            self.verify_running,
        ) else {
            return;
        };
        if data.remember {
            let folders = data
                .paths
                .iter()
                .filter_map(|path| {
                    let path = path.canonicalize().ok()?;
                    if path.is_dir() {
                        Some(path)
                    } else {
                        path.parent().map(Path::to_path_buf)
                    }
                })
                .collect();
            self.append_import_with_recents(data.paths, false, folders, cx);
        } else {
            self.append_import(data.paths, false, cx);
        }
    }

    pub(crate) fn remember_project(&mut self, path: PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        crate::settings::push_recent(&mut self.structure.settings.recent_projects, path, 8);
        self.persist_recent_locations();
    }

    pub(crate) fn persist_recent_locations(&mut self) {
        if let Err(error) = self.structure.settings.save() {
            self.record_job_error("recent locations", error);
        }
    }

    pub(crate) fn open_import_picker(&mut self, folder: Option<PathBuf>, cx: &mut Context<Self>) {
        let generation = self.project_generation;
        self.status = import_location_status(folder.as_deref()).into();
        cx.notify();
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: true,
            multiple: true,
            // On macOS, prompt is the confirm button label, not a directory or message.
            prompt: Some("Import".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await {
                this.update(cx, |app, cx| {
                    if app.project_generation == generation {
                        app.route_paths(paths, true, cx);
                    } else {
                        app.status =
                            "Project changed while the picker was open. Choose Import… again."
                                .into();
                        cx.notify();
                    }
                })
                .ok();
            }
        })
        .detach();
    }

    pub(crate) fn path_route_overlay(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let card = self.path_route.as_ref()?;
        let t = self.theme;
        let mut content = div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .w(px(560.))
            .max_w_full()
            .bg(t.surface)
            .border_1()
            .border_color(t.border)
            .rounded_lg();
        match &card.route {
            DropRoute::ImportData(_) => {
                let data = card.data.clone()?;
                content = content
                    .child("Project could not be opened. The dropped data is still available.")
                    .child(
                        button(&t, "route-data-only", "Import data only", true).on_click(
                            cx.listener(move |app, _, window, cx| {
                                app.dismiss_path_route(window, cx);
                                app.import_routed_data(data.clone(), cx);
                            }),
                        ),
                    );
            }
            DropRoute::Mixed { projects, data } => {
                let projects = projects.clone();
                let data = RoutedImport {
                    paths: data.clone(),
                    remember: card.data.as_ref().is_some_and(|d| d.remember),
                };
                let only_data = data.clone();
                content = content
                    .child("Project and data dropped together")
                    .child(
                        button(
                            &t,
                            "route-open-then-import",
                            "Open project, then import data…",
                            true,
                        )
                        .on_click(cx.listener(
                            move |app, _, window, cx| {
                                if let [path] = projects.as_slice() {
                                    app.dismiss_path_route(window, cx);
                                    app.load_project_then_import(
                                        path.clone(),
                                        Some(data.clone()),
                                        cx,
                                    );
                                } else {
                                    app.path_route = Some(RoutingCard {
                                        route: DropRoute::ChooseProject(projects.clone()),
                                        data: Some(data.clone()),
                                    });
                                    cx.notify();
                                }
                            },
                        )),
                    )
                    .child(
                        button(&t, "route-data-only", "Import data only", false).on_click(
                            cx.listener(move |app, _, window, cx| {
                                app.dismiss_path_route(window, cx);
                                app.import_routed_data(only_data.clone(), cx);
                            }),
                        ),
                    );
            }
            DropRoute::ChooseProject(projects) => {
                content = content.child("Choose a project to open");
                let mut list = div()
                    .id("route-project-list")
                    .max_h(px(300.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_2();
                for (i, path) in projects.iter().enumerate() {
                    let path = path.clone();
                    let data = card.data.clone().filter(|data| !data.paths.is_empty());
                    list = list.child(
                        button(&t, ("route-project", i), path.display().to_string(), false)
                            .overflow_hidden()
                            .on_click(cx.listener(move |app, _, window, cx| {
                                app.dismiss_path_route(window, cx);
                                app.load_project_then_import(path.clone(), data.clone(), cx);
                            })),
                    );
                }
                content = content.child(list);
            }
            _ => return None,
        }
        Some(
            div()
                .id("path-route-overlay")
                .absolute()
                .inset_0()
                .occlude()
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::rgba(0x00000066))
                .key_context("PathRoute")
                .on_action(cx.listener(|app, _: &DismissPathRoute, window, cx| {
                    app.dismiss_path_route(window, cx)
                }))
                .child(
                    content.child(button(&t, "route-cancel", "Cancel", false).on_click(
                        cx.listener(|app, _, window, cx| app.dismiss_path_route(window, cx)),
                    )),
                ),
        )
    }

    pub(crate) fn empty_drop_target(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.theme;
        div()
            .id("empty-drop-target")
            .flex_1()
            .flex()
            .flex_col()
            .gap_3()
            .items_center()
            .justify_center()
            .border_2()
            .border_color(t.border)
            .drag_over::<ExternalPaths>(move |style, _, _, _| {
                style.border_color(t.accent).bg(t.raised)
            })
            .on_drop(cx.listener(|app, paths: &ExternalPaths, _, cx| {
                app.route_paths(paths.paths().to_vec(), false, cx)
            }))
            .child("Drop data files, folders, or a .rxs project here")
            .child(
                button(&t, "empty-import", "Import…", true)
                    .on_click(cx.listener(|app, _, _, cx| app.open_folder(cx))),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_route_waits_for_restore_and_verification_then_imports_once() {
        let data = RoutedImport {
            paths: vec!["/data/new.dat".into()],
            remember: true,
        };
        let mut pending = VecDeque::from([data.clone()]);
        for (scanning, verifying) in [(true, false), (true, true), (false, true)] {
            assert_eq!(take_ready_import(&mut pending, scanning, verifying), None);
            assert_eq!(pending, VecDeque::from([data.clone()]));
        }
        assert_eq!(take_ready_import(&mut pending, false, false), Some(data));
        assert_eq!(take_ready_import(&mut pending, false, false), None);
        assert_eq!(take_ready_import(&mut VecDeque::new(), false, false), None);
    }

    #[test]
    fn ordinary_drops_during_verification_queue_without_losing_earlier_drops() {
        let first = RoutedImport {
            paths: vec!["/outside/first.dat".into()],
            remember: false,
        };
        let second = RoutedImport {
            paths: vec!["/outside/second.dat".into()],
            remember: true,
        };
        let mut pending = VecDeque::from([first.clone()]);
        assert_eq!(take_ready_import(&mut pending, false, true), None);
        pending.push_back(second.clone());
        assert_eq!(take_ready_import(&mut pending, false, true), None);
        assert_eq!(take_ready_import(&mut pending, false, false), Some(first));
        assert_eq!(take_ready_import(&mut pending, true, false), None);
        assert_eq!(take_ready_import(&mut pending, false, false), Some(second));
        assert!(pending.is_empty());
    }

    #[test]
    fn failed_project_retains_all_mixed_drop_data_for_import_only() {
        let data = RoutedImport {
            paths: (0..20)
                .map(|n| PathBuf::from(format!("/{n}.dat")))
                .collect(),
            remember: true,
        };
        let card = RoutingCard::after_failed_project(Some(data.clone())).unwrap();
        assert_eq!(card.route, DropRoute::ImportData(data.paths.clone()));
        assert_eq!(card.data, Some(data));
        assert!(RoutingCard::after_failed_project(None).is_none());
        assert!(
            RoutingCard::after_failed_project(Some(RoutedImport {
                paths: vec![],
                remember: false
            }))
            .is_none()
        );
    }

    #[test]
    fn single_folder_uses_indexed_scan_but_mixed_and_multiple_roots_use_intake() {
        for folder in ["/data/huge-folder", "folder.rxs"] {
            assert_eq!(
                route_paths(vec![folder.into()], |_| true),
                DropRoute::ImportFolder(folder.into())
            );
        }
        let folders = vec!["/one".into(), "/two".into()];
        assert_eq!(
            route_paths(folders.clone(), |_| true),
            DropRoute::ImportData(folders)
        );
        assert_eq!(
            route_paths(vec!["project.rxs".into(), "/data".into()], |p| p
                == Path::new("/data")),
            DropRoute::Mixed {
                projects: vec!["project.rxs".into()],
                data: vec!["/data".into()]
            }
        );
    }

    #[test]
    fn route_explicit_paths() {
        let paths = |names: &[&str]| names.iter().map(PathBuf::from).collect();
        let route = |names: &[&str]| route_paths(paths(names), |p| p == Path::new("folder.rxs"));
        assert_eq!(route(&[]), DropRoute::Nothing);
        assert_eq!(route(&["a.RXS"]), DropRoute::OpenProject("a.RXS".into()));
        assert_eq!(
            route(&["a.rxs", "b.rxs"]),
            DropRoute::ChooseProject(paths(&["a.rxs", "b.rxs"]))
        );
        assert_eq!(
            route(&["a.dat", "folder.rxs"]),
            DropRoute::ImportData(paths(&["a.dat", "folder.rxs"]))
        );
        assert_eq!(
            route(&["a.dat", "a.rxs", "b.rxs"]),
            DropRoute::Mixed {
                projects: paths(&["a.rxs", "b.rxs"]),
                data: paths(&["a.dat"])
            }
        );
    }

    #[test]
    fn import_status_describes_the_last_folder_without_promising_a_picker_location() {
        assert_eq!(import_location_status(None), "Import files or folders");
        assert_eq!(
            import_location_status(Some(Path::new("/data/run 12"))),
            "Import files or folders · Last import folder: /data/run 12"
        );
    }
}
