//! Native accessibility for the immediate-mode GPUI shell.
//!
//! Nodes follow rendered controls and share their real callbacks/focus handles.
//! Platform requests are queued onto the UI thread; no scientific or permission
//! action is inferred from a label. Hidden modal backgrounds leave the tree.
use accesskit::{
    Action, ActionData, ActionRequest, Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate,
};
use futures::{StreamExt, channel::mpsc};
use gpui::{
    App, Bounds, ClickEvent, Div, Element, ElementId, FocusHandle, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, ParentElement, Pixels, SharedString, Stateful,
    Window, WindowId, div, prelude::*,
};
use raw_window_handle::HasWindowHandle;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    rc::Rc,
    sync::{Arc, Mutex},
};

const ROOT: NodeId = NodeId(1);
mod tree;

type Click = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type Request = Rc<dyn Fn(Action, Option<ActionData>, &mut Window, &mut App)>;

#[cfg(target_os = "macos")]
type Adapter = accesskit_macos::SubclassingAdapter;
#[cfg(target_os = "windows")]
type Adapter = accesskit_windows::SubclassingAdapter;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
type Adapter = accesskit_unix::Adapter;

struct Activate(Arc<Mutex<Option<Arc<TreeUpdate>>>>);
impl accesskit::ActivationHandler for Activate {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.0.lock().ok()?.as_deref().cloned()
    }
}
struct Actions(mpsc::UnboundedSender<ActionRequest>);
impl accesskit::ActionHandler for Actions {
    fn do_action(&mut self, request: ActionRequest) {
        let _ = self.0.unbounded_send(request);
    }
}
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
struct Deactivate;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
impl accesskit::DeactivationHandler for Deactivate {
    fn deactivate_accessibility(&mut self) {}
}

#[derive(Clone)]
struct Handler {
    focus: Option<FocusHandle>,
    click: Option<Click>,
    request: Option<Request>,
    bounds: Bounds<Pixels>,
    expanded: Option<bool>,
}
struct Record {
    id: NodeId,
    parent: NodeId,
    node: Node,
    handler: Handler,
}
struct WindowTree {
    adapter: Adapter,
    snapshot: Arc<Mutex<Option<Arc<TreeUpdate>>>>,
    focus: HashMap<NodeId, FocusHandle>,
    records: Vec<Record>,
    handlers: HashMap<NodeId, Handler>,
    parents: Vec<NodeId>,
    modal: Option<NodeId>,
    title: String,
}
#[derive(Default)]
struct Registry(HashMap<WindowId, Rc<RefCell<WindowTree>>>);
impl gpui::Global for Registry {}
fn registry(window: &Window, cx: &App) -> Option<Rc<RefCell<WindowTree>>> {
    cx.try_global::<Registry>()?
        .0
        .get(&window.window_handle().window_id())
        .cloned()
}

/// Install before the window is first shown. Adapters retain/subclass the native
/// handle and are removed by the window-closed observer before GPUI drops it.
pub(crate) fn install(window: &mut Window, cx: &mut App) {
    if cx.try_global::<Registry>().is_none() {
        cx.set_global(Registry::default());
        cx.on_window_closed(|cx, id| {
            cx.global_mut::<Registry>().0.remove(&id);
        })
        .detach();
    }
    if registry(window, cx).is_some() {
        return;
    }
    let snapshot = Arc::new(Mutex::new(None));
    let (sender, mut receiver) = mpsc::unbounded();
    let activate = Activate(snapshot.clone());
    let actions = Actions(sender);
    #[cfg(target_os = "macos")]
    let adapter = {
        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return;
        };
        let raw_window_handle::RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return;
        };
        // SAFETY: GPUI owns a live NSView; AccessKit retains it until uninstall.
        unsafe { Adapter::new(handle.ns_view.as_ptr(), activate, actions) }
    };
    #[cfg(target_os = "windows")]
    let adapter = {
        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return;
        };
        let raw_window_handle::RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        Adapter::new(
            windows::Win32::Foundation::HWND(handle.hwnd.get() as *mut _),
            activate,
            actions,
        )
    };
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    let adapter = Adapter::new(activate, actions, Deactivate);
    let state = Rc::new(RefCell::new(WindowTree {
        adapter,
        snapshot,
        focus: HashMap::new(),
        records: Vec::new(),
        handlers: HashMap::new(),
        parents: vec![ROOT],
        modal: None,
        title: "rexafs".into(),
    }));
    let handle = Window::window_handle(window);
    cx.global_mut::<Registry>()
        .0
        .insert(handle.window_id(), state);
    cx.spawn(async move |cx| {
        while let Some(request) = receiver.next().await {
            let _ = handle.update(cx, |_, window, cx| {
                let Some(state) = registry(window, cx) else {
                    return;
                };
                let handler = state.borrow().handlers.get(&request.target_node).cloned();
                let Some(handler) = handler else {
                    return;
                };
                if request.target_tree != TreeId::ROOT {
                    return;
                }
                match request.action {
                    Action::Focus => {
                        if let Some(focus) = handler.focus {
                            focus.focus(window, cx);
                        }
                    }
                    Action::Click | Action::Expand | Action::Collapse => {
                        if request.action == Action::Expand && handler.expanded != Some(false)
                            || request.action == Action::Collapse && handler.expanded != Some(true)
                        {
                            return;
                        }
                        if let Some(focus) = &handler.focus {
                            focus.focus(window, cx);
                        }
                        if let Some(click) = handler.click {
                            click(
                                &ClickEvent::Keyboard(gpui::KeyboardClickEvent {
                                    bounds: handler.bounds,
                                    ..Default::default()
                                }),
                                window,
                                cx,
                            );
                        }
                    }
                    _ => {
                        if let Some(action) = handler.request {
                            action(request.action, request.data, window, cx);
                        }
                    }
                }
            });
        }
    })
    .detach();
}

