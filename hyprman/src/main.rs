use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::{env, thread};
use std::error::Error;
use serde::{Serialize, Deserialize};

/// Represents a Hyprland event emitted from a UnixStream.
/// Each variant corresponds to a specific event type with its associated data.
/// The enum is annotated for JSON (de)serialization with serde.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
enum HyprlandEvent {
    /// Emitted on workspace change.
    /// Triggered ONLY when a user explicitly requests a workspace change (not due to mouse movements).
    /// Data: workspace_name.
    Workspace {
        workspace_name: String,
    },
    /// Emitted on workspace change (v2).
    /// Triggered ONLY when a user explicitly requests a workspace change.
    /// Data: workspace_id, workspace_name.
    WorkspaceV2 {
        workspace_id: u8,
        workspace_name: String,
    },
    /// Emitted when the active monitor changes.
    /// Data: monitor_name, workspace_name.
    FocusedMon {
        monitor_name: String,
        workspace_name: String,
    },
    /// Emitted when the active monitor changes (v2).
    /// Data: monitor_name, workspace_id.
    FocusedMonV2 {
        monitor_name: String,
        workspace_id: u8,
    },
    /// Emitted on active window change.
    /// Data: window_class, window_title.
    ActiveWindow {
        window_class: String,
        window_title: String,
    },
    /// Emitted on active window change (v2).
    /// Data: window_address.
    ActiveWindowV2 {
        window_address: String,
    },
    /// Emitted when a window enters or exits fullscreen mode.
    /// Data: status (0 for exit fullscreen, 1 for enter fullscreen).
    Fullscreen {
        status: u8,
    },
    /// Emitted when a monitor is removed (disconnected).
    /// Data: monitor_name.
    MonitorRemoved {
        monitor_name: String,
    },
    /// Emitted when a monitor is added (connected).
    /// Data: monitor_name.
    MonitorAdded {
        monitor_name: String,
    },
    /// Emitted when a monitor is added (connected) (v2).
    /// Data: monitor_id, monitor_name, monitor_description.
    MonitorAddedV2 {
        monitor_id: u8,
        monitor_name: String,
        monitor_description: String,
    },
    /// Emitted when a workspace is created.
    /// Data: workspace_name.
    CreateWorkspace {
        workspace_name: String,
    },
    /// Emitted when a workspace is created (v2).
    /// Data: workspace_id, workspace_name.
    CreateWorkspaceV2 {
        workspace_id: u8,
        workspace_name: String,
    },
    /// Emitted when a workspace is destroyed.
    /// Data: workspace_name.
    DestroyWorkspace {
        workspace_name: String,
    },
    /// Emitted when a workspace is destroyed (v2).
    /// Data: workspace_id, workspace_name.
    DestroyWorkspaceV2 {
        workspace_id: u8,
        workspace_name: String,
    },
    /// Emitted when a workspace is moved to a different monitor.
    /// Data: workspace_name, monitor_name.
    MoveWorkspace {
        workspace_name: String,
        monitor_name: String,
    },
    /// Emitted when a workspace is moved to a different monitor (v2).
    /// Data: workspace_id, workspace_name, monitor_name.
    MoveWorkspaceV2 {
        workspace_id: u8,
        workspace_name: String,
        monitor_name: String,
    },
    /// Emitted when a workspace is renamed.
    /// Data: workspace_id, new_name.
    RenameWorkspace {
        workspace_id: u8,
        new_name: String,
    },
    /// Emitted when the special workspace opened in a monitor changes.
    /// Data: workspace_name, monitor_name.
    ActiveSpecial {
        workspace_name: String,
        monitor_name: String,
    },
    /// Emitted when the layout of the active keyboard changes.
    /// Data: keyboard_name, layout_name.
    ActiveLayout {
        keyboard_name: String,
        layout_name: String,
    },
    /// Emitted when a window is opened.
    /// Data: window_address, workspace_name, window_class, window_title.
    OpenWindow {
        window_address: String,
        workspace_name: String,
        window_class: String,
        window_title: String,
    },
    /// Emitted when a window is closed.
    /// Data: window_address.
    CloseWindow {
        window_address: String,
    },
    /// Emitted when a window is moved to a workspace.
    /// Data: window_address, workspace_name.
    MoveWindow {
        window_address: String,
        workspace_name: String,
    },
    /// Emitted when a window is moved to a workspace (v2).
    /// Data: window_address, workspace_id, workspace_name.
    MoveWindowV2 {
        window_address: String,
        workspace_id: u8,
        workspace_name: String,
    },
    /// Emitted when a layerSurface is mapped.
    /// Data: namespace.
    OpenLayer {
        namespace: String,
    },
    /// Emitted when a layerSurface is unmapped.
    /// Data: namespace.
    CloseLayer {
        namespace: String,
    },
    /// Emitted when a keybind submap changes.
    /// Data: submap_name.
    Submap {
        submap_name: String,
    },
    /// Emitted when a window changes its floating mode.
    /// Data: window_address, floating (0 for non‑floating, 1 for floating).
    ChangeFloatingMode {
        window_address: String,
        floating: u8,
    },
    /// Emitted when a window requests an urgent state.
    /// Data: window_address.
    Urgent {
        window_address: String,
    },
    /// Emitted when a screencast state changes.
    /// Data: state (0 or 1), owner (0 for monitor share, 1 for window share).
    Screencast {
        state: u8,
        owner: u8,
    },
    /// Emitted when a window title changes.
    /// Data: window_address.
    WindowTitle {
        window_address: String,
    },
    /// Emitted when a window title changes (v2).
    /// Data: window_address, window_title.
    WindowTitleV2 {
        window_address: String,
        window_title: String,
    },
    /// Emitted when the togglegroup command is used.
    /// Data: toggle_status (0 means group destroyed, 1 means group exists),
    /// and window_addresses (one or more window addresses).
    ToggleGroup {
        toggle_status: u8,
        window_addresses: Vec<String>,
    },
    /// Emitted when a window is merged into a group.
    /// Data: window_address.
    MoveIntoGroup {
        window_address: String,
    },
    /// Emitted when a window is removed from a group.
    /// Data: window_address.
    MoveOutOfGroup {
        window_address: String,
    },
    /// Emitted when ignoregrouplock is toggled.
    /// Data: value (0 or 1).
    IgnoreGroupLock {
        value: u8,
    },
    /// Emitted when lockgroups is toggled.
    /// Data: value (0 or 1).
    LockGroups {
        value: u8,
    },
    /// Emitted when the configuration is done reloading.
    /// No data.
    ConfigReloaded,
    /// Emitted when a window is pinned or unpinned.
    /// Data: window_address, pin_state (0 or 1).
    Pin {
        window_address: String,
        pin_state: u8,
    },
}

