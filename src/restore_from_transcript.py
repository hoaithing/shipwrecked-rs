import json
import subprocess

transcript_path = "/home/cherry/.gemini/antigravity-cli/brain/4010a8e5-d883-4fc5-8f61-526be178c78b/.system_generated/logs/transcript_full.jsonl"

# Read the initial main.rs content (from git HEAD)
initial_content = subprocess.check_output(["git", "show", "HEAD:src/main.rs"]).decode('utf-8')

def normalize_text(text):
    if text is None:
        return ""
    # Split by lines, strip trailing spaces, filter out carriage returns
    return "\n".join(line.rstrip() for line in text.splitlines())

replacements = []

with open(transcript_path, 'r', encoding='utf-8') as f:
    for line in f:
        # Quick raw string check to avoid parsing JSON for unrelated lines
        if "PLANNER_RESPONSE" not in line or "tool_calls" not in line or "main.rs" not in line:
            continue
        try:
            step = json.loads(line)
            if step.get("type") == "PLANNER_RESPONSE" and "tool_calls" in step:
                for tc in step["tool_calls"]:
                    name = tc["name"]
                    args = tc.get("args", {})
                    if isinstance(args, str):
                        try:
                            args = json.loads(args)
                        except:
                            pass
                    if isinstance(args, dict):
                        target = args.get("TargetFile") or args.get("Target")
                        if target and "main.rs" in target:
                            replacements.append({
                                "step_index": step["step_index"],
                                "name": name,
                                "args": args
                            })
        except:
            pass

replacements.sort(key=lambda x: x["step_index"])

applied = set()
current_content = normalize_text(initial_content)

print(f"Initial main.rs: {len(current_content)} chars")

for rep in replacements:
    step_idx = rep["step_index"]
    name = rep["name"]
    args = rep["args"]
    
    args_str = json.dumps(args, sort_keys=True)
    key = (step_idx, name, args_str)
    if key in applied:
        continue
    applied.add(key)
        
    print(f"Applying Step {step_idx}: {name}")
    if name == "replace_file_content":
        target = args.get("TargetContent")
        replacement = args.get("ReplacementContent")
        if target is None or replacement is None:
            continue
        
        norm_target = normalize_text(target)
        norm_replacement = normalize_text(replacement)
        
        new_content = current_content.replace(norm_target, norm_replacement, 1)
        if new_content != current_content:
            current_content = new_content
            print("  Successfully replaced contiguous chunk!")
        else:
            print("  WARNING: Target content not found!")
            print(f"  Target snippet: {repr(norm_target[:100])}")
            
    elif name == "multi_replace_file_content":
        chunks = args.get("ReplacementChunks", [])
        if chunks is None:
            continue
            
        success_count = 0
        for chunk in chunks:
            target = chunk.get("TargetContent")
            replacement = chunk.get("ReplacementContent")
            if target is None or replacement is None:
                continue
            
            norm_target = normalize_text(target)
            norm_replacement = normalize_text(replacement)
            
            new_content = current_content.replace(norm_target, norm_replacement, 1)
            if new_content != current_content:
                current_content = new_content
                success_count += 1
            else:
                print(f"  WARNING: Chunk target not found: {repr(norm_target[:50])}")
        print(f"  Multi-replace: applied {success_count}/{len(chunks)} chunks.")

with open("src/main.rs", "w", encoding="utf-8") as f:
    f.write(current_content)

print(f"Reconstructed file: {len(current_content)} chars, {len(current_content.splitlines())} lines")
