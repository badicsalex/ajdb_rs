#!/bin/sh
set -ex

RECALC_WHAT=$(realpath important_acts.txt)
PARSED_ACTS=$(realpath db/parsed_acts)
mkdir -p "${PARSED_ACTS}"

(
    cd ../hun_law
    xargs -a "${RECALC_WHAT}" -d"\n" cargo run --release -- -o "${PARSED_ACTS}" -i
)
sed 's!.*!db/parsed_acts/\0.yml!' "${RECALC_WHAT}" | xargs -d"\n" cargo run --profile dev-fast -- add
RUST_LOG=warn cargo run --profile dev-fast -- recalculate 2010-01-01 2024-12-02
