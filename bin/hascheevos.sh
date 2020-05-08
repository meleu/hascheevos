#!/bin/bash
# hascheevos.sh
###############
#
# A tool to check if your ROMs have cheevos (RetroAchievements.org).

# globals ####################################################################

readonly USAGE="
USAGE:
$(basename "$0") [OPTIONS] romfile1 [romfile2 ...]"

readonly GIT_REPO="https://github.com/meleu/hascheevos.git"
readonly SCRIPT_URL="https://raw.githubusercontent.com/meleu/hascheevos/master/bin/hascheevos.sh"
readonly SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
readonly DATA_DIR="$SCRIPT_DIR/../data"
readonly GAMEID_REGEX='^[1-9][0-9]{0,9}$'
readonly HASH_REGEX='[A-Fa-f0-9]{32}'
readonly URL="https://retroachievements.org"
#readonly URL='http://localhost'

# flags
CHECK_FALSE_FLAG=0
COPY_ROMS_FLAG=0
CHECK_RA_SERVER_FLAG=0
TAB_FLAG=0
ARCADE_FLAG=0

RA_USER=
RA_PASSWORD=
RA_TOKEN=
FILES_TO_CHECK=()
COPY_ROMS_DIR=
TMP_DIR="/tmp/hascheevos-$$"
mkdir -p "$TMP_DIR"
GAME_CONSOLE_NAME="$(mktemp -p "$TMP_DIR")"

# these will be increased later, based on extensions supported by the systems
EXTENSIONS='zip|7z'
SUPPORTED_SYSTEMS=()
declare -A CONSOLE_IDS

# format: [consoleId]='sysname|sysalias:ext1|extN:Long Name'
# see the console IDs here: https://github.com/RetroAchievements/RAWeb/blob/master/lib/database/release.php
declare -A SYSTEMS_INFO=(
    [1]='megadrive|genesis:bin|gen|md|sg|smd:Sega Mega Drive:genesis'
    [2]='n64:z64|n64|v64:Nintendo 64'
    [3]='snes:fig|mgd|sfc|smc|swc:Super Nintendo Entertainment System'
    [4]='gb:gb:GameBoy'
    [5]='gba:gba:GameBoy Advance'
    [6]='gbc:gbc:GameBoy Color'
    [7]='nes|fds:nes|fds:fds:Nintendo Entertainment System'
    [8]='pcengine|pcenginecd|tg16|tg16cd:ccd|chd|cue|:PC Engine'
    [9]='segacd:bin|chd|cue|iso:Sega CD'
    [10]='sega32x:32x|bin|md|smd:Sega 32X'
    [11]='mastersystem:bin|sms:Sega Master System'
    [12]='psx:cue|ccd|chd|exe|iso|m3u|pbp|toc:PlayStation'
    [13]='atarilynx:lnx:Atari Lynx'
    [14]='ngp|ngpc:ngp|ngc:NeoGeo Pocket [Color]'
    [15]='gamegear:bin|gg|sms:Game Gear'
    [17]='atarijaguar:j64|jag:Atari Jaguar'
    [18]='nds:nds:Nintendo DS'
    [24]='pokemini:min:Pokemon Mini'
    [25]='atari2600:a26|bin|rom:Atari 2600'
    [27]='arcade|fba|fbneo:fba:Arcade'
    [28]='virtualboy:vb:VirtualBoy'
    [33]='sg-1000:bin|sg:SG-1000'
    [44]='coleco:col|rom:ColecoVision'
    [51]='atari7800:a78|bin:Atari 7800'
    [53]='wonderswan|wonderswancolor:ws|wsc:WonderSwan [Color]'
)

# RetroPie specific variables
readonly RP_ROMS_DIR="$HOME/RetroPie/roms"
GAMELIST=
GAMELIST_BAK=
ROMS_DIR=()
SCRAPE_FLAG=0
COLLECTIONS_FLAG=0
SINGLE_COLLECTION_FLAG=0


# functions ###################################################################

function safe_exit() {
    rm -rf "$TMP_DIR"
    if [[ -f "$GAMELIST" && -f "$GAMELIST_BAK" ]]; then
        diff "$GAMELIST" "$GAMELIST_BAK" > /dev/null && rm -f "$GAMELIST_BAK"
    fi
    exit "$1"
}



