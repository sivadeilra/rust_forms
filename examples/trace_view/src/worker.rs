use super::*;
use regex::Regex;

#[derive(Debug)]
pub(crate) enum WorkerResponse {
    OpenFailed(trace_reader::Error),
    OpenSucceeded,
    QueryResult { dir: String, name: String },
    QueryDone,
}

#[derive(Debug)]
pub(crate) enum WorkerCommand {
    OpenTraceFile(String),
    CloseTraceFile,
    Query { regex: Regex, max_results: u32 },
}

pub(crate) fn worker_thread(
    commands: mpsc::Receiver<WorkerCommand>,
    mut responses: Sender<WorkerResponse>,
) {
    let mut trace_file_opt: Option<TraceFile> = None;

    while let Ok(command) = commands.recv() {
        trace!("worker received command: {:?}", command);
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
                if let Some(trace_file) = trace_file_opt.as_mut() {
                    let _ = process_query(&regex, max_results, trace_file, &mut responses);
                }

                responses.send(WorkerResponse::QueryDone);
            }
        }
    }
}

fn process_query(
    query: &Regex,
    max_results: u32,
    trace_file: &mut TraceFile,
    responses: &mut Sender<WorkerResponse>,
) -> trace_reader::Result<()> {
    let (mut process_strings, process_table) = trace_file.read_process_table()?;

    let mut num_results: u32 = 0;

    for entry in process_table {
        let command_string = process_strings.get_string(entry.command_string_offset);

        if query.is_match(command_string) {
            let name = process_strings
                .get_string(entry.name_string_offset)
                .to_string();
            let dir = process_strings
                .get_string(entry.path_string_offset)
                .to_string();

            responses.send(WorkerResponse::QueryResult { name, dir });

            num_results += 1;
            if num_results == max_results {
                break;
            }
        }
    }

    Ok(())
}
