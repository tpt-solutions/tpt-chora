use crate::semantic::{BridgeNode, SemanticIR};

pub struct A11yBridge {
    last_update: Option<A11yTreeUpdate>,
    announcements: Vec<String>,
    focused_node: Option<u64>,
    os_backend: OsBackend,
}

#[cfg(all(feature = "native-a11y-backends", target_os = "windows"))]
mod windows_uia {
    //! Real Windows UI Automation provider.
    //!
    //! Exposes the `tpt-chora` semantic tree to a screen reader through a
    //! fragment provider hosted on a message-only window. `A11yBridge`
    //! pushes `BridgeNode`s via `sync_tree`; each node becomes a
    //! `UiaNodeProvider` implementing `IRawElementProviderFragment` (+
    //! `IRawElementProviderSimple`), the tree root additionally implements
    //! `IRawElementProviderFragmentRoot`. The window answers `WM_GETOBJECT`
    //! with the root provider, so Narrator/Inspect/UIA client apps can
    //! traverse the rendered Chora content, read its role/label/state, and
    //! receive focus-change and notification (live-region) events.

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use windows::core::{implement, w, ComObject, Interface, HRESULT};
    use windows::Win32::Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED, SAFEARRAY};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Ole::{
        SafeArrayCreateVector, SafeArrayDestroy, SafeArrayPutElement,
    };
    use windows::Win32::System::Variant::VT_I4;
    use windows::Win32::UI::Accessibility::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    use crate::semantic::{AccessibilityRole, AccessibilityState, BridgeNode};

    /// Sentinel id used for the fragment-root provider: it always resolves to
    /// the current tree root so the provider object is stable across tree
    /// updates.
    const ROOT_PROVIDER_ID: u64 = u64::MAX;
    const WINDOW_CLASS: windows::core::PCWSTR = w!("tpt_chora_uia_window");
    const ROOT_PROP: windows::core::PCWSTR = w!("tpt_chora_uia_root");

    /// E_NOTIMPL: the "no element / no pattern here" answer for UIA.
    fn not_implemented<T>() -> windows::core::Result<T> {
        Err(HRESULT(0x8000_4001u32 as i32).into())
    }

    /// Shared, thread-safe snapshot of the semantic tree plus the live
    /// provider instances. Every `UiaNodeProvider` holds an `Arc` to this,
    /// so a provider stays functional even after the tree changes.
    struct UiaState {
        nodes: Mutex<HashMap<u64, BridgeNode>>,
        parents: Mutex<HashMap<u64, u64>>,
        root_id: Mutex<Option<u64>>,
        focused: Mutex<Option<u64>>,
        providers: Mutex<HashMap<u64, IRawElementProviderFragment>>,
        root_provider: Mutex<Option<IRawElementProviderFragment>>,
        host: Mutex<Option<IRawElementProviderSimple>>,
        hwnd_raw: usize,
    }

    impl UiaState {
        fn new(hwnd: HWND) -> Arc<Self> {
            Arc::new(Self {
                nodes: Mutex::new(HashMap::new()),
                parents: Mutex::new(HashMap::new()),
                root_id: Mutex::new(None),
                focused: Mutex::new(None),
                providers: Mutex::new(HashMap::new()),
                root_provider: Mutex::new(None),
                host: Mutex::new(None),
                hwnd_raw: hwnd.0 as usize,
            })
        }

        fn hwnd(&self) -> HWND {
            HWND(self.hwnd_raw as *mut core::ffi::c_void)
        }

        fn node(&self, id: u64) -> Option<BridgeNode> {
            self.nodes.lock().ok()?.get(&id).cloned()
        }

        fn effective_id(&self, id: u64) -> u64 {
            if id == ROOT_PROVIDER_ID {
                self.root_id
                    .lock()
                    .ok()
                    .and_then(|g| *g)
                    .unwrap_or(ROOT_PROVIDER_ID)
            } else {
                id
            }
        }

        fn parent_of(&self, id: u64) -> Option<u64> {
            self.parents.lock().ok()?.get(&id).copied()
        }

        fn is_root(&self, id: u64) -> bool {
            id == ROOT_PROVIDER_ID || self.root_id.lock().ok().and_then(|g| *g) == Some(id)
        }

        /// Returns the cached fragment provider for `id`, creating and caching
        /// it on first use so UIA sees stable object identities.
        fn fragment(
            self: &Arc<Self>,
            id: u64,
        ) -> windows::core::Result<IRawElementProviderFragment> {
            if id == ROOT_PROVIDER_ID {
                if let Some(root) = self
                    .root_provider
                    .lock()
                    .ok()
                    .as_ref()
                    .and_then(|guard| guard.as_ref())
                {
                    return Ok(root.clone());
                }
                return not_implemented();
            }
            if let Some(cached) = self
                .providers
                .lock()
                .ok()
                .and_then(|map| map.get(&id).cloned())
            {
                return Ok(cached);
            }
            let com: ComObject<UiaNodeProvider> = ComObject::new(UiaNodeProvider {
                id,
                state: Arc::clone(self),
            });
            let fragment: IRawElementProviderFragment = com.into_interface();
            if let Ok(mut map) = self.providers.lock() {
                if let Some(cached) = map.get(&id) {
                    return Ok(cached.clone());
                }
                map.insert(id, fragment.clone());
            }
            Ok(fragment)
        }

        /// Casts the provider for `id` to `IRawElementProviderSimple`.
        fn simple(self: &Arc<Self>, id: u64) -> windows::core::Result<IRawElementProviderSimple> {
            self.fragment(id)?.cast()
        }

        /// The HWND's native host provider (cached).
        fn host_provider(&self) -> windows::core::Result<IRawElementProviderSimple> {
            if let Some(host) = self
                .host
                .lock()
                .ok()
                .as_ref()
                .and_then(|guard| guard.as_ref())
            {
                return Ok(host.clone());
            }
            // SAFETY: `UiaHostProviderFromHwnd` is a pure Win32 call with no
            // pointer parameters beyond the HWND; we hold the window open.
            #[allow(unsafe_code)]
            let host = unsafe { UiaHostProviderFromHwnd(self.hwnd())? };
            if let Ok(mut slot) = self.host.lock() {
                *slot = Some(host.clone());
            }
            Ok(host)
        }

        /// The fragment root interface (the root provider, if set).
        fn fragment_root(
            self: &Arc<Self>,
        ) -> windows::core::Result<IRawElementProviderFragmentRoot> {
            self.fragment(ROOT_PROVIDER_ID)?.cast()
        }

        /// Raises a notification/live-region announcement on the root element.
        fn announce(self: &Arc<Self>, message: &str) -> Result<(), crate::A11yError> {
            let provider = match self.simple(ROOT_PROVIDER_ID) {
                Ok(p) => p,
                Err(_) => return Ok(()),
            };
            let display = windows::core::BSTR::from(message);
            let activity = windows::core::BSTR::new();
            // SAFETY: `UiaRaiseNotificationEvent` takes interface/string
            // parameters only; both live for the duration of the call.
            #[allow(unsafe_code)]
            let result = unsafe {
                UiaRaiseNotificationEvent(
                    &provider,
                    NotificationKind_Other,
                    NotificationProcessing_ImportantAll,
                    &display,
                    &activity,
                )
            };
            result.map_err(|e| crate::A11yError::PlatformError(e.to_string()))
        }
    }

    // SAFETY: `UiaState` is only shared via `Arc`. All mutable state is
    // guarded by `Mutex`, and the cached COM interfaces are only ever
    // AddRef'd/Release'd (thread-safe interlocked ops) or handed back to
    // UIA, which invokes provider callbacks from arbitrary threads. There
    // is no interior unsynchronized mutation, so sharing the snapshot is
    // sound.
    #[allow(unsafe_code)]
    unsafe impl Send for UiaState {}
    #[allow(unsafe_code)]
    unsafe impl Sync for UiaState {}

    /// One COM object per semantic node. The tree root also acts as the
    /// fragment root.
    #[implement(
        IRawElementProviderSimple,
        IRawElementProviderFragment,
        IRawElementProviderFragmentRoot
    )]
    struct UiaNodeProvider {
        id: u64,
        state: Arc<UiaState>,
    }

    fn control_type_id(role: AccessibilityRole) -> UIA_CONTROLTYPE_ID {
        use AccessibilityRole::*;
        match role {
            Button => UIA_ButtonControlTypeId,
            Link => UIA_HyperlinkControlTypeId,
            Heading => UIA_TextControlTypeId,
            Text => UIA_TextControlTypeId,
            Image => UIA_ImageControlTypeId,
            TextField | TextArea => UIA_EditControlTypeId,
            CheckBox => UIA_CheckBoxControlTypeId,
            RadioButton => UIA_RadioButtonControlTypeId,
            Slider => UIA_SliderControlTypeId,
            ProgressBar => UIA_ProgressBarControlTypeId,
            ComboBox => UIA_ComboBoxControlTypeId,
            ListBox | List => UIA_ListControlTypeId,
            MenuItem => UIA_MenuItemControlTypeId,
            Menu => UIA_MenuControlTypeId,
            Tab => UIA_TabControlTypeId,
            TabPanel => UIA_TabItemControlTypeId,
            Document => UIA_DocumentControlTypeId,
            Group => UIA_GroupControlTypeId,
            Table => UIA_TableControlTypeId,
            ListItem | TreeItem => UIA_ListItemControlTypeId,
            Tree => UIA_TreeControlTypeId,
            Toolbar => UIA_ToolBarControlTypeId,
            StatusIndicator => UIA_StatusBarControlTypeId,
            Separator => UIA_SeparatorControlTypeId,
            Scrollbar => UIA_ScrollBarControlTypeId,
            Dialog | AlertDialog | Region | TableRow | TableCell | Generic => UIA_PaneControlTypeId,
        }
    }

    impl IRawElementProviderSimple_Impl for UiaNodeProvider_Impl {
        fn ProviderOptions(&self) -> windows::core::Result<ProviderOptions> {
            Ok(ProviderOptions_ServerSideProvider)
        }

        fn GetPatternProvider(
            &self,
            _patternid: UIA_PATTERN_ID,
        ) -> windows::core::Result<windows::core::IUnknown> {
            not_implemented()
        }

        #[allow(non_upper_case_globals)]
        fn GetPropertyValue(
            &self,
            propertyid: UIA_PROPERTY_ID,
        ) -> windows::core::Result<windows::core::VARIANT> {
            use windows::core::VARIANT;
            let id = self.state.effective_id(self.id);
            let node = self.state.node(id);
            let empty = VARIANT::default();
            let variant = match propertyid {
                UIA_NamePropertyId => node
                    .as_ref()
                    .map_or(empty, |n| VARIANT::from(n.label.as_str())),
                UIA_AutomationIdPropertyId => {
                    VARIANT::from(format!("tpt-chora-node-{id}").as_str())
                }
                UIA_ControlTypePropertyId => node
                    .as_ref()
                    .map_or(empty, |n| VARIANT::from(control_type_id(n.role).0)),
                UIA_IsEnabledPropertyId => node.as_ref().map_or(empty, |n| {
                    VARIANT::from(!n.state.contains(AccessibilityState::DISABLED))
                }),
                UIA_IsKeyboardFocusablePropertyId => node.as_ref().map_or(empty, |n| {
                    VARIANT::from(
                        !n.state.contains(AccessibilityState::DISABLED)
                            && !n.state.contains(AccessibilityState::HIDDEN),
                    )
                }),
                UIA_IsOffscreenPropertyId => node.as_ref().map_or(empty, |n| {
                    VARIANT::from(n.state.contains(AccessibilityState::HIDDEN))
                }),
                _ => empty,
            };
            Ok(variant)
        }

        fn HostRawElementProvider(&self) -> windows::core::Result<IRawElementProviderSimple> {
            self.state.host_provider()
        }
    }

    impl IRawElementProviderFragment_Impl for UiaNodeProvider_Impl {
        #[allow(non_upper_case_globals)]
        fn Navigate(
            &self,
            direction: NavigateDirection,
        ) -> windows::core::Result<IRawElementProviderFragment> {
            let id = self.state.effective_id(self.id);
            let Some(node) = self.state.node(id) else {
                return not_implemented();
            };
            let target = match direction {
                NavigateDirection_Parent => {
                    if self.state.is_root(self.id) {
                        None
                    } else {
                        self.state.parent_of(id)
                    }
                }
                NavigateDirection_FirstChild => node.children.first().copied(),
                NavigateDirection_LastChild => node.children.last().copied(),
                NavigateDirection_NextSibling => self
                    .state
                    .parent_of(id)
                    .and_then(|p| self.state.node(p))
                    .and_then(|parent| {
                        let children = parent.children;
                        children
                            .iter()
                            .position(|&c| c == id)
                            .and_then(|i| children.get(i + 1))
                            .copied()
                    }),
                NavigateDirection_PreviousSibling => self
                    .state
                    .parent_of(id)
                    .and_then(|p| self.state.node(p))
                    .and_then(|parent| {
                        let children = parent.children;
                        children
                            .iter()
                            .position(|&c| c == id)
                            .and_then(|i| i.checked_sub(1))
                            .and_then(|i| children.get(i))
                            .copied()
                    }),
                _ => None,
            };
            match target {
                Some(target_id) => self.state.fragment(target_id),
                None => not_implemented(),
            }
        }

        fn GetRuntimeId(&self) -> windows::core::Result<*mut SAFEARRAY> {
            let id = self.state.effective_id(self.id);
            // SAFETY: SafeArrayCreateVector allocates a fresh SAFEARRAY that
            // UIA takes ownership of; SafeArrayPutElement writes one i32 per
            // index into it. Failure paths destroy the array before returning.
            #[allow(unsafe_code)]
            unsafe {
                let array = SafeArrayCreateVector(VT_I4, 0, 2);
                if array.is_null() {
                    return Err(windows::core::Error::from_win32());
                }
                let base: i32 = 0x5443_4841; // 'TCHA' — unique per app
                let index0: i32 = 0;
                let mut ok = SafeArrayPutElement(array, &index0, (&base as *const i32).cast());
                if ok.is_ok() {
                    let index1: i32 = 1;
                    let id_val: i32 = (id & 0x7FFF_FFFF) as i32;
                    ok = SafeArrayPutElement(array, &index1, (&id_val as *const i32).cast());
                }
                if let Err(err) = ok {
                    let _ = SafeArrayDestroy(array);
                    return Err(err);
                }
                Ok(array)
            }
        }

        fn BoundingRectangle(&self) -> windows::core::Result<UiaRect> {
            let id = self.state.effective_id(self.id);
            let bounds = self.state.node(id).map_or([0.0; 4], |n| n.bounds);
            Ok(UiaRect {
                left: bounds[0] as f64,
                top: bounds[1] as f64,
                width: bounds[2] as f64,
                height: bounds[3] as f64,
            })
        }

        fn GetEmbeddedFragmentRoots(&self) -> windows::core::Result<*mut SAFEARRAY> {
            Ok(core::ptr::null_mut())
        }

        fn SetFocus(&self) -> windows::core::Result<()> {
            let id = self.state.effective_id(self.id);
            if let Ok(mut focused) = self.state.focused.lock() {
                *focused = Some(id);
            }
            if let Ok(provider) = self.state.simple(self.id) {
                // SAFETY: `UiaRaiseAutomationEvent` only consumes the
                // provider interface and a well-known event id constant.
                #[allow(unsafe_code)]
                let _ = unsafe {
                    UiaRaiseAutomationEvent(&provider, UIA_AutomationFocusChangedEventId)
                };
            }
            Ok(())
        }

        fn FragmentRoot(&self) -> windows::core::Result<IRawElementProviderFragmentRoot> {
            self.state.fragment_root()
        }
    }

    impl IRawElementProviderFragmentRoot_Impl for UiaNodeProvider_Impl {
        fn ElementProviderFromPoint(
            &self,
            x: f64,
            y: f64,
        ) -> windows::core::Result<IRawElementProviderFragment> {
            let root_id = self.state.root_id.lock().ok().and_then(|g| *g);
            let Some(root_id) = root_id else {
                return not_implemented();
            };
            let nodes = self
                .state
                .nodes
                .lock()
                .map_err(|_| windows::core::Error::from_win32())?;
            // Deepest node whose bounds contain the point (later in the
            // depth-first list is deeper).
            let mut hit: Option<u64> = None;
            for node in nodes.values() {
                let [bx, by, bw, bh] = node.bounds;
                if x >= bx as f64 && y >= by as f64 && x < (bx + bw) as f64 && y < (by + bh) as f64
                {
                    hit = Some(node.id);
                }
            }
            drop(nodes);
            match hit {
                Some(id) => self.state.fragment(id),
                None => self.state.fragment(root_id),
            }
        }

        fn GetFocus(&self) -> windows::core::Result<IRawElementProviderFragment> {
            let focused = self.state.focused.lock().ok().and_then(|g| *g);
            match focused {
                Some(id) => self.state.fragment(id),
                None => self.state.fragment(ROOT_PROVIDER_ID),
            }
        }
    }

    /// Payload stored in the message-only window's property list so the
    /// window procedure can answer `WM_GETOBJECT` without capturing state.
    struct RootProp {
        simple: IRawElementProviderSimple,
    }

    /// Creates the message-only window that hosts the fragment root and
    /// registers the provider for `WM_GETOBJECT`.
    fn create_provider_window() -> Result<HWND, crate::A11yError> {
        // SAFETY: `GetModuleHandleW` returns the current module's handle;
        // HMODULE and HINSTANCE are interchangeable.
        #[allow(unsafe_code)]
        let instance = unsafe { GetModuleHandleW(None) }
            .map(|m| HINSTANCE(m.0))
            .unwrap_or_default();

        // SAFETY: WNDCLASSW/RegisterClassW are stateless struct + call; the
        // window procedure is a free function valid for the process lifetime.
        #[allow(unsafe_code)]
        let class_atom = unsafe {
            RegisterClassW(&WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(uia_wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: Default::default(),
                hCursor: Default::default(),
                hbrBackground: Default::default(),
                lpszMenuName: windows::core::PCWSTR::null(),
                lpszClassName: WINDOW_CLASS,
            })
        };
        // A non-zero atom from a previous registration (another `A11yBridge`
        // instance) is fine; a zero with the class already present is too.
        let _ = class_atom;

        // SAFETY: `CreateWindowExW` for a message-only window (parent
        // HWND_MESSAGE) with no styles; the class and instance are valid.
        #[allow(unsafe_code)]
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                WINDOW_CLASS,
                w!("tpt-chora UIA provider"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                HMENU::default(),
                instance,
                None,
            )
        }
        .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
        Ok(hwnd)
    }

    /// Window procedure for the message-only UIA host window.
    ///
    /// SAFETY contract: this is a Win32 `WNDPROC`. `WM_GETOBJECT` hands the
    /// root provider to UIA via `UiaReturnRawElementProvider`; `WM_NCDESTROY`
    /// frees the `RootProp` box previously attached with `SetPropW`.
    #[allow(unsafe_code)]
    unsafe extern "system" fn uia_wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_GETOBJECT => {
                let object_id = (lparam.0 as u32) & 0xFFFF;
                if object_id == (UiaRootObjectId as u32) & 0xFFFF {
                    let handle = GetPropW(hwnd, ROOT_PROP);
                    if !handle.0.is_null() {
                        let prop = &*(handle.0 as *const RootProp);
                        return UiaReturnRawElementProvider(hwnd, wparam, lparam, &prop.simple);
                    }
                }
                LRESULT(0)
            }
            WM_NCDESTROY => {
                let handle = GetPropW(hwnd, ROOT_PROP);
                if !handle.0.is_null() {
                    let _ = RemovePropW(hwnd, ROOT_PROP);
                    drop(Box::from_raw(handle.0 as *mut RootProp));
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    pub struct WindowsUiaBackend {
        state: Arc<UiaState>,
        #[allow(dead_code)]
        hwnd: HWND,
    }

    impl WindowsUiaBackend {
        pub fn new() -> Result<Self, crate::A11yError> {
            // SAFETY: initializes COM for this thread (STA); matching
            // `CoUninitialize` is unnecessary because the backend lives for
            // the whole process lifetime in practice.
            #[allow(unsafe_code)]
            let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };

            let hwnd = create_provider_window()?;
            let state = UiaState::new(hwnd);

            let com: ComObject<UiaNodeProvider> = ComObject::new(UiaNodeProvider {
                id: ROOT_PROVIDER_ID,
                state: Arc::clone(&state),
            });
            let fragment: IRawElementProviderFragment = com.into_interface();
            let simple: IRawElementProviderSimple = fragment
                .cast()
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            *state.root_provider.lock().unwrap() = Some(fragment);

            let prop = Box::into_raw(Box::new(RootProp {
                simple: simple.clone(),
            }));
            // SAFETY: `SetPropW` stores an opaque HANDLE (our boxed pointer)
            // in the window property list; the box is freed on WM_NCDESTROY.
            #[allow(unsafe_code)]
            let set = unsafe { SetPropW(hwnd, ROOT_PROP, HANDLE(prop.cast())) };
            set.map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;

            Ok(Self { state, hwnd })
        }

        pub fn sync_tree(&mut self, nodes: &[BridgeNode]) -> Result<(), crate::A11yError> {
            let mut map = HashMap::with_capacity(nodes.len());
            let mut parents = HashMap::with_capacity(nodes.len());
            for node in nodes {
                for &child in &node.children {
                    parents.insert(child, node.id);
                }
                map.insert(node.id, node.clone());
            }
            let root_id = nodes
                .iter()
                .find(|n| !parents.contains_key(&n.id))
                .map(|n| n.id)
                .or_else(|| nodes.first().map(|n| n.id));

            if let Ok(mut lock) = self.state.nodes.lock() {
                *lock = map;
            }
            if let Ok(mut lock) = self.state.parents.lock() {
                *lock = parents;
            }
            if let Ok(mut lock) = self.state.root_id.lock() {
                *lock = root_id;
            }
            // Drop cached providers for nodes that no longer exist.
            if let (Ok(mut providers), Ok(nodes)) =
                (self.state.providers.lock(), self.state.nodes.lock())
            {
                providers.retain(|id, _| nodes.contains_key(id));
            }
            Ok(())
        }

        pub fn announce(&self, message: &str) -> Result<(), crate::A11yError> {
            self.state.announce(message)
        }

        pub fn set_focus(&self, node_id: u64) -> Result<(), crate::A11yError> {
            if let Ok(mut focused) = self.state.focused.lock() {
                *focused = Some(node_id);
            }
            match self.state.simple(node_id) {
                Ok(provider) => {
                    // SAFETY: `UiaRaiseAutomationEvent` consumes only the
                    // provider interface and a constant event id.
                    #[allow(unsafe_code)]
                    let result = unsafe {
                        UiaRaiseAutomationEvent(&provider, UIA_AutomationFocusChangedEventId)
                    };
                    result.map_err(|e| crate::A11yError::PlatformError(e.to_string()))
                }
                Err(_) => Ok(()),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn node(id: u64, label: &str, children: Vec<u64>) -> BridgeNode {
            BridgeNode {
                id,
                role: AccessibilityRole::Generic,
                label: label.to_string(),
                state: AccessibilityState::empty(),
                bounds: [0.0, 0.0, 100.0, 100.0],
                children,
            }
        }

        #[test]
        fn control_type_mapping_covers_all_roles() {
            for role in [
                AccessibilityRole::Button,
                AccessibilityRole::Link,
                AccessibilityRole::Heading,
                AccessibilityRole::Text,
                AccessibilityRole::Image,
                AccessibilityRole::TextField,
                AccessibilityRole::TextArea,
                AccessibilityRole::CheckBox,
                AccessibilityRole::RadioButton,
                AccessibilityRole::Slider,
                AccessibilityRole::ProgressBar,
                AccessibilityRole::ComboBox,
                AccessibilityRole::ListBox,
                AccessibilityRole::MenuItem,
                AccessibilityRole::Menu,
                AccessibilityRole::Tab,
                AccessibilityRole::TabPanel,
                AccessibilityRole::Dialog,
                AccessibilityRole::AlertDialog,
                AccessibilityRole::Document,
                AccessibilityRole::Group,
                AccessibilityRole::Region,
                AccessibilityRole::Table,
                AccessibilityRole::TableRow,
                AccessibilityRole::TableCell,
                AccessibilityRole::List,
                AccessibilityRole::ListItem,
                AccessibilityRole::Tree,
                AccessibilityRole::TreeItem,
                AccessibilityRole::Toolbar,
                AccessibilityRole::StatusIndicator,
                AccessibilityRole::Separator,
                AccessibilityRole::Scrollbar,
                AccessibilityRole::Generic,
            ] {
                let _ = control_type_id(role);
            }
        }

        #[test]
        fn navigate_finds_parent_and_siblings_from_tree() {
            let mut backend = WindowsUiaBackend::new().expect("create backend");
            backend
                .sync_tree(&[
                    node(1, "root", vec![2, 3]),
                    node(2, "a", vec![]),
                    node(3, "b", vec![]),
                ])
                .expect("sync");
            let state = &backend.state;
            assert_eq!(*state.root_id.lock().unwrap(), Some(1));
            assert_eq!(state.parent_of(2), Some(1));
            assert_eq!(state.parent_of(3), Some(1));
            assert!(state.parent_of(1).is_none());
            let two = state.node(2).expect("node 2");
            assert_eq!(two.label, "a");
        }

        #[test]
        fn effective_id_resolves_root_sentinel() {
            let mut backend = WindowsUiaBackend::new().expect("create backend");
            backend.sync_tree(&[node(7, "root", vec![])]).expect("sync");
            let state = &backend.state;
            assert_eq!(state.effective_id(ROOT_PROVIDER_ID), 7);
            assert_eq!(state.effective_id(9), 9);
        }

        #[test]
        fn fragment_instances_are_stable_and_cast_to_simple() {
            let mut backend = WindowsUiaBackend::new().expect("create backend");
            backend
                .sync_tree(&[node(1, "root", vec![2]), node(2, "child", vec![])])
                .expect("sync");
            let state = &backend.state;
            let a = state.fragment(2).expect("fragment");
            let b = state.fragment(2).expect("fragment again");
            assert_eq!(a.as_raw(), b.as_raw(), "same node must be the same object");
            assert!(state.simple(2).is_ok());
        }

        #[test]
        fn sync_tree_drops_stale_providers() {
            let mut backend = WindowsUiaBackend::new().expect("create backend");
            backend
                .sync_tree(&[node(1, "root", vec![2]), node(2, "gone", vec![])])
                .expect("sync");
            let state = Arc::clone(&backend.state);
            let _ = state.fragment(2).expect("fragment for node 2");
            assert!(state.providers.lock().unwrap().get(&2).is_some());
            backend
                .sync_tree(&[node(1, "root", vec![])])
                .expect("sync again");
            assert!(state.providers.lock().unwrap().get(&2).is_none());
        }
    }
}

#[cfg(all(feature = "native-a11y-backends", target_os = "macos"))]
mod macos_nsaccessibility {
    use objc2::rc::Retained;
    use objc2_app_kit::{NSAccessibility, NSAccessibilityRole, NSAccessibilitySubrole, NSView};
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
    use std::collections::HashMap;

    pub struct MacOsNsAccessibilityBackend {
        view_cache: HashMap<u64, Retained<NSView>>,
    }

    impl MacOsNsAccessibilityBackend {
        pub fn new() -> Self {
            Self {
                view_cache: HashMap::new(),
            }
        }

        pub fn sync_tree(&mut self, nodes: &[BridgeNode]) -> Result<(), crate::A11yError> {
            let valid_ids: std::collections::HashSet<u64> = nodes.iter().map(|n| n.id).collect();
            self.view_cache.retain(|k, _| valid_ids.contains(k));

            for node in nodes {
                self.create_or_update_view(node)?;
            }
            Ok(())
        }

        fn create_or_update_view(&mut self, node: &BridgeNode) -> Result<(), crate::A11yError> {
            let view = self.view_cache.entry(node.id).or_insert_with(|| {
                // In a real implementation, this would create a custom NSView subclass
                // that implements the NSAccessibility protocol. For now, we create
                // a basic view as a placeholder.
                unsafe { NSView::new() }
            });

            self.set_accessibility_properties(view, node)?;
            Ok(())
        }

        fn set_accessibility_properties(
            &self,
            view: &NSView,
            node: &BridgeNode,
        ) -> Result<(), crate::A11yError> {
            unsafe {
                // Set role
                let role = role_to_nsaccessibility_role(node.role);
                view.setAccessibilityRole(&role);

                // Set label
                let label = NSString::from_str(&node.label);
                view.setAccessibilityLabel(&label);

                // Set value if present
                if let Some(value) = &node.value {
                    let value_str = NSString::from_str(value);
                    view.setAccessibilityValue(&value_str);
                }

                // Set frame
                if let Some(bounds) = node.bounds {
                    let frame = NSRect::new(
                        NSPoint::new(bounds.x, bounds.y),
                        NSSize::new(bounds.width, bounds.height),
                    );
                    view.setAccessibilityFrame(frame);
                }

                // Set enabled state
                let enabled = node
                    .states
                    .contains(&crate::semantic::AccessibilityState::Enabled);
                view.setAccessibilityEnabled(enabled);

                // Set focused state
                let focused = node
                    .states
                    .contains(&crate::semantic::AccessibilityState::Focused);
                view.setAccessibilityFocused(focused);
            }
            Ok(())
        }

        pub fn announce(&self, message: &str) -> Result<(), crate::A11yError> {
            let announcement = NSString::from_str(message);
            unsafe {
                NSAccessibility::postNotification(
                    nil,
                    NSAccessibility::AnnouncementRequestedNotification(),
                    &announcement,
                );
            }
            Ok(())
        }

        pub fn set_focus(&self, node_id: u64) -> Result<(), crate::A11yError> {
            if let Some(view) = self.view_cache.get(&node_id) {
                unsafe {
                    NSAccessibility::postNotification(
                        view.as_ref(),
                        NSAccessibility::FocusedUIElementChangedNotification(),
                        view.as_ref(),
                    );
                }
            }
            Ok(())
        }
    }

    fn role_to_nsaccessibility_role(
        role: crate::semantic::AccessibilityRole,
    ) -> Retained<NSString> {
        use crate::semantic::AccessibilityRole::*;
        let role_str = match role {
            Button => "AXButton",
            CheckBox => "AXCheckBox",
            ComboBox => "AXComboBox",
            Edit => "AXTextField",
            Hyperlink => "AXLink",
            Image => "AXImage",
            ListItem => "AXRow",
            List => "AXList",
            Menu => "AXMenu",
            MenuBar => "AXMenuBar",
            MenuItem => "AXMenuItem",
            Pane => "AXGroup",
            ProgressBar => "AXProgressIndicator",
            RadioButton => "AXRadioButton",
            ScrollBar => "AXScrollBar",
            Slider => "AXSlider",
            Spinner => "AXIncrementor",
            StatusBar => "AXStatusBar",
            Tab => "AXTabGroup",
            TabItem => "AXTab",
            Table => "AXTable",
            Text => "AXStaticText",
            ToolBar => "AXToolbar",
            ToolTip => "AXHelpTag",
            Tree => "AXOutline",
            TreeItem => "AXOutlineRow",
            Window => "AXWindow",
            Custom => "AXGroup",
            Group => "AXGroup",
            Header => "AXGroup",
            HeaderItem => "AXGroup",
            Link => "AXLink",
            Separator => "AXSplitGroup",
            Thumb => "AXScrollBar",
            TitleBar => "AXTitleBar",
            ScrollViewer => "AXScrollArea",
            SemanticZoom => "AXGroup",
            AppBar => "AXGroup",
            Calendar => "AXGroup",
            DataGrid => "AXTable",
            DataItem => "AXRow",
            Document => "AXDocument",
            Flyout => "AXPopover",
            Grid => "AXTable",
            GridItem => "AXCell",
            Label => "AXStaticText",
            _ => "AXGroup",
        };
        unsafe { NSString::from_str(role_str) }
    }
}

#[cfg(all(feature = "native-a11y-backends", target_os = "android"))]
mod android_node_info {
    use jni::objects::{JClass, JObject, JString};
    use jni::sys::{jboolean, jint, jlong};
    use jni::JNIEnv;
    use std::collections::HashMap;

    pub struct AndroidNodeInfoBackend {
        node_cache: HashMap<u64, jlong>, // Stores AccessibilityNodeInfoCompat handles
        accessibility_manager: jni::objects::GlobalRef,
    }

    impl AndroidNodeInfoBackend {
        pub fn new(env: &JNIEnv, context: JObject) -> Result<Self, crate::A11yError> {
            let am_class = env
                .find_class("android/view/accessibility/AccessibilityManager")
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            let get_service = env
                .get_static_method_id(
                    am_class,
                    "getInstance",
                    "(Landroid/content/Context;)Landroid/view/accessibility/AccessibilityManager;",
                )
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            let manager = env
                .call_static_method(am_class, get_service, &[context.into()])
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?
                .l()
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;

            let global_manager = env
                .new_global_ref(manager)
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;

            Ok(Self {
                node_cache: HashMap::new(),
                accessibility_manager: global_manager,
            })
        }

        pub fn sync_tree(
            &mut self,
            env: &JNIEnv,
            nodes: &[BridgeNode],
        ) -> Result<(), crate::A11yError> {
            let valid_ids: std::collections::HashSet<u64> = nodes.iter().map(|n| n.id).collect();
            self.node_cache.retain(|k, _| valid_ids.contains(k));

            for node in nodes {
                self.create_or_update_node(env, node)?;
            }
            Ok(())
        }

        fn create_or_update_node(
            &mut self,
            env: &JNIEnv,
            node: &BridgeNode,
        ) -> Result<(), crate::A11yError> {
            let node_info = if let Some(&handle) = self.node_cache.get(&node.id) {
                // Reuse existing node
                JObject::from_raw(handle as _)
            } else {
                // Create new AccessibilityNodeInfoCompat
                let compat_class = env
                    .find_class("androidx/core/view/accessibility/AccessibilityNodeInfoCompat")
                    .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
                let constructor = env
                    .get_method_id(compat_class, "<init>", "()V")
                    .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
                let new_node = env
                    .new_object(compat_class, constructor, &[])
                    .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
                let global_ref = env
                    .new_global_ref(new_node)
                    .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
                self.node_cache
                    .insert(node.id, global_ref.as_raw() as jlong);
                global_ref
            };

            self.set_node_properties(env, &node_info, node)?;
            Ok(())
        }

        fn set_node_properties(
            &self,
            env: &JNIEnv,
            node_info: &JObject,
            node: &BridgeNode,
        ) -> Result<(), crate::A11yError> {
            let compat_class = env
                .find_class("androidx/core/view/accessibility/AccessibilityNodeInfoCompat")
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;

            // Set class name (role)
            let class_name = role_to_android_class_name(node.role);
            let j_class_name = env
                .new_string(class_name)
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            let set_class_name = env
                .get_method_id(compat_class, "setClassName", "(Ljava/lang/CharSequence;)V")
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            env.call_method(node_info, set_class_name, &[j_class_name.into()])
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;

            // Set content description (label)
            let j_label = env
                .new_string(&node.label)
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            let set_content_desc = env
                .get_method_id(
                    compat_class,
                    "setContentDescription",
                    "(Ljava/lang/CharSequence;)V",
                )
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            env.call_method(node_info, set_content_desc, &[j_label.into()])
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;

            // Set bounds
            if let Some(bounds) = node.bounds {
                let set_bounds = env
                    .get_method_id(
                        compat_class,
                        "setBoundsInScreen",
                        "(Landroid/graphics/Rect;)V",
                    )
                    .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
                let rect_class = env
                    .find_class("android/graphics/Rect")
                    .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
                let rect_constructor = env
                    .get_method_id(rect_class, "<init>", "(IIII)V")
                    .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
                let rect = env
                    .new_object(
                        rect_class,
                        rect_constructor,
                        &[
                            (bounds.x as jint).into(),
                            (bounds.y as jint).into(),
                            ((bounds.x + bounds.width) as jint).into(),
                            ((bounds.y + bounds.height) as jint).into(),
                        ],
                    )
                    .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
                env.call_method(node_info, set_bounds, &[rect.into()])
                    .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            }

            // Set enabled state
            let enabled = node
                .states
                .contains(&crate::semantic::AccessibilityState::Enabled);
            let set_enabled = env
                .get_method_id(compat_class, "setEnabled", "(Z)V")
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            env.call_method(node_info, set_enabled, &[enabled.into()])
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;

            // Set focusable
            let focusable = node
                .states
                .contains(&crate::semantic::AccessibilityState::Focusable);
            let set_focusable = env
                .get_method_id(compat_class, "setFocusable", "(Z)V")
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            env.call_method(node_info, set_focusable, &[focusable.into()])
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;

            Ok(())
        }

        pub fn announce(&self, env: &JNIEnv, message: &str) -> Result<(), crate::A11yError> {
            let event_class = env
                .find_class("android/view/accessibility/AccessibilityEvent")
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            let obtain = env
                .get_static_method_id(
                    event_class,
                    "obtain",
                    "(I)Landroid/view/accessibility/AccessibilityEvent;",
                )
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            let event = env
                .call_static_method(event_class, obtain, &[16.into()]) // TYPE_ANNOUNCEMENT = 16
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?
                .l()
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;

            let j_message = env
                .new_string(message)
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            let get_text = env
                .get_method_id(event_class, "getText", "()Ljava/util/List;")
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            let text_list = env
                .call_method(&event, get_text, &[])
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?
                .l()
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            let list_class = env
                .find_class("java/util/List")
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            let add = env
                .get_method_id(list_class, "add", "(Ljava/lang/Object;)Z")
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            env.call_method(text_list, add, &[j_message.into()])
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;

            let send_event = env
                .get_method_id(
                    env.get_object_class(self.accessibility_manager.as_obj())?,
                    "sendAccessibilityEvent",
                    "(Landroid/view/accessibility/AccessibilityEvent;)V",
                )
                .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            env.call_method(
                self.accessibility_manager.as_obj(),
                send_event,
                &[event.into()],
            )
            .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;

            Ok(())
        }

        pub fn set_focus(&self, env: &JNIEnv, node_id: u64) -> Result<(), crate::A11yError> {
            if let Some(&handle) = self.node_cache.get(&node_id) {
                let node_info = JObject::from_raw(handle as _);
                let compat_class = env
                    .find_class("androidx/core/view/accessibility/AccessibilityNodeInfoCompat")
                    .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
                let perform_action = env
                    .get_method_id(compat_class, "performAction", "(I)Z")
                    .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
                // ACTION_ACCESSIBILITY_FOCUS = 64
                env.call_method(node_info, perform_action, &[64.into()])
                    .map_err(|e| crate::A11yError::PlatformError(e.to_string()))?;
            }
            Ok(())
        }
    }

    fn role_to_android_class_name(role: crate::semantic::AccessibilityRole) -> &'static str {
        use crate::semantic::AccessibilityRole::*;
        match role {
            Button => "android.widget.Button",
            CheckBox => "android.widget.CheckBox",
            ComboBox => "android.widget.Spinner",
            Edit => "android.widget.EditText",
            Hyperlink => "android.widget.TextView",
            Image => "android.widget.ImageView",
            ListItem => "android.widget.TextView",
            List => "android.widget.ListView",
            Menu => "android.view.Menu",
            MenuBar => "android.view.Menu",
            MenuItem => "android.view.MenuItem",
            Pane => "android.view.ViewGroup",
            ProgressBar => "android.widget.ProgressBar",
            RadioButton => "android.widget.RadioButton",
            ScrollBar => "android.widget.ScrollBar",
            Slider => "android.widget.SeekBar",
            Spinner => "android.widget.Spinner",
            StatusBar => "android.widget.TextView",
            Tab => "android.widget.TabHost",
            TabItem => "android.widget.TextView",
            Table => "android.widget.GridView",
            Text => "android.widget.TextView",
            ToolBar => "android.widget.Toolbar",
            ToolTip => "android.widget.TextView",
            Tree => "android.widget.ExpandableListView",
            TreeItem => "android.widget.TextView",
            Window => "android.view.View",
            Custom => "android.view.View",
            Group => "android.view.ViewGroup",
            Header => "android.view.ViewGroup",
            HeaderItem => "android.widget.TextView",
            Link => "android.widget.TextView",
            Separator => "android.view.View",
            Thumb => "android.view.View",
            TitleBar => "android.view.ViewGroup",
            ScrollViewer => "android.widget.ScrollView",
            SemanticZoom => "android.view.ViewGroup",
            AppBar => "android.widget.Toolbar",
            Calendar => "android.widget.CalendarView",
            DataGrid => "android.widget.GridView",
            DataItem => "android.widget.TextView",
            Document => "android.view.View",
            Flyout => "android.widget.PopupWindow",
            Grid => "android.widget.GridView",
            GridItem => "android.widget.TextView",
            Label => "android.widget.TextView",
            _ => "android.view.View",
        }
    }
}

enum OsBackend {
    #[cfg(all(feature = "native-a11y-backends", target_os = "windows"))]
    WindowsUiAutomation(windows_uia::WindowsUiaBackend),
    #[cfg(all(feature = "native-a11y-backends", target_os = "macos"))]
    MacOsNsAccessibility(macos_nsaccessibility::MacOsNsAccessibilityBackend),
    #[cfg(all(feature = "native-a11y-backends", target_os = "android"))]
    AndroidAccessibilityNodeInfo(android_node_info::AndroidNodeInfoBackend),
    InMemory,
}

#[derive(Debug, Clone)]
pub enum A11yBridgeEvent {
    FocusChanged(u64),
    ValueChanged(u64, String),
    StateChanged(u64),
    Announcement(String),
}

#[derive(Debug, Clone)]
pub struct A11yTreeUpdate {
    pub nodes: Vec<BridgeNode>,
    pub focused_node: Option<u64>,
}

impl A11yBridge {
    pub fn new() -> Self {
        let os_backend = if cfg!(all(feature = "native-a11y-backends", target_os = "windows")) {
            #[cfg(all(feature = "native-a11y-backends", target_os = "windows"))]
            {
                match windows_uia::WindowsUiaBackend::new() {
                    Ok(backend) => OsBackend::WindowsUiAutomation(backend),
                    Err(_) => OsBackend::InMemory,
                }
            }
            #[cfg(not(all(feature = "native-a11y-backends", target_os = "windows")))]
            OsBackend::InMemory
        } else if cfg!(all(feature = "native-a11y-backends", target_os = "macos")) {
            #[cfg(all(feature = "native-a11y-backends", target_os = "macos"))]
            {
                OsBackend::MacOsNsAccessibility(
                    macos_nsaccessibility::MacOsNsAccessibilityBackend::new(),
                )
            }
            #[cfg(not(all(feature = "native-a11y-backends", target_os = "macos")))]
            OsBackend::InMemory
        } else if cfg!(all(feature = "native-a11y-backends", target_os = "android")) {
            #[cfg(all(feature = "native-a11y-backends", target_os = "android"))]
            {
                // Android backend requires JNIEnv and Context, which aren't available here.
                // The Android backend must be initialized separately via `init_android`.
                OsBackend::InMemory
            }
            #[cfg(not(all(feature = "native-a11y-backends", target_os = "android")))]
            OsBackend::InMemory
        } else {
            OsBackend::InMemory
        };
        Self {
            last_update: None,
            announcements: Vec::new(),
            focused_node: None,
            os_backend,
        }
    }

    #[cfg(all(feature = "native-a11y-backends", target_os = "android"))]
    pub fn init_android(
        &mut self,
        env: &jni::JNIEnv,
        context: jni::objects::JObject,
    ) -> Result<(), crate::A11yError> {
        let backend = android_node_info::AndroidNodeInfoBackend::new(env, context)?;
        self.os_backend = OsBackend::AndroidAccessibilityNodeInfo(backend);
        Ok(())
    }

    pub fn update_tree(&mut self, ir: &SemanticIR) -> Result<(), crate::A11yError> {
        let nodes = ir.serialize_for_bridge();
        let focused = ir.root().and_then(|r| r.children.first().map(|&id| id.0));
        let update = A11yTreeUpdate {
            nodes,
            focused_node: focused,
        };
        self.push_update(&update)
    }

    pub fn push_update(&mut self, update: &A11yTreeUpdate) -> Result<(), crate::A11yError> {
        if let Some(focused_id) = update.focused_node {
            if !update.nodes.iter().any(|n| n.id == focused_id) {
                return Err(crate::A11yError::NodeNotFound(focused_id));
            }
        }

        self.focused_node = update.focused_node;
        self.last_update = Some(update.clone());

        match &mut self.os_backend {
            #[cfg(all(feature = "native-a11y-backends", target_os = "windows"))]
            OsBackend::WindowsUiAutomation(backend) => {
                backend.sync_tree(&update.nodes)?;
            }
            #[cfg(all(feature = "native-a11y-backends", target_os = "macos"))]
            OsBackend::MacOsNsAccessibility(backend) => {
                backend.sync_tree(&update.nodes)?;
            }
            #[cfg(all(feature = "native-a11y-backends", target_os = "android"))]
            OsBackend::AndroidAccessibilityNodeInfo(backend) => {
                // Android sync requires JNIEnv, which isn't available here.
                // The Android backend must be synced via a separate JNI call.
            }
            OsBackend::InMemory => {}
        }

        Ok(())
    }

    pub fn announce(&mut self, message: &str) -> Result<(), crate::A11yError> {
        self.announcements.push(message.to_string());
        match &mut self.os_backend {
            #[cfg(all(feature = "native-a11y-backends", target_os = "windows"))]
            OsBackend::WindowsUiAutomation(backend) => {
                backend.announce(message)?;
            }
            #[cfg(all(feature = "native-a11y-backends", target_os = "macos"))]
            OsBackend::MacOsNsAccessibility(backend) => {
                backend.announce(message)?;
            }
            #[cfg(all(feature = "native-a11y-backends", target_os = "android"))]
            OsBackend::AndroidAccessibilityNodeInfo(backend) => {
                // Android announce requires JNIEnv
            }
            OsBackend::InMemory => {}
        }
        Ok(())
    }

    pub fn set_focus(&mut self, node_id: u64) -> Result<(), crate::A11yError> {
        if let Some(ref update) = self.last_update {
            if !update.nodes.iter().any(|n| n.id == node_id) {
                return Err(crate::A11yError::NodeNotFound(node_id));
            }
        }
        self.focused_node = Some(node_id);
        match &mut self.os_backend {
            #[cfg(all(feature = "native-a11y-backends", target_os = "windows"))]
            OsBackend::WindowsUiAutomation(backend) => {
                backend.set_focus(node_id)?;
            }
            #[cfg(all(feature = "native-a11y-backends", target_os = "macos"))]
            OsBackend::MacOsNsAccessibility(backend) => {
                backend.set_focus(node_id)?;
            }
            #[cfg(all(feature = "native-a11y-backends", target_os = "android"))]
            OsBackend::AndroidAccessibilityNodeInfo(backend) => {
                // Android set_focus requires JNIEnv
            }
            OsBackend::InMemory => {}
        }
        Ok(())
    }

    pub fn last_update(&self) -> Option<&A11yTreeUpdate> {
        self.last_update.as_ref()
    }

    pub fn announcements(&self) -> &[String] {
        &self.announcements
    }

    pub fn focused_node(&self) -> Option<u64> {
        self.focused_node
    }

    pub fn drain_announcements(&mut self) -> Vec<String> {
        std::mem::take(&mut self.announcements)
    }
}

impl Default for A11yBridge {
    fn default() -> Self {
        Self::new()
    }
}