function urlencode() {
    local LC_ALL=C
    local string="$*"
    local length="${#string}"
    local char

    for (( i = 0; i < length; i++ )); do
        char="${string:i:1}"
        if [[ "$char" == [a-zA-Z0-9.~_-] ]]; then
            printf "$char" 
        else
            printf '%%%02X' "'$char" 
        fi
    done
}



function get_password() {
    local password
    IFS= read -rsp 'Enter the password: ' password < /dev/tty && urlencode "$password"
}



function join_by() {
    local IFS="$1"
    echo "${*:2}"
}



function fill_data() {
    local sysnames
    local sysname
    local id
    local entry
    local temp_extensions

    for id in "${!SYSTEMS_INFO[@]}"; do
        entry="${SYSTEMS_INFO[$id]}"
        sysnames="$(cut -d: -f1 <<< "$entry" | tr '|' ' ' )"

        SUPPORTED_SYSTEMS+=( $sysnames ) # <--- no quotes is mandatory

        # no quotes in $sysnames below is mandatory
        for sysname in $sysnames; do
            CONSOLE_IDS[$sysname]="$id"
        done

        temp_extensions="$(
            join_by '|' "$temp_extensions" "$(cut -d: -f2 <<< "$entry" )" 
        )"
    done

    EXTENSIONS="$(join_by '|' "$EXTENSIONS" $(tr '|' '\n' <<< "$temp_extensions" | sort -u) )"
    #  this subshell must NOT be quoted! ---^
}



function get_console_shortname_by_id() {
    local id="$1"
    local shortname
    for shortname in "${!CONSOLE_IDS[@]}"; do
        if [[ "$id" == "${CONSOLE_IDS[$shortname]}" ]]; then
            echo "$shortname"
            return 0
        fi
    done
    return 1
}



function help_message() {
    echo "$USAGE"
    echo
    echo "Where [OPTIONS] are:"
    echo
    # getting the help message from the comments in this source code
    sed -n 's/^#H //p' "$0"
    safe_exit 0
}


function check_dependencies() {
    local cmd
    local answer
    local deps=(jq curl unzip 7z)

    for cmd in "${deps[@]}"; do
        if ! which "$cmd" >/dev/null 2>&1; then
            if ! which apt-get >/dev/null 2>&1; then
                echo "ERROR: missing dependency: $cmd" >&2
                echo "To use this tool you need to install \"$cmd\" package. Please, install it and try again."
                safe_exit 1
            fi
            echo "To use this tool you need to install \"$cmd\"."
            echo "Do you want to install \"$cmd\" now? (if you're sure, type \"yes\" and press ENTER)"
            read -p 'Answer: ' answer

            if ! [[ "$answer" =~ ^[Yy][Ee][Ss]$ ]]; then
                echo "Aborting..."
                safe_exit 1
            fi

            [[ "$cmd" == 7z ]] && cmd=p7zip-full
            sudo apt-get install "$cmd"
        fi
    done
}



function is_retropie() {
    if [[ -d "$RP_ROMS_DIR" ]]; then
        mkdir -p "$HOME/.emulationstation/collections"
        return 0
    fi
    return 1
}


function regex_safe() {
    sed -e 's/[]\/$*.^|[]/\\&/g' <<< "$@"
}


function get_game_title_hascheevos() {
    cut -d: -f3- <<< "$@"
}


# XXX: this function needs more intensive tests
function update() {
    local err_flag=0
    local dir="$SCRIPT_DIR/.."

    if [[ -d "$dir/.git" ]]; then
        pushd "$dir" > /dev/null
        if ! git pull --rebase 2>/dev/null; then
            git fetch && git reset --hard origin/master || err_flag=1
        fi
        popd > /dev/null
    else
        echo "ERROR: \"$dir/.git\": directory not found!" >&2
        echo "Looks like this tool wasn't installed as instructed in repo's README." >&2
        echo "Aborting..." >&2
        err_flag=1
    fi

    if [[ "$err_flag" != 0 ]]; then
        echo "UPDATE: Failed to update." >&2
        safe_exit 1
    fi

    echo
    echo "UPDATE: The files have been successfully updated."
    safe_exit 0
}


