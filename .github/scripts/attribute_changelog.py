#!/usr/bin/env python3
"""Add contributor and first-contribution attribution to the newest changelog block."""

import os
import re
import subprocess
import sys

MAINTAINERS = {"mulhamna", "badrus123"}
_PREVTAG_RE = re.compile(r"/compare/(.+?)\.\.\.")
_BULLET_RE = re.compile(r"^(\* (?:\*\*[^:*]+:\*\* )?)(.*?)( \(\[[0-9a-f]+\]\([^)]*\)\))\s*$")
_SHA_RE = re.compile(r"\[([0-9a-f]+)\]")


def split_top_block(text):
    parts = re.split(r"(?m)(?=^## )", text)
    if len(parts) < 2:
        return text, "", ""
    return parts[0], parts[1], "".join(parts[2:])


def gh_author(sha):
    env = dict(os.environ)
    token = env.get("GH_TOKEN") or env.get("GITHUB_TOKEN")
    if token:
        env["GH_TOKEN"] = token
    process = subprocess.run(
        [
            "gh",
            "api",
            f"repos/suiflex/kurir/commits/{sha}",
            "--jq",
            r"[(.author.login // \"\"), (.commit.author.name // \"\"), (.commit.author.email // \"\")] | @tsv",
        ],
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    if process.returncode != 0:
        return None, None, None
    values = process.stdout.strip().split("\t")
    values += [""] * (3 - len(values))
    return tuple(value or None for value in values[:3])


def previous_emails(previous_tag):
    if not previous_tag:
        return set()
    process = subprocess.run(
        ["git", "log", previous_tag, "--format=%ae"],
        capture_output=True,
        text=True,
        check=False,
    )
    if process.returncode != 0:
        return set()
    return {line.strip() for line in process.stdout.splitlines() if line.strip()}


def token(login, name):
    return f"@{login}" if login else (name or "an unknown contributor")


def attribute(text):
    before, block, after = split_top_block(text)
    if not block or "### Thanks" in block:
        return text
    previous_match = _PREVTAG_RE.search(block)
    seen_emails = previous_emails(previous_match.group(1) if previous_match else None)
    contributors = {}
    new_contributors = {}
    output = []

    for line in block.splitlines():
        match = _BULLET_RE.match(line)
        if not match:
            output.append(line)
            continue
        prefix, subject, link = match.groups()
        sha_match = _SHA_RE.search(link)
        if not sha_match:
            output.append(line)
            continue
        login, name, email = gh_author(sha_match.group(1))
        if login in MAINTAINERS or (not login and not name):
            output.append(line)
            continue
        contributor = token(login, name)
        contributors[contributor] = None
        if email and email not in seen_emails:
            new_contributors[contributor] = None
        output.append(f"{prefix}{subject} ({contributor}){link}")

    if not contributors:
        return text
    thanks = ["", "### Thanks", ""] + [f"* {value}" for value in sorted(contributors)]
    if new_contributors:
        thanks += ["", "### New Contributors", ""]
        thanks += [f"* {value} made their first contribution" for value in sorted(new_contributors)]
    return before + "\n".join(output).rstrip() + "\n" + "\n".join(thanks) + "\n\n" + after


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else "CHANGELOG.md"
    with open(path, encoding="utf-8") as source:
        content = source.read()
    with open(path, "w", encoding="utf-8") as destination:
        destination.write(attribute(content))
    print(f"attributed contributors in {path}")


if __name__ == "__main__":
    main()
