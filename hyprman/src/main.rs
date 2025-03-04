use std::{
    collections::HashSet,
    env,
    error::Error,
    fs,
    io::{BufRead, BufReader, BufWriter, Write},
    os::unix::net::{UnixListener, UnixStream},
    process::Command,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};
use std::io::Read;
use daemonize::Daemonize;
use log::{error, info};
use serde::{Deserialize, Serialize};
use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};

/// === Hyprland Event Types and Parsing ===

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "event", content = "data")]
enum HyprlandEvent {
    Workspace { workspace_name: String },
    WorkspaceV2 { workspace_id: u8, workspace_name: String },
    FocusedMon { monitor_name: String, workspace_name: String },
    FocusedMonV2 { monitor_name: String, workspace_id: u8 },
    ActiveWindow { window_class: String, window_title: String },
    ActiveWindowV2 { window_address: String },
    Fullscreen { status: u8 },
    MonitorRemoved { monitor_name: String },
    MonitorAdded { monitor_name: String },
    MonitorAddedV2 { monitor_id: u8, monitor_name: String, monitor_description: String },
    CreateWorkspace { workspace_name: String },
    CreateWorkspaceV2 { workspace_id: u8, workspace_name: String },
    DestroyWorkspace { workspace_name: String },
    DestroyWorkspaceV2 { workspace_id: u8, workspace_name: String },
    MoveWorkspace { workspace_name: String, monitor_name: String },
    MoveWorkspaceV2 { workspace_id: u8, workspace_name: String, monitor_name: String },
    RenameWorkspace { workspace_id: u8, new_name: String },
    ActiveSpecial { workspace_name: String, monitor_name: String },
    ActiveLayout { keyboard_name: String, layout_name: String },
    OpenWindow { window_address: String, workspace_name: String, window_class: String, window_title: String },
    CloseWindow { window_address: String },
    MoveWindow { window_address: String, workspace_name: String },
    MoveWindowV2 { window_address: String, workspace_id: u8, workspace_name: String },
    OpenLayer { namespace: String },
    CloseLayer { namespace: String },
    Submap { submap_name: String },
    ChangeFloatingMode { window_address: String, floating: u8 },
    Urgent { window_address: String },
    Screencast { state: u8, owner: u8 },
    WindowTitle { window_address: String },
    WindowTitleV2 { window_address: String, window_title: String },
    ToggleGroup { toggle_status: u8, window_addresses: Vec<String> },
    MoveIntoGroup { window_address: String },
    MoveOutOfGroup { window_address: String },
    IgnoreGroupLock { value: u8 },
    LockGroups { value: u8 },
    ConfigReloaded,
    Pin { window_address: String, pin_state: u8 },
}