# Getting the RetroAchievements token
# input: RA_USER, RA_PASSWORD
# updates: RA_TOKEN
# exit if fails
# TODO: cache the token in some file?
function get_cheevos_token() {
    if [[ -z "$RA_USER" ]]; then
        echo "WARNING: undefined RetroAchievements.org user (see \"--user\" option)." >&2
        return 1
    fi

    [[ -n "$RA_TOKEN" ]] && return 0

    if [[ -z "$RA_PASSWORD" ]]; then
        echo "WARNING: undefined RetroAchievements.org password (see \"--password\" option)." >&2
        return 1
    fi

    RA_TOKEN="$(curl -s "$URL/dorequest.php?r=login&u=${RA_USER}&p=${RA_PASSWORD}" | jq -e -r .Token)"
    if [[ "$?" -ne 0 || "$RA_TOKEN" == null || -z "$RA_TOKEN" ]]; then
        echo "ERROR: cheevos authentication failed. Aborting..."
        safe_exit 1
    fi
}


function is_supported_system() {
    local sys
    local match="$1"
    for sys in "${SUPPORTED_SYSTEMS[@]}"; do
        [[ "$sys" == "$match" ]] && return 0
    done
    return 1
}



# download ra_data for a specific console
# $1 is the type of data you want: hashlibrary or officialgameslist
# $2 is the system shortname
function download_ra_data() {
    local data_type="$1"
    local system="$2"
    local json_file="$DATA_DIR/${system}_${data_type}.json"

    [[ $data_type =~ ^(officialgameslist|hashlibrary)$ ]] || return 1

    echo "--- getting the $data_type for \"$system\"..." >&2
    curl -s "$URL/dorequest.php?r=${data_type}&c=${CONSOLE_IDS[$system]}" \
        | jq '.' > "$json_file" 2> /dev/null \
        || echo "ERROR: failed to download $data_type for \"$system\"!" >&2

    [[ -s "$json_file" ]] || rm -f "$json_file"
}



