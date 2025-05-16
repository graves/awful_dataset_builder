use std::{error::Error, fs, path::PathBuf, time::Duration};

use awful_aj::{
    api::ask,
    config::{self, AwfulJadeConfig},
    template::{self, ChatTemplate},
};
use clap::Parser;
use clap::command;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

/// CLI arguments
#[derive(Parser, Debug)]
#[command(name = "awful_dataset_builder")]
#[command(about = "Generate final exam questions from YAML book chunks", long_about = None)]
struct Args {
    /// Path to directory of .yaml book files
    #[arg(short, long)]
    dir: PathBuf,
    /// Configuration file
    #[arg(short, long)]
    config: PathBuf,
    /// Start processing file from this chunk
    #[arg(short, long)]
    start: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct DatasetRow {
    pub prompt: String,
    pub prompt_without_reference_text: String,
    pub exagerated_prompt: String,
    pub answer: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct ExamQuestions {
    pub prompt: Option<String>,
    pub finalExamQuestion1: Option<String>,
    pub finalExamQuestion2: Option<String>,
    pub finalExamQuestion3: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let dir_path = args.dir;
    let conf_file = args.config;
    let start_chunk = args.start;

    let template = template::load_template("book_question_asker").await?;

    let config =
        config::load_config(conf_file.to_str().expect("Not a valid config filename")).unwrap();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            let filename = path.file_name().unwrap().to_string_lossy();
            let contents = fs::read_to_string(&path)?;

            println!("File: {filename}\n");

            let title = filename.split_terminator('.').collect::<Vec<&str>>()[0].trim();
            let exam_questions: Result<Vec<ExamQuestions>, serde_yaml::Error> =
                serde_yaml::from_str(&contents);

            match exam_questions {
                Ok(questions) => {
                    let mut count = start_chunk;
                    let total = questions.len();

                    for exam_questions_row in questions[(start_chunk - 1)..].iter() {
                        println!("Processing chunk {count}/{total}");

                        if let Some(ref final_exam_question) = exam_questions_row.finalExamQuestion1
                        {
                            let reference_text_intro =
                                if let Some(prompt) = exam_questions_row.prompt.clone() {
                                    format!("Here is some reference text:\n\n{prompt}")
                                } else {
                                    "".to_string()
                                };

                            let formatted_question =
                                format!("{reference_text_intro}\n\n{final_exam_question}");
                            let answer =
                                fetch_with_backoff(&config, &formatted_question, &template).await;

                            let prompt = final_exam_question.clone();
                            let _res = write_row_to_file(
                                formatted_question,
                                prompt.clone(),
                                clean_prompt(&prompt),
                                answer,
                                title.to_string(),
                            );
                        }

                        println!("Wrote dataset row for question1");

                        if let Some(ref final_exam_question) = exam_questions_row.finalExamQuestion2
                        {
                            let reference_text_intro =
                                if let Some(prompt) = exam_questions_row.prompt.clone() {
                                    format!("Here is some reference text:\n\n{prompt}")
                                } else {
                                    "".to_string()
                                };

                            let formatted_question = format!(
                                "{reference_text_intro}\n\n{final_exam_question}\n\n\\nothink"
                            );
                            let answer =
                                fetch_with_backoff(&config, &formatted_question, &template).await;

                            let prompt = final_exam_question.clone();
                            let _res = write_row_to_file(
                                formatted_question,
                                prompt.clone(),
                                clean_prompt(&prompt),
                                answer,
                                title.to_string(),
                            );
                        }

                        println!("Wrote dataset row for question2");

                        if let Some(ref final_exam_question) = exam_questions_row.finalExamQuestion3
                        {
                            let reference_text_intro =
                                if let Some(prompt) = exam_questions_row.prompt.clone() {
                                    format!("Here is some reference text:\n\n{prompt}")
                                } else {
                                    "".to_string()
                                };

                            let formatted_question = format!(
                                "{reference_text_intro}\n\n{final_exam_question}\n\n\\nothink"
                            );
                            let answer =
                                fetch_with_backoff(&config, &formatted_question, &template).await;

                            let prompt = final_exam_question.clone();
                            let _res = write_row_to_file(
                                formatted_question,
                                prompt.clone(),
                                clean_prompt(&prompt),
                                answer,
                                title.to_string(),
                            );
                        }

                        println!("Wrote dataset row for question3");

                        count += 1;
                    }
                }
                _ => println!("Failed to deserialize: {filename}"),
            }
        };
    }

    Ok(())
}

pub fn write_row_to_file(
    prompt: String,
    prompt_without_reference_text: String,
    exagerated_prompt: String,
    answer_res: Result<String, Box<dyn std::error::Error>>,
    title: String,
) -> Result<(), Box<dyn std::error::Error>> {
    match answer_res {
        Ok(answer) => {
            let row = DatasetRow {
                prompt,
                prompt_without_reference_text,
                exagerated_prompt,
                answer,
            };

            // Serialize as single-item YAML
            let yaml_entry = serde_yaml::to_string(&vec![row])?; // serialize as 1-item array
            let out_path = format!("{title}_dataset.yaml");

            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&out_path)?;

            writeln!(file, "{yaml_entry}")?;
            println!("Wrote to {out_path}");

            Ok(())
        }
        Err(err) => {
            println!("ERROR: {err:?}");

            Err(err)
        }
    }
}

const MAX_RETRIES: u32 = 5;
const BASE_DELAY_MS: u64 = 500;

async fn fetch_with_backoff(
    config: &AwfulJadeConfig,
    chunk: &str,
    template: &ChatTemplate,
) -> Result<String, Box<dyn std::error::Error>> {
    for attempt in 0..=MAX_RETRIES {
        let res = ask(config, chunk.to_string(), template, None, None).await;

        match res {
            Ok(response) => {
                return Ok(response);
            }
            Err(err) => {
                eprintln!("Request failed: {err}");
            }
        }

        if attempt < MAX_RETRIES {
            let backoff = BASE_DELAY_MS * (2u64.pow(attempt));

            eprintln!("Retrying in {backoff}ms...");

            sleep(Duration::from_millis(backoff)).await;
        }
    }

    Err("Hyper timeout".into())
}

pub fn clean_prompt(input: &str) -> String {
    let lines = input.split("\\n"); // treat string-literal \n as a break

    let step_re = Regex::new(r"\*\*Step \d+\*\*:\s*").unwrap();
    let part_re = Regex::new(r"\*\*Part [A-Z]\*\*:\s*").unwrap();
    let answer_re = Regex::new(r"\*\*Answer Requirement\*\*:\s*").unwrap();

    lines
        .skip(1)
        .map(|line| {
            let line = step_re.replace(line, "");
            let line = part_re.replace(&line, "");
            let line = answer_re.replace(&line, "");
            line.trim().to_string()
        })
        .filter(|line| !line.is_empty()) // remove empty strings
        .collect::<Vec<_>>()
        .join("\\n") // keep escaped newlines
}
