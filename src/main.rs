//! # awful_dataset_builder
//!
//! A command-line tool that turns YAML chunks (book chapters, manpages, mdBooks,
//! tldr pages, or code docs) into supervised training rows by asking an LLM to
//! answer “final exam”–style questions. Each answer is appended to a `*_dataset.yaml`
//! file alongside several prompt variants for downstream use.
//!
//! ## How it works
//! 1. Load a **chat template** (`book_question_asker`) and a runtime **config** for the
//!    LLM backend (via the `awful_aj` crate).
//! 2. Walk a directory of `.yaml` files. For each file:
//!    - Deserialize it into one of several question-set schemas (based on `--source-type`).
//!    - For each chunk starting at `--start`, send up to three questions to the model,
//!      using exponential backoff on failures.
//!    - Write one dataset row per question to `<title>_dataset.yaml` (append-only).
//!
//! ## Input formats
//! The input YAML must deserialize into one of the types represented by
//! [`SourceType`]. See the `*Questions` structs for expected keys.
//!
//! - [`ExamQuestions`] (Book)
//! - [`MdbookQuestions`] (mdBook)
//! - [`ManpageQuestions`] (manpages)
//! - [`TealdeerQuestions`] (tealdeer / tldr)
//! - [`CodeQuestions`] (code-focused docs)
//!
//! Each “questions row” contains an optional `prompt` field (reference text) and
//! up to three question fields whose names vary by source type.
//!
//! ## Output format
//! Each successful model call produces a serialized [`DatasetRow`] and is appended
//! (as a single-item YAML array) to `<title>_dataset.yaml` where `title` is either
//! the basename of the source file (without `.yaml`) or `manpages` for `--source-type manpage`.
//!
//! ## CLI
//! ```text
//! awful_dataset_builder \
//!   --dir ./chunks \
//!   --config ./awfuljade.yaml \
//!   --start 1 \
//!   --source-type book
//! ```
//!
//! - `--dir` points to YAML files with question rows.
//!
//! - `--config` is passed to `awful_aj::config::load_config` (see that crate for schema).
//!
//! - `--start` (1-based) skips rows prior to this index within each file.
//!
//! - `--source-type` selects the question schema to deserialize and how to read its fields.
//!
//! ## Failure behavior
//! - Model calls retry with exponential backoff (see [`fetch_with_backoff`]) up to
//!   [`MAX_RETRIES`] times; persistent failure yields a `"Hyper timeout"` error.
//! - File IO and YAML (de)serialization errors bubble up and are printed; failed
//!   rows are **not** written.
//!
//! ## Examples
//! ```bash
//! # Generate dataset rows from a directory of book-chunk YAMLs
//! awful_dataset_builder --dir ./book_chunks --config ./aj.yaml --start 1 --source-type book
//!
//! # Start midway through a large mdBook file
//! awful_dataset_builder --dir ./mdbook_yaml --config ./aj.yaml --start 42 --source-type mdbook
//! ```
//!
//! ## Notes
//! - The helper [`clean_prompt`] removes step/part/answer headings from prompts while
//!   preserving escaped newlines (`\\n`), producing a concise `prompt_without_reference_text`.
//! - When a reference `prompt` exists, question #2 and #3 are suffixed with `\\nothink`
//!   to influence the model’s behavior.

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

/// The semantic source of a question set, used to pick the YAML schema.
///
/// This also determines which `*_dataset.yaml` file receives the output.
///
/// - [`SourceType::Book`] → [`ExamQuestions`]
/// - [`SourceType::Mdbook`] → [`MdbookQuestions`]
/// - [`SourceType::Manpage`] → [`ManpageQuestions`]
/// - [`SourceType::Tealdeer`] → [`TealdeerQuestions`]
/// - [`SourceType::Code`] → [`CodeQuestions`]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, clap::ValueEnum, Ord, Debug)]
enum SourceType {
    Book,
    Manpage,
    Mdbook,
    Tealdeer,
    Code,
}

