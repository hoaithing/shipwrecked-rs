import json

transcript_path = "/home/cherry/.gemini/antigravity-cli/brain/4010a8e5-d883-4fc5-8f61-526be178c78b/.system_generated/logs/transcript_full.jsonl"

with open(transcript_path, 'r', encoding='utf-8') as f:
    for line in f:
        try:
            step = json.loads(line)
            if step.get("type") == "VIEW_FILE" and "main.rs" in step.get("content", ""):
                print(f"Step {step['step_index']}: VIEW_FILE main.rs")
                # print first few lines of content
                content = step["content"]
                lines = content.splitlines()
                print(f"  Total lines in view output: {len(lines)}")
                # Find line count info
                for l in lines[:10]:
                    if "Total Lines:" in l:
                        print(f"  {l}")
                    if "Showing lines" in l:
                        print(f"  {l}")
        except:
            pass
