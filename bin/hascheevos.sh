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
readonly SCRIPT_DIR="$(cd "$(dirname $0)" && pwd)"
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
FILES_TO_CHECK=()
COPY_ROMS_DIR=
TMP_DIR="/tmp/hascheevos-$$"
mkdir -p "$TMP_DIR"
GAME_CONSOLE_NAME="$(mktemp -p "$TMP_DIR")"

SUPPORTED_SYSTEMS=(megadrive n64 snes gb gba gbc nes pcengine mastersystem atarilynx ngp)

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
CONSOLE_NAME[12]=psx
CONSOLE_NAME[13]=atarilynx
CONSOLE_NAME[14]=ngp
CONSOLE_NAME[15]=xbox360
CONSOLE_NAME[16]=gamecube
CONSOLE_NAME[17]=jaguar
CONSOLE_NAME[18]=nds
CONSOLE_NAME[19]=wii
CONSOLE_NAME[20]=wiiu
CONSOLE_NAME[21]=ps2
CONSOLE_NAME[22]=xbox
CONSOLE_NAME[23]=skynet
CONSOLE_NAME[24]=xone
CONSOLE_NAME[25]=atari2600
CONSOLE_NAME[26]=dos
CONSOLE_NAME[27]=arcade
CONSOLE_NAME[28]=virtualboy
CONSOLE_NAME[29]=msx
CONSOLE_NAME[30]=commodore64
CONSOLE_NAME[31]=zx81

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
    echo "$@" |  sed -e 's/[]\/$*.^|[]/\\&/g'
}


