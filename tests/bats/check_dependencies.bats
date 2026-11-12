#!/usr/bin/env bats
# hascheevos.sh needs only jq and curl.
#
# Temporary: these tests (and test_helper/) live only on the drop-archive-tools
# branch and are removed before merging into master.

bats_require_minimum_version 1.5.0

setup() {
  load 'test_helper/bats-support/load'
  load 'test_helper/bats-assert/load'

  # shellcheck source-path=SCRIPTDIR source=../../bin/hascheevos.sh
  source "${BATS_TEST_DIRNAME}/../../bin/hascheevos.sh"
}

teardown() {
  # created by the script at load time; safe_exit removes it in real runs
  rm -rf "$TMP_DIR"
}

@test "check_dependencies succeeds when only jq and curl are present" {
  # the lookup check_dependencies uses: only jq and curl are found
  which() {
    [[ "$1" == jq || "$1" == curl ]]
  }

  run --separate-stderr check_dependencies < /dev/null

  assert_success
  assert_output ''
  [[ -z "$stderr" ]] || fail "unexpected stderr: $stderr"
}