pub(crate) fn show(window: &Window) {
    #[cfg(target_os = "windows")]
    if let Ok(handle) = HasWindowHandle::window_handle(window)
        && let raw_window_handle::RawWindowHandle::Win32(raw) = handle.as_raw()
    {
        // SAFETY: GPUI still owns this window; only visibility changes.
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(
                windows::Win32::Foundation::HWND(raw.hwnd.get() as *mut _),
                windows::Win32::UI::WindowsAndMessaging::SW_SHOW,
            );
        }
    }
    window.activate_window();
}

fn node_id(id: Option<&GlobalElementId>) -> NodeId {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hash);
    NodeId(hash.finish().max(2))
}
fn rect(bounds: Bounds<Pixels>, scale: f32) -> accesskit::Rect {
    accesskit::Rect::new(
        f32::from(bounds.left()) as f64 * scale as f64,
        f32::from(bounds.top()) as f64 * scale as f64,
        f32::from(bounds.right()) as f64 * scale as f64,
        f32::from(bounds.bottom()) as f64 * scale as f64,
    )
}
fn publish(window: &mut Window, cx: &mut App) {
    let Some(state) = registry(window, cx) else {
        return;
    };
    let events = {
        let mut state = state.borrow_mut();
        let modal = state.modal;
        let mut keep = HashSet::new();
        if let Some(modal) = modal {
            keep.insert(modal);
        }
        for record in &state.records {
            if modal.is_none() || keep.contains(&record.parent) {
                keep.insert(record.id);
            }
        }
        let mut children: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for record in &state.records {
            if keep.contains(&record.id) {
                children
                    .entry(if modal == Some(record.id) {
                        ROOT
                    } else {
                        record.parent
                    })
                    .or_default()
                    .push(record.id);
            }
        }
        let mut root = Node::new(Role::Window);
        root.set_label(state.title.clone());
        root.set_children(children.remove(&ROOT).unwrap_or_default());
        let mut nodes = vec![(ROOT, root)];
        let mut handlers = HashMap::new();
        let mut focus = ROOT;
        for record in &state.records {
            if !keep.contains(&record.id) {
                continue;
            }
            let mut node = record.node.clone();
            node.set_children(children.remove(&record.id).unwrap_or_default());
            if record
                .handler
                .focus
                .as_ref()
                .is_some_and(|f| f.is_focused(window))
            {
                focus = record.id;
            }
            nodes.push((record.id, node));
            handlers.insert(record.id, record.handler.clone());
        }
        let mounted: HashSet<_> = state.records.iter().map(|r| r.id).collect();
        state.focus.retain(|id, _| mounted.contains(id));
        state.handlers = handlers;
        let tree = TreeUpdate {
            nodes,
            tree: Some(TreeInfo::new(ROOT)),
            tree_id: TreeId::ROOT,
            focus,
        };
        // Keep one complete snapshot for activation, and diff by node ID.
        // A plot/hover repaint must not clone the whole previous tree or
        // submit an empty native accessibility update.
        let previous = state.snapshot.lock().unwrap().clone();
        let update = tree::changes_since(&tree, previous.as_deref());
        if update.is_some() {
            *state.snapshot.lock().unwrap() = Some(Arc::new(tree));
        }
        #[cfg(target_os = "macos")]
        let focus_events = state
            .adapter
            .update_view_focus_state(window.is_window_active());
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        state
            .adapter
            .update_window_focus_state(window.is_window_active());
        // Mac/Windows return queued events; Unix dispatches in place and returns
        // unit. Preserve each adapter's return type when there is no change.
        let events = if let Some(update) = update {
            state.adapter.update_if_active(|| update)
        } else {
            Default::default()
        };
        #[cfg(target_os = "macos")]
        {
            (events, focus_events)
        }
        #[cfg(not(target_os = "macos"))]
        {
            events
        }
    };
    // Raising native notifications may re-enter the adapter. Release state first.
    #[cfg(target_os = "macos")]
    {
        if let Some(events) = events.0 {
            events.raise();
        }
        if let Some(events) = events.1 {
            events.raise();
        }
    }
    #[cfg(target_os = "windows")]
    if let Some(events) = events {
        events.raise();
    }
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    let _ = events;
}

