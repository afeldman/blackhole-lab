#!/bin/bash
# M2 acceptance checks for the Black Hole Lab CLI.
#
# Every case prints its command, its output and a computed PASS/FAIL verdict, so
# the transcript is evidence rather than a claim. A case is checked on its
# *content*, not only on its exit status: a case that exits 0 without printing a
# Results block is a failure, because an exit status alone cannot distinguish "it
# worked" from "it silently computed nothing" — which is exactly how a broken
# scripted dialogue survived in an earlier version of this transcript.
#
# Failure paths are exercised on purpose, so `set -e` is deliberately not used.
#
# Reproduce with:  bash docs/reports/m2_cli_checks.sh

cd "$(dirname "$0")/../.." || exit 1
BIN=./target/debug/blackhole

passed=0
failed=0

echo "Black Hole Lab — M2 CLI acceptance transcript"
echo "binary:      $(pwd)/target/debug/blackhole"
echo "rustc:       $(rustc --version)"
echo "cargo:       $(cargo --version)"
echo "host:        $(uname -srm)"
echo "date (UTC):  $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

cargo build --quiet -p blackhole-cli || exit 1

report() {
  local description="$1" expect="$2" status="$3" out="$4"
  local verdict="FAIL"
  if [ "$expect" = "results" ]; then
    if [ "$status" -eq 0 ] && printf '%s' "$out" | grep -q '^Results$'; then
      verdict="PASS"
    fi
  elif [ "$expect" = "ok" ]; then
    if [ "$status" -eq 0 ]; then
      verdict="PASS"
    fi
  else
    if [ "$status" -ne 0 ]; then
      verdict="PASS"
    fi
  fi
  echo "exit status: $status   expected: $expect   verdict: $verdict"
  echo
  if [ "$verdict" = "PASS" ]; then
    passed=$((passed + 1))
  else
    failed=$((failed + 1))
    echo "  !! FAILED CASE: $description"
    echo
  fi
}

# check <description> <results|error> <command...>
check() {
  local description="$1" expect="$2"
  shift 2
  echo "======================================================================"
  echo "# $description"
  echo "\$ $*"
  echo "----------------------------------------------------------------------"
  local out status
  out="$("$@" 2>&1)"
  status=$?
  printf '%s\n' "$out"
  echo "----------------------------------------------------------------------"
  report "$description" "$expect" "$status" "$out"
}

# check_stdin <description> <results|error> <input> <command...>
check_stdin() {
  local description="$1" expect="$2" input="$3"
  shift 3
  echo "======================================================================"
  echo "# $description"
  echo "\$ printf '%s' | $*" "${input//$'\n'/\\n}"
  echo "----------------------------------------------------------------------"
  local out status
  out="$(printf '%s' "$input" | "$@" 2>&1)"
  status=$?
  printf '%s\n' "$out"
  echo "----------------------------------------------------------------------"
  report "$description" "$expect" "$status" "$out"
}

echo "######################################################################"
echo "# 1. Non-interactive: the three two-of-three combinations"
echo "######################################################################"
echo

check "radius + mean density (1 km sphere of water)" results \
  "$BIN" calc --radius 1 --radius-unit km --density water --label "1 km water sphere"

check "mass + physical radius (twenty Earth masses at one Earth radius)" results \
  "$BIN" calc --mass 20 --mass-unit Mearth --radius 1 --radius-unit Rearth \
  --label "20 Mearth in 1 Rearth"

check "mass + mean density (a solar mass of granite), radius derived" results \
  "$BIN" calc --mass 1 --mass-unit Msun --density granite --label "1 Msun of granite"

check "cgs density input with scientific notation" results \
  "$BIN" calc --radius 6371 --radius-unit km --density "2.7 g/cm3" --label "rock planet"

echo "######################################################################"
echo "# 2. Known objects, by name and by index, including one with no surface"
echo "######################################################################"
echo

check "the Sun"                      results "$BIN" object Sun
check "Sagittarius A* (no surface)"  results "$BIN" object "sgr a*"
check "the Moon"                     results "$BIN" object moon
check "Earth selected by index"      results "$BIN" object 2

echo "######################################################################"
echo "# 3. Catalogues"
echo "######################################################################"
echo

check "list both catalogues" ok "$BIN" list

echo "######################################################################"
echo "# 4. Usage and failure paths (non-zero exit statuses are expected)"
echo "######################################################################"
echo

