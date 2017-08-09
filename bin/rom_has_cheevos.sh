#!/bin/bash
# rom_has_cheevos.sh
####################
#
# A tool to check if your ROMs have cheevos (RetroAchievements.org).
#
# valid ROM extensions for:
# nes, snes, megadrive, mastersystem, gb, gba, gbc, n64, pcengine
# zip 7z nes fds gb gba gbc sms bin smd gen md sg smc sfc fig swc mgd iso cue z64 n64 v64 pce ccd cue

        

# TODO: check dependencies curl, xmlstarlet, jq, md5sum, zcat, 7z,
#       cheevoshash (from this repo).

readonly USAGE="USAGE:
$0 romfile1 [romfile2 ...]"

readonly GAMEID_REGEX='^[1-9][0-9]{1,9}$'

# the extensions below was taken from RetroPie's configs
EXTENSIONS='zip|7z|nes|fds|gb|gba|gbc|sms|bin|smd|gen|md|sg|smc|sfc|fig|swc|mgd|iso|cue|z64|n64|v64|pce|ccd|cue'
SCRIPT_DIR="$(dirname $0)"
SCRIPT_DIR="$(cd "$SCRIPT_DIR" && pwd)"
# TODO: DATA_DIR="$SCRIPT_DIR/data"
DATA_DIR="$SCRIPT_DIR"
RA_USER=
RA_PASSWORD=
RA_TOKEN=

# Getting cheevos account info (needed to retrieve achievements list).
function get_cheevos_token() {
    local user="$RA_USER"
    local password="$RA_PASSWORD"

    if [[ -z "$user" ]]; then
        echo "ERROR: undefined RetroAchievements.org user (see \"--user\" option)." >&2
        exit 1
    fi

    if [[ -z "$password" ]]; then
        echo "ERROR: undefined RetroAchievements.org password (see \"--password\" option)." >&2
        exit 1
    fi

    RA_TOKEN="$(curl -s "http://retroachievements.org/dorequest.php?r=login&u=${USER}&p=${password}" | jq -r .Token)"
    if [[ "$RA_TOKEN" == null || -z "$RA_TOKEN" ]]; then
        echo "ERROR: cheevos authentication failed. Aborting..."
        exit 1
    fi

#    if [[ -z "$user" || -z "$password" ]]; then
#        # just a shortcut for RetroPie users
#        local retroarchcfg="/opt/retropie/configs/all/retroarch.cfg"
#        if [[ -f "$retroarchcfg" ]]; then
##        's/^[ |\t]*cheevos_username[ |\t]*=[ |\t]*"*\([^"|\r]*\)"*.*/\1/p'
#            local regex1="^[ |\t]*"
#            local regex2="[ |\t]*=[ |\t]*\"*\([^\"|\r]*\)\"*.*"
#            user="$(sed -n "s/${regex1}cheevos_username${regex2}/\1/p" "$retroarchcfg")"
#            password="$(sed -n "s/${regex1}cheevos_password${regex2}/\1/p" "$retroarchcfg")"
#        fi
#    fi

}


# print the game ID of a given rom file
# XXX: DONE!
function get_game_id() {
    local rom="$1"
    local hash
    local hash_i
    local gameid

    hash="$(get_rom_hash "$rom")" || return 1

    for hash_i in $(echo "$hash" | sed 's/^\(SNES\|NES\|Genesis\|plain MD5\): //'); do
        echo "--- hash:    $hash_i" >&2
        gameid="$(grep -h "\"$hash_i\"" *_hashlibrary.json | cut -d: -f2 | tr -d ' ,')"
        [[ $gameid =~ $GAMEID_REGEX ]] && break
    done

    if [[ ! $gameid =~ $GAMEID_REGEX ]]; then
        echo "--- checking at RetroAchievements.org server..." >&2
        for hash_i in $(echo "$hash" | sed 's/^\(SNES\|NES\|Genesis\|plain MD5\): //'); do
            echo "--- hash:    $hash_i" >&2
            gameid="$(curl -s "http://retroachievements.org/dorequest.php?r=gameid&m=$hash_i" | jq .GameID)"
            [[ $gameid =~ $GAMEID_REGEX ]] && break
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
    echo "$gameid"
}


