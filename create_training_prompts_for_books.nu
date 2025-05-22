# For each exaggerated_prompt:
#   - If exaggerated_prompt is empty, store prompt_without_reference_text in exagerated_prompt
#   - Substitute all instances of "The Text " with "A Mind For Numbers"
#   - Store the result as an additional record named training_prompt:
#      - if answer contains "<nothink>\n\n</nothink>" add "\n\n/nothink" tag to training_prompt:
#!/opt/homebrew/bin/nu

def main [input_file: string, text_replacement: string] {
  print $"Working on file: ($input_file)"

  open $input_file
  | each { |row|
      let updated_row = if ($row.exagerated_prompt | str trim | is-empty) {
        $row | update exagerated_prompt $row.prompt_without_reference_text
      } else {
        $row
      }

      # Optional: add a new training_prompt field with substitutions
      let training_prompt = ($updated_row.exagerated_prompt 
        | str replace --all --regex '(?i)the text' $text_replacement
        | str replace --all --regex '(?i)the reference text' $text_replacement)

      let training_prompt = ($training_prompt 
        | str replace --all --regex '\\n' "\n")

      let training_prompt = if ($training_prompt =~ "\\nothink") {
        $training_prompt | str replace --all --regex "\\nothink" ""
      } else {
        $training_prompt
      }

      let training_prompt = if ($row.answer | str contains "<think>\n\n</think>") {
        $training_prompt + "\n\n/nothink" | str trim
      } else {
        $training_prompt | str trim
      }

      $updated_row | insert training_prompt $training_prompt
  }
  | collect
  | save $input_file -f
}
