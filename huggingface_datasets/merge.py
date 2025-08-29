from datasets import load_dataset, concatenate_datasets, DatasetDict

# List of dataset IDs and splits (in your desired order)
dataset_order = [
    ("dougiefresh/grammar_logic_rhetoric_and_math", "train"),
    ("dougiefresh/systems_programming_and_administration", "train"),
    ("dougiefresh/systems_programming_code_conversations", "train"),
    ("dougiefresh/manpages", "train")
]

# Load each dataset
datasets_list = [
    load_dataset(path, split=split)
    for path, split in dataset_order
]

# Concatenate in order
merged_dataset = concatenate_datasets(datasets_list)

train_testvalid = merged_dataset.train_test_split(test_size=0.2)
# Split the 10% test + valid in half test, half valid
test_valid = train_testvalid['test'].train_test_split(test_size=0.5)

# gather everyone if you want to have a single DatasetDict
ds = DatasetDict({
    'train': train_testvalid['train'],
    'test': test_valid['test'],
    'valid': test_valid['train']})

# Save to disk (optional)
ds['train'].to_json("train.jsonl")
ds['test'].to_json("test.jsonl")
ds['valid'].to_json("valid.jsonl")