# Check if a game has cheevos.
# returns 0 if yes; 1 if not; 2 if an error occurred
function game_has_cheevos() {
    local gameid="$1"
    local hascheevos_file
    local boolean

    if [[ ! $gameid =~ $GAMEID_REGEX ]]; then
        echo "ERROR: \"$gameid\" invalid game ID." >&2
        return 1
    fi

    echo "--- game ID: $gameid" >&2

    # TODO: checar se $DATA_DIR existe, do contrário pode sobrecarregar o servidor
    hascheevos_file="$(grep -l "^$gameid:" "$DATA_DIR"/*_hascheevos.txt)"
    if [[ -f "$hascheevos_file" ]]; then
        boolean="$(grep "^$gameid:" "$hascheevos_file" | cut -d: -f2)"
        [[ "$boolean" == false ]] && return 1
        [[ "$boolean" == true  ]] && return 0
    fi
    
    [[ -z "$RA_TOKEN" ]] && get_cheevos_token

    echo "--- checking at RetroAchievements.org server..." >&2
    local number_of_cheevos="$(
        curl -s "http://retroachievements.org/dorequest.php?r=patch&u=${USER}&g=${gameid}&f=3&l=1&t=${RA_TOKEN}" \
        | jq '.PatchData.Achievements | length'
    )"
    [[ -z "$number_of_cheevos" || "$number_of_cheevos" -lt 1 ]] && return 1

    # if the logic reaches this point, the game has cheevos
    # XXX: update the _hascheevos.txt accordingly?
    sleep 1 # XXX: a small delay to not stress the server
    return 0
}


# print the hash of a given rom file
# DEP: cheevoshash, zcat, 7z
function get_rom_hash() {
    local rom="$1"
    local hash
    local uncompressed_rom

    case "$rom" in
        # TODO: check if "inflating" and "Extracting" are really OK for any locale config
        *.zip|*.ZIP)
            uncompressed_rom="$(unzip -o -d /tmp "$rom" | sed -e '/\/tmp/!d; s/.*inflating: //; s/ *$//')"
            validate_rom_file "$uncompressed_rom" || return 1
            hash="$(./cheevoshash "$uncompressed_rom")"
            rm -f "$uncompressed_rom"
            ;;
        *.7z|*.7Z)
            uncompressed_rom="/tmp/$(7z e -y -bd -o/tmp "$rom" | sed -e '/Extracting/!d; s/Extracting  //')"
            validate_rom_file "$uncompressed_rom" || return 1
            hash="$(./cheevoshash "$uncompressed_rom")"
            rm -f "$uncompressed_rom"
            ;;
        *)
            hash="$(./cheevoshash "$rom")"
            ;;
    esac
    [[ "$hash" =~ :\ [^\ ]{32} ]] || return 1
    echo "$hash"
}


function validate_rom_file() {
    local rom="$1"

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

    if game_has_cheevos "$gameid"; then
        echo "--- HAS CHEEVOS!" >&2
    else
        echo "--- no cheevos. :(" >&2
    fi
    return $?
}


# FUNCTIONS TO DEAL WITH ARGUMENTS ############################################

function check_argument() {
    # limitation: the argument 2 can NOT start with '-'
    if [[ -z "$2" || "$2" =~ ^- ]]; then
        echo "$1: missing argument" >&2
        return 1
    fi
}



# START HERE ##################################################################

while [[ -n "$1" ]]; do
    case "$1" in

#H -h|--help                Print the help message and exit.
#H 
        -h|--help)
            echo "$USAGE"
            echo
            # getting the help message from the comments in this source code
            sed -n 's/^#H //p' "$0"
            exit 0
            ;;

#H -u|--user USER           USER is your RetroAchievements.org username.
#H 
        -u|--user)
            check_argument "$1" "$2" || exit 1
            shift
            RA_USER="$1"
            ;;

#H -p|--password PASSWORD   PASSWORD is your RetroAchievements.org password.
#H 
        -p|--password)
            check_argument "$1" "$2" || exit 1
            shift
            RA_PASSWORD="$1"
            ;;

        *)  break
            ;;
    esac
    shift
done

get_cheevos_token
rom_has_cheevos "$1" || exit 1
