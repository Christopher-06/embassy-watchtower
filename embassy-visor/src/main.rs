use std::{
    fs,
    sync::{Arc, OnceLock, atomic::AtomicBool},
};

use anyhow::{Context, bail};

use crate::{
    cargo::{
        cargo_build::{self, CargoBuildStatus},
        cargo_child,
    },
    tracing::{instance::TracingInstance, time::ComputerTime, trace_data::TraceItem},
};

mod cargo;
mod elf_file;
mod tracing;
mod visualizer;

pub static FIRMWARE_ADDR_MAP: OnceLock<std::collections::HashMap<u64, String>> = OnceLock::new();

fn main() -> anyhow::Result<()> {
    // let (trace_tx, trace_rx) = crossbeam::channel::unbounded();
    // let (logs_tx, logs_recver) = crossbeam::channel::unbounded();
    // let instance = TracingInstance::new(trace_rx);
    // visualizer::run_main_tui(instance, logs_recver).context("Failed running TUI")?;
    // return Ok(());

    // TODO: STDERR not inherit (overrides TUI output!!!)

    let args: Vec<String> = std::env::args().collect();

    let cargo_child_process = cargo_child::start_cargo_run(args[1..].to_vec())
        .expect("Failed to start cargo run process");
    let stdout_listener = cargo_child_process.get_stdout_receiver();

    let (build_tx, build_rx) = crossbeam::channel::unbounded();
    let (logs_tx, logs_recver) = crossbeam::channel::unbounded();
    let (trace_tx, trace_rx) = crossbeam::channel::unbounded();
    let first_trace_item_received = Arc::new(AtomicBool::new(false));
    let first_trace_item_received_clone = first_trace_item_received.clone();
    std::thread::spawn(move || {
        let mut temp_buffer = Vec::new();
        let mut cargo_build_finished = false;
        loop {
            match stdout_listener.recv() {
                Ok(c) => {
                    temp_buffer.push(c);

                    // Check if '\n' is in buffer
                    let newline_pos = temp_buffer.iter().position(|&b| b == b'\n' as u8);
                    if let Some(pos) = newline_pos {
                        let line = String::from_utf8(temp_buffer.drain(..=pos).collect())
                            .unwrap_or_else(|_| String::from("<Invalid UTF-8>"));

                        if !cargo_build_finished {
                            // build output
                            build_tx.send(line.clone()).unwrap();

                            if line.contains(r#"{"reason":"build-finished","success":true}"#) {
                                cargo_build_finished = true;
                            }
                        } else {
                            // Trace or log line of program
                            if line.contains("embassy executor tracer - ")
                                && line.contains(" - embassy executor tracer")
                            {
                                // Parse Trace line
                                let pc_timestamp = ComputerTime::now();
                                match TraceItem::parse_from_line(&line, pc_timestamp) {
                                    Ok(item) => {
                                        // Send trace item
                                        trace_tx.send(item).unwrap();
                                        // println!("Parsed trace item: {:?}", item);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to parse trace item: {:?}", e);
                                    }
                                }

                                first_trace_item_received_clone
                                    .store(true, std::sync::atomic::Ordering::Relaxed);
                            } else {
                                // Propagate log line
                                if first_trace_item_received_clone
                                    .load(std::sync::atomic::Ordering::Relaxed)
                                {
                                    logs_tx.send(line).unwrap();
                                } else {
                                    // Pre-trace log line, just print to console
                                    println!("{}", line);
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
    });

    // handle cargo build
    let build_status = cargo_build::handle_cargo_build(&build_rx);
    match build_status {
        CargoBuildStatus::Success(Some(elf_path)) => {
            // read elf file and create address map
            let bin_data = fs::read(elf_path).expect("Konnte ELF-Datei nicht lesen");
            let file: object::File<'_> =
                object::File::parse(&*bin_data).expect("Konnte ELF-Format nicht parsen");
            let addr_map = elf_file::get_addr_map(file);
            FIRMWARE_ADDR_MAP.set(addr_map).unwrap();
        }
        CargoBuildStatus::Success(None) => {
            println!("Build succeeded! No executable path found.");
        }
        CargoBuildStatus::Failed => {
            eprintln!("Build failed!");
            bail!("Build process failed");
        }
        CargoBuildStatus::Aborted => {
            eprintln!("Build process was aborted!");
            bail!("Build process was aborted");
        }
    }

    // print logs
    // loop {
    //     match logs_rx.recv() {
    //         Ok(log_line) => {
    //             print!("{}", log_line);
    //         }
    //         Err(_) => {
    //             break;
    //         }
    //     }
    // }

    // wait for first trace item
    loop {
        if first_trace_item_received.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // run executor steps
    let instance = TracingInstance::new(trace_rx);
    visualizer::run_main_tui(instance, logs_recver).context("Failed running TUI")?;

    // pipe output to visualizer

    // visualize output

    // filter output (without tracing messages)

    // show other logs

    cargo_child_process
        .kill()
        .context("Tried killing Cargo Run Child Process")?;
    Ok(())
}
