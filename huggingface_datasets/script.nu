let new_messages =  open jade_identity_dataset.jsonl --raw
| from json
| get messages
| each { |message|
    let user_message = $message.0
    let assistant_message = $message.1
    let user_message = $user_message | update content $"($user_message.content) /nothink"
    [$user_message, $assistant_message]
    }
| each { |collection|
}
