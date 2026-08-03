use crate::semantic::{BridgeNode, SemanticIR};

pub struct A11yBridge {
    last_update: Option<A11yTreeUpdate>,
    announcements: Vec<String>,
    focused_node: Option<u64>,
    os_backend: OsBackend,
}

#[cfg(all(feature = "native-a11y-backends", target_os = "windows"))]
mod windows_uia {
    use std::collections::HashMap;
    use crate::semantic::BridgeNode;

    // Stub implementation for Windows UIA - compiles but doesn't connect to real UIA
    // A full implementation would require proper Windows FFI bindings and testing on Windows
    pub struct WindowsUiaBackend {
        element_cache: HashMap<u64, ()>,
    }

    impl WindowsUiaBackend {
        pub fn new() -> Result<Self, crate::A11yError> {
            // In a real implementation, this would initialize UIA via CoCreateInstance
            // For now, we return a stub that compiles but doesn't connect to real UIA
            Ok(Self {
                element_cache: HashMap::new(),
            })
        }

        pub fn sync_tree(&mut self, nodes: &[BridgeNode]) -> Result<(), crate::A11yError> {
            // Clear stale cache entries
            let valid_ids: std::collections::HashSet<u64> = nodes.iter().map(|n| n.id).collect();
            self.element_cache.retain(|k, _| valid_ids.contains(k));

            // In a real implementation, this would create/update UIA elements
            // For now, we just track the node IDs
            for node in nodes {
                self.element_cache.insert(node.id, ());
            }
            Ok(())
        }

        pub fn announce(&self, message: &str) -> Result<(), crate::A11yError> {
            // UIA announcements typically done via live regions
            eprintln!("UIA Announcement (stub): {}", message);
            Ok(())
        }

        pub fn set_focus(&self, _node_id: u64) -> Result<(), crate::A11yError> {
            // In a real implementation, this would call SetFocus on the UIA element
            Ok(())
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