/// Parses a single event line from the Hyprland stream into a `HyprlandEvent`.
///
/// The expected format is:
///
///     EVENT>>DATA\n
///
/// where DATA may be a single value or a comma‑separated list of fields.
/// For example:
/// - `"workspace>>Development"`
/// - `"workspacev2>>2,Development"`
///
/// Returns an error if the event type is unknown or if required fields are missing/cannot be parsed.
fn parse_event_line(line: &str) -> Result<HyprlandEvent, Box<dyn Error>> {
    // Trim whitespace and any newline characters.
    let line = line.trim();
    // Split into event name and data using ">>" as delimiter.
    let mut parts = line.split(">>");
    let event_name = parts.next().ok_or("Missing event name")?;
    let data = parts.next().unwrap_or("").trim();

    match event_name {
        "workspace" => {
            // Data: WORKSPACENAME
            Ok(HyprlandEvent::Workspace {
                workspace_name: data.to_string(),
            })
        }
        "workspacev2" => {
            // Data: WORKSPACEID,WORKSPACENAME
            let mut fields = data.split(',');
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            Ok(HyprlandEvent::WorkspaceV2 { workspace_id, workspace_name })
        }
        "focusedmon" => {
            // Data: MONITORNAME,WORKSPACENAME
            let mut fields = data.split(',');
            let monitor_name = fields.next().ok_or("Missing monitor_name")?.to_string();
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            Ok(HyprlandEvent::FocusedMon { monitor_name, workspace_name })
        }
        "focusedmonv2" => {
            // Data: MONITORNAME,WORKSPACEID
            let mut fields = data.split(',');
            let monitor_name = fields.next().ok_or("Missing monitor_name")?.to_string();
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            Ok(HyprlandEvent::FocusedMonV2 { monitor_name, workspace_id })
        }
        "activewindow" => {
            // Data: WINDOWCLASS,WINDOWTITLE
            let mut fields = data.split(',');
            let window_class = fields.next().ok_or("Missing window_class")?.to_string();
            let window_title = fields.next().ok_or("Missing window_title")?.to_string();
            Ok(HyprlandEvent::ActiveWindow { window_class, window_title })
        }
        "activewindowv2" => {
            // Data: WINDOWADDRESS
            Ok(HyprlandEvent::ActiveWindowV2 { window_address: data.to_string() })
        }
        "fullscreen" => {
            // Data: 0/1
            let status = data.parse::<u8>()?;
            Ok(HyprlandEvent::Fullscreen { status })
        }
        "monitorremoved" => {
            // Data: MONITORNAME
            Ok(HyprlandEvent::MonitorRemoved { monitor_name: data.to_string() })
        }
        "monitoradded" => {
            // Data: MONITORNAME
            Ok(HyprlandEvent::MonitorAdded { monitor_name: data.to_string() })
        }
        "monitoraddedv2" => {
            // Data: MONITORID,MONITORNAME,MONITORDESCRIPTION
            let mut fields = data.split(',');
            let monitor_id = fields.next().ok_or("Missing monitor_id")?.parse::<u8>()?;
            let monitor_name = fields.next().ok_or("Missing monitor_name")?.to_string();
            let monitor_description = fields.next().ok_or("Missing monitor_description")?.to_string();
            Ok(HyprlandEvent::MonitorAddedV2 { monitor_id, monitor_name, monitor_description })
        }
        "createworkspace" => {
            // Data: WORKSPACENAME
            Ok(HyprlandEvent::CreateWorkspace { workspace_name: data.to_string() })
        }
        "createworkspacev2" => {
            // Data: WORKSPACEID,WORKSPACENAME
            let mut fields = data.split(',');
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            Ok(HyprlandEvent::CreateWorkspaceV2 { workspace_id, workspace_name })
        }
        "destroyworkspace" => {
            // Data: WORKSPACENAME
            Ok(HyprlandEvent::DestroyWorkspace { workspace_name: data.to_string() })
        }
        "destroyworkspacev2" => {
            // Data: WORKSPACEID,WORKSPACENAME
            let mut fields = data.split(',');
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            Ok(HyprlandEvent::DestroyWorkspaceV2 { workspace_id, workspace_name })
        }
        "moveworkspace" => {
            // Data: WORKSPACENAME,MONITORNAME
            let mut fields = data.split(',');
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            let monitor_name = fields.next().ok_or("Missing monitor_name")?.to_string();
            Ok(HyprlandEvent::MoveWorkspace { workspace_name, monitor_name })
        }
        "moveworkspacev2" => {
            // Data: WORKSPACEID,WORKSPACENAME,MONITORNAME
            let mut fields = data.split(',');
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            let monitor_name = fields.next().ok_or("Missing monitor_name")?.to_string();
            Ok(HyprlandEvent::MoveWorkspaceV2 { workspace_id, workspace_name, monitor_name })
        }
        "renameworkspace" => {
            // Data: WORKSPACEID,NEWNAME
            let mut fields = data.split(',');
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            let new_name = fields.next().ok_or("Missing new_name")?.to_string();
            Ok(HyprlandEvent::RenameWorkspace { workspace_id, new_name })
        }
        "activespecial" => {
            // Data: WORKSPACENAME,MONITORNAME
            let mut fields = data.split(',');
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            let monitor_name = fields.next().ok_or("Missing monitor_name")?.to_string();
            Ok(HyprlandEvent::ActiveSpecial { workspace_name, monitor_name })
        }
        "activelayout" => {
            // Data: KEYBOARDNAME,LAYOUTNAME
            let mut fields = data.split(',');
            let keyboard_name = fields.next().ok_or("Missing keyboard_name")?.to_string();
            let layout_name = fields.next().ok_or("Missing layout_name")?.to_string();
            Ok(HyprlandEvent::ActiveLayout { keyboard_name, layout_name })
        }
        "openwindow" => {
            // Data: WINDOWADDRESS,WORKSPACENAME,WINDOWCLASS,WINDOWTITLE
            let mut fields = data.split(',');
            let window_address = fields.next().ok_or("Missing window_address")?.to_string();
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            let window_class = fields.next().ok_or("Missing window_class")?.to_string();
            let window_title = fields.next().ok_or("Missing window_title")?.to_string();
            Ok(HyprlandEvent::OpenWindow { window_address, workspace_name, window_class, window_title })
        }
        "closewindow" => {
            // Data: WINDOWADDRESS
            Ok(HyprlandEvent::CloseWindow { window_address: data.to_string() })
        }
        "movewindow" => {
            // Data: WINDOWADDRESS,WORKSPACENAME
            let mut fields = data.split(',');
            let window_address = fields.next().ok_or("Missing window_address")?.to_string();
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            Ok(HyprlandEvent::MoveWindow { window_address, workspace_name })
        }
        "movewindowv2" => {
            // Data: WINDOWADDRESS,WORKSPACEID,WORKSPACENAME
            let mut fields = data.split(',');
            let window_address = fields.next().ok_or("Missing window_address")?.to_string();
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            Ok(HyprlandEvent::MoveWindowV2 { window_address, workspace_id, workspace_name })
        }
        "openlayer" => {
            // Data: NAMESPACE
            Ok(HyprlandEvent::OpenLayer { namespace: data.to_string() })
        }
        "closelayer" => {
            // Data: NAMESPACE
            Ok(HyprlandEvent::CloseLayer { namespace: data.to_string() })
        }
        "submap" => {
            // Data: SUBMAPNAME
            Ok(HyprlandEvent::Submap { submap_name: data.to_string() })
        }
        "changefloatingmode" => {
            // Data: WINDOWADDRESS,FLOATING
            let mut fields = data.split(',');
            let window_address = fields.next().ok_or("Missing window_address")?.to_string();
            let floating = fields.next().ok_or("Missing floating")?.parse::<u8>()?;
            Ok(HyprlandEvent::ChangeFloatingMode { window_address, floating })
        }
        "urgent" => {
            // Data: WINDOWADDRESS
            Ok(HyprlandEvent::Urgent { window_address: data.to_string() })
        }
        "screencast" => {
            // Data: STATE,OWNER
            let mut fields = data.split(',');
            let state = fields.next().ok_or("Missing state")?.parse::<u8>()?;
            let owner = fields.next().ok_or("Missing owner")?.parse::<u8>()?;
            Ok(HyprlandEvent::Screencast { state, owner })
        }
        "windowtitle" => {
            // Data: WINDOWADDRESS
            Ok(HyprlandEvent::WindowTitle { window_address: data.to_string() })
        }
        "windowtitlev2" => {
            // Data: WINDOWADDRESS,WINDOWTITLE
            let mut fields = data.split(',');
            let window_address = fields.next().ok_or("Missing window_address")?.to_string();
            let window_title = fields.next().ok_or("Missing window_title")?.to_string();
            Ok(HyprlandEvent::WindowTitleV2 { window_address, window_title })
        }
        "togglegroup" => {
            // Data: TOGGLE_STATUS,WINDOWADDRESS(ES)
            let mut fields = data.split(',');
            let toggle_status = fields.next().ok_or("Missing toggle_status")?.parse::<u8>()?;
            let window_addresses: Vec<String> = fields.map(|s| s.to_string()).collect();
            Ok(HyprlandEvent::ToggleGroup { toggle_status, window_addresses })
        }
        "moveintogroup" => {
            // Data: WINDOWADDRESS
            Ok(HyprlandEvent::MoveIntoGroup { window_address: data.to_string() })
        }
        "moveoutofgroup" => {
            // Data: WINDOWADDRESS
            Ok(HyprlandEvent::MoveOutOfGroup { window_address: data.to_string() })
        }
        "ignoregrouplock" => {
            // Data: VALUE
            let value = data.parse::<u8>()?;
            Ok(HyprlandEvent::IgnoreGroupLock { value })
        }
        "lockgroups" => {
            // Data: VALUE
            let value = data.parse::<u8>()?;
            Ok(HyprlandEvent::LockGroups { value })
        }
        "configreloaded" => {
            // No data.
            Ok(HyprlandEvent::ConfigReloaded)
        }
        "pin" => {
            // Data: WINDOWADDRESS,PINSTATE
            let mut fields = data.split(',');
            let window_address = fields.next().ok_or("Missing window_address")?.to_string();
            let pin_state = fields.next().ok_or("Missing pin_state")?.parse::<u8>()?;
            Ok(HyprlandEvent::Pin { window_address, pin_state })
        }
        _ => Err(format!("Unknown event type: {}", event_name).into()),
    }
}
/// Helper to fetch environment variables or panic with a clear message.
fn get_env_var(var: &str) -> String {
    env::var(var).unwrap_or_else(|_| panic!("Environment variable {} is not set", var))
}