fn parse_event_line(line: &str) -> Result<HyprlandEvent, Box<dyn Error>> {
    let line = line.trim();
    let mut parts = line.split(">>");
    let event_name = parts.next().ok_or("Missing event name")?;
    let data = parts.next().unwrap_or("").trim();

    match event_name {
        "workspace" => Ok(HyprlandEvent::Workspace { workspace_name: data.to_string() }),
        "workspacev2" => {
            let mut fields = data.split(',');
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            Ok(HyprlandEvent::WorkspaceV2 { workspace_id, workspace_name })
        }
        "focusedmon" => {
            let mut fields = data.split(',');
            let monitor_name = fields.next().ok_or("Missing monitor_name")?.to_string();
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            Ok(HyprlandEvent::FocusedMon { monitor_name, workspace_name })
        }
        "focusedmonv2" => {
            let mut fields = data.split(',');
            let monitor_name = fields.next().ok_or("Missing monitor_name")?.to_string();
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            Ok(HyprlandEvent::FocusedMonV2 { monitor_name, workspace_id })
        }
        "activewindow" => {
            let mut fields = data.split(',');
            let window_class = fields.next().ok_or("Missing window_class")?.to_string();
            let window_title = fields.next().ok_or("Missing window_title")?.to_string();
            Ok(HyprlandEvent::ActiveWindow { window_class, window_title })
        }
        "activewindowv2" => Ok(HyprlandEvent::ActiveWindowV2 { window_address: data.to_string() }),
        "fullscreen" => {
            let status = data.parse::<u8>()?;
            Ok(HyprlandEvent::Fullscreen { status })
        }
        "monitorremoved" => Ok(HyprlandEvent::MonitorRemoved { monitor_name: data.to_string() }),
        "monitoradded" => Ok(HyprlandEvent::MonitorAdded { monitor_name: data.to_string() }),
        "monitoraddedv2" => {
            let mut fields = data.split(',');
            let monitor_id = fields.next().ok_or("Missing monitor_id")?.parse::<u8>()?;
            let monitor_name = fields.next().ok_or("Missing monitor_name")?.to_string();
            let monitor_description = fields.next().ok_or("Missing monitor_description")?.to_string();
            Ok(HyprlandEvent::MonitorAddedV2 { monitor_id, monitor_name, monitor_description })
        }
        "createworkspace" => Ok(HyprlandEvent::CreateWorkspace { workspace_name: data.to_string() }),
        "createworkspacev2" => {
            let mut fields = data.split(',');
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            Ok(HyprlandEvent::CreateWorkspaceV2 { workspace_id, workspace_name })
        }
        "destroyworkspace" => Ok(HyprlandEvent::DestroyWorkspace { workspace_name: data.to_string() }),
        "destroyworkspacev2" => {
            let mut fields = data.split(',');
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            Ok(HyprlandEvent::DestroyWorkspaceV2 { workspace_id, workspace_name })
        }
        "moveworkspace" => {
            let mut fields = data.split(',');
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            let monitor_name = fields.next().ok_or("Missing monitor_name")?.to_string();
            Ok(HyprlandEvent::MoveWorkspace { workspace_name, monitor_name })
        }
        "moveworkspacev2" => {
            let mut fields = data.split(',');
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            let monitor_name = fields.next().ok_or("Missing monitor_name")?.to_string();
            Ok(HyprlandEvent::MoveWorkspaceV2 { workspace_id, workspace_name, monitor_name })
        }
        "renameworkspace" => {
            let mut fields = data.split(',');
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            let new_name = fields.next().ok_or("Missing new_name")?.to_string();
            Ok(HyprlandEvent::RenameWorkspace { workspace_id, new_name })
        }
        "activespecial" => {
            let mut fields = data.split(',');
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            let monitor_name = fields.next().ok_or("Missing monitor_name")?.to_string();
            Ok(HyprlandEvent::ActiveSpecial { workspace_name, monitor_name })
        }
        "activelayout" => {
            let mut fields = data.split(',');
            let keyboard_name = fields.next().ok_or("Missing keyboard_name")?.to_string();
            let layout_name = fields.next().ok_or("Missing layout_name")?.to_string();
            Ok(HyprlandEvent::ActiveLayout { keyboard_name, layout_name })
        }
        "openwindow" => {
            let mut fields = data.split(',');
            let window_address = fields.next().ok_or("Missing window_address")?.to_string();
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            let window_class = fields.next().ok_or("Missing window_class")?.to_string();
            let window_title = fields.next().ok_or("Missing window_title")?.to_string();
            Ok(HyprlandEvent::OpenWindow { window_address, workspace_name, window_class, window_title })
        }
        "closewindow" => Ok(HyprlandEvent::CloseWindow { window_address: data.to_string() }),
        "movewindow" => {
            let mut fields = data.split(',');
            let window_address = fields.next().ok_or("Missing window_address")?.to_string();
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            Ok(HyprlandEvent::MoveWindow { window_address, workspace_name })
        }
        "movewindowv2" => {
            let mut fields = data.split(',');
            let window_address = fields.next().ok_or("Missing window_address")?.to_string();
            let workspace_id = fields.next().ok_or("Missing workspace_id")?.parse::<u8>()?;
            let workspace_name = fields.next().ok_or("Missing workspace_name")?.to_string();
            Ok(HyprlandEvent::MoveWindowV2 { window_address, workspace_id, workspace_name })
        }
        "openlayer" => Ok(HyprlandEvent::OpenLayer { namespace: data.to_string() }),
        "closelayer" => Ok(HyprlandEvent::CloseLayer { namespace: data.to_string() }),
        "submap" => Ok(HyprlandEvent::Submap { submap_name: data.to_string() }),
        "changefloatingmode" => {
            let mut fields = data.split(',');
            let window_address = fields.next().ok_or("Missing window_address")?.to_string();
            let floating = fields.next().ok_or("Missing floating")?.parse::<u8>()?;
            Ok(HyprlandEvent::ChangeFloatingMode { window_address, floating })
        }
        "urgent" => Ok(HyprlandEvent::Urgent { window_address: data.to_string() }),
        "screencast" => {
            let mut fields = data.split(',');
            let state = fields.next().ok_or("Missing state")?.parse::<u8>()?;
            let owner = fields.next().ok_or("Missing owner")?.parse::<u8>()?;
            Ok(HyprlandEvent::Screencast { state, owner })
        }
        "windowtitle" => Ok(HyprlandEvent::WindowTitle { window_address: data.to_string() }),
        "windowtitlev2" => {
            let mut fields = data.split(',');
            let window_address = fields.next().ok_or("Missing window_address")?.to_string();
            let window_title = fields.next().ok_or("Missing window_title")?.to_string();
            Ok(HyprlandEvent::WindowTitleV2 { window_address, window_title })
        }
        "togglegroup" => {
            let mut fields = data.split(',');
            let toggle_status = fields.next().ok_or("Missing toggle_status")?.parse::<u8>()?;
            let window_addresses: Vec<String> = fields.map(|s| s.to_string()).collect();
            Ok(HyprlandEvent::ToggleGroup { toggle_status, window_addresses })
        }
        "moveintogroup" => Ok(HyprlandEvent::MoveIntoGroup { window_address: data.to_string() }),
        "moveoutofgroup" => Ok(HyprlandEvent::MoveOutOfGroup { window_address: data.to_string() }),
        "ignoregrouplock" => {
            let value = data.parse::<u8>()?;
            Ok(HyprlandEvent::IgnoreGroupLock { value })
        }
        "lockgroups" => {
            let value = data.parse::<u8>()?;
            Ok(HyprlandEvent::LockGroups { value })
        }
        "configreloaded" => Ok(HyprlandEvent::ConfigReloaded),
        "pin" => {
            let mut fields = data.split(',');
            let window_address = fields.next().ok_or("Missing window_address")?.to_string();
            let pin_state = fields.next().ok_or("Missing pin_state")?.parse::<u8>()?;
            Ok(HyprlandEvent::Pin { window_address, pin_state })
        }
        _ => Err(format!("Unknown event type: {}", event_name).into()),
    }
}

