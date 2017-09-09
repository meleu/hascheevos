#!/bin/bash
# hascheevos.sh
###############
#
# A tool to check if your ROMs have cheevos (RetroAchievements.org).
#
# TODO: check dependencies curl, jq, zcat, unzip, 7z, cheevoshash (from this repo).

# globals ####################################################################

readonly USAGE="
USAGE:
$(basename "$0") [OPTIONS] romfile1 [romfile2 ...]"

readonly GIT_REPO="https://github.com/meleu/hascheevos.git"
readonly SCRIPT_URL="https://raw.githubusercontent.com/meleu/hascheevos/master/bin/hascheevos.sh"
readonly SCRIPT_DIR="$(cd "$(dirname $0)" && pwd)"
readonly SCRIPT_NAME="$(basename "$0")"
readonly SCRIPT_FULL="$SCRIPT_DIR/$SCRIPT_NAME"
readonly DATA_DIR="$SCRIPT_DIR/../data"
readonly GAMEID_REGEX='^[1-9][0-9]{0,9}$'

# the extensions below were taken from RetroPie's configs (es_systems.cfg)
readonly EXTENSIONS='zip|7z|nes|fds|gb|gba|gbc|sms|bin|smd|gen|md|sg|smc|sfc|fig|swc|mgd|iso|cue|z64|n64|v64|pce|ccd|cue'

# flags
CHECK_FALSE_FLAG=0
COPY_ROMS_FLAG=0
CHECK_RA_SERVER_FLAG=0

RA_USER=
RA_PASSWORD=
RA_TOKEN=
COPY_ROMS_DIR=
TMP_DIR="/tmp/hascheevos-$$"
mkdir -p "$TMP_DIR"
GAME_CONSOLE_NAME="$(mktemp -p "$TMP_DIR")"

CONSOLE_NAME=()
CONSOLE_NAME[1]=megadrive
CONSOLE_NAME[2]=n64
CONSOLE_NAME[3]=snes
CONSOLE_NAME[4]=gb
CONSOLE_NAME[5]=gba
CONSOLE_NAME[6]=gbc
CONSOLE_NAME[7]=nes
CONSOLE_NAME[8]=pcengine
CONSOLE_NAME[9]=segacd
CONSOLE_NAME[10]=sega32x
CONSOLE_NAME[11]=mastersystem
#CONSOLE_NAME[12]=xbox360
#CONSOLE_NAME[13]=atari
#CONSOLE_NAME[14]=neogeo


# functions ##################################################################

function safe_exit() {
    rm -rf "$TMP_DIR"
    exit $1
}


function help_message() {
    echo "$USAGE"
    echo
    echo "Where [OPTIONS] are:"
    echo
    # getting the help message from the comments in this source code
    sed -n 's/^#H //p' "$0"
    safe_exit
}


function update_files() {
    local err_flag=0
    local dir="$SCRIPT_DIR/.."

    if [[ -d "$dir/.git" ]]; then
        pushd "$dir" > /dev/null
        if ! git pull --rebase ; then
            git merge --abort && git pull -X theirs || err_flag=1
        fi
        if [[ $err_flag -eq 0 ]]; then
            git submodule update --init --recursive || err_flag=1
        fi
        popd > /dev/null
    else
        echo "ERROR: \"$dir/.git\": directory not found!" >&2
        echo "Looks like this tool wasn't installed as instructed in repo's README." >&2
        echo "Aborting..." >&2
        err_flag=1
    fi

    if [[ $err_flag -ne 0 ]]; then
        echo "UPDATE: Failed to update \"$SCRIPT_NAME\"." >&2
        safe_exit 1
    fi
    
    echo "UPDATE: The files have been successfully updated."
    safe_exit 0
}


# Getting the RetroAchievements token
# input: RA_USER, RA_PASSWORD
# updates: RA_TOKEN
# exit if fails
# TODO: cache the token in some file
function get_cheevos_token() {
    if [[ -z "$RA_USER" ]]; then
        echo "ERROR: undefined RetroAchievements.org user (see \"--user\" option)." >&2
        safe_exit 1
    fi

    [[ -n "$RA_TOKEN" ]] && return 0

    if [[ -z "$RA_PASSWORD" ]]; then
        echo "ERROR: undefined RetroAchievements.org password (see \"--password\" option)." >&2
        safe_exit 1
    fi

    RA_TOKEN="$(curl -s "http://retroachievements.org/dorequest.php?r=login&u=${RA_USER}&p=${RA_PASSWORD}" | jq -r .Token)"
    if [[ "$RA_TOKEN" == null || -z "$RA_TOKEN" ]]; then
        echo "ERROR: cheevos authentication failed. Aborting..."
        safe_exit 1
    fi
}


