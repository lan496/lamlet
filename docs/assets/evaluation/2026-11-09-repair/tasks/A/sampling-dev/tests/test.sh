#!/usr/bin/env bash
set -eu
python3 /tests/repair_tasks.py --task sampling-dev --source /app/submission/solution.in --output /logs/verifier/assessment.json
