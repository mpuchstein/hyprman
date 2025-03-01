use std::{env, io};
use std::io::Read;
use async_std::os::unix::net::UnixStream;
use async_std::prelude::*;
use async_std::task::spawn;

async fn create_socket(socket_path : String) -> UnixStream {
    return match UnixStream::connect(socket_path).await{
        Ok(socket) => {
            println!("Connected to socket: {:?}", socket.peer_addr());
            socket
        },
        Err(error) => panic!("Could not connect to socket: {}", error),
    };
}

async fn handle_line_socket2(line : String){
    println!("{}", line);
}

#[tokio::main]
async fn main() {
    let env_var_xdg_runtime_dir = "XDG_RUNTIME_DIR";
    let env_var_hyprland_instance_signature = "HYPRLAND_INSTANCE_SIGNATURE";
    let mut hypr_rundir_path: String = match env::var(env_var_xdg_runtime_dir){
        Ok(path) => path,
        Err(_e) => panic!("XDG_RUNTIME_DIR is not set."),
    };
    hypr_rundir_path.push_str("/hypr/");
    hypr_rundir_path.push_str(&*match env::var(env_var_hyprland_instance_signature){
        Ok(path) => path,
        Err(_e) => panic!("HYPRLAND_INSTANCE_SIGNATURE is not set. Hyprland running?"),
    });
    println!("Using hypr runtime directory: {}", hypr_rundir_path);
    let socket1_path = format!("{}/.socket.sock", hypr_rundir_path);
    let socket2_path = format!("{}/.socket2.sock", hypr_rundir_path);
    println!("Using socket1 path: {}", socket1_path);
    println!("Using socket2 path: {}", socket2_path);
    let mut socket1 = create_socket(socket1_path).await;
    let mut socket2= create_socket(socket2_path);
    let mut buffer = String::new();
    //TODO: Read from UnixSocket instead from stdin
    loop{
        io::stdin().read_line(&mut buffer).expect("Could not read line from standard input");
        println!("{} bytes read", buffer.len());
        let line = buffer.trim();
        spawn(handle_line_socket2(line.to_string()));
        buffer.clear();
    }
}