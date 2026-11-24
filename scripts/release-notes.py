import re
import sys
from pathlib import Path


def main():
    tag = sys.argv[1]
    changelog = Path(__file__).resolve().parents[1] / "CHANGELOG.md"
    sections = re.split(r"(?m)^##[ \t]+([^\n]+)\n?", changelog.read_text(encoding="utf-8"))
    matches = [
        body.strip()
        for heading, body in zip(sections[1::2], sections[2::2])
        if heading.rstrip() == tag
    ]
    if len(matches) != 1 or not matches[0]:
        sys.exit(f"CHANGELOG.md must contain exactly one non-empty '## {tag}' section.")
    print(matches[0])


if __name__ == "__main__":
    main()