/// Command-line arguments for `awful_dataset_builder`.
#[derive(Parser, Debug)]
#[command(name = "awful_dataset_builder")]
#[command(about = "Generate final exam questions from YAML book chunks", long_about = None)]
struct Args {
    /// Path to a directory containing `.yaml` files to process.
    #[arg(short, long)]
    dir: PathBuf,

    /// Path to the LLM/backend configuration file (see `awful_aj` crate).
    #[arg(short, long)]
    config: PathBuf,

    /// 1-based index of the first chunk in each file to process.
    ///
    /// Chunks before this index are skipped. Useful for resuming long runs.
    #[arg(short, long)]
    start: usize,

    /// Schema to expect for each `.yaml` file and output bucket naming.
    #[clap(value_enum)]
    #[arg(long)]
    source_type: SourceType,
}

/// A single supervised row appended to `<title>_dataset.yaml`.
///
/// - `prompt`: The exact formatted prompt sent to the model (reference text + question).
/// - `prompt_without_reference_text`: The original question text (no reference block).
/// - `exagerated_prompt`: A cleaned/“exaggerated” prompt variant (see [`clean_prompt`]).
/// - `answer`: The model’s answer string.
#[derive(Debug, Deserialize, Serialize)]
struct DatasetRow {
    pub prompt: String,
    pub prompt_without_reference_text: String,
    pub exagerated_prompt: String,
    pub answer: String,
}

/// Book-style question row (up to three questions plus optional reference prompt).
#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct ExamQuestions {
    pub prompt: Option<String>,
    pub finalExamQuestion1: Option<String>,
    pub finalExamQuestion2: Option<String>,
    pub finalExamQuestion3: Option<String>,
}

/// mdBook-style question row.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct MdbookQuestions {
    pub prompt: Option<String>,
    pub documentationQuestion1: Option<String>,
    pub documentationQuestion2: Option<String>,
    pub documentationQuestion3: Option<String>,
}

/// Manpage-style question row.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct ManpageQuestions {
    pub prompt: Option<String>,
    pub manpageQuestion1: Option<String>,
    pub manpageQuestion2: Option<String>,
    pub manpageQuestion3: Option<String>,
}

/// tealdeer (tldr)-style question row.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct TealdeerQuestions {
    pub prompt: Option<String>,
    pub tealdeerQuestion1: Option<String>,
    pub tealdeerQuestion2: Option<String>,
    pub tealdeerQuestion3: Option<String>,
}

/// Code-/API-style question row.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct CodeQuestions {
    pub prompt: Option<String>,
    pub codeQuestion1: Option<String>,
    pub codeQuestion2: Option<String>,
    pub codeQuestion3: Option<String>,
}

/// A type-erased container for any supported question schema.
///
/// This allows uniform iteration over questions regardless of the underlying source type.
enum AnyQuestions {
    Book(Vec<ExamQuestions>),
    Mdbook(Vec<MdbookQuestions>),
    Manpage(Vec<ManpageQuestions>),
    Tealdeer(Vec<TealdeerQuestions>),
    Code(Vec<CodeQuestions>),
}

impl AnyQuestions {
    /// Returns a homogeneous vector of `&dyn QuestionSet` for iteration.
    ///
    /// This is primarily used to keep the main processing loop generic.
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

/// A uniform view over any question row, exposing an optional reference prompt
/// and up to three question fields.
///
/// Implemented by each `*Questions` struct.
trait QuestionSet {
    /// Optional reference text for the questions in this row.
    fn get_prompt(&self) -> Option<&String>;
    /// The first question in the row, if any.
    fn get_question1(&self) -> Option<&String>;
    /// The second question in the row, if any.
    fn get_question2(&self) -> Option<&String>;
    /// The third question in the row, if any.
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

/// Entry point: iterates YAML files, queries the model, and appends dataset rows.
///
/// Returns `Ok(())` on success; IO/YAML/model errors are surfaced as `Err`.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let dir_path = args.dir;
    let conf_file = args.config;
    let start_chunk = args.start;
    let source_type = args.source_type;

