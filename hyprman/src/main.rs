use std::env;
use std::io::Read;
use std::os::unix::net::UnixStream;

fn main() {
    let mut socket2_path: String = env::var("XDG_RUNTIME_DIR").unwrap();
    socket2_path.push_str("/hypr/");
    socket2_path.push_str(&*env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap());
    socket2_path.push_str("/.socket2.sock");
    let mut stream = UnixStream::connect(socket2_path).unwrap();
    println!("Connected to socket2: {:?}", stream.local_addr());
    let mut line = String::new();
    stream.read_to_string(&mut line).unwrap();
    println!("{line}");
}