/// === Utility: Extract event type string for filtering ===

fn event_type(event: &HyprlandEvent) -> &'static str {
    match event {
        HyprlandEvent::Workspace { .. } => "workspace",
        HyprlandEvent::WorkspaceV2 { .. } => "workspacev2",
        HyprlandEvent::FocusedMon { .. } => "focusedmon",
        HyprlandEvent::FocusedMonV2 { .. } => "focusedmonv2",
        HyprlandEvent::ActiveWindow { .. } => "activewindow",
        HyprlandEvent::ActiveWindowV2 { .. } => "activewindowv2",
        HyprlandEvent::Fullscreen { .. } => "fullscreen",
        HyprlandEvent::MonitorRemoved { .. } => "monitorremoved",
        HyprlandEvent::MonitorAdded { .. } => "monitoradded",
        HyprlandEvent::MonitorAddedV2 { .. } => "monitoraddedv2",
        HyprlandEvent::CreateWorkspace { .. } => "createworkspace",
        HyprlandEvent::CreateWorkspaceV2 { .. } => "createworkspacev2",
        HyprlandEvent::DestroyWorkspace { .. } => "destroyworkspace",
        HyprlandEvent::DestroyWorkspaceV2 { .. } => "destroyworkspacev2",
        HyprlandEvent::MoveWorkspace { .. } => "moveworkspace",
        HyprlandEvent::MoveWorkspaceV2 { .. } => "moveworkspacev2",
        HyprlandEvent::RenameWorkspace { .. } => "renameworkspace",
        HyprlandEvent::ActiveSpecial { .. } => "activespecial",
        HyprlandEvent::ActiveLayout { .. } => "activelayout",
        HyprlandEvent::OpenWindow { .. } => "openwindow",
        HyprlandEvent::CloseWindow { .. } => "closewindow",
        HyprlandEvent::MoveWindow { .. } => "movewindow",
        HyprlandEvent::MoveWindowV2 { .. } => "movewindowv2",
        HyprlandEvent::OpenLayer { .. } => "openlayer",
        HyprlandEvent::CloseLayer { .. } => "closelayer",
        HyprlandEvent::Submap { .. } => "submap",
        HyprlandEvent::ChangeFloatingMode { .. } => "changefloatingmode",
        HyprlandEvent::Urgent { .. } => "urgent",
        HyprlandEvent::Screencast { .. } => "screencast",
        HyprlandEvent::WindowTitle { .. } => "windowtitle",
        HyprlandEvent::WindowTitleV2 { .. } => "windowtitlev2",
        HyprlandEvent::ToggleGroup { .. } => "togglegroup",
        HyprlandEvent::MoveIntoGroup { .. } => "moveintogroup",
        HyprlandEvent::MoveOutOfGroup { .. } => "moveoutofgroup",
        HyprlandEvent::IgnoreGroupLock { .. } => "ignoregrouplock",
        HyprlandEvent::LockGroups { .. } => "lockgroups",
        HyprlandEvent::ConfigReloaded => "configreloaded",
        HyprlandEvent::Pin { .. } => "pin",
    }
}