# If a valid system is given in $1, the function tries to update only the
# data for that system.
# Otherwise update data older than 1 day.
function update_ra_data() {
    local given_system="$1"
    local line
    local file
    local system
    local data_type

    echo "Checking local files..." >&2
    for system in "${SUPPORTED_SYSTEMS[@]}"; do
        if ! [[ \
            -f "$DATA_DIR/${system}_hashlibrary.json" \
            || -f "$DATA_DIR/${system}_officialgameslist.json" \
        ]]; then
            download_ra_data officialgameslist "$system" \
                && download_ra_data hashlibrary "$system"
        fi
    done
    echo "Done!" >&2

    if [[ -n "$given_system" ]]; then
        file="$DATA_DIR/${given_system}_officialgameslist.json"

        # check if the file exists and is older than 1 minute
        if [[ -n "$(find "$file" -mmin +1 2>/dev/null)" ]]; then
            echo "Updating data for \"$given_system\"..." >&2
            download_ra_data officialgameslist "$given_system" \
                && download_ra_data hashlibrary "$given_system" \
                && echo "Done!" >&2
            return "$?"
        else
            if [[ -f "$file" ]]; then
                echo "The data for \"$given_system\" is already up-to-date." >&2
                return 0
            else
                echo "ERROR: invalid system: \"$given_system\""
                return 1
            fi
        fi
    else
        # update data older than one day
        while read -r line; do
            file="${line##*/}"
            file="${file%.json}"
            system="${file%_*}"
            data_type="${file#*_}"

            download_ra_data "$data_type" "$system"
        done < <(find "$DATA_DIR" -maxdepth 1 -type f -mtime +1 \
            \( -name "*_officialgameslist.json" -o -name "*_haslibrary.json" \)
        )
    fi
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
    local console_shortname

    echo -n > "$GAME_CONSOLE_NAME"

    hash="$(get_rom_hash "$rom")" || return 1

    while read -r line; do
        echo "--- $line" >&2
        hash_i="$(echo "$line" | sed 's/^\(SNES\|NES\|Genesis\|Lynx\|plain MD5\): //')"
        line="$(grep -i "\"$hash_i\"" "$DATA_DIR"/*_hashlibrary.json 2> /dev/null)"
        echo -n "$(basename "${line%_hashlibrary.json*}")" > "$GAME_CONSOLE_NAME"
        gameid="$(echo ${line##*: } | tr -d ' ,')"
        [[ $gameid =~ $GAMEID_REGEX ]] && break
    done <<< "$hash"

    if [[ "$CHECK_RA_SERVER_FLAG" -eq 1 && ! $gameid =~ $GAMEID_REGEX ]]; then
        echo "--- checking at RetroAchievements.org server..." >&2
        for hash_i in $(echo "$hash" | sed 's/^\(SNES\|NES\|Genesis\|Lynx\|plain MD5\): //'); do
            echo "--- hash:    $hash_i" >&2
            gameid="$(curl -s "$URL/dorequest.php?r=gameid&m=$hash_i" | jq .GameID)"
            if [[ $gameid =~ $GAMEID_REGEX ]]; then
                # if the logic reaches this point, mark this game's console to download the hashlibrary
                console_id="$(
                    curl -s "$URL/dorequest.php?r=patch&u=${RA_USER}&g=${gameid}&f=3&l=1&t=${RA_TOKEN}" \
                        | jq '.PatchData.ConsoleID'
                )"
                console_shortname="$(get_console_shortname_by_id "$console_id")"
                echo "$console_shortname" > "$GAME_CONSOLE_NAME"
                break
            fi
        done
    fi

    if [[ "$gameid" == 0 ]]; then
        echo "--- WARNING: this ROM file doesn't feature achievements." >&2
        return 1
    fi

    if [[ ! $gameid =~ $GAMEID_REGEX ]]; then
        echo "--- unable to get game ID." >&2
        return 1
    fi

    # if the logic reaches this point, we have a valid game ID

    [[ -n "$console_shortname" ]] && download_ra_data hashlibrary "$console_shortname"

    echo "$gameid"
}

##############################################################
# _____     ____        
#|_   _|__ |  _ \  ___  
#  | |/ _ \| | | |/ _ \ 
#  | | (_) | |_| | (_) |
#  |_|\___/|____/ \___/ 
#                       
##############################################################
#
# Check if a game has cheevos.
# returns 0 if yes; 1 if not; 2 if an error occurred
function game_has_cheevos() {
    local gameid="$1"
    local hascheevos_file
    local boolean
    local game_title
    local patch_json
    local console_id
    local console_shortname
    local number_of_cheevos

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
            game_title="$(get_game_title_hascheevos "$(grep "^$gameid:" "$hascheevos_file")" )"
            [[ -n "$game_title" ]] && echo "--- Game Title: $game_title" >&2
            [[ "$boolean" == true ]] && return 0
            [[ "$boolean" == false && "$CHECK_FALSE_FLAG" -eq 0 ]] && return 1
        fi
    fi

    if [[ -z "$RA_TOKEN" ]]; then
        get_cheevos_token || return $?
    fi

    echo "--- checking at RetroAchievements.org server..." >&2
    patch_json="$(curl -s "$URL/dorequest.php?r=patch&u=${RA_USER}&g=${gameid}&f=3&l=1&t=${RA_TOKEN}")"

    console_id="$(echo "$patch_json" | jq -e '.PatchData.ConsoleID')"
    if [[ "$?" -ne 0 || "$console_id" -lt 1 || -z "$console_id" ]]; then
        echo "--- WARNING: unable to find the Console ID for Game #$gameid!" >&2
        return 1
    fi

    console_shortname="$(get_console_shortname_by_id "$console_id")"

    hascheevos_file="$DATA_DIR/${console_shortname}_hascheevos-local.txt"

    game_title="$(echo "$patch_json" | jq -e '.PatchData.Title')" || game_title=
    [[ -n "$game_title" ]] && echo "--- Game Title: $game_title" >&2

    number_of_cheevos="$(echo "$patch_json" | jq '.PatchData.Achievements | length')"

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

    if [[ "$ARCADE_FLAG" == 1 ]]; then
        rom="$(basename "$rom")"
        hash="$(echo -n "${rom%.*}" | md5sum | grep -Eo "^$HASH_REGEX")"
        [[ "$hash" =~ ^$HASH_REGEX$ ]] || return 1
        echo "$hash"
        return 0
    fi

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
    local gameid

    echo "Checking \"$rom\"..." >&2
    [[ "$TAB_FLAG" == 1 ]] && echo -en "${rom/ (*/}"

    gameid="$(get_game_id "$rom")"
    if [[ -z "$gameid" ]]; then
        echo -e "\t"
        return 1
    fi

    [[ "$TAB_FLAG" == 1 ]] && echo -en "\t$gameid"

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