function get_game_title_hascheevos() {
    echo "$@" | sed 's/^[^:]\+:[^:]\+://'
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

    # after updating, silently check hascheevos-local.txt files
    check_hascheevos_file >/dev/null 2>&1
    rm "$DATA_DIR/*.bkp" 2> /dev/null

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

    RA_TOKEN="$(curl -s "http://retroachievements.org/dorequest.php?r=login&u=${RA_USER}&p=${RA_PASSWORD}" | jq -e -r .Token)"
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


# download hashlibrary for a specific console
# $1 is the console_id
function download_hashlibrary() {
    local console_id="$1"

    if [[ "$console_id" -le 0 || "$console_id" -gt "${#CONSOLE_NAME[@]}" ]]; then
        echo "ERROR: invalid console ID: $console_id" >&2
        safe_exit 1
    fi

    if ! is_supported_system "${CONSOLE_NAME[console_id]}"; then
        # XXX: print the line below when --verbose (but I didn't implement --verbose yet)
#        echo "WARNING: ignoring unsupported system ${CONSOLE_NAME[console_id]} (console ID=$console_id)" >&2
        return
    fi

    local json_file="$DATA_DIR/${CONSOLE_NAME[console_id]}_hashlibrary.json"

    echo "--- getting the console hash library for \"${CONSOLE_NAME[console_id]}\"..." >&2
    curl -s "http://retroachievements.org/dorequest.php?r=hashlibrary&c=$console_id" \
        | jq '.' > "$json_file" 2> /dev/null \
        || echo "ERROR: failed to download hash library for \"${CONSOLE_NAME[console_id]}\"!" >&2

    [[ -s "$json_file" ]] || rm -f "$json_file"
}


# if a valid system is given in $1, the function tries to update only the
# hashlib for that system.
# Otherwise update hashlibraries older than 1 day.
function update_hashlib() {
    local line
    local system="$1"
    local file
    local i

    echo "Checking JSON hash libraries..." >&2
    for i in "${!CONSOLE_NAME[@]}"; do
        [[ -f "$DATA_DIR/${CONSOLE_NAME[i]}_hashlibrary.json" ]] || download_hashlibrary "$i"
    done
    echo "Done!" >&2

    if [[ -n "$system" ]]; then
        file="$DATA_DIR/${system}_hashlibrary.json"
        # check if the file exists and is older than 1 minute
        if [[ -n "$(find "$file" -mmin +1 2>/dev/null)" ]]; then
            echo "Updating \"$system\" hashlib..." >&2
            for i in "${!CONSOLE_NAME[@]}"; do
                if [[ "${CONSOLE_NAME[i]}" == "$system" ]]; then
                    download_hashlibrary "$i" && echo "Done!" >&2
                    return "$?"
                fi
            done
        else
            if [[ -f "$file" ]]; then
                echo "The \"$system\" hashlib is already up-to-date." >&2
                return 0
            else
                echo "ERROR: invalid system: \"$system\""
                return 1
            fi
        fi
    else
        while read -r line; do
            system="$(basename "${line%_hashlibrary.json*}")"
            for i in "${!CONSOLE_NAME[@]}"; do
                if [[ "${CONSOLE_NAME[i]}" == "$system" ]]; then
                    download_hashlibrary "$i"
                    break
                fi
            done
        done < <(find "$DATA_DIR" -type f -name '*_hashlibrary.json' -mtime +1)
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
    echo -n > "$GAME_CONSOLE_NAME"

    hash="$(get_rom_hash "$rom")" || return 1

    while read -r line; do
        echo "--- $line" >&2
        hash_i="$(echo "$line" | sed 's/^\(SNES\|NES\|Genesis\|plain MD5\): //')"
        line="$(grep -i "\"$hash_i\"" "$DATA_DIR"/*_hashlibrary.json 2> /dev/null)"
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
        echo "--- WARNING: this ROM file doesn't feature achievements." >&2
        return 1
    fi

    if [[ ! $gameid =~ $GAMEID_REGEX ]]; then
        echo "--- unable to get game ID." >&2
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
    local patch_json="$(curl -s "http://retroachievements.org/dorequest.php?r=patch&u=${RA_USER}&g=${gameid}&f=3&l=1&t=${RA_TOKEN}")"

    local console_id="$(echo "$patch_json" | jq -e '.PatchData.ConsoleID')"
    if [[ "$?" -ne 0 || "$console_id" -lt 1 || "$console_id" -gt "${#CONSOLE_NAME[@]}" || -z "$console_id" ]]; then
        echo "--- WARNING: unable to find the Console ID for Game #$gameid!" >&2
        return 1
    fi
    hascheevos_file="$DATA_DIR/${CONSOLE_NAME[console_id]}_hascheevos-local.txt"

    game_title="$(echo "$patch_json" | jq -e '.PatchData.Title')" || game_title=
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
    local file_pr # file for Pull Request
    local line_local
    local line_orig
    local gameid
    local bool_local
    local bool_orig
    local title_local
    local title_orig
    local ret
    local updated
    local pr_files
    local ret=0
    local tmp_ret

    is_updated
    ret="$?"
    case "$ret" in
        0)  updated=true
            ;;
        1)  update=false
            echo "ERROR: your hascheevos files are outdated. Perform an '--update' and try again." >&2
            return "$ret"
            ;;
        2)  updated=false
            echo "WARNING: unable to compare your local files with remote ones from hascheevos repository." >&2
            return "$ret"
            ;;
    esac

    while read -r file_local; do
        tmp_ret=0
        file_orig="${file_local/-local/}"
        file_pr="${file_orig/.txt/-PR.txt}"
        [[ "$updated" == true ]] && cat "$file_orig" > "$file_pr"

        echo
        echo "Checking \"$(basename "$file_orig")\"..."

        while read -r line_local; do
            gameid=$(echo "$line_local" | cut -d: -f1)
            line_orig=$(grep "^$gameid:" "$file_orig")
            bool_local=$(echo "$line_local" | cut -d: -f2)
            bool_orig=$( echo "$line_orig"  | cut -d: -f2)
            title_local="$(get_game_title_hascheevos "$line_local")"
            title_orig="$( get_game_title_hascheevos "$line_orig")"

            if [[ -z "$line_orig" ]]; then
                echo "* there's no Game ID #$gameid ($title_local) on your \"$(basename "$file_orig")\"."
                ret=3
                tmp_ret=1
            elif [[ "$bool_local" == "$bool_orig" ]]; then
                if [[ "$title_local" == "$title_orig" ]]; then
                    sed -i "/$(regex_safe "$line_local")/d" "$file_local"
                    [[ -s "$file_local" ]] || rm "$file_local"
                else
                    echo "* Game ID #$gameid is named $title_local locally but it's $title_orig in the original file."
                    ret=3
                    tmp_ret=1
                fi
            else
                echo "* Game ID #$gameid ($title_local) is marked as \"$bool_local\" locally but it's \"$bool_orig\" in the original file."
                ret=3
                tmp_ret=1
            fi

            if [[ "$updated" == true && "$ret" != 0 ]]; then
                sed -i "/^$gameid/d" "$file_pr"
                echo "$line_local" >> "$file_pr"
                sort -o "$file_pr" -un "$file_pr"
            fi

        done < "$file_local"
        if [[ "$updated" == true ]]; then
            diff -q "$file_pr" "$file_orig" >/dev/null && rm "$file_pr"
        fi
    done < <(find "$DATA_DIR" -type f -name '*_hascheevos-local.txt')

    while read -r file_pr; do
        file_orig="${file_pr/-PR.txt/.txt}"
        if diff -q "$file_pr" "$file_orig" >/dev/null; then
            rm "$file_pr"
        else
            pr_files+=("$file_pr")
        fi
    done < <(find "$DATA_DIR" -maxdepth 1 -name '*-PR.txt')

    if [[ -n "$pr_files" ]]; then
        # XXX: yeah! I shouldn't hardcode this thing, but it helps to keep the repo updated! :)
        if [[ "$updated" == true && "$RA_USER" == meleu ]]; then
            update_repository || echo "WARNING: hascheevos repository was NOT updated!" >&2
        else
            echo -e "\n-----"
            echo "Consider helping to keep the hascheevos files synchronized with RetroAchievements.org data."
            echo "Please, copy the output's content above and paste it in a new issue at https://github.com/meleu/hascheevos/issues"
            echo "Attaching the file(s) below to your issue would be really useful:"
            echo "${pr_files[@]}"
        fi
    fi

    return "$ret"
}


# this function exists only for me, sorry :)
# needs to be called from check_hascheevos_files() to access its variables
function update_repository() {
    [[ "$RA_USER" != meleu ]] && return 1

    local file_bkp
    local commit_msg=()

    commit_msg=(-m "updated *_hascheevos.txt files ($(date +'%d-%b-%Y %H:%M'))")

    pushd "$dir" > /dev/null
    for file_pr in "${pr_files[@]}"; do
        file_orig="${file_pr/-PR.txt/.txt}"
        file_bkp="${file_orig/.txt/.bkp}"

        cat "$file_orig" > "$file_bkp"
        cat "$file_pr" > "$file_orig"

        git add "$file_orig"
        commit_msg+=(-m "$(basename "$file_orig")" )
    done
    echo
    git commit "${commit_msg[@]}"
    git push origin master

    # revert things if failed to push
    if [[ "$?" != 0 ]]; then
        for file_pr in "${pr_files[@]}"; do
            file_orig="${file_pr/-PR.txt/.txt}"
            file_bkp="${file_orig/.txt/.bkp}"
            cat "$file_bkp" > "$file_orig"
        done
        git reset --soft HEAD^
    fi

    popd > /dev/null
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
        if rom_has_cheevos "$f"; then
            [[ "$COLLECTIONS_FLAG" -eq 1 ]] && set_cheevos_custom_collection "$f" true
            [[ "$SCRAPE_FLAG" -eq 1 ]]      && set_cheevos_gamelist_xml      "$f" true
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
                check_argument "$1" "$2" || safe_exit 1
                shift
                RA_PASSWORD="$1"
                ;;

# TODO: is it really necessary?
##H -t|--token TOKEN         TOKEN is your RetroAchievements.org token.
##H 
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

#H --get-hashlib SYSTEM     Download JSON hash library for a given SYSTEM (console)
#H                          and exit.
#H 
            --get-hashlib)
                check_argument "$1" "$2" || safe_exit 1
                shift
                update_hashlib "$1"
                safe_exit "$?"
                ;;

#H -f|--check-false         Check at RetroAchievements.org server even if the
#H                          game ID is marked as "has no cheevos" (false) in
#H                          the local *_hascheevos.txt files.
#H 
            -f|--check-false)
                CHECK_FALSE_FLAG=1
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

#H -c|--check-hascheevos    Check if your local data is synchronized with the
#H                          repository, print a report and exit.
#H 
            -c|--check-hascheevos)
                if check_hascheevos_files; then
                    echo "Your hascheevos files are up-to-date."
                    safe_exit "0"
                fi
                safe_exit "1"
                ;;

# TODO: is it really necessary?
##H --print-token            Print the user's RetroAchievements.org token and exit.
##H 
            --print-token)
                get_cheevos_token
                echo "$RA_TOKEN"
                safe_exit
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
    trap safe_exit SIGHUP SIGINT SIGQUIT SIGKILL SIGTERM

    if [[ "$(id -u)" == 0 ]]; then
        echo "ERROR: You can't use this script as super user." >&2
        echo "       Please, try again as a regular user." >&2
        safe_exit 1
    fi

    check_dependencies

    [[ -z "$1" ]] && help_message

    update_hashlib

    parse_args "$@"

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