/// === Client Subscription Infrastructure ===

#[derive(Debug, Clone)]
enum Subscription {
    All,
    Filtered(HashSet<String>),
}

struct ClientHandle {
    sender: mpsc::Sender<HyprlandEvent>,
    subscription: Subscription,
}

/// === Configuration Loading ===

#[derive(Debug, Deserialize)]
struct Config {
    // Socket path where clients connect to receive events.
    // If relative, it will be interpreted relative to $XDG_RUNTIME_DIR/hyprman/
    client_socket_path: String,
}

fn load_config(path: &str) -> Config {
    let content = fs::read_to_string(path).expect("Failed to read config file");
    toml::from_str(&content).expect("Failed to parse config file")
}

fn get_hypr_rundir_path() -> String {
    let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| panic!("Environment variable XDG_RUNTIME_DIR is not set"));
    let hypr_instance_signature = env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .unwrap_or_else(|_| panic!("Environment variable HYPRLAND_INSTANCE_SIGNATURE is not set"));
    format!("{}/hypr/{}", xdg_runtime_dir, hypr_instance_signature)
}

/// === Daemon Mode Functions ===

fn client_handler(stream: UnixStream, subscriptions: Arc<Mutex<Vec<ClientHandle>>>) {
    let mut reader = BufReader::new(stream.try_clone().expect("Failed to clone stream"));
    let mut writer = BufWriter::new(stream);
    // Read a line from the client to get subscription preferences.
    let mut subscription_line = String::new();
    if let Err(e) = reader.read_line(&mut subscription_line) {
        error!("Failed to read subscription from client: {}", e);
        return;
    }
    let subscription_line = subscription_line.trim();
    let subscription = if subscription_line.is_empty() || subscription_line.to_lowercase() == "all" {
        Subscription::All
    } else {
        let filters: HashSet<String> = subscription_line
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .collect();
        Subscription::Filtered(filters)
    };
    info!("Client subscribed to: {:?}", subscription);

    // Create a channel for sending events to this client.
    let (tx, rx) = mpsc::channel::<HyprlandEvent>();

    {
        let mut subs = subscriptions.lock().unwrap();
        subs.push(ClientHandle { sender: tx, subscription });
    }

    // Loop and write events to the client.
    loop {
        match rx.recv() {
            Ok(event) => {
                let json = serde_json::to_string(&event).unwrap();
                if let Err(e) = writeln!(writer, "{}", json) {
                    error!("Failed to write to client: {}", e);
                    break;
                }
                if let Err(e) = writer.flush() {
                    error!("Failed to flush writer: {}", e);
                    break;
                }
            }
            Err(e) => {
                error!("Channel error: {}", e);
                break;
            }
        }
    }
}

fn hyprland_event_thread(subscriptions: Arc<Mutex<Vec<ClientHandle>>>) {
    let hypr_rundir_path = get_hypr_rundir_path();
    info!("Using hypr runtime directory: {}", hypr_rundir_path);

    let socket2_path = format!("{}/.socket2.sock", hypr_rundir_path);
    info!("Using hypr socket2 path: {}", socket2_path);
    let socket2 = create_socket(&socket2_path);
    let reader = BufReader::new(socket2);

    for line in reader.lines() {
        match line {
            Ok(line_content) => {
                match parse_event_line(&line_content) {
                    Ok(event) => {
                        let event_name = event_type(&event);
                        let json = serde_json::to_string(&event).unwrap();
                        info!("Received event: {}", json);
                        let mut subs = subscriptions.lock().unwrap();
                        // Dispatch events to matching clients.
                        subs.retain(|client| {
                            let send_result = match &client.subscription {
                                Subscription::All => client.sender.send(event.clone()),
                                Subscription::Filtered(filters) => {
                                    if filters.contains(&event_name.to_string()) {
                                        client.sender.send(event.clone())
                                    } else {
                                        Ok(())
                                    }
                                }
                            };
                            send_result.is_ok()
                        });
                    }
                    Err(e) => error!("Error parsing event '{}': {}", line_content, e),
                }
            }
            Err(e) => error!("Error reading line: {}", e),
        }
    }
}