/// Creates a UnixStream from a given socket path.
fn create_socket(socket_path: &str) -> UnixStream {
    UnixStream::connect(socket_path).unwrap_or_else(|err| {
        panic!("Could not connect to socket {}: {}", socket_path, err)
    })
}

/// Handles a single event string: parses it and prints its JSON representation.
fn handle_event(event_str: String) {
    match parse_event_line(&event_str) {
        Ok(event) => {
            let json = serde_json::to_string(&event).unwrap();
            println!("{}", json);
        }
        Err(e) => eprintln!("Error parsing event '{}': {}", event_str, e),
    }
}

fn main() {
    let xdg_runtime_dir = get_env_var("XDG_RUNTIME_DIR");
    let hypr_instance_signature = get_env_var("HYPRLAND_INSTANCE_SIGNATURE");

    let hypr_rundir_path = format!("{}/hypr/{}", xdg_runtime_dir, hypr_instance_signature);
    println!("Using hypr runtime directory: {}", hypr_rundir_path);

    let socket1_path = format!("{}/.socket.sock", hypr_rundir_path);
    let socket2_path = format!("{}/.socket2.sock", hypr_rundir_path);
    println!("Using socket1 path: {}", socket1_path);
    println!("Using socket2 path: {}", socket2_path);

    // let socket1 = create_socket(socket1_path);
    let socket2 = create_socket(&socket2_path);
    let stream = BufReader::new(socket2);
    for line in stream.lines() {
        match line {
            Ok(line_content) => {
                // Spawn a thread to handle the event. The closure takes ownership of `line_content`.
                thread::spawn(move || handle_event(line_content));
            }
            Err(e) => eprintln!("Error reading line: {}", e),
        }
    }
}
