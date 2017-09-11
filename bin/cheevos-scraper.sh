#!/bin/bash
# cheevos-scraper.sh
####################
#
# meleu

# globals #####################################################################
readonly RP_ROMS_DIR="$HOME/RetroPie/roms"
readonly CONFIG_DIR="/opt/retropie/configs"
readonly SCRIPT_DIR="$(cd "$(dirname $0)" && pwd)"
readonly HASCHEEVOS_SH="$SCRIPT_DIR/hascheevos.sh"
readonly EXTENSIONS='zip|7z|nes|fds|gb|gba|gbc|sms|bin|smd|gen|md|sg|smc|sfc|fig|swc|mgd|iso|cue|z64|n64|v64|pce|ccd|cue'
readonly VALID_SYSTEMS="megadrive|genesis|n64|snes|gb|gba|gbc|nes|pcengine|segacd|sega32x|mastersystem"

readonly TMP_DIR="/tmp/hascheevos-$$"
mkdir -p "$TMP_DIR"

readonly USAGE="
USAGE:
$(basename "$0") [OPTIONS] [--system SYSTEM] [-r ROMFILE]"


SYSTEM=
RA_USER=
RA_TOKEN=
GAMELIST=
GAMELIST_BAK=




# functions ###################################################################

function safe_exit() {
    rm -rf "$TMP_DIR"
    if [[ -f "$GAMELIST" && -f "$GAMELIST_BAK" ]]; then
        diff "$GAMELIST" "$GAMELIST_BAK" > /dev/null && rm -f "$GAMELIST_BAK"
    fi
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


# a trick for getting the system based on the folder where the rom is stored
function get_rom_system() {
    echo "$1" | sed 's|\(.*/RetroPie/roms/[^/]*\).*|\1|' | xargs basename
}


# Getting cheevos account info (needed to retrieve achievements list).
function get_cheevos_token() {
    iniConfig ' = ' '"'

    local global_retroarchcfg="$CONFIG_DIR/all/retroarch.cfg"
    local retroarchcfg="$CONFIG_DIR/$SYSTEM/retroarch.cfg"
    local cfgfile
    local password

    if [[ -n "$SYSTEM" && -f "$retroarchcfg" ]]; then
        included_cfgs="$(sed '/^#include/!d; s/^#include \+"\([^"]*\)"/\1/' "$retroarchcfg")"
        # FIXME: a path with space can cause problems here
        for cfgfile in "$retroarchcfg" $included_cfgs ; do
            iniGet cheevos_username "$cfgfile"
            RA_USER="$ini_value"
            [[ -z "$RA_USER" ]] && continue

            iniGet cheevos_password "$cfgfile"
            password="$ini_value"
            break
        done
    else
        iniGet cheevos_username "$global_retroarchcfg"
        RA_USER="$ini_value"
        iniGet cheevos_password "$global_retroarchcfg"
        password="$ini_value"
    fi

    if [[ -z "$RA_USER" || -z "$password" ]]; then
        echo "ERROR: failed to get cheevos account info. Aborting..." >&2
        safe_exit 1
    fi

    RA_TOKEN="$("$HASCHEEVOS_SH" --user "$RA_USER" --password "$password" --print-token)"
    if [[ "$RA_TOKEN" == null || -z "$RA_TOKEN" ]]; then
        echo "ERROR: cheevos authentication failed. Aborting..." >&2
        safe_exit 1
    fi
}


# update gamelist.xml info
function set_cheevos_gamelist_xml() {
    local set="$2"
    local rom_full_path="$1"
    local rom
    local game_name
    local new_entry_flag
    local has_cheevos_xml_element

    SYSTEM=$(get_rom_system "$rom_full_path")

    [[ -f "$rom_full_path" ]] || return 1
    rom="$(basename "$rom_full_path")"

    # From https://github.com/RetroPie/EmulationStation/blob/master/GAMELISTS.md
    # ES will check three places for a gamelist.xml in the following order, using
    # the first one it finds:
    # - [SYSTEM_PATH]/gamelist.xml
    # - ~/.emulationstation/gamelists/[SYSTEM_NAME]/gamelist.xml
    # - /etc/emulationstation/gamelists/[SYSTEM_NAME]/gamelist.xml
    for GAMELIST in \
        "$RP_ROMS_DIR/$SYSTEM/gamelist.xml" \
        "$HOME/.emulationstation/gamelists/$SYSTEM/gamelist.xml" \
        "/etc/emulationstation/gamelists/$SYSTEM/gamelist.xml"
    do
        [[ -f "$GAMELIST" ]] && break
        GAMELIST=
    done
    [[ -f "$GAMELIST" ]] || return 1

    GAMELIST_BAK="${GAMELIST}-$(date +'%Y%m%d-%H%M%S').bak"
    cp "$GAMELIST" "$GAMELIST_BAK"

    # if set != true, just delete <achievements> element (it's considered false).
    if [[ "$set" != true ]]; then
        xmlstarlet ed -L -d "/gameList/game[contains(path,\"$rom\")]/achievements" "$GAMELIST"
        return "$?"
    fi

    # 0 means new entry
    new_entry_flag="$(xmlstarlet sel -t -v "count(/gameList/game[contains(path,\"$rom\")])" "$GAMELIST")"

    # 0 means no <achievements> xml element
    has_cheevos_xml_element="$(xmlstarlet sel -t -v "count(/gameList/game[contains(path,\"$rom\")]/achievements)" "$GAMELIST")"

    # if it's a new entry in gamelist.xml...
    if [[ "$new_entry_flag" -eq 0 ]]; then
        game_name="${rom%.*}"
        xmlstarlet ed -L -s "/gameList" -t elem -n "game" -v "" \
            -s "/gameList/game[last()]" -t elem -n "name" -v "$game_name" \
            -s "/gameList/game[last()]" -t elem -n "path" -v "$rom_full_path" \
            -s "/gameList/game[last()]" -t elem -n "achievements" -v "true" \
            "$GAMELIST" || return 1
    elif [[ "$has_cheevos_xml_element" -gt 0 ]]; then
        xmlstarlet ed -L \
            -u "/gameList/game[contains(path,\"$rom\")]/achievements" -v "true" \
            "$GAMELIST" || return 1
    else
        xmlstarlet ed -L \
            -s "/gameList/game[contains(path,\"$rom\")]" -t elem -n achievements -v "true" \
            "$GAMELIST" || return 1
    fi
}


function scrape_system() {
    if [[ ! "$SYSTEM" =~ ^($VALID_SYSTEMS)$ ]]; then
        echo "WARNING: \"$SYSTEM\" is an invalid system." >&2
        return 1
    fi

    local rom
    local rom_list="$(mktemp -p "$TMP_DIR" "$SYSTEM".XXXX)"
    local roms_dir="$RP_ROMS_DIR/$SYSTEM"

    if [[ ! -d "$roms_dir" ]]; then
        echo "WARNING: \"$roms_dir\": directory not found!" >&2
        return 1
    fi

    find "$roms_dir" -type f -regextype egrep -iregex ".*\.($EXTENSIONS)$" | sort > "$rom_list"

    # TODO: files that doesn't have cheevos should be marked as false?
    while read -r rom; do
        set_cheevos_gamelist_xml "$rom" "true"
    done < <(xargs -a "$rom_list" -d '\n' "$HASCHEEVOS_SH" --user "$RA_USER" --token "$RA_TOKEN")
}


function scrape_rom() {
    local rom="$1"
    local rom_hascheevos

    if [[ ! -f "$rom" ]]; then
        echo "WARNING: \"$rom\": file not found!" >&2
        return 1
    fi

    rom_hascheevos="$("$HASCHEEVOS_SH" --user "$RA_USER" --token "$RA_TOKEN" "$rom")"

    # TODO: files that doesn't have cheevos should be marked as false?
    if [[ -n "$rom_hascheevos" ]]; then
        set_cheevos_gamelist_xml "$rom_hascheevos" "true"
        return $?
    else
        return 1
    fi
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

if [[ ! -d "$RP_ROMS_DIR" ]]; then
    echo "ERROR: \"$RP_ROMS_DIR\" directory not found! Aborting..."
    safe_exit 1
fi

if ! source /opt/retropie/lib/inifuncs.sh ; then
    echo "ERROR: \"inifuncs.sh\" file not found! Aborting..." >&2
    safe_exit 1
fi

get_cheevos_token

while [[ -n "$1" ]]; do
    case "$1" in

#H -h|--help                Print the help message and exit.
#H 
        -h|--help)
            help_message
            ;;

#H -s|--system SYSTEM       Check if every ROM for the give system(s) have
#H                          achievements. Accept more than one system separated
#H                          by commas, ex: nes,megadrive,gba
#H 

        -s|--system)
            check_argument "$1" "$2" || safe_exit 1
            ret=0
            IFS=, # XXX: not sure if it will impact other parts
            for i in $2; do
                scrape_system "$i" || ((ret++))
            done
            safe_exit "$ret"
            ;;

#H -r|--rom ROM             Check if a ROM file has achievements. Accept more than
#H                          one ROM separated by commas.
#H 
        -r|--rom)
            check_argument "$1" "$2" || safe_exit 1
            ret=0
            IFS=, # XXX: not sure if it will impact other parts
            for i in $2; do
                scrape_rom "$i" || ((ret++))
            done
            safe_exit "$ret"
            ;;

        *)  break
            ;;
    esac
    shift
done

safe_exit
