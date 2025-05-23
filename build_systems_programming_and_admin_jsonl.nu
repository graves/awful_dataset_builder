#!/opt/homebrew/bin/nu:wq

def main [] {
  let files = [
    "complete/books/SystemsProgramming/SystemsProgramming.csv",
    "complete/mdbooks/MdBooks.csv",
    "complete/tealdeer/Tealdeer.csv"
  ]

  let all_rows = (
    $files
      | enumerate
      | each { |f|
        let rows = open $f.item
        if $f.index == 0 {
          $rows  # keep header
        } else {
          $rows | skip 1  # skip header in subsequent files
        }
      }
      | flatten
  )

  $all_rows
    | each { |it|
      {
        messages: [
          { role: "user", content: ($it.question | into string | str trim) },
          { role: "assistant", content: ($it.answer | into string | str trim) }
        ]
      } | to json -r
    }
    | str join (char nl)
    | save SystemsProgrammingAndAdministration_chat.jsonl -f

    print "Chat fine-tuning JSONL saved as: SystemsProgrammingAndAdministration_chat.jsonl"
}
