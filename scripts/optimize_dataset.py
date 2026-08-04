import os
import re
import json
import glob
import pandas as pd
from datetime import datetime

def main():
    repo_dir = "/Users/lodripeter/workspace/peterlodri-sec/ultrawhale-dogfood-temp"
    print(f"Starting dataset optimization in {repo_dir}...")

    # Pattern to match loop files: dogfeed-loop-<index>-<date>-<time>.jsonl
    pattern = os.path.join(repo_dir, "dogfeed-loop-*.jsonl")
    files = glob.glob(pattern)
    
    print(f"Found {len(files)} loop JSONL files.")
    
    all_records = []
    
    # Regex to extract loop index and timestamp from filename
    # Example: dogfeed-loop-103-20260624-021543.jsonl
    fn_regex = re.compile(r"dogfeed-loop-(\d+)-(\d{8})-(\d{6})\.jsonl")
    
    for f in files:
        filename = os.path.basename(f)
        match = fn_regex.match(filename)
        
        loop_index = None
        timestamp_str = None
        
        if match:
            loop_index = int(match.group(1))
            date_str = match.group(2)
            time_str = match.group(3)
            try:
                # Parse timestamp: YYYYMMDD-HHMMSS
                dt = datetime.strptime(f"{date_str}-{time_str}", "%Y%m%d-%H%MS")
                timestamp_str = dt.isoformat() + "Z"
            except Exception:
                pass
        
        # Read the JSONL file
        with open(f, "r", encoding="utf-8") as infile:
            for line in infile:
                if not line.strip():
                    continue
                try:
                    record = json.loads(line)
                    # Inject SOTA metadata columns
                    record["loop_index"] = loop_index
                    if "timestamp" not in record or not record["timestamp"]:
                        record["timestamp"] = timestamp_str
                    
                    # Ensure standard keys exist
                    for key in ["text", "reference", "role", "source", "topic"]:
                        if key not in record:
                            record[key] = None
                            
                    all_records.append(record)
                except Exception as e:
                    print(f"Error parsing line in {filename}: {e}")

    if not all_records:
        print("No records found to consolidate!")
        return

    # Convert to DataFrame
    df = pd.DataFrame(all_records)
    
    # Sort chronologically by loop_index and timestamp
    df = df.sort_values(by=["loop_index", "timestamp"], ascending=[True, True]).reset_index(drop=True)
    
    # Save as optimized Parquet
    parquet_path = os.path.join(repo_dir, "dogfeed.parquet")
    df.to_parquet(parquet_path, engine="pyarrow", compression="snappy", index=False)
    print(f"Successfully wrote {len(df)} rows to {parquet_path}")

    # Update README.md metadata to point to the new parquet file
    readme_path = os.path.join(repo_dir, "README.md")
    if os.path.exists(readme_path):
        with open(readme_path, "r", encoding="utf-8") as rfile:
            content = rfile.read()
            
        # Check if configs is already in frontmatter, if not, inject it
        if "configs:" not in content:
            # Find the end of frontmatter (second ---)
            parts = content.split("---", 2)
            if len(parts) >= 3:
                frontmatter = parts[1]
                body = parts[2]
                
                # Add configs to frontmatter
                config_str = "\nconfigs:\n- config_name: default\n  data_files:\n  - split: train\n    path: dogfeed.parquet\n"
                new_content = f"---{frontmatter}{config_str}---{body}"
                
                with open(readme_path, "w", encoding="utf-8") as wfile:
                    wfile.write(new_content)
                print("Updated README.md with default dataset configuration.")

if __name__ == "__main__":
    main()
