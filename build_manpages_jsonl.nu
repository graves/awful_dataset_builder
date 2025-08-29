#!/opt/homebrew/bin/nu

def main [] {
  let files = [
    "complete/manpages/manpages_dataset.yaml",
  ]

  let all_rows = (
    $files
      | enumerate
      | each { |f|
        open $f.item
      }
      | flatten
  )

  $all_rows
    | each { |it|
      {
        messages: [
          { role: "user", content: ($it.training_prompt | into string | str trim) },
          { role: "assistant", content: ($it.answer | into string | str trim) }
        ]
      } | to json -r
    }
    | str join (char nl)
    | save Manpages.jsonl -f

    print "Chat fine-tuning JSONL saved as: Manpages.jsonl"
}
