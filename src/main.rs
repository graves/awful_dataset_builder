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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, clap::ValueEnum, Ord, Debug)]
enum SourceType {
    Book,
    Manpage,
    Mdbook,
    Tealdeer,
    Code,
}

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
    /// Source type
    #[clap(value_enum)]
    #[arg(long)]
    source_type: SourceType,
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

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct MdbookQuestions {
    pub prompt: Option<String>,
    pub documentationQuestion1: Option<String>,
    pub documentationQuestion2: Option<String>,
    pub documentationQuestion3: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct ManpageQuestions {
    pub prompt: Option<String>,
    pub manpageQuestion1: Option<String>,
    pub manpageQuestion2: Option<String>,
    pub manpageQuestion3: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct TealdeerQuestions {
    pub prompt: Option<String>,
    pub tealdeerQuestion1: Option<String>,
    pub tealdeerQuestion2: Option<String>,
    pub tealdeerQuestion3: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct CodeQuestions {
    pub prompt: Option<String>,
    pub codeQuestion1: Option<String>,
    pub codeQuestion2: Option<String>,
    pub codeQuestion3: Option<String>,
}

enum AnyQuestions {
    Book(Vec<ExamQuestions>),
    Mdbook(Vec<MdbookQuestions>),
    Manpage(Vec<ManpageQuestions>),
    Tealdeer(Vec<TealdeerQuestions>),
    Code(Vec<CodeQuestions>),
}

impl AnyQuestions {
    fn as_question_vec(&self) -> Vec<&dyn QuestionSet> {
        match self {
            AnyQuestions::Book(vec) => vec.iter().map(|x| x as &dyn QuestionSet).collect(),
            AnyQuestions::Mdbook(vec) => vec.iter().map(|x| x as &dyn QuestionSet).collect(),
            AnyQuestions::Manpage(vec) => vec.iter().map(|x| x as &dyn QuestionSet).collect(),
            AnyQuestions::Tealdeer(vec) => vec.iter().map(|x| x as &dyn QuestionSet).collect(),
            AnyQuestions::Code(vec) => vec.iter().map(|x| x as &dyn QuestionSet).collect(),
        }
    }
}

trait QuestionSet {
    fn get_prompt(&self) -> Option<&String>;
    fn get_question1(&self) -> Option<&String>;
    fn get_question2(&self) -> Option<&String>;
    fn get_question3(&self) -> Option<&String>;
}

impl QuestionSet for ExamQuestions {
    fn get_prompt(&self) -> Option<&String> {
        self.prompt.as_ref()
    }
    fn get_question1(&self) -> Option<&String> {
        self.finalExamQuestion1.as_ref()
    }
    fn get_question2(&self) -> Option<&String> {
        self.finalExamQuestion2.as_ref()
    }
    fn get_question3(&self) -> Option<&String> {
        self.finalExamQuestion3.as_ref()
    }
}

impl QuestionSet for MdbookQuestions {
    fn get_prompt(&self) -> Option<&String> {
        self.prompt.as_ref()
    }
    fn get_question1(&self) -> Option<&String> {
        self.documentationQuestion1.as_ref()
    }
    fn get_question2(&self) -> Option<&String> {
        self.documentationQuestion2.as_ref()
    }
    fn get_question3(&self) -> Option<&String> {
        self.documentationQuestion3.as_ref()
    }
}

impl QuestionSet for ManpageQuestions {
    fn get_prompt(&self) -> Option<&String> {
        self.prompt.as_ref()
    }
    fn get_question1(&self) -> Option<&String> {
        self.manpageQuestion1.as_ref()
    }
    fn get_question2(&self) -> Option<&String> {
        self.manpageQuestion2.as_ref()
    }
    fn get_question3(&self) -> Option<&String> {
        self.manpageQuestion3.as_ref()
    }
}

impl QuestionSet for TealdeerQuestions {
    fn get_prompt(&self) -> Option<&String> {
        self.prompt.as_ref()
    }
    fn get_question1(&self) -> Option<&String> {
        self.tealdeerQuestion1.as_ref()
    }
    fn get_question2(&self) -> Option<&String> {
        self.tealdeerQuestion2.as_ref()
    }
    fn get_question3(&self) -> Option<&String> {
        self.tealdeerQuestion3.as_ref()
    }
}

impl QuestionSet for CodeQuestions {
    fn get_prompt(&self) -> Option<&String> {
        self.prompt.as_ref()
    }
    fn get_question1(&self) -> Option<&String> {
        self.codeQuestion1.as_ref()
    }
    fn get_question2(&self) -> Option<&String> {
        self.codeQuestion2.as_ref()
    }
    fn get_question3(&self) -> Option<&String> {
        self.codeQuestion3.as_ref()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let dir_path = args.dir;
    let conf_file = args.config;
    let start_chunk = args.start;
    let source_type = args.source_type;

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

            let title = if source_type == SourceType::Manpage {
                "manpages"
            } else {
                filename.split_terminator('.').collect::<Vec<&str>>()[0].trim()
            };

            let any_questions = match source_type {
                SourceType::Book => AnyQuestions::Book(serde_yaml::from_str(&contents)?),
                SourceType::Mdbook => AnyQuestions::Mdbook(serde_yaml::from_str(&contents)?),
                SourceType::Manpage => AnyQuestions::Manpage(serde_yaml::from_str(&contents)?),
                SourceType::Tealdeer => AnyQuestions::Tealdeer(serde_yaml::from_str(&contents)?),
                SourceType::Code => AnyQuestions::Code(serde_yaml::from_str(&contents)?),
            };

            let question_rows = any_questions.as_question_vec();
            let mut count = start_chunk;
            let total = question_rows.len();

            for row in question_rows.into_iter().skip(start_chunk - 1) {
                println!("Processing chunk {count}/{total}");

                for (i, question) in [
                    row.get_question1(),
                    row.get_question2(),
                    row.get_question3(),
                ]
                .into_iter()
                .enumerate()
                {
                    if let Some(q) = question {
                        let intro = row
                            .get_prompt()
                            .map(|p| format!("Here is some reference text:\n\n{p}"))
                            .unwrap_or_default();

                        let formatted_question = if i == 0 {
                            format!("{intro}\n\n{q}")
                        } else {
                            format!("{intro}\n\n{q}\n\n\\nothink")
                        };

                        let answer =
                            fetch_with_backoff(&config, &formatted_question, &template).await;

                        let prompt = q.clone();
                        let _res = write_row_to_file(
                            formatted_question,
                            prompt.clone(),
                            clean_prompt(&prompt),
                            answer,
                            title.to_string(),
                        );

                        println!("Wrote dataset row for question{}", i + 1);
                    }
                }

                count += 1;
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