# a trick for getting the system based on the folder where the rom is stored
function get_rom_system() {
    echo "$1" | sed 's|\(.*/RetroPie/roms/[^/]*\).*|\1|' | xargs basename
}


# add the game to the system specific custom collection
# XXX: RetroPie specific
function set_cheevos_custom_collection() {
    [[ -f "$1" ]] || return 1

    local set="$2"
    local system
    local rom_full_path="$1"
    local collection_cfg

    if [[ "$SINGLE_COLLECTION_FLAG" == 1 ]]; then
        collection_cfg="$HOME/.emulationstation/collections/custom-achievements.cfg"
    else
        system=$(get_rom_system "$rom_full_path")
        collection_cfg="$HOME/.emulationstation/collections/custom-achievements ${system}.cfg"
    fi

    if [[ -z "$set" || "$set" == true ]]; then
        echo "$rom_full_path" >> "$collection_cfg"
    elif [[ "$set" == false ]]; then
        sed -i "/$(regex_safe "$rom_full_path")/d" "$collection_cfg"
    else
        return 1
    fi

    sort -o "$collection_cfg" -u "$collection_cfg" \
    && echo "--- This game has been added to \"$collection_cfg\"." >&2
}


# update gamelist.xml info
# TODO: will it be useful? this feature will be useful only if the related PR gets merged on ES.
# XXX: RetroPie specific
function set_cheevos_gamelist_xml() {
    local set="$2"
    local system
    local rom_full_path="$1"
    local rom
    local game_name
    local new_entry_flag
    local has_cheevos_xml_element

    system=$(get_rom_system "$rom_full_path")

    [[ -f "$rom_full_path" ]] || return 1
    rom="$(basename "$rom_full_path")"

    # From https://github.com/RetroPie/EmulationStation/blob/master/GAMELISTS.md
    # ES will check three places for a gamelist.xml in the following order, using
    # the first one it finds:
    # - [SYSTEM_PATH]/gamelist.xml
    # - ~/.emulationstation/gamelists/[SYSTEM_NAME]/gamelist.xml
    # - /etc/emulationstation/gamelists/[SYSTEM_NAME]/gamelist.xml
    for GAMELIST in \
        "$RP_ROMS_DIR/$system/gamelist.xml" \
        "$HOME/.emulationstation/gamelists/$system/gamelist.xml" \
        "/etc/emulationstation/gamelists/$system/gamelist.xml"
    do
        [[ -f "$GAMELIST" ]] && break
        GAMELIST=
    done
    [[ -f "$GAMELIST" ]] || return 1

    GAMELIST_BAK="${GAMELIST}-$(date +'%Y%m%d').bak"
    [[ -f "$GAMELIST_BAK" ]] || cp "$GAMELIST" "$GAMELIST_BAK"

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
    echo "--- This game has been defined as having cheevos in \"$GAMELIST\"." >&2
}


