# hascheevos
A way to check if your ROM is OK for RetroAchievements.

## installing

0. **Dependencies**: `jq`, `curl`, `unzip`, `gzip` and `p7zip-full`. On typical Linux distro you'll probably already have most of these packages installed and the only new one will be `jq` (it's a tool to parse JSON data). On a Debian based system the command below should install everything you need:
```
sudo apt-get install jq unzip gzip p7zip-full curl
```

If you're using another Linux distro (or even Cygwin on Windows), the script is still useful for you. Just be sure to install the equivalent packages on your shell.


1. Go to the directory where you want to "install" the tool (if unsure, your home directory can be the easiest choice):
```
cd # /path/to/the/chosen/directory
```

2. Clone the repo and go to the created directory:
```
git clone --depth 1 https://github.com/meleu/hascheevos
cd hascheevos
```

3. Compile the "cheevos hash calculator":
```
make
```
(yes, the command is right: just `make` and nothing more! This compiles the `src/cheevoshash.c` and creates the executable `bin/cheevoshash`.)

4. **[OPTIONAL]** Include the tool's directory on your PATH:
```
# adapt the path below to your setup!
# you probably want to paste it at the end of your ~/.bashrc
export PATH="$PATH:/path/to/hascheevos/bin"
```

4. Done! The tool is ready to work!


## how to use it

**THE** tool of this repo is the [`hascheevos.sh`](https://github.com/meleu/hascheevos/blob/master/bin/hascheevos.sh) script. Run it with `--help` to see the available options.

### Checking if a single ROM is OK for cheevos

This is the simplest way to use the script:

```
hascheevos.sh /path/to/the/ROM
```

#### Example 1 - the ROM is OK for cheevos

```
$ hascheevos.sh /path/to/megadrive/Sonic\ the\ Hedgehog\ \(USA\,\ Europe\).zip 
Checking "/path/to/megadrive/Sonic the Hedgehog (USA, Europe).zip"...
--- hash:    2e912d4a3164b529bbe82295970169c6
--- game ID: 1
--- "/path/to/megadrive/Sonic the Hedgehog (USA, Europe).zip" HAS CHEEVOS!
```

#### Example 2: there is no cheevos for your ROM

```
$ hascheevos.sh /path/to/nes/Qix\ \(USA\).zip 
Checking "/path/to/nes/Qix (USA).zip"...
--- hash:    40089153660f092b5cbb6e204efce1b7
--- game ID: 1892
--- "/path/to/nes/Qix (USA).zip" has no cheevos. :(
```


### Copy all ROMs that have cheevos to a directory.

If you have a big ROM set and want to copy only those which have cheevos, you can use the `--copy-roms-to` option.

In the example below we will copy all ROMs that have cheevos from `/path/to/megadrive/roms/` to `folder/for/cheevos/with/roms/megadrive`.

```
hascheevos.sh --copy-roms-to folder/for/cheevos/with/roms /path/to/megadrive/roms/*
```

**Notes**

- if the destination directory doesn't exist, it will be created.

- the script automatically creates a subdirectory below the directory passed as argument to `--copy-roms-to` with the console name (megadrive, snes, etc.) of the respective ROM. Example: if you pass the directory `cheevos_roms`, the script creates subdirectories like `cheevos_roms/megadrive` or `cheevos_roms/nes`, according to the ROM's console name.

- Don't worry about non-ROM files in the same directory (like `gamelist.xml` or `.srm` files), the script ignores files with invalid extensions. ;-)


### [RETROPIE ONLY] Check if each ROM of a given console has cheevos.

***Note:** This feature is only usable on a RetroPie system*

On RetroPie the roms are placed at `$HOME/RetroPie/roms/CONSOLE_NAME`. When using this script on a RetroPie system, you can check all ROMs for a given console using the the `--system` option. Example:  

```
hascheevos.sh --system nes
```

**Note**: If you pass `all` for `--system` option, the script will look all supported system's directory. Namely: `megadrive`, `nes`, `snes`, `gb`, `gbc`, `gba`, `pcengine`, `mastersystem` and `n64`


### [RETROPIE ONLY] Create an EmulationStation custom collection (for each console) with all games that have cheevos

***Notes:***

- *This feature is only usable on a RetroPie system.*
- *This feature is only useful if you're using EmulationStation 2.6.0+.*
- *Info on how to use ES custom collections can be found [here](https://github.com/retropie/retropie-setup/wiki/EmulationStation#custom-collections).*

The command below creates custom collections for all supported systems, populating them with the your games that have cheevos.

```
hascheevos.sh --collection --system all
```

Depending on how many ROMs you have this command will take a few minutes.

After the script finish, restart EmulationStation, press `Start` to access the **MAIN MENU** and then go to **GAME COLLECTIONS SETTING** -> **CUSTOM GAME COLLECTIONS** and enable the achievements collections you see there.

Now you have a custom collection for each system that supports RetroAchievements and populated only with your games that have achievements.

---

**What's the point of creating this tool?!**

Links to the answer:

- https://retropie.org.uk/forum/topic/11859/what-about-adding-a-cheevos-flag-in-gamelist-xml

- http://retroachievements.org/viewtopic.php?t=5025