check "no quantity given"           error "$BIN" calc
check "only one quantity given"     error "$BIN" calc --mass 1
check "all three quantities given"  error "$BIN" calc --mass 1 --radius 1 --density 1000
check "unknown mass unit"           error "$BIN" calc --mass 1 --mass-unit stone --radius 1
check "unknown length unit"         error "$BIN" calc --radius 1 --radius-unit furlong --density 1000
check "unknown material"            error "$BIN" calc --radius 1 --density unobtainium
check "negative mass"               error "$BIN" calc --mass -5 --radius 1
check "negative density"            error "$BIN" calc --radius 1 --density -5
check "zero radius"                 error "$BIN" calc --radius 0 --density 1000
check "a flag where a value belongs" error "$BIN" calc --mass --radius 1
check "unknown object"              error "$BIN" object Vulcan
check "out-of-range object index"   error "$BIN" object 99
check "help"                        ok "$BIN" --help

echo "######################################################################"
echo "# 5. Interactive dialogue"
echo "######################################################################"
echo

check_stdin "mode 2, then quit at the repeat prompt" results \
"2
5.9722e24
kg
6.371e6
m
Earth
n
" "$BIN"

check_stdin "mode 1 with a material name and an inline unit" results \
"1
1 km
water
ball of water
q
" "$BIN"

check_stdin "mode 4, combination 1 (mass and density, radius derived)" results \
"4
1
1
Msun
granite
granite sphere
n
" "$BIN"

check_stdin "recovery: bad mode, bad number, invalid value, then a valid run" results \
"9
2
twelve
5.9722e24
kg
0
2e3
m
Earth after typos
n
" "$BIN"

check_stdin "invalid UTF-8 on stdin is recovered from" results \
"$(printf '\377\376')
3
Earth
n
" "$BIN"

check_stdin "end of input quits cleanly" ok "" "$BIN"

echo "######################################################################"
echo "# 6. Determinism and front-end agreement"
echo "######################################################################"
echo

TMP="$(mktemp -d)"

"$BIN" object Earth > "$TMP/first.txt" 2>&1
"$BIN" object Earth > "$TMP/second.txt" 2>&1
if diff -q "$TMP/first.txt" "$TMP/second.txt" > /dev/null; then
  echo "object Earth:       two runs byte-identical (sha256 $(shasum -a 256 "$TMP/first.txt" | cut -c1-16)...)"
  passed=$((passed + 1))
else
  echo "object Earth:       RUNS DIFFER — determinism violated"
  failed=$((failed + 1))
fi

"$BIN" calc --radius 1 --density water --label same > "$TMP/c1.txt" 2>&1
"$BIN" calc --radius 1 --density water --label same > "$TMP/c2.txt" 2>&1
if diff -q "$TMP/c1.txt" "$TMP/c2.txt" > /dev/null; then
  echo "calc water sphere:  two runs byte-identical (sha256 $(shasum -a 256 "$TMP/c1.txt" | cut -c1-16)...)"
  passed=$((passed + 1))
else
  echo "calc water sphere:  RUNS DIFFER — determinism violated"
  failed=$((failed + 1))
fi

# The interactive path and the non-interactive path must agree on the physics.
"$BIN" calc --radius 1 --radius-unit km --density water --label "ball of water" \
  > "$TMP/calc.txt" 2>&1
printf '1\n1 km\nwater\nball of water\nq\n' | "$BIN" > "$TMP/interactive.txt" 2>&1
if diff <(grep -E '^  (Mass|Schwarzschild|Compactness|Critical)' "$TMP/calc.txt") \
        <(grep -E '^  (Mass|Schwarzschild|Compactness|Critical)' "$TMP/interactive.txt") \
        > /dev/null; then
  echo "calc vs interactive: value rows identical for the same input"
  passed=$((passed + 1))
else
  echo "calc vs interactive: VALUE ROWS DIFFER"
  diff <(grep -E '^  (Mass|Schwarzschild|Compactness|Critical)' "$TMP/calc.txt") \
       <(grep -E '^  (Mass|Schwarzschild|Compactness|Critical)' "$TMP/interactive.txt")
  failed=$((failed + 1))
fi

rm -rf "$TMP"

echo
echo "======================================================================"
echo "SUMMARY: $passed passed, $failed failed"
echo "======================================================================"
exit "$failed"