# download hashlibrary for a specific console
# $1 is the console_id
function download_hashlibrary() {
    local console_id="$1"

    if [[ "$console_id" -le 0 || "$console_id" -gt "${#CONSOLE_NAME[@]}" ]]; then
        echo "ERROR: invalid console ID: $console_id" >&2
        safe_exit 1
    fi

    local json_file="$DATA_DIR/${CONSOLE_NAME[console_id]}_hashlibrary.json"

    echo "--- getting the console hash library for \"${CONSOLE_NAME[console_id]}\"..." >&2
    curl -s "http://retroachievements.org/dorequest.php?r=hashlibrary&c=$console_id" \
        | jq '.' > "$json_file" 2> /dev/null \
        || echo "ERROR: failed to download hash library for \"${CONSOLE_NAME[console_id]}\"!" >&2

    [[ -s "$json_file" ]] || rm -f "$json_file"
}


# download hashlibrary for all consoles
function download_hash_libraries() {
    local i

    echo "Getting hash libraries..."

    for i in $(seq 1 ${#CONSOLE_NAME[@]}); do
        # XXX: do not get hashlibrary for sega32x and segacd (currently unsupported)
        [[ $i -eq 9 || $i -eq 10 ]] && continue
        download_hashlibrary "$i"
    done
}


# update hashlibraries older than 1 day
function update_hash_libraries() {
    local line
    local system
    local i

    echo "Checking JSON hash libraries..."

    for i in "${!CONSOLE_NAME[@]}"; do
        [[ -f "$DATA_DIR/${CONSOLE_NAME[i]}_hashlibrary.json" ]] || download_hashlibrary "$i"
    done

    while read -r line; do
        system="$(basename "${line%_hashlibrary.json*}")"
        for i in "${!CONSOLE_NAME[@]}"; do
            if [[ "${CONSOLE_NAME[i]}" == "$system" ]]; then
                download_hashlibrary "$i"
                break
            fi
        done
    done < <(find "$DATA_DIR" -type f -name '*_hashlibrary.json' -mtime +1)
}


# Print (echo) the game ID of a given rom file
# This function try to get the game id from local *_hashlibrary.json files, if
# these files don't exist the script will try to get them from RA server.
# input:
# $1 is a rom file (should be previously validated with validate_rom_file())
# also needs RA_TOKEN
function get_game_id() {
    local rom="$1"
    local line
    local hash
    local hash_i
    local gameid
    local console_id=0
    echo -n > "$GAME_CONSOLE_NAME"

    hash="$(get_rom_hash "$rom")" || return 1

    while read -r line; do
        echo "--- $line" >&2
        hash_i="$(echo "$line" | sed 's/^\(SNES\|NES\|Genesis\|plain MD5\): //')"
        line="$(grep "\"$hash_i\"" "$DATA_DIR"/*_hashlibrary.json 2> /dev/null)"
        echo -n "$(basename "${line%_hashlibrary.json*}")" > "$GAME_CONSOLE_NAME"
        gameid="$(echo ${line##*: } | tr -d ' ,')"
        [[ $gameid =~ $GAMEID_REGEX ]] && break
    done <<< "$hash"

    if [[ "$CHECK_RA_SERVER_FLAG" -eq 1 && ! $gameid =~ $GAMEID_REGEX ]]; then
        echo "--- checking at RetroAchievements.org server..." >&2
        for hash_i in $(echo "$hash" | sed 's/^\(SNES\|NES\|Genesis\|plain MD5\): //'); do
            echo "--- hash:    $hash_i" >&2
            gameid="$(curl -s "http://retroachievements.org/dorequest.php?r=gameid&m=$hash_i" | jq .GameID)"
            if [[ $gameid =~ $GAMEID_REGEX ]]; then
                # if the logic reaches this point, mark this game's console to download the hashlibrary
                console_id="$(
                    curl -s "http://retroachievements.org/dorequest.php?r=patch&u=${RA_USER}&g=${gameid}&f=3&l=1&t=${RA_TOKEN}" \
                        | jq '.PatchData.ConsoleID'
                )"
                echo -n "${CONSOLE_NAME[console_id]}" > "$GAME_CONSOLE_NAME"
                break
            fi
        done
    fi

    if [[ "$gameid" == 0 ]]; then
        echo "WARNING: this ROM file doesn't feature achievements." >&2
        return 1
    fi

    if [[ ! $gameid =~ $GAMEID_REGEX ]]; then
        echo "ERROR: \"$rom\": unable to get game ID." >&2
        return 1
    fi

    # if the logic reaches this point, we have a valid game ID

    [[ "$console_id" -ne 0 ]] && download_hashlibrary "$console_id"

    echo "$gameid"
}


# Check if a game has cheevos.
# returns 0 if yes; 1 if not; 2 if an error occurred
function game_has_cheevos() {
    local gameid="$1"
    local hascheevos_file
    local boolean
    local game_title

    if [[ ! $gameid =~ $GAMEID_REGEX ]]; then
        echo "ERROR: \"$gameid\" invalid game ID." >&2
        return 1
    fi

    echo "--- game ID: $gameid" >&2

    # check if $DATA_DIR exist.
    if [[ ! -d "$DATA_DIR" ]]; then
        echo "ERROR: \"$DATA_DIR\": directory not found!" >&2
        echo "Looks like this tool wasn't installed as instructed in repo's README." >&2
        echo "Aborting..." >&2
        safe_exit 1
    fi

    if [[ "$CHECK_RA_SERVER_FLAG" -ne 1 ]]; then
        hascheevos_file="$(grep -l "^$gameid:" "$DATA_DIR"/*_hascheevos-local.txt 2> /dev/null)"
        [[ -f "$hascheevos_file" ]] || hascheevos_file="$(grep -l "^$gameid:" "$DATA_DIR"/*_hascheevos.txt 2> /dev/null)"

        if [[ -f "$hascheevos_file" ]]; then
            boolean="$(   grep "^$gameid:" "$hascheevos_file" | cut -d: -f2)"
            game_title="$(grep "^$gameid:" "$hascheevos_file" | sed 's/^[^:]\+:[^:]\+://' )"
            [[ -n "$game_title" ]] && echo "--- Game Title: $game_title" >&2
            [[ "$boolean" == true ]] && return 0
            [[ "$boolean" == false && "$CHECK_FALSE_FLAG" -eq 0 ]] && return 1
        fi
    fi

    [[ -z "$RA_TOKEN" ]] && get_cheevos_token

    echo "--- checking at RetroAchievements.org server..." >&2
    local patch_json="$(curl -s "http://retroachievements.org/dorequest.php?r=patch&u=${RA_USER}&g=${gameid}&f=3&l=1&t=${RA_TOKEN}")"

    local console_id="$(echo "$patch_json" | jq '.PatchData.ConsoleID')"
    if [[ "$console_id" -lt 1 || "$console_id" -gt "${#CONSOLE_NAME[@]}" || "$console_id" == null || -z "$console_id" ]]; then
        echo "--- WARNING: unable to find the Console ID for Game #$gameid!" >&2
        return 1
    fi
    hascheevos_file="$DATA_DIR/${CONSOLE_NAME[console_id]}_hascheevos-local.txt"

    game_title="$(echo "$patch_json" | jq '.PatchData.game_title')"
    [[ "$game_title" == null ]] && game_title=
    [[ -n "$game_title" ]] && echo "--- Game Title: $game_title" >&2

    local number_of_cheevos="$(echo "$patch_json" | jq '.PatchData.Achievements | length')"

    # if the game has no cheevos...
    if [[ -z "$number_of_cheevos" || "$number_of_cheevos" -lt 1 ]]; then
        sed -i "s/^${gameid}:true/${gameid}:false/" "$hascheevos_file" 2> /dev/null
        if ! grep -q "^${gameid}:false" "$hascheevos_file" 2> /dev/null; then
            echo "${gameid}:false:${game_title}" >> "$hascheevos_file"
            sort -un "$hascheevos_file" -o "$hascheevos_file"
        fi
        return 1
    fi

    # if the logic reaches this point, the game has cheevos.
    sed -i "s/^${gameid}:false/${gameid}:true/" "$hascheevos_file" 2> /dev/null
    if ! grep -q "^${gameid}:true" "$hascheevos_file" 2> /dev/null; then
        echo "${gameid}:true:${game_title}" >> "$hascheevos_file"
        sort -un "$hascheevos_file" -o "$hascheevos_file"
    fi

    sleep 1 # XXX: a small delay to not stress the server
    return 0
}


# print the hash of a given rom file
function get_rom_hash() {
    local rom="$1"
    local hash
    local uncompressed_rom
    local ret=0

    case "$rom" in
        *.zip|*.ZIP)
            uncompressed_rom="$TMP_DIR/$(unzip -Z1 "$rom" | head -1)"
            unzip -o -d "$TMP_DIR" "$rom" >/dev/null
            validate_rom_file "$uncompressed_rom" || ret=1
            ;;
        *.7z|*.7Z)
            uncompressed_rom="$TMP_DIR/$(7z l -slt "$rom" | sed -n 's/^Path = //p' | sed '2q;d')"
            7z e -y -bd -o"$TMP_DIR" "$rom" >/dev/null
            validate_rom_file "$uncompressed_rom" || ret=1
            ;;
    esac
    if [[ $ret -ne 0 ]]; then
        rm -f "$uncompressed_rom"
        return $ret
    fi

    if [[ -n "$uncompressed_rom" ]]; then
        hash="$($SCRIPT_DIR/cheevoshash "$uncompressed_rom")"
        rm -f "$uncompressed_rom"
    else
        hash="$($SCRIPT_DIR/cheevoshash "$rom")"
    fi

    [[ "$hash" =~ :\ [^\ ]{32} ]] || return 1
    echo "$hash"
}


# check if the file exists and has a valid extension
function validate_rom_file() {
    local rom="$1"

    if [[ -z "$rom" ]]; then
        echo "ERROR: missing ROM file name." >&2
        echo "$USAGE" >&2
        return 1
    fi

    if [[ ! -f "$rom" ]]; then
        echo "ERROR: \"$rom\": file not found!" >&2
        return 1
    fi

    if [[ ! "${rom##*.}" =~ ^($EXTENSIONS)$ ]]; then
        echo "ERROR: \"$rom\": invalid file extension." >&2
        return 1
    fi

    return 0
}


# Check if a game has cheevos.
# returns 0 if yes; 1 if not; 2 if an error occurred
function rom_has_cheevos() {
    local rom="$1"
    validate_rom_file "$rom" || return 1

    echo "Checking \"$rom\"..." >&2

    local gameid
    gameid="$(get_game_id "$rom")" || return 1

    game_has_cheevos "$gameid"
}


# check if the local hascheevos repository is synchronized with the remote one.
# returns
#   0 if yes
#   1 if no
#   2 if unable to check
function is_updated() {
    local version_local
    local version_remote

    version_local=$(git -C "$SCRIPT_DIR" log -1 --format="%H") || return 2
    version_remote=$(git ls-remote "$GIT_REPO" | head -1 | cut -f1) || return 2

    [[ "$version_local" == "$version_remote" ]]
}



function check_hascheevos_files() {
    local file_local
    local file_orig
    local line_local
    local line_orig
    local gameid
    local bool_local
    local bool_orig
    local ret=0
    local updated

    while read -r file_local; do
        file_orig="${file_local/-local/}"
        echo "Checking \"$(basename "$file_orig")\"..."

        while read -r line_local; do
            gameid=$(echo "$line_local" | cut -d: -f1)

            line_orig=$(grep "^$gameid:" "$file_orig")
            if [[ -z "$line_orig" ]]; then
                echo "* there's no Game ID #$gameid on your \"$(basename "$file_orig")\"."
                ret=1
            elif [[ "$line_local" == "$line_orig" ]]; then
                sed -i "/$line_local/d" "$file_local"
                [[ -s "$file_local" ]] || rm "$file_local"
            else
                bool_local=$(echo "$line_local" | cut -d: -f2)
                bool_orig=$( echo "$line_orig"  | cut -d: -f2)
                echo "* Game ID #$gameid is marked as \"$bool_local\" locally but it's \"$bool_orig\" in the original file."
                ret=1
            fi

        done < "$file_local"
    done < <(find "$DATA_DIR" -type f -name '*_hascheevos-local.txt')

    is_updated
    updated="$?"
    if [[ "$updated" -eq 0 && "$ret" -ne 0 ]]; then
        echo -e "\n-----"
        echo "Consider helping to keep the hascheevos files synchronized with RetroAchievements.org data."
        echo "Please, copy the output's content above and paste it in a new issue at https://github.com/meleu/hascheevos/issues"
    elif [[ "$updated" -eq 1 ]]; then
        echo "WARNING: your hascheevos files are outdated. Try to '--update' and then '--check-hascheevos' again."
    fi

    safe_exit "$ret"
}


# helping to deal with command line arguments
function check_argument() {
    # limitation: the argument 2 can NOT start with '-'
    if [[ -z "$2" || "$2" =~ ^- ]]; then
        echo "$1: missing argument" >&2
        return 1
    fi
}



# START HERE ##################################################################

trap safe_exit SIGHUP SIGINT SIGQUIT SIGKILL SIGTERM

[[ -z "$1" ]] && help_message

while [[ -n "$1" ]]; do
    case "$1" in

#H -h|--help                Print the help message and exit.
#H 
        -h|--help)
            help_message
            ;;

#H --update                 Update hascheevos files and exit.
#H 
        --update)
            update_files
            ;;

#H -u|--user USER           USER is your RetroAchievements.org username.
#H 
        -u|--user)
            check_argument "$1" "$2" || safe_exit 1
            shift
            RA_USER="$1"
            ;;

#H -p|--password PASSWORD   PASSWORD is your RetroAchievements.org password.
#H 
        -p|--password)
            check_argument "$1" "$2" || safe_exit 1
            shift
            RA_PASSWORD="$1"
            ;;

#H -t|--token TOKEN         TOKEN is your RetroAchievements.org token.
#H 
        -t|--token)
            check_argument "$1" "$2" || safe_exit 1
            shift
            RA_TOKEN="$1"
            get_cheevos_token
            ;;

#H -g|--game-id GAME_ID     Check if there are cheevos for a given GAME_ID and 
#H                          exit. Accept game IDs separated by commas, ex: 1,2,3
#H                          Note: this option should be the last argument.
#H 
        -g|--game-id)
            check_argument "$1" "$2" || safe_exit 1
            ret=0
            IFS=, # XXX: not sure if it will impact other parts
            for i in $2; do
                if game_has_cheevos "$i"; then
                    echo "--- Game ID $i HAS CHEEVOS!" >&2
                else
                    echo "--- Game ID $i has no cheevos. :(" >&2
                    ret=1
                fi
            done
            safe_exit "$ret"
            ;;

#H --get-hashlibs           Download JSON hash libraries for all supported
#H                          consoles and exit.
#H 
        --get-hashlibs)
            download_hash_libraries
            safe_exit
            ;;

#H -f|--check-false         Check at RetroAchievements.org server even if the game
#H                          ID is marked as "has no cheevos" (false) in the
#H                          *_hascheevos.txt files. Implies --check-ra-server.
#H 
        -f|--check-false)
            CHECK_FALSE_FLAG=1
            CHECK_RA_SERVER_FLAG=1
            ;;

#H -r|--check-ra-server     Check at RetroAchievements.org remote server if fail
#H                          to find info locally.
#H 
        -r|--check-ra-server)
            CHECK_RA_SERVER_FLAG=1
            ;;

#H -d|--copy-roms-to DIR    Create a copy of the ROMs that has cheevos and put
#H                          them at "DIR/ROM_CONSOLE_NAME/". There's no need to
#H                          specify the console name, the script detects it.
#H 
        -d|--copy-roms-to)
            check_argument "$1" "$2" || safe_exit 1
            shift
            COPY_ROMS_FLAG=1
            COPY_ROMS_DIR="$1"
            ;;

#H -c|--check-hascheevos    Check if the *_hascheevos.txt files are outdated
#H                          comparing them with the respective *_hascheevos-local.txt
#H                          and exit.
#H 
        -c|--check-hascheevos)
            check_hascheevos_files
            ;;

#H --print-token            Print the user's RetroAchievements.org token and exit.
#H 
        --print-token)
            get_cheevos_token
            echo "$RA_TOKEN"
            safe_exit
            ;;

        *)  break
            ;;
    esac
    shift
done

get_cheevos_token
update_hash_libraries

for f in "$@"; do
    if rom_has_cheevos "$f"; then
        echo -n "--- \"" >&2
        echo -n "$f"
        echo "\" HAS CHEEVOS!" >&2
        if [[ "$COPY_ROMS_FLAG" -eq 1 ]]; then
            console_name="$(cat "$GAME_CONSOLE_NAME")"
            mkdir -p "$COPY_ROMS_DIR/$console_name"
            cp -v "$f" "$COPY_ROMS_DIR/$console_name"
        fi
        echo
    else
        echo -e "\"$f\" has no cheevos. :(\n" >&2
    fi
done

safe_exit
