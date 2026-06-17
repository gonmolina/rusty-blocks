with open("blockdiagram.xml", "r", encoding="utf-8", errors="ignore") as f:
    content = f.read()

# search for pc01 or PC01
import re
matches = [m.start() for m in re.finditer(r"pc01", content, re.IGNORECASE)]

print(f"Found {len(matches)} matches for PC01:")
for m in matches:
    start = max(0, m - 100)
    end = min(len(content), m + 100)
    print(f"Context: {content[start:end]}")
