#!/usr/bin/env python3
"""Fine-tune kompress (ModernBERT-149M) on dogfeed compression pairs.

Loads (input, output) JSONL pairs and fine-tunes a ModernBERT model
with LoRA to improve the compression/rewriting capability.

The training task: given the original text, predict the compressed version.
This is treated as a seq2seq task — input → output generation.

Usage:
    python3 scripts/feedback-loop/train-kompress.py \
        --data /tmp/kompress-train.jsonl \
        --output PeetPedro/hf-model-repo \
        --epochs 3
"""
import argparse
import json
import os
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description="Fine-tune kompress on dogfeed data")
    parser.add_argument("--data", required=True, help="Training JSONL file (input/output pairs)")
    parser.add_argument("--base-model", default="answerdotai/ModernBERT-base",
                        help="Base model for fine-tuning")
    parser.add_argument("--output", default=None,
                        help="HF repo to push LoRA adapter to")
    parser.add_argument("--epochs", type=int, default=3,
                        help="Training epochs")
    parser.add_argument("--batch-size", type=int, default=8,
                        help="Batch size")
    parser.add_argument("--learning-rate", type=float, default=2e-4,
                        help="Learning rate")
    parser.add_argument("--lora-r", type=int, default=16,
                        help="LoRA rank")
    parser.add_argument("--lora-alpha", type=int, default=32,
                        help="LoRA alpha")
    parser.add_argument("--dry-run", action="store_true",
                        help="Validate data only, skip training")
    args = parser.parse_args()

    # --- Load data ---
    data_path = Path(args.data)
    if not data_path.exists():
        print(f"ERROR: data file not found: {args.data}", file=sys.stderr)
        sys.exit(1)

    pairs = []
    with open(data_path) as f:
        for line in f:
            line = line.strip()
            if line:
                pairs.append(json.loads(line))

    print(f"Loaded {len(pairs)} training pairs")

    if args.dry_run:
        print("Dry run — data validated, skipping training.")
        print(f"  Base model: {args.base_model}")
        print(f"  Pairs: {len(pairs)}")
        print(f"  Epochs: {args.epochs}")
        print(f"  LoRA rank: {args.lora_r}")
        # Show sample
        if pairs:
            sample = pairs[0]
            print(f"\n  Sample pair:")
            print(f"    Input  ({len(sample['input'])} chars): {sample['input'][:120]}...")
            print(f"    Output ({len(sample['output'])} chars): {sample['output'][:120]}...")
        return

    # --- Fine-tuning with transformers + peft ---
    try:
        import torch
        from transformers import (
            AutoTokenizer,
            AutoModelForSeq2SeqLM,
            Seq2SeqTrainingArguments,
            Seq2SeqTrainer,
            DataCollatorForSeq2Seq,
        )
        from peft import LoraConfig, get_peft_model, TaskType
    except ImportError as e:
        print(f"ERROR: missing dependency: {e}", file=sys.stderr)
        print("Install: pip install torch transformers peft datasets accelerate", file=sys.stderr)
        sys.exit(1)

    device = "cuda" if torch.cuda.is_available() else "cpu"
    print(f"Using device: {device}")

    tokenizer = AutoTokenizer.from_pretrained(args.base_model)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    model = AutoModelForSeq2SeqLM.from_pretrained(args.base_model).to(device)

    # LoRA config
    lora_config = LoraConfig(
        r=args.lora_r,
        lora_alpha=args.lora_alpha,
        target_modules=["q_proj", "v_proj"],
        lora_dropout=0.1,
        bias="none",
        task_type=TaskType.SEQ_2_SEQ_LM,
    )
    model = get_peft_model(model, lora_config)
    model.print_trainable_parameters()

    # Tokenize dataset
    inputs = [p["input"] for p in pairs]
    outputs = [p["output"] for p in pairs]

    model_inputs = tokenizer(inputs, max_length=512, truncation=True, padding=True)
    labels = tokenizer(outputs, max_length=512, truncation=True, padding=True)

    # Build torch dataset
    class KompressDataset(torch.utils.data.Dataset):
        def __init__(self, model_inputs, labels):
            self.input_ids = model_inputs["input_ids"]
            self.attention_mask = model_inputs["attention_mask"]
            self.labels = labels["input_ids"]

        def __len__(self):
            return len(self.input_ids)

        def __getitem__(self, idx):
            return {
                "input_ids": torch.tensor(self.input_ids[idx]),
                "attention_mask": torch.tensor(self.attention_mask[idx]),
                "labels": torch.tensor(self.labels[idx]),
            }

    dataset = KompressDataset(model_inputs, labels)

    # Training args
    training_args = Seq2SeqTrainingArguments(
        output_dir="./kompress-checkpoints",
        per_device_train_batch_size=args.batch_size,
        num_train_epochs=args.epochs,
        learning_rate=args.learning_rate,
        logging_steps=10,
        save_strategy="epoch",
        report_to="none",
    )

    trainer = Seq2SeqTrainer(
        model=model,
        args=training_args,
        train_dataset=dataset,
        tokenizer=tokenizer,
        data_collator=DataCollatorForSeq2Seq(tokenizer, model=model),
    )

    print(f"Starting fine-tuning: {len(pairs)} pairs, {args.epochs} epochs")
    trainer.train()

    # Save LoRA adapter
    adapter_path = Path("./kompress-lora-adapter")
    model.save_pretrained(adapter_path)
    tokenizer.save_pretrained(adapter_path)
    print(f"LoRA adapter saved to {adapter_path}")

    # Push to HF Hub if output specified
    if args.output:
        try:
            from huggingface_hub import HfApi
            api = HfApi()
            api.upload_folder(
                folder_path=str(adapter_path),
                repo_id=args.output,
                repo_type="model",
                commit_message="feat(kompress): fine-tune on dogfeed compression pairs [auto]",
            )
            print(f"Pushed adapter to https://huggingface.co/{args.output}")
        except Exception as e:
            print(f"WARNING: HF push failed: {e}", file=sys.stderr)
            print(f"Adapter saved locally at {adapter_path}")


if __name__ == "__main__":
    main()