/// A styled GPUI control whose native action invokes the same listener.
pub(crate) struct Control {
    inner: Stateful<Div>,
    label: SharedString,
    role: Role,
    selected: Option<bool>,
    expanded: Option<bool>,
    disabled: bool,
    focus: Option<FocusHandle>,
    click: Option<Click>,
    request: Option<Request>,
    value: Option<String>,
    numeric: Option<(f64, f64, f64, f64)>,
    description: Option<String>,
    placeholder: Option<String>,
    actions: Vec<Action>,
    modal: bool,
}
impl Control {
    pub(crate) fn new(inner: Stateful<Div>, label: impl Into<SharedString>, role: Role) -> Self {
        Self {
            inner,
            label: label.into(),
            role,
            selected: None,
            expanded: None,
            disabled: false,
            focus: None,
            click: None,
            request: None,
            value: None,
            numeric: None,
            description: None,
            placeholder: None,
            actions: Vec::new(),
            modal: false,
        }
    }
    pub(crate) fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }
    pub(crate) fn name_if_empty(mut self, name: impl Into<SharedString>) -> Self {
        if self.label.is_empty() {
            self.label = name.into();
        }
        self
    }
    pub(crate) fn selected(mut self, value: bool) -> Self {
        self.selected = Some(value);
        self
    }
    pub(crate) fn expanded(mut self, value: bool) -> Self {
        self.expanded = Some(value);
        self
    }
    pub(crate) fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }
    pub(crate) fn modal(mut self) -> Self {
        self.modal = true;
        self
    }
    pub(crate) fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }
    pub(crate) fn placeholder(mut self, value: impl Into<String>) -> Self {
        self.placeholder = Some(value.into());
        self
    }
    pub(crate) fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }
    pub(crate) fn numeric(mut self, value: f64, min: f64, max: f64, step: f64) -> Self {
        self.numeric = Some((value, min, max, step));
        self
    }
    pub(crate) fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self.inner = self.inner.track_focus(focus);
        self
    }
    pub(crate) fn on_request(
        mut self,
        actions: Vec<Action>,
        request: impl Fn(Action, Option<ActionData>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions = actions;
        self.request = Some(Rc::new(request));
        self
    }
    pub(crate) fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        let listener: Click = Rc::new(listener);
        self.click = Some(listener.clone());
        self.inner = self
            .inner
            .on_click(move |event, window, cx| listener(event, window, cx));
        self
    }
}
impl From<Stateful<Div>> for Control {
    fn from(inner: Stateful<Div>) -> Self {
        Self::new(inner, "", Role::Button)
    }
}
impl gpui::Styled for Control {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.inner.style()
    }
}
impl gpui::InteractiveElement for Control {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.inner.interactivity()
    }
}
impl gpui::StatefulInteractiveElement for Control {}
impl ParentElement for Control {
    fn extend(&mut self, children: impl IntoIterator<Item = gpui::AnyElement>) {
        self.inner.extend(children);
    }
}
impl IntoElement for Control {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Control {
    type RequestLayoutState = <Stateful<Div> as Element>::RequestLayoutState;
    type PrepaintState = <Stateful<Div> as Element>::PrepaintState;
    fn id(&self) -> Option<ElementId> {
        Element::id(&self.inner)
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        self.inner.source_location()
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        if !self.disabled
            && matches!(
                self.role,
                Role::Button
                    | Role::Tab
                    | Role::TextInput
                    | Role::CheckBox
                    | Role::MenuItem
                    | Role::DisclosureTriangle
                    | Role::ComboBox
                    | Role::Slider
            )
        {
            if self.focus.is_none()
                && let Some(state) = registry(window, cx)
            {
                self.focus = Some(
                    state
                        .borrow_mut()
                        .focus
                        .entry(node_id(id))
                        .or_insert_with(|| cx.focus_handle().tab_index(0).tab_stop(true))
                        .clone(),
                );
            }
            if let Some(focus) = self.focus.clone() {
                let focus = focus.tab_index(0).tab_stop(true);
                self.focus = Some(focus.clone());
                let inner =
                    std::mem::replace(&mut self.inner, div().id("accessibility-placeholder"));
                self.inner = inner.track_focus(&focus).tab_index(0);
            }
        } else if self.disabled {
            let inner = std::mem::replace(&mut self.inner, div().id("accessibility-placeholder"));
            self.inner = inner.tab_stop(false);
        }
        self.inner.request_layout(id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let state = registry(window, cx);
        let clipped = bounds.intersect(&window.content_mask().bounds);
        let visible = clipped.size.width > gpui::px(0.) && clipped.size.height > gpui::px(0.);
        if let Some(state) = &state
            && visible
        {
            let id = node_id(id);
            let mut node = Node::new(self.role);
            node.set_label(self.label.to_string());
            node.set_bounds(rect(clipped, window.scale_factor()));
            if let Some(value) = &self.value {
                node.set_value(value.clone());
            }
            if let Some((value, min, max, step)) = self.numeric {
                node.set_numeric_value(value);
                node.set_min_numeric_value(min);
                node.set_max_numeric_value(max);
                node.set_numeric_value_step(step);
            }
            if let Some(description) = &self.description {
                node.set_description(description.clone());
            }
            if let Some(placeholder) = &self.placeholder {
                node.set_placeholder(placeholder.clone());
            }
            if let Some(selected) = self.selected {
                if self.role == Role::CheckBox {
                    node.set_toggled(if selected {
                        accesskit::Toggled::True
                    } else {
                        accesskit::Toggled::False
                    });
                } else {
                    node.set_selected(selected);
                }
            }
            if let Some(expanded) = self.expanded {
                node.set_expanded(expanded);
            }
            if self.disabled {
                node.set_disabled();
            } else {
                if self.focus.is_some() {
                    node.add_action(Action::Focus);
                }
                if self.click.is_some() {
                    node.add_action(Action::Click);
                    if self.expanded.is_some() {
                        node.add_action(Action::Expand);
                        node.add_action(Action::Collapse);
                    }
                }
                for &action in &self.actions {
                    node.add_action(action);
                }
            }
            let mut state = state.borrow_mut();
            let parent = *state.parents.last().unwrap_or(&ROOT);
            let handler = Handler {
                focus: self.focus.clone().filter(|_| !self.disabled),
                click: self.click.clone().filter(|_| !self.disabled),
                request: self.request.clone().filter(|_| !self.disabled),
                bounds,
                expanded: self.expanded,
            };
            state.records.push(Record {
                id,
                parent,
                node,
                handler,
            });
            state.parents.push(id);
            if self.modal {
                state.modal = Some(id);
            }
        }
        let prepaint = self
            .inner
            .prepaint(id, inspector, bounds, layout, window, cx);
        if let Some(state) = state
            && visible
        {
            state.borrow_mut().parents.pop();
        }
        prepaint
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
    }
}

pub(crate) fn root(element: impl IntoElement, title: impl Into<String>) -> Root {
    Root {
        inner: element.into_any_element(),
        title: title.into(),
    }
}
pub(crate) struct Root {
    inner: gpui::AnyElement,
    title: String,
}
impl IntoElement for Root {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Root {
    type RequestLayoutState = <gpui::AnyElement as Element>::RequestLayoutState;
    type PrepaintState = <gpui::AnyElement as Element>::PrepaintState;
    fn id(&self) -> Option<ElementId> {
        Some("accessible-window".into())
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        Element::request_layout(&mut self.inner, id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(state) = registry(window, cx) {
            let mut state = state.borrow_mut();
            state.records.clear();
            state.parents = vec![ROOT];
            state.modal = None;
            state.title = self.title.clone();
        }
        Element::prepaint(&mut self.inner, id, inspector, bounds, layout, window, cx)
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        Element::paint(
            &mut self.inner,
            id,
            inspector,
            bounds,
            layout,
            prepaint,
            window,
            cx,
        );
        let handle = Window::window_handle(window);
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| publish(window, cx));
        });
    }
}

pub(crate) fn scroll(
    inner: Stateful<Div>,
    name: impl Into<SharedString>,
    handle: &gpui::ScrollHandle,
) -> Control {
    let scroll = handle.clone();
    Control::new(inner, name, Role::ScrollView)
        .track_scroll(handle)
        .on_request(
            vec![
                Action::ScrollUp,
                Action::ScrollDown,
                Action::SetScrollOffset,
            ],
            move |action, data, window, _| {
                let mut offset = scroll.offset();
                match action {
                    Action::ScrollUp => offset.y += window.viewport_size().height * 0.6,
                    Action::ScrollDown => offset.y -= window.viewport_size().height * 0.6,
                    Action::SetScrollOffset => {
                        if let Some(ActionData::SetScrollOffset(point)) = data {
                            offset =
                                gpui::point(gpui::px(-point.x as f32), gpui::px(-point.y as f32));
                        }
                    }
                    _ => return,
                }
                scroll.set_offset(offset);
                window.refresh();
            },
        )
}
