# 🏗️ **Awful Dataset Builder: Turn Reference Text/Exam Question mappings into Question/Answer pairs!** 📚

> *“Turn your study notes into interrogation scripts for robots.”*

```
                                           __
                               _____....--' .'
                     ___...---'._ o      -`(
           ___...---'            \   .--.  `\
 ___...---'                      |   \   \ `|
|                                |o o |  |  |
|                                 \___'.-`.  '.
|                                      |   `---'
'^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^' LGB
```

---

## 🧠 **What It Does**  
`awful_dataset_builder` is a command-line tool that takes structured YAML files (from `awkward_knowledge_synthesizer`) and generates **question-answer pairs** using Large Language Models (LLMs). It's your go-to tool for building datasets for finetuning LLMs.

---

## 🎯 **Features**  
- ✅ **Multi-source support**: Books, manpages, mdbooks, tealdeer (command-line snippets), and code files can be turned into exam questions by [awful_knowledge_synthesizer](https://github.com/graves/awful_knowledge_synthesizer).  
- 🧠 **LLM-powered QA pairs**: Fetches answers for final exam questions using `awful_aj`.  
- 📄 **YAML output**: Saves results as structured YAML files (e.g., `math_questions.yaml`).  
- 🔄 **Chunked processing**: Splits text into chunks for robust LLM queries.  

---

## 📦 **How to Use**  
### 🔧 Sample Command  
```bash
λ awful_dataset_builder --dir ./books --config config.yaml --source_type Book --start 1
```
- `--dir`: Path to YAML files (e.g., `books/`).  
- `--config`: Configuration file for LLM API (OpenAI, etc.).  
- `--source_type`: Choose from: `Book`, `Manpage`, `Mdbook`, `Tealdeer`, or `Code`.  
- `--start`: Skip files from this chunk (useful for parallel processing).  

### 📄 Example Output  
For a book YAML file:
```yaml
title: "Calculus for Dummies"
chunks:
  - "What is the derivative of f(x) = x²?"
```
Output:
```yaml
- prompt: "Here is some reference text:\n\nWhat is the derivative of f(x) = x²?"
  answer: "The derivative of $ f(x) = x^2 $ is $ 2x $."
```

---

## 🤓 **How It Works**  
1. **Parse YAML**: Extracts structured Reference Text to Final Exam Question mappings.  
2. **LLM Query**: Uses templates to generate questions and fetch answers via `awful_aj`.  
3. **Output**: Saves QA pairs in YAML format (e.g., `math_questions.yaml`).  

---

## 🧪 **Supported Sources**  
| Source Type | Description |
|-------------|-------------|
| `Book`      | YAML files with questions generated from book excerpts (e.g., `"Title: Math for Dummies"`) |
| `Manpage`   | YAML files with questions generated from manpages excerpts |
| `Mdbook`    | YAML files withquestions generated from Markdown excerpts (`mdbook` built documentation) |
| `Tealdeer`  | YAML files with questions generated from Command-line snippets (`tldr` commands) |
| `Code`      | YAML files with questions generated from C, Rust, or Assembly source code repositories (language-aware tokenization) |

---

## 🧪 **Implementation Notes**  
- Uses `clap` for CLI parsing.  
- Relies on `serde`, `tokio`, and `regex`.  
- LLM queries are handled with exponential backoff (`MAX_RETRIES = 5`).  

---

## 🧠 **Contributing**  
- Report bugs or suggest improvements via GitHub Issues.  
- Fork and extend to support new source types!  

---

## 🧠 **Final Thoughts**  
Building datasets is the most dificult, time-consuming labor involved with the Synthetic Finetuning of LLMs. A well thought out workflow using [Awful Book Sanitizer](https://github.com/graves/awful_book_sanitizer), [Awful Knowledge Synthesizer](https://github.com/graves/awful_knowledge_synthesizer), and [Awful Dataset Builder](https://github.com/graves/awful_dataset_builder) will allow you to experiment with your wildest curiosities about human language, on the cutting edge of technological advancement for as long as written language exists 🎉  

You can find Open Source datasets I've generated using these tools on [Huggingface](https://huggingface.co/dougiefresh/datasets)

> *Remember: Even "awful" data can become awesome with the right tool.* 🧠✨
