use super::*;
use regex::Regex;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(crate) enum WorkerCommand {
    OpenTraceFile(String),
    CloseTraceFile,
    Query {
        regex: Regex,
        max_results: u32,
    },
    GetProcessDetail {
        sequence_number: u64,
        command_string_offset: StringIndex,
    },
}

#[derive(Debug)]
pub(crate) enum WorkerResponse {
    OpenFailed(trace_reader::Error),
    OpenSucceeded,
    QueryResult {
        dir: String,
        name: String,
        command_string_offset: StringIndex,
    },
    QueryDone {
        num_records_scanned: u64,
        elapsed: Duration,
    },
    ProcessDetail {
        sequence_number: u64,
        command_string: String,
    },
}

pub(crate) fn worker_thread(
    commands: mpsc::Receiver<WorkerCommand>,
    mut responses: Sender<WorkerResponse>,
) {
    let mut trace_file_opt: Option<TraceFile> = None;
    let mut process_strings: Option<trace_reader::StringTable> = None;

    while let Ok(command) = commands.recv() {
        match command {
            WorkerCommand::OpenTraceFile(filename) => {
                // Drop the existing trace file.
                trace_file_opt = None;

                match TraceFile::open_file(&filename) {
                    Ok(trace_file) => {
                        responses.send(WorkerResponse::OpenSucceeded);
                        trace_file_opt = Some(trace_file);
                    }
                    Err(e) => {
                        responses.send(WorkerResponse::OpenFailed(e));
                    }
                }
            }

            WorkerCommand::CloseTraceFile => {
                trace_file_opt = None;
            }

            WorkerCommand::Query { regex, max_results } => {
                let mut num_records_scanned: u64 = 0;
                let time_started = Instant::now();
                if let Some(trace_file) = trace_file_opt.as_mut() {
                    let _ = process_query(
                        &regex,
                        max_results,
                        trace_file,
                        &mut responses,
                        &mut process_strings,
                        &mut num_records_scanned,
                    );
                }
                let elapsed = time_started.elapsed();

                responses.send(WorkerResponse::QueryDone {
                    num_records_scanned,
                    elapsed,
                });
            }

            WorkerCommand::GetProcessDetail {
                sequence_number,
                command_string_offset,
            } => {
                let command_string = if let Some(process_strings) = process_strings.as_mut() {
                    process_strings
                        .get_string(command_string_offset)
                        .to_string()
                } else {
                    "<error>".to_string()
                };
                responses.send(WorkerResponse::ProcessDetail {
                    sequence_number,
                    command_string,
                });
            }
        }
    }
}

fn process_query(
    query: &Regex,
    max_results: u32,
    trace_file: &mut TraceFile,
    responses: &mut Sender<WorkerResponse>,
    process_strings_opt: &mut Option<StringTable>,
    num_records_scanned: &mut u64,
) -> trace_reader::Result<()> {
    let (mut process_strings, process_table) = trace_file.read_process_table()?;

    let mut num_results: u32 = 0;

    for entry in process_table {
        let command_string = process_strings.get_string(entry.command_string_offset);

        *num_records_scanned += 1;

        if query.is_match(command_string) {
            let name = process_strings
                .get_string(entry.name_string_offset)
                .to_string();
            let dir = process_strings
                .get_string(entry.path_string_offset)
                .to_string();

            responses.send(WorkerResponse::QueryResult {
                name,
                dir,
                command_string_offset: entry.command_string_offset,
            });

            num_results += 1;
            if num_results == max_results {
                break;
            }
        }
    }

    // Keep the string table, so we can answer queries about process details.
    *process_strings_opt = Some(process_strings);

    Ok(())
}