function process_files() {
    local f
    readonly local max=10
    
    # avoiding to stress the server
    if [[ "$CHECK_RA_SERVER_FLAG" == 1 && "$#" -gt "$max" ]]; then
        echo >&2
        echo "ABORTING!" >&2
        echo "Using the --check-ra-server option to check more than $max files isn't allowed!" >&2
        return 1
    fi

    for f in "$@"; do
        validate_rom_file "$f" || continue
        if rom_has_cheevos "$f"; then
            if [[ "$TAB_FLAG" == 1 ]]; then
                echo -e "\tx"
            else
                [[ "$COLLECTIONS_FLAG" -eq 1 ]] && set_cheevos_custom_collection "$f" true
                [[ "$SCRAPE_FLAG" -eq 1 ]]      && set_cheevos_gamelist_xml      "$f" true
                echo -n "--- \"" >&2
                echo -n "$f"
                echo "\" HAS CHEEVOS!" >&2
                echo
            fi

            if [[ "$COPY_ROMS_FLAG" -eq 1 ]]; then
                console_name="$(cat "$GAME_CONSOLE_NAME")"
                mkdir -p "$COPY_ROMS_DIR/$console_name"
                cp -v "$f" "$COPY_ROMS_DIR/$console_name"
            fi
        else
            if [[ "$TAB_FLAG" == 1 ]]; then
                # echo -e "\tno"
                echo
            else
                echo -e "\"$f\" has no cheevos. :(\n" >&2
            fi
        fi
    done
}


# helping to deal with command line arguments
function check_argument() {
    # limitation: the argument 2 can NOT start with '-'
    if [[ -z "$2" || "$2" =~ ^- ]]; then
        echo "$1: missing argument" >&2
        return 1
    fi
}


function parse_args() {
    local i
    local ret
    local oldIFS

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
                update
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
                RA_PASSWORD="$(get_password)"
                echo
                ;;

# TODO: is it really necessary?
##H --token TOKEN         TOKEN is your RetroAchievements.org token.
##H 
           --token)
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