fn client_server_thread(client_socket_path: String, subscriptions: Arc<Mutex<Vec<ClientHandle>>>) {
    // Remove existing socket file if present.
    let _ = fs::remove_file(&client_socket_path);
    let listener = UnixListener::bind(&client_socket_path).unwrap_or_else(|e| {
        panic!("Failed to bind client socket {}: {}", client_socket_path, e)
    });
    info!("Client server listening on {}", client_socket_path);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let subs = subscriptions.clone();
                thread::spawn(move || client_handler(stream, subs));
            }
            Err(e) => error!("Failed to accept client connection: {}", e),
        }
    }
}

fn create_socket(socket_path: &str) -> UnixStream {
    UnixStream::connect(socket_path).unwrap_or_else(|err| {
        panic!("Could not connect to socket {}: {}", socket_path, err)
    })
}

/// The main daemon functionality: spawn threads, handle signals, etc.
fn run_daemon(config: Config) {
    // Global subscription registry.
    let subscriptions = Arc::new(Mutex::new(Vec::<ClientHandle>::new()));

    // Setup signal handling for graceful shutdown.
    let mut signals = Signals::new(TERM_SIGNALS).expect("Unable to setup signal handling");
    let signals_handle = signals.handle();
    let shutdown_flag = Arc::new(Mutex::new(false));
    {
        let shutdown_flag = shutdown_flag.clone();
        thread::spawn(move || {
            for signal in signals.forever() {
                info!("Received termination signal: {}", signal);
                *shutdown_flag.lock().unwrap() = true;
                break;
            }
        });
    }

    // Spawn thread to read and dispatch Hyprland events.
    let subs_clone = subscriptions.clone();
    thread::spawn(move || {
        hyprland_event_thread(subs_clone);
    });

    // Spawn thread to accept client connections.
    let client_socket_path = config.client_socket_path;
    let subs_clone = subscriptions.clone();
    thread::spawn(move || {
        client_server_thread(client_socket_path, subs_clone);
    });

    // Main thread waits for shutdown.
    loop {
        if *shutdown_flag.lock().unwrap() {
            info!("Shutting down daemon");
            signals_handle.close();
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }
}

/// === Client Mode Function ===
/// Accepts a subscription filter (e.g. "all" or "activewindow")
fn run_client(config: &Config, subscription: &str) {
    match UnixStream::connect(&config.client_socket_path) {
        Ok(mut stream) => {
            // Send subscription preferences.
            let subscription_line = format!("{}\n", subscription);
            if let Err(e) = stream.write_all(subscription_line.as_bytes()) {
                eprintln!("Failed to send subscription: {}", e);
                std::process::exit(1);
            }
            println!("Subscribed to '{}' events. Waiting for events...", subscription);

            let reader = BufReader::new(stream);
            for line in reader.lines() {
                match line {
                    Ok(msg) => println!("{}", msg),
                    Err(e) => {
                        eprintln!("Error reading from daemon: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to daemon. Is it running? Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_activewindow_client(config: &Config) {
    let hypr_rundir_path = get_hypr_rundir_path();
    info!("Using hypr runtime directory: {}", hypr_rundir_path);

    let socket1_path = format!("{}/.socket1.sock", hypr_rundir_path);
    info!("Using hypr socket2 path: {}", socket1_path);
    match UnixStream::connect(&config.client_socket_path) {
        Ok(mut stream) => {
            let subscription_line = "activewindowv2,openwindow,closewindow,movewindow\n";
            if let Err(e) = stream.write_all(subscription_line.as_bytes()) {
                eprintln!("Failed to send subscription: {}", e);
                std::process::exit(1);
            }
            println!("Successfully connected to daemon.");
            match UnixStream::connect(&socket1_path) {
                Ok(mut stream1) => {
                    let query = "activewindow";
                    let reader = BufReader::new(stream);
                    for line in reader.lines() {
                        match line {
                            Ok(msg) => {
                                println!("{}", msg);
                                if let Err(e) = stream1.write_all(query.as_bytes()) {
                                    eprintln!("Failed to send query: {}", e);
                                    std::process::exit(1);
                                }
                                println!("Successfully sent query to daemon.");
                                let mut response = String::new();
                                stream1.read_to_string(&mut response).unwrap();
                                println!("{}", response);
                            },
                            Err(e) => {
                                eprintln!("Error reading from daemon: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to connect to socket1: {}", e);
                    std::process::exit(1);
                }

            }
        }
        Err(e) => {
            eprintln!("Failed to connect to daemon. Is it running? Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// === Daemon Control Functions ===

fn stop_daemon() -> Result<(), Box<dyn Error>> {
    // Compute pid file path from $XDG_RUNTIME_DIR/hyprman/
    let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR")
        .expect("XDG_RUNTIME_DIR not set");
    let hyprman_dir = format!("{}/hyprman", xdg_runtime_dir);
    let pid_file_path = format!("{}/hyprman.pid", hyprman_dir);
    let pid_str = fs::read_to_string(&pid_file_path)?;
    let pid: i32 = pid_str.trim().parse()?;
    unsafe {
        if libc::kill(pid, libc::SIGTERM) != 0 {
            return Err(format!("Failed to kill process {}", pid).into());
        }
    }
    fs::remove_file(&pid_file_path)?;
    println!("Daemon stopped.");
    Ok(())
}

fn restart_daemon() -> Result<(), Box<dyn Error>> {
    stop_daemon()?;
    thread::sleep(Duration::from_secs(1));
    let current_exe = env::current_exe()?;
    Command::new(current_exe).arg("-d").spawn()?;
    println!("Daemon restarted.");
    Ok(())
}

/// Print usage help text.
fn print_help() {
    println!("Usage:");
    println!("  -d, --daemon             Run as daemon");
    println!("  -r, --restart            Restart the daemon");
    println!("  -k, --kill               Stop the daemon");
    println!("  -f, --filter [event]     Run in client mode, subscribing to [event] (default: all)");
    println!("  -h, --help               Show this help message");
}

/// === Main Entry Point: Mode Selection Based on Command‑Line Arguments ===

fn main() {
    // Load configuration from $XDG_CONFIG_HOME/hyprman/config.toml
    let config_dir = env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| panic!("Environment variable XDG_CONFIG_HOME is not set"));
    let config_path = format!("{}/hyprman/config.toml", config_dir);
    let mut config = load_config(&config_path);
    env_logger::init();

    // Ensure $XDG_RUNTIME_DIR/hyprman/ exists.
    let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR")
        .expect("XDG_RUNTIME_DIR not set");
    let hyprman_dir = format!("{}/hyprman", xdg_runtime_dir);
    if fs::metadata(&hyprman_dir).is_err() {
        fs::create_dir_all(&hyprman_dir)
            .expect("Failed to create hyprman runtime directory");
    }
    // If the socket path from the config is relative, interpret it relative to hyprman_dir.
    if !config.client_socket_path.starts_with("/") {
        config.client_socket_path = format!("{}/{}", hyprman_dir, config.client_socket_path);
    }

    // Also, compute the PID file path to be used.
    let pid_file_path = format!("{}/hyprman.pid", hyprman_dir);

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "-d" | "--daemon" => {
                // Check if daemon is already running.
                if let Ok(pid_str) = fs::read_to_string(&pid_file_path) {
                    if let Ok(pid) = pid_str.trim().parse::<i32>() {
                        if unsafe { libc::kill(pid, 0) } == 0 {
                            eprintln!("Daemon already running with PID {}.", pid);
                            std::process::exit(1);
                        }
                    }
                }
                let daemonize = Daemonize::new()
                    .pid_file(&pid_file_path)
                    .working_directory("/")
                    .umask(0o022)
                    .privileged_action(|| {
                        info!("Daemon started successfully");
                    });
                if let Err(e) = daemonize.start() {
                    eprintln!("Error daemonizing: {}", e);
                    std::process::exit(1);
                }
                run_daemon(config);
            }
            "-r" | "--restart" => {
                if let Err(e) = restart_daemon() {
                    eprintln!("Error restarting daemon: {}", e);
                    std::process::exit(1);
                }
            }
            "-k" | "--kill" => {
                if let Err(e) = stop_daemon() {
                    eprintln!("Error stopping daemon: {}", e);
                    std::process::exit(1);
                }
            }
            "-f" | "--filter" => {
                // Client mode with a subscription filter.
                let filter = if args.len() > 2 {
                    args[2].clone()
                } else {
                    "all".to_string()
                };
                run_client(&config, &filter);
            }
            "-a" | "--activewindow" => {
                run_activewindow_client(&config);
            }
            "-h" | "--help" => {
                print_help();
            }
            _ => {
                eprintln!("Unknown option.");
                print_help();
                std::process::exit(1);
            }
        }
    } else {
        // No arguments provided: run as client with "all" subscription.
        run_client(&config, "all");
    }
}
