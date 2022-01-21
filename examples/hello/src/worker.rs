use super::*;
use regex::Regex;
use std::io::{BufRead, BufReader};
use walkdir::WalkDir;

pub(crate) enum WorkerCommand {
    Search {
        root_directory: String,
        regex: Regex,
        max_results: u32,
        recursive: bool,
    },
}

pub(crate) enum WorkerResponse {
    MatchResult {
        file_path: String,
        line: String,
    },
    FileError {
        file_path: String,
        error: std::io::Error,
    },
    SearchDone,
}

pub(crate) fn worker_thread(
    commands: mpsc::Receiver<WorkerCommand>,
    responses: Sender<WorkerResponse>,
) {
    while let Ok(command) = commands.recv() {
        match command {
            WorkerCommand::Search {
                root_directory,
                regex,
                max_results,
                recursive,
            } => {
                let max_depth = if recursive { usize::MAX } else { 1 };
                let mut num_results: u32 = 0;
                for item in WalkDir::new(&root_directory).max_depth(max_depth) {
                    if let Ok(item) = item {
                        let file_path = item.path().as_os_str().to_string_lossy().to_string();
                        let file_result: std::io::Result<()> = (|| {
                            let md = std::fs::metadata(item.path())?;
                            if !md.is_file() {
                                return Ok(());
                            }

                            let f = BufReader::new(std::fs::File::open(item.path())?);
                            for line in f.lines() {
                                let line = line?;
                                if regex.is_match(&line) {
                                    responses.send(WorkerResponse::MatchResult {
                                        file_path: file_path.to_string(),
                                        line: line.trim_end().to_string(),
                                    });
                                }
                            }

                            Ok(())
                        })();
                        match file_result {
                            Ok(()) => {}
                            Err(e) => {
                                responses.send(WorkerResponse::FileError {
                                    file_path: file_path.to_string(),
                                    error: e,
                                });
                            }
                        }
                    }

                    num_results += 1;
                    if num_results >= max_results {
                        break;
                    }
                }
                responses.send(WorkerResponse::SearchDone)
            }
        }
    }
}