#H --hash CHECKSUM          Check if there are cheevos for a given CHECKSUM and exit.
#H                          Note: this option should be the last argument.
#H 
            --hash)
                local line
                local gameid

                check_argument "$1" "$2" || safe_exit 1
                ret=0

                if [[ ! $2 =~ ^$HASH_REGEX$ ]]; then
                    echo "--- invalid checksum: $2" >&2
                    safe_exit 1
                fi

                line="$(grep -i "\"$2\"" "$DATA_DIR"/*_hashlibrary.json 2> /dev/null)"
                echo -n "$(basename "${line%_hashlibrary.json*}")" > "$GAME_CONSOLE_NAME"
                gameid="$(echo ${line##*: } | tr -d ' ,')"
                if [[ ! $gameid =~ $GAMEID_REGEX ]]; then
                    echo "--- unable to get game ID." >&2
                    safe_exit 1
                fi

                if game_has_cheevos "$gameid"; then
                    echo "--- Game ID $gameid HAS CHEEVOS!" >&2
                else
                    echo "--- Game ID $gameid has no cheevos. :(" >&2
                    ret=1
                fi
                safe_exit "$ret"
                ;;

#H --get-data SYSTEM        Download JSON hash library for a given SYSTEM (console)
#H                          and exit.
#H 
            --get-data)
                check_argument "$1" "$2" || safe_exit 1
                shift
                update_ra_data "$1"
                safe_exit "$?"
                ;;

#H -a|--arcade              Arcade hashes are calculated agains the ROM filename
#H                          and then needs a different treatment. Use -a to check
#H                          arcade ROM files.
#H 
            -a|--arcade)
                ARCADE_FLAG=1
                ;;

#H -t|--tab-output          Instead of the normal output, the -t option makes
#H                          it be as in this example:
#H                          Game With Cheevos	yes
#H                          Game With No Cheevos	no
#H 
            -t|--tab-output)
                TAB_FLAG=1
                ;;

# XXX: is it a good idea to let users use the script this way? can stress the server
# answer: it's useful to check if a game doesn't have cheevos anymore.
#H -r|--check-ra-server     Force checking info at RetroAchievements.org server
#H                          ignoring some info you may have locally.
#H                          Note: do NOT use this option to check many files at once.
#H 
            -r|--check-ra-server)
                CHECK_RA_SERVER_FLAG=1
                ;;

#H -d|--copy-roms-to DIR    Create a copy of the ROMs that has cheevos and put
#H                          them at "DIR/CONSOLE_NAME/". There's no need to
#H                          specify the console name, the script detects it.
#H 
            -d|--copy-roms-to)
                check_argument "$1" "$2" || safe_exit 1
                shift
                COPY_ROMS_FLAG=1
                COPY_ROMS_DIR="$1"
                ;;

# TODO: is it really necessary?
##H --print-token            Print the user's RetroAchievements.org token and exit.
##H 
            --print-token)
                get_cheevos_token
                echo "$RA_TOKEN"
                safe_exit 0
                ;;

# TODO: will it be useful? this feature will be useful only if the related PR will be merged on ES.
##H --scrape                 [RETROPIE ONLY] Updates the gamelist.xml file with
##H                          <achievements>true</achievements> if the ROM has
##H                          cheevos.
##H 
            --scrape)
                if ! is_retropie; then
                    echo "ERROR: not a RetroPie system." >&2
                    echo "The \"$1\" option is available only for RetroPie systems." >&2
                    safe_exit 1
                fi
                SCRAPE_FLAG=1
                ;;

#H --collection             [RETROPIE ONLY] Creates a custom collection file
#H                          to use on RetroPie's EmulationStation. The resulting
#H                          files will be named as 
#H                          "~/.emuationstation/collections/custom-SYSTEM achievements.cfg"
#H                          and filled with full paths for ROMs that have cheevos.
#H 
            --collection)
                if ! is_retropie; then
                    echo "ERROR: not a RetroPie system." >&2
                    echo "The \"$1\" option is available only for RetroPie systems." >&2
                    safe_exit 1
                fi
                COLLECTIONS_FLAG=1
                ;;

#H --single-collection      [RETROPIE ONLY] Creates one big custom collection file
#H                          to use on RetroPie's EmulationStation. The resulting
#H                          file will be named 
#H                          "~/.emuationstation/collections/custom-achievements.cfg"
#H                          and filled with full paths to ALL ROMs that have cheevos.
#H 
            --single-collection)
                if ! is_retropie; then
                    echo "ERROR: not a RetroPie system." >&2
                    echo "The \"$1\" option is available only for RetroPie systems." >&2
                    safe_exit 1
                fi
                COLLECTIONS_FLAG=1
                SINGLE_COLLECTION_FLAG=1
                ;;

#H -s|--system SYSTEM       [RETROPIE ONLY] Check if each ROM in the respective
#H                          "~/RetroPie/roms/SYSTEM" directory has cheevos. You
#H                          can specifie multiple systems separeted by commas or
#H                          use "all" to check all supported systems' directory.
#H 
            -s|--system)
                local directories=()

                if ! is_retropie; then
                    echo "ERROR: not a RetroPie system." >&2
                    echo "The \"$1\" option is available only for RetroPie systems." >&2
                    safe_exit 1
                fi

                check_argument "$1" "$2" || safe_exit 1
                shift

                if [[ "$1" == all ]]; then
                    directories=("${SUPPORTED_SYSTEMS[@]}")
                else
                    oldIFS="$IFS"
                    IFS=, # XXX: not sure if it will impact other parts
                    for i in $1; do
                        directories+=("$i")
                    done
                    IFS="$oldIFS"
                fi

                for i in "${directories[@]}"; do
                    if [[ -d "$RP_ROMS_DIR/$i" ]]; then
                        ROMS_DIR+=("$RP_ROMS_DIR/$i")
                        continue
                    fi
                    echo "WARNING: ignoring \"$(basename "$i")\": not found." >&2
                done
                ;;

            *)  break
                ;;
        esac
        shift
    done

    FILES_TO_CHECK=("$@")
}



# START HERE ##################################################################

function main() {
    trap 'safe_exit 1' SIGHUP SIGINT SIGQUIT SIGKILL SIGTERM

    if [[ "$(id -u)" == 0 ]]; then
        echo "ERROR: You can't use this script as super user." >&2
        echo "       Please, try again as a regular user." >&2
        safe_exit 1
    fi

    check_dependencies

    [[ -z "$1" ]] && help_message

    fill_data

    parse_args "$@"

    update_ra_data

    if is_retropie && [[ -n "$ROMS_DIR" ]]; then
        local line
        while read -r line; do
            FILES_TO_CHECK+=("$line")
        done < <(find "${ROMS_DIR[@]}" -type f -regextype egrep -iregex ".*\.($EXTENSIONS)$")
    fi

    process_files "${FILES_TO_CHECK[@]}"
    safe_exit "$?"
}

main "$@"
