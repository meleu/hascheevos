#!/usr/bin/env bats
# Characterization of how hascheevos.sh hashes archives.
#
# Temporary: these tests (and test_helper/) live only on the drop-archive-tools
# branch and are removed before merging into master.
#
# Needs bin/hc built (`make hc`) and 7z installed. Never touches the network.

bats_require_minimum_version 1.5.0

readonly NES_ZIP="${BATS_TEST_DIRNAME}/../fixtures/nes/Zooming-Secretary.zip"
readonly NES_CONSOLE_ID=7

setup() {
  load 'test_helper/bats-support/load'
  load 'test_helper/bats-assert/load'

  HC="${BATS_TEST_DIRNAME}/../../bin/hc"
  [[ -x "$HC" ]] || fail "bin/hc not found; run 'make hc' first"

  # unpack the NES ROM, to hash it directly and to build other archives
  ROM_DIR="${BATS_TEST_TMPDIR}/rom"
  mkdir -p "$ROM_DIR"
  7z e -bd -y -o"$ROM_DIR" "$NES_ZIP" > /dev/null
  NES_ROM="$(find "$ROM_DIR" -type f -name '*.nes')"

  # shellcheck source-path=SCRIPTDIR source=../../bin/hascheevos.sh
  source "${BATS_TEST_DIRNAME}/../../bin/hascheevos.sh"
  fill_data
}

teardown() {
  # created by the script at load time; safe_exit removes it in real runs
  rm -rf "$TMP_DIR"
}

@test "get_rom_hash on a zipped ROM prints the hash of the ROM inside" {
  local expected
  expected="$("$HC" hash --console "$NES_CONSOLE_ID" "$NES_ROM")"

  run get_rom_hash "$NES_ZIP"

  assert_success
  assert_output "$expected"
}

@test "get_rom_hash on a 7zipped ROM prints the hash of the ROM inside" {
  local expected
  local archive="${BATS_TEST_TMPDIR}/Zooming-Secretary.7z"
  expected="$("$HC" hash --console "$NES_CONSOLE_ID" "$NES_ROM")"
  7z a -bd -t7z "$archive" "$NES_ROM" > /dev/null

  run get_rom_hash "$archive"

  assert_success
  assert_output "$expected"
}

@test "get_rom_hash on a zip whose first file is readme.txt fails with empty stdout" {
  local archive="${BATS_TEST_TMPDIR}/with-readme.zip"
  local src="${BATS_TEST_TMPDIR}/src"
  mkdir -p "$src"
  echo "not a ROM" > "${src}/readme.txt"
  # 7z sorts entries by name: a lowercase "z" keeps the ROM after readme.txt
  cp "$NES_ROM" "${src}/zooming-secretary.nes"
  (cd "$src" && 7z a -bd -tzip "$archive" readme.txt zooming-secretary.nes > /dev/null)
  # the first "Path =" is the archive itself; the second is its first entry
  [[ "$(7z l -slt "$archive" | sed -n 's/^Path = //p' | sed -n 2p)" == readme.txt ]] \
    || fail "readme.txt is not the archive's first entry"

  run --separate-stderr get_rom_hash "$archive"

  assert_failure
  assert_output ''
}

@test "get_rom_hash with the arcade flag hashes an arcade zip by its file name" {
  # a neutral parent directory: rcheevos prefixes the name for some (e.g. nes/)
  local archive="${BATS_TEST_TMPDIR}/roms/Zooming-Secretary.zip"
  mkdir -p "${archive%/*}"
  cp "$NES_ZIP" "$archive"
  ARCADE_FLAG=1

  run get_rom_hash "$archive"

  assert_success
  # md5 of "Zooming-Secretary"
  assert_output '2f3f193d054f32d38dbbec6d7ce5d79c'
}
