use log::warn;
use std::env;
use std::io::{BufRead, BufReader};
use std::thread::{spawn, JoinHandle};
use std::{os::unix::net::UnixListener, path::Path, u32};
use uuid::Uuid;
use vmm::VMM;

#[derive(Debug)]
pub enum Error {
    VmmNew(vmm::Error),

    VmmConfigure(vmm::Error),

    VmmRun(vmm::Error),
}

struct VMMOpts {
    /// Linux kernel path
    kernel: String,
    /// Number of virtual CPUs assigned to the guest
    cpus: u8,
    /// Memory amount (in MBytes) assigned to the guest
    memory: u32,
    /// Stdout console file path
    console: Option<String>,
    /// Path to the socket used for communication with the VMM
    socket: Option<String>,
}

pub fn run_vmm() -> Result<String, Error> {
    let socket_name = format!("{}.sock", Uuid::new_v4().to_string());

    let socket_path = Path::new(socket_name.as_str());
    if std::fs::metadata(socket_path).is_ok() {
        std::fs::remove_file(socket_path).unwrap();
    }

    let opts: VMMOpts = VMMOpts {
        kernel: env::var("KERNEL_PATH").unwrap(),
        cpus: 1,
        memory: 1024,
        console: None,
        socket: Some(socket_name.clone()),
    };

    let unix_listener = UnixListener::bind(socket_path).unwrap();

    let listener_handler = listen(unix_listener);

    run(opts)?;

    let response = listener_handler.join().unwrap();

    match std::fs::remove_file(socket_path) {
        Ok(_) => {}
        Err(e) => {
            warn!(
                "Unable to close socket {} : {}",
                socket_path.to_string_lossy(),
                e
            );
        }
    }

    Ok(response)
}

fn run(opts: VMMOpts) -> Result<(), Error> {
    let mut vmm = VMM::new().map_err(Error::VmmNew)?;
    vmm.configure(
        opts.cpus,
        opts.memory,
        &opts.kernel,
        opts.console,
        None,
        None,
        opts.socket,
        true,
    )
    .map_err(Error::VmmConfigure)?;

    // Run the VMM
    vmm.run(true).map_err(Error::VmmRun)?;
    Ok(())
}

fn listen(unix_listener: UnixListener) -> JoinHandle<String> {
    let listener_handler = spawn(move || {
        // read from socket
        let (stream, _) = unix_listener.accept().unwrap();
        let mut response = "".to_string();

        let stream = BufReader::new(stream.try_clone().unwrap());

        for line in stream.lines() {
            response = format!("{}{}", response, line.unwrap());
        }
        println!("response: {}", response);

        response
    });
    listener_handler
}
