#!/opt/homebrew/bin/nu

def get-if-exists [record field: string] {
  if ($record | columns | any { |col| $col == $field }) {
    $record | get $field
  } else {
    ""
  }
}

def main [input_file: string] {
  print $"Working on file: ($input_file)"

  open $input_file
    | enumerate
    | group-by { |row| $row.index // 3 }
    | values
    | each { |chunk|
        let first_row = ($chunk | get 0?)
        let second_row = ($chunk | get 1?)
        let third_row = ($chunk | get 2?)

        {
          first_interaction: {
            training_prompt: (
              if $first_row != null {
                get-if-exists $first_row.item "first_training_prompt"
              } else {
                ""
              }
            ),
            answer: (
              if $first_row != null {
                get-if-exists $first_row.item "answer"
              } else {
                ""
              }
            )
          },
          second_interaction: {
            training_prompt: (
              if $second_row != null {
                get-if-exists $second_row.item "training_prompt"
              } else {
                ""
              }
            ),
            answer: (
              if $second_row != null {
                get-if-exists $second_row.item "answer"
              } else {
                ""
              }
            )
          },
          third_interaction: {
            training_prompt: (
              if $third_row != null {
                get-if-exists $third_row.item "training_prompt"
              } else {
                ""
              }
            ),
            answer: (
              if $third_row != null {
                get-if-exists $third_row.item "answer"
              } else {
                ""
              }
            )
          }
        }
    }
    | to yaml
    | save $"($input_file)_grouped_output.yaml"
}
