use std::process::{Command, Stdio};

use anyhow::Context;
use crossbeam::channel::Receiver;

pub struct CargoChildProcess {
    child: std::process::Child,

    stdout_recver: Receiver<u8>,
}

impl CargoChildProcess {
    pub fn kill(mut self) -> anyhow::Result<()> {
        self.child.kill().context("Tried to kill child process")?;

        // Dropping this struct will close the stdout receiver channel and so the reading thread will end

        Ok(())
    }

    pub fn get_stdout_receiver(&self) -> Receiver<u8> {
        self.stdout_recver.clone()
    }
}

pub fn start_cargo_run(args: Vec<String>) -> std::io::Result<CargoChildProcess> {
    let (stdout_tx, stdout_rx) = crossbeam::channel::unbounded();

    // Create Command
    let mut cmd = Command::new("cargo");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::inherit()); // directly inherit stderr to main process

    // Add arguments
    cmd.arg("run");
    cmd.arg("--message-format")
        .arg("json-diagnostic-rendered-ansi"); // for easier parsing of build output
    // cmd.arg("--color").arg("always"); // keep colors in output
    for arg in args {
        cmd.arg(arg);
    }

    // Spawn process and take stdout
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().ok_or(std::io::ErrorKind::Other)?;
    read_to_channel_threaded(stdout, stdout_tx);

    Ok(CargoChildProcess {
        child,
        stdout_recver: stdout_rx,
    })
}

/// Reads from the given reader and sends the output to the provided channel sender.
fn read_to_channel_threaded<R: std::io::Read + Send + 'static>(
    mut reader: R,
    sender: crossbeam::channel::Sender<u8>,
) {
    std::thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            match reader.read(&mut buffer) {
                Ok(n) => {
                    for &byte in &buffer[..n] {
                        if sender.send(byte).is_err() {
                            // Receiver has been dropped -> stop reading
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading output: {}", e);
                    break;
                }
            }
        }
    });
}
