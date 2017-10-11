# hascheevos
A way to check if your ROM is OK for RetroAchievements.

## installing

0. Be sure to have all dependencies installed. On a Debian based system the command below should install everything you need. Probably you already have most of these packages installed and the only new one will be `jq` (it's a tool to parse JSON data):
```
sudo apt-get install jq unzip gzip p7zip-full curl
```

If you're using another Linux distro (or even Cygwin on Windows), the script is still useful for you. Just be sure to install the equivalent packages on your system.


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
export PATH="$PATH:/path/to/hascheevos/bin"
```

4. Done! The tool is ready to work!


## how to use it

**THE** tool of this repo is the [`hascheevos.sh`](https://github.com/meleu/hascheevos/blob/master/bin/hascheevos.sh) script.

The usual way to use it is:
```
./hascheevos.sh -u YOUR_RA_USERNAME -p YOUR_RA_PASSWORD /path/to/the/ROM/file
```

**Run it with `--help` to see more options.**

## examples

### When there are cheevos for your ROM/game.

```
$ ./hascheevos.sh -u USER -p PASSWORD /path/to/megadrive/Sonic\ the\ Hedgehog\ \(USA\,\ Europe\).zip 
Checking "/path/to/megadrive/Sonic the Hedgehog (USA, Europe).zip"...
--- hash:    2e912d4a3164b529bbe82295970169c6
--- game ID: 1
--- "/path/to/megadrive/Sonic the Hedgehog (USA, Europe).zip" HAS CHEEVOS!
```

### When there are no cheevos for your ROM/game.

```
$ ./hascheevos.sh -u USER -p PASSWORD /path/to/nes/Qix\ \(USA\).zip 
Checking "/path/to/nes/Qix (USA).zip"...
--- hash:    40089153660f092b5cbb6e204efce1b7
--- game ID: 1892
--- "/path/to/nes/Qix (USA).zip" has no cheevos. :(
```

### When your ROM is incompatible.

```
$ ./hascheevos.sh -u USER -p PASSWORD  /path/to/mastersystem/Alex\ Kidd\ in\ Miracle\ World\ \(USA\,\ Europe\).zip 
Checking "/path/to/mastersystem/Alex Kidd in Miracle World (USA, Europe).zip"...
--- hash:    1b494dd760aef7929313d6a803c2d003
--- hash:    50a29e43423cc77564d6f49b289eef1d
--- checking at RetroAchievements.org server...
--- hash:    1b494dd760aef7929313d6a803c2d003
--- hash:    50a29e43423cc77564d6f49b289eef1d
WARNING: this ROM file doesn't feature achievements.
```

### Create a list of all ROMs that have cheevos in a directory.

The only thing the script puts on stdout are file names of ROMs that have cheevos. Everything else are printed in stderr. Then if you want a list of all ROMs that have cheevos in a directory, do something like this:

```
$ ./hascheevos.sh -u USER -p PASSWORD /path/to/megadrive/* > ~/megadrive-roms-with-cheevos.txt
```

Don't worry about non-ROM files in the same directory (like `gamelist.xml` or `.srm` files), the script ignores files with invalid extensions. ;-)


### [RETROPIE ONLY] Check if a ROM has cheevos and if yes, add `<achievements>true</achievements>` entry in the respective `gamelist.xml` file.

***Note:** This feature is only usable on a RetroPie system*

```
$ ./hascheevos.sh -u USER -p PASSWORD --scrape ~/RetroPie/roms/nes/Mega\ Man\ \(USA\).zip 
Checking "/home/pi/RetroPie/roms/nes/Mega Man (USA).zip"...
--- NES: 4de82cfceadbf1a5e693b669b1221107
--- game ID: 1448
--- Game Title: "Mega Man"
--- This game has been defined as having cheevos in "/home/pi/RetroPie/roms/nes/gamelist.xml".
--- "/home/pi/RetroPie/roms/nes/Mega Man (USA).zip" HAS CHEEVOS!
```


### [RETROPIE ONLY] Check if each ROM in a system directory has cheevos.

***Note:** This feature is only usable on a RetroPie system*

```
$ ./hascheevos.sh -u USER -p PASSWORD --system nes
```

This option is specially useful when used in conjunction with `--scrape`. The example below will scan all your NES' ROMs and update the NES' `gamelist.xml` with `<achievements>true</achievements>` for every ROM that has cheevos.

```
$ ./hascheevos.sh -u USER -p PASSWORD --system nes --scrape
```


---

**What's the point of creating this tool?!**

Links to the answer:

- https://retropie.org.uk/forum/topic/11859/what-about-adding-a-cheevos-flag-in-gamelist-xml

- http://retroachievements.org/viewtopic.php?t=5025

