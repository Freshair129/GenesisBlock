import os
import re
import yaml
import json

GKS_DIR = r"C:\Users\freshair\cognitive_system\gks"
EXCLUDE_DIRS = ["00_index", "episode"]

def get_files():
    files = []
    for root, dirs, filenames in os.walk(GKS_DIR):
        dirs[:] = [d for d in dirs if d not in EXCLUDE_DIRS]
        for f in filenames:
            if f.endswith(".md"):
                files.append(os.path.join(root, f))
    return files

def parse_frontmatter(content):
    match = re.match(r"^---\s*\n(.*?)\n---\s*\n", content, re.DOTALL)
    if match:
        try:
            return yaml.safe_load(match.group(1)), match.end()
        except:
            return None, 0
    return None, 0

def get_expected_id(filepath, frontmatter):
    filename = os.path.basename(filepath)
    basename = os.path.splitext(filename)[0]
    
    # If it's already in PREFIX--SLUG format, that's likely the ID
    if "--" in basename:
        return basename.upper()
    
    # Otherwise derive from folder and filename
    parent_dir = os.path.basename(os.path.dirname(filepath))
    prefix = parent_dir.upper()
    slug = basename.replace(" ", "-").replace("_", "-").upper()
    
    # Some files are directly in gks/
    if prefix == "GKS":
        return slug
        
    return f"{prefix}--{slug}"

def scan():
    files = get_files()
    report = {
        "total_files": len(files),
        "missing_id": [],
        "mismatched_id": [],
        "relative_links": [],
        "missing_crosslinks": [],
        "hubs": {} # id -> count of connections
    }
    
    id_to_file = {}
    
    for f in files:
        with open(f, 'r', encoding='utf-8') as file:
            content = file.read()
        
        frontmatter, body_start = parse_frontmatter(content)
        body = content[body_start:]
        
        rel_f = os.path.relpath(f, GKS_DIR)
        
        # 1. ID Check
        expected_id = get_expected_id(f, frontmatter)
        actual_id = frontmatter.get('id') if frontmatter else None
        
        if not actual_id:
            report["missing_id"].append({"file": rel_f, "suggested": expected_id})
            id_to_file[expected_id] = rel_f
        else:
            id_to_file[actual_id] = rel_f
            if actual_id != expected_id and "--" in expected_id:
                report["mismatched_id"].append({"file": rel_f, "actual": actual_id, "suggested": expected_id})
            
        # 2. Relative links check
        rel_links = re.findall(r"\[.*?\]\((?!https?://)(.*?\.md)\)", body)
        if rel_links:
            report["relative_links"].append({"file": rel_f, "links": rel_links})
            
        # 3. Crosslinks mapping
        wikilinks = re.findall(r"\[\[(.*?)\]\]", body)
        current_crosslinks = []
        if frontmatter and "crosslinks" in frontmatter:
            cl = frontmatter["crosslinks"]
            if isinstance(cl, dict):
                for k, v in cl.items():
                    if isinstance(v, list):
                        current_crosslinks.extend(v)
        
        missing = [w for w in wikilinks if w not in current_crosslinks and (actual_id is None or w != actual_id)]
        if missing:
            report["missing_crosslinks"].append({"file": rel_f, "missing": list(set(missing))})
            
        # Hub tracking
        if actual_id:
            report["hubs"][actual_id] = len(wikilinks) + len(current_crosslinks)

    return report

if __name__ == "__main__":
    print(json.dumps(scan(), indent=2))
