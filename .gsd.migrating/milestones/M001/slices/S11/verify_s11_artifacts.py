#!/usr/bin/env python3
"""S11 artifact verifier for validation traceability remediation.

The verifier intentionally lives under .gsd so product tests do not depend on
planning artifacts. Use --requirements for T01 traceability checks, and use the
other modes while finishing the rest of S11.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

EXPECTED_IDS = [f"R{i:03d}" for i in range(1, 55)]
KNOWN_SLICES = {f"S{i:02d}" for i in range(1, 12)}
ROOT = Path(__file__).resolve().parents[5]
REQ_PATH = ROOT / ".gsd" / "REQUIREMENTS.md"


class VerificationError(Exception):
    """Raised when an S11 verifier check fails."""


def fail(message: str) -> None:
    raise VerificationError(message)


def read_text(path: Path) -> str:
    if not path.exists():
        fail(f"missing required file: {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8")


def parse_requirement_records(text: str) -> dict[str, dict[str, str]]:
    matches = list(re.finditer(r"^### (R\d{3}) — (.+)$", text, re.MULTILINE))
    records: dict[str, dict[str, str]] = {}
    for index, match in enumerate(matches):
        req_id = match.group(1)
        title = match.group(2).strip()
        if req_id in records:
            fail(f"duplicate requirement heading: {req_id}")
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        body = text[match.end() : end]
        records[req_id] = {"title": title, "body": body}
    return records


def find_field(body: str, field: str) -> str:
    found = re.search(rf"^- {re.escape(field)}: (.+)$", body, re.MULTILINE)
    return found.group(1).strip() if found else ""


def parse_trace_rows(text: str) -> dict[str, dict[str, str]]:
    rows: dict[str, dict[str, str]] = {}
    for line in text.splitlines():
        if not re.match(r"^\| R\d{3} \|", line):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) != 7:
            fail(f"trace row has {len(cells)} cells, expected 7: {line}")
        req_id, requirement, req_class, status, owner, supporting, proof = cells
        if req_id in rows:
            fail(f"duplicate traceability row: {req_id}")
        rows[req_id] = {
            "requirement": requirement,
            "class": req_class,
            "status": status,
            "owner": owner,
            "supporting": supporting,
            "proof": proof,
        }
    return rows


def parse_summary_counts(text: str) -> dict[str, int]:
    wanted = {
        "Total v1 requirements": "total",
        "Active requirements": "active",
        "Validated requirements": "validated",
        "Deferred requirements": "deferred",
        "Out-of-scope v1 requirements": "out_of_scope",
        "Requirements with primary owner": "with_owner",
        "Requirements with proof text": "with_proof",
        "Requirements without primary owner": "without_owner",
        "Requirements without proof text": "without_proof",
    }
    counts: dict[str, int] = {}
    for label, key in wanted.items():
        match = re.search(rf"^- {re.escape(label)}: (\d+)$", text, re.MULTILINE)
        if not match:
            fail(f"coverage summary is missing count: {label}")
        counts[key] = int(match.group(1))
    return counts


def check_requirements() -> list[str]:
    text = read_text(REQ_PATH)
    problems: list[str] = []

    if re.search(r"\bUntitled\b", text):
        problems.append("placeholder title 'Untitled' remains")
    if re.search(r"\bunmapped\b", text, re.IGNORECASE):
        problems.append("placeholder proof text 'unmapped' remains")

    records = parse_requirement_records(text)
    record_ids = set(records)
    expected = set(EXPECTED_IDS)
    missing = sorted(expected - record_ids)
    extra = sorted(record_ids - expected)
    if missing:
        problems.append(f"missing requirement records: {', '.join(missing)}")
    if extra:
        problems.append(f"unexpected requirement records: {', '.join(extra)}")

    statuses: dict[str, str] = {}
    owners: dict[str, str] = {}
    proofs: dict[str, str] = {}
    for req_id in EXPECTED_IDS:
        if req_id not in records:
            continue
        title = records[req_id]["title"]
        body = records[req_id]["body"]
        status = find_field(body, "Status")
        owner = find_field(body, "Primary owning slice")
        proof = find_field(body, "Validation")
        req_class = find_field(body, "Class")
        description = find_field(body, "Description")

        statuses[req_id] = status
        owners[req_id] = owner
        proofs[req_id] = proof

        if not title or title.lower().endswith("untitled"):
            problems.append(f"{req_id} has a placeholder or missing title")
        if not req_class:
            problems.append(f"{req_id} is missing Class")
        if status not in {"active", "validated", "deferred", "out-of-scope"}:
            problems.append(f"{req_id} has invalid or missing Status: {status!r}")
        if not re.fullmatch(r"S\d{2}", owner):
            problems.append(f"{req_id} has missing or invalid primary owner: {owner!r}")
        if owner and owner not in KNOWN_SLICES:
            problems.append(f"{req_id} references unknown primary owner: {owner}")
        if not description or description in {"-", "none", "None"}:
            problems.append(f"{req_id} is missing Description")
        if not proof or proof in {"-", "none", "None"} or len(proof) < 40:
            problems.append(f"{req_id} has missing or too-short proof text")
        if status == "active" and "- Gap:" not in body:
            problems.append(f"{req_id} is active but lacks an explicit Gap field")

    rows = parse_trace_rows(text)
    row_ids = set(rows)
    missing_rows = sorted(expected - row_ids)
    extra_rows = sorted(row_ids - expected)
    if missing_rows:
        problems.append(f"missing traceability rows: {', '.join(missing_rows)}")
    if extra_rows:
        problems.append(f"unexpected traceability rows: {', '.join(extra_rows)}")

    for req_id, row in rows.items():
        if req_id in statuses and row["status"] != statuses[req_id]:
            problems.append(f"{req_id} status mismatch between record and traceability table")
        if req_id in owners and row["owner"] != owners[req_id]:
            problems.append(f"{req_id} primary owner mismatch between record and traceability table")
        if not re.fullmatch(r"S\d{2}", row["owner"]):
            problems.append(f"{req_id} traceability row has invalid owner: {row['owner']!r}")
        if not row["proof"] or row["proof"] in {"-", "none", "None"} or len(row["proof"]) < 40:
            problems.append(f"{req_id} traceability row has missing or too-short proof")

    counts = parse_summary_counts(text)
    active_count = sum(1 for status in statuses.values() if status == "active")
    validated_count = sum(1 for status in statuses.values() if status == "validated")
    deferred_count = sum(1 for status in statuses.values() if status == "deferred")
    out_of_scope_count = sum(1 for status in statuses.values() if status == "out-of-scope")
    owner_count = sum(1 for owner in owners.values() if re.fullmatch(r"S\d{2}", owner))
    proof_count = sum(
        1
        for proof in proofs.values()
        if proof and proof not in {"-", "none", "None"} and len(proof) >= 40
    )

    expected_counts = {
        "total": len(EXPECTED_IDS),
        "active": active_count,
        "validated": validated_count,
        "deferred": deferred_count,
        "out_of_scope": out_of_scope_count,
        "with_owner": owner_count,
        "with_proof": proof_count,
        "without_owner": len(EXPECTED_IDS) - owner_count,
        "without_proof": len(EXPECTED_IDS) - proof_count,
    }
    for key, expected_value in expected_counts.items():
        if counts[key] != expected_value:
            problems.append(
                f"coverage summary count mismatch for {key}: "
                f"summary={counts[key]} actual={expected_value}"
            )

    if problems:
        fail("requirements check failed:\n- " + "\n- ".join(problems))
    return [f"requirements ok: {len(records)} records, {validated_count} validated, {active_count} active"]


def check_artifacts() -> list[str]:
    required = [
        ROOT / ".gsd" / "milestones" / "M001" / "slices" / "S01" / "S01-ASSESSMENT.md",
        ROOT / ".gsd" / "milestones" / "M001" / "slices" / "S01" / "S01-UAT.md",
        ROOT / ".gsd" / "milestones" / "M001" / "slices" / "S10" / "S10-ASSESSMENT.md",
        ROOT / ".gsd" / "milestones" / "M001" / "slices" / "S10" / "S10-UAT.md",
        ROOT / ".gsd" / "milestones" / "M001" / "slices" / "S11" / "S11-CONTEXT.md",
        ROOT / ".gsd" / "milestones" / "M001" / "slices" / "S11" / "S11-RESEARCH.md",
    ]
    missing = [str(path.relative_to(ROOT)) for path in required if not path.exists()]
    if missing:
        fail("artifact check failed; missing files:\n- " + "\n- ".join(missing))

    problems: list[str] = []
    texts: dict[Path, str] = {}
    for path in required:
        text = path.read_text(encoding="utf-8")
        texts[path] = text
        if not text.strip():
            problems.append(f"empty required artifact: {path.relative_to(ROOT)}")

    s01_uat_path = ROOT / ".gsd" / "milestones" / "M001" / "slices" / "S01" / "S01-UAT.md"
    s01_uat = texts[s01_uat_path]
    required_caveats = [
        r"S11 did not manually run GUI UAT",
        r"not evidence of a newly executed manual desktop session",
        r"These are procedure steps, not claims that S11 executed them",
    ]
    if not any(re.search(pattern, s01_uat, re.IGNORECASE) for pattern in required_caveats):
        problems.append(
            "S01-UAT.md must explicitly state that S11 did not perform fresh manual GUI UAT"
        )

    unsupported_manual_claims = [
        r"live manual GUI UAT (?:was )?(?:performed|completed|executed)",
        r"manual (?:desktop/)?game-install UAT (?:was )?(?:performed|completed|executed)",
        r"manual real-install UAT (?:was )?(?:performed|completed|executed)",
        r"real Fallout 4 install (?:was )?(?:used|tested)",
    ]
    for pattern in unsupported_manual_claims:
        if re.search(pattern, s01_uat, re.IGNORECASE):
            problems.append(
                "S01-UAT.md appears to claim manual real-install/desktop UAT without a recorded run"
            )
            break

    if problems:
        fail("artifact check failed:\n- " + "\n- ".join(problems))
    return [f"artifacts ok: {len(required)} required files present, non-empty, and caveated"]


def check_provenance() -> list[str]:
    text = read_text(REQ_PATH)
    rows = parse_trace_rows(text)
    problems: list[str] = []
    for sid in [f"S{i:02d}" for i in range(1, 11)]:
        summary = ROOT / ".gsd" / "milestones" / "M001" / "slices" / sid / f"{sid}-SUMMARY.md"
        if not summary.exists():
            problems.append(f"missing completed slice summary: {summary.relative_to(ROOT)}")

    s07_summary = ROOT / ".gsd" / "milestones" / "M001" / "slices" / "S07" / "S07-SUMMARY.md"
    s07_text = read_text(s07_summary)
    s01_dependency = re.search(r"(?m)^  - slice: S01\n    provides: (?P<provides>.+)$", s07_text)
    s02_dependency = re.search(r"(?m)^  - slice: S02\n    provides: (?P<provides>.+)$", s07_text)
    if not s01_dependency:
        problems.append("S07 summary is missing S01 as the shell/tab-wiring dependency")
    elif not re.search(
        r"Main shell|reference tab order|MainWindow|tab wiring",
        s01_dependency.group("provides"),
        re.IGNORECASE,
    ):
        problems.append("S07 S01 dependency does not describe shell/tab-wiring provenance")
    if not s02_dependency:
        problems.append("S07 summary is missing S02 as the settings/scanner-settings dependency")
    else:
        s02_provides = s02_dependency.group("provides")
        if re.search(r"Main shell|MainWindow|tab wiring", s02_provides, re.IGNORECASE):
            problems.append("S07 summary still attributes main shell/tab wiring provenance to S02")
        if not re.search(r"settings persistence|scanner settings", s02_provides, re.IGNORECASE):
            problems.append("S07 S02 dependency does not describe settings/scanner-settings provenance")

    for req_id, row in rows.items():
        referenced = {row["owner"]}
        if row["supporting"] != "-":
            referenced.update(part.strip() for part in row["supporting"].split(","))
        for sid in sorted(referenced):
            if not re.fullmatch(r"S\d{2}", sid):
                problems.append(f"{req_id} references invalid slice token: {sid!r}")
                continue
            if sid < "S01" or sid > "S11":
                problems.append(f"{req_id} references unknown slice: {sid}")
    if problems:
        fail("provenance check failed:\n- " + "\n- ".join(problems))
    return ["provenance ok: traceability references existing completed-slice summary IDs and S07 dependency attribution"]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Verify S11 validation remediation artifacts.")
    parser.add_argument("--requirements", action="store_true", help="check .gsd/REQUIREMENTS.md traceability")
    parser.add_argument("--artifacts", action="store_true", help="check required validation artifact presence")
    parser.add_argument("--provenance", action="store_true", help="check traceability slice provenance references")
    parser.add_argument("--all", action="store_true", help="run all checks")
    args = parser.parse_args(argv)

    if not any([args.requirements, args.artifacts, args.provenance, args.all]):
        args.all = True

    checks = []
    if args.all or args.requirements:
        checks.append(check_requirements)
    if args.all or args.artifacts:
        checks.append(check_artifacts)
    if args.all or args.provenance:
        checks.append(check_provenance)

    try:
        messages: list[str] = []
        for check in checks:
            messages.extend(check())
    except VerificationError as exc:
        print(str(exc), file=sys.stderr)
        return 1

    for message in messages:
        print(message)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
