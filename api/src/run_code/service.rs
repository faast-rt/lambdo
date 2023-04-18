use log::warn;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::thread::{spawn, JoinHandle};
use std::{os::unix::net::UnixListener, path::Path, u32};
use uuid::Uuid;
use vmm::VMM;

use crate::run_code::model::{AgentExecution, AgentExecutionStep, AgentExecutionFile};

#[derive(Debug)]
pub enum Error {
    VmmNew(vmm::Error),

    VmmConfigure(vmm::Error),

    VmmRun(vmm::Error),

    BadAgentStatus,
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
    let socket_name = format!("/tmp/{}.sock", Uuid::new_v4().to_string());

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
        let (mut stream, _) = unix_listener.accept().unwrap();
        let mut response = "".to_string();

        let stream_reader = BufReader::new(stream.try_clone().unwrap());

        for line in stream_reader.lines() {
            let parsed_line = parse_response(line.unwrap(), &mut stream).unwrap();
            if parsed_line == "" {
                continue;
            }

            response = format!("{}{}\n", response, parsed_line);
            log::trace!("response line: {}", response);
        }
        log::debug!("response: {}", response);

        response
    });
    listener_handler
}

fn parse_response(response: String, stream: &mut UnixStream) -> Result<String, Error> {
    log::trace!("received response from agent: {}", response);
    if response.contains("\"type\":\"status\"") {
        // match the status code
        let status_code = response
            .split("\"code\":")
            .nth(1)
            .unwrap()
            .split("}")
            .nth(0)
            .unwrap()
            .split("\"")
            .nth(1)
            .unwrap();
        log::debug!("received status code from agent: {}", status_code);

        if status_code == "ready" {
            send_instructions(stream);
            Ok("".to_string())
        } else {
            Err(Error::BadAgentStatus)
        }
    } else {
        Ok(response)
    }
}

fn send_instructions(stream: &mut UnixStream) {
    // create a new agent execution
    let mut agent_execution = AgentExecution::new();
    agent_execution.add_step(AgentExecutionStep {
        command: "echo 'hello world!'".to_string(),
        enable_output: true,
    });
    agent_execution.add_file(AgentExecutionFile::new("test.txt".to_string(), "hello world!".to_string()));

    let agent_execution_json = serde_json::to_string(&agent_execution).unwrap();
    let message = format_message(agent_execution_json.as_str());
    log::debug!("sending agent execution json: {}", message);

    // send the agent execution to the socket
    let _ = stream.write_all(message.as_bytes()).unwrap();
}

fn format_message(message: &str) -> String {
    let message_size = message.len();
    format!("{:0>8}{}", message_size, message)
}