    // Load the chat template used to format requests to the model.
    let template = template::load_template("book_question_asker").await?;

    // Load runtime configuration for the model backend.
    let config =
        config::load_config(conf_file.to_str().expect("Not a valid config filename")).unwrap();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            let filename = path.file_name().unwrap().to_string_lossy();
            let contents = fs::read_to_string(&path)?;

            println!("File: {filename}\n");

            // Output title: basename for most sources; fixed "manpages" for manpage mode.
            let title = if source_type == SourceType::Manpage {
                "manpages"
            } else {
                filename.split_terminator('.').collect::<Vec<&str>>()[0].trim()
            };

            // Deserialize to the appropriate question schema.
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

            // Process chunks starting at the 1-based index `start_chunk`.
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
                        // Inline reference text if present.
                        let intro = row
                            .get_prompt()
                            .map(|p| format!("Here is some reference text:\n\n{p}"))
                            .unwrap_or_default();

                        // Add `\nothink` for questions beyond the first to steer the model.
                        let formatted_question = if i == 0 {
                            format!("{intro}\n\n{q}")
                        } else {
                            format!("{intro}\n\n{q}\n\n\\nothink")
                        };

                        // Invoke the model with exponential backoff.
                        let answer =
                            fetch_with_backoff(&config, &formatted_question, &template).await;

                        // Prepare and append the dataset row (on success).
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

/// Append a single [`DatasetRow`] to `<title>_dataset.yaml`.
///
/// On success, the row is serialized as a **single-item YAML array** (one per line)
/// and appended (creating the file if needed).
///
/// # Arguments
/// - `prompt`: The exact formatted prompt sent to the model.
/// - `prompt_without_reference_text`: The human-facing question text (no ref block).
/// - `exagerated_prompt`: A normalized/cleaned version of the question (see [`clean_prompt`]).
/// - `answer_res`: The model call result; only `Ok(answer)` is written.
/// - `title`: Output file prefix (see module docs for rules).
///
/// # Errors
/// Returns any serialization or filesystem errors, or forwards the original error
/// contained in `answer_res` when the model call failed.
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

/// Maximum number of retries for a model request.
const MAX_RETRIES: u32 = 5;
/// Initial backoff in milliseconds; doubles on each retry.
const BASE_DELAY_MS: u64 = 500;

/// Call the model with exponential backoff and jitter-free delays.
///
/// Attempts the request up to [`MAX_RETRIES`] + 1 times. Between attempts,
/// wait `BASE_DELAY_MS * 2^attempt` milliseconds.
///
/// # Errors
/// Returns `"Hyper timeout"` if all attempts fail. Intermediate errors are
/// logged to stderr.
///
/// # Examples
/// ```no_run
/// # async fn demo(cfg: &awful_aj::config::AwfulJadeConfig, t: &awful_aj::template::ChatTemplate)
/// # -> Result<(), Box<dyn std::error::Error>> {
/// let answer = fetch_with_backoff(cfg, "What is a monoid?", t).await?;
/// println!("{answer}");
/// # Ok(())
/// # }
/// ```
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

/// Normalize a question prompt by removing bolded “Step/Part/Answer Requirement”
/// headings and trimming whitespace, while preserving escaped newlines (`\\n`).
///
/// The function treats the literal two-character sequence `\` + `n` as a line break
/// delimiter to avoid collapsing formatted prompts, and then rejoins the cleaned
/// lines using the same escaped newline sequence.
///
/// # Examples
/// ```
/// let raw = "**Step 1**: Read\\n**Part A**: Explain\\nReal question here\\n\\n**Answer Requirement**: ...";
/// let cleaned = clean_prompt(raw);
/// assert!(cleaned.contains("Real question here"));
/// assert!(!cleaned.contains("Step 1"));
/// assert!(!cleaned.contains("Part A"));
/// assert!(!cleaned.contains("Answer Requirement"));
/// assert!(cleaned.contains("\\n")); // escaped newlines preserved
/// ```
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
