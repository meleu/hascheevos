# hascheevos
A kind of "cheevos scraper".

## installing

1. Go to the directory where you want to "install" the tool (if unsure, your home directory can be the easier choice):
```bash
cd # /path/to/the/chosen/directory
```

2. Clone the repo and go to the created directory:
```bash
git clone --depth 1 https://github.com/meleu/hascheevos
cd hascheevos
```

3. Compile the "cheevos hash calculator":
```bash
make
```

4. **[OPTIONAL]** Include the tool's directory on your PATH:
```bash
# adapt the path below to your setup!
export PATH="$PATH:/path/to/hascheevos/bin"
```

4. Done! The tool is ready to work!


## how to use it

**THE** tool of this repo is the [`hascheevos.sh`](https://github.com/meleu/hascheevos/blob/master/bin/hascheevos.sh) script.

The usual way to use it is:
```bash
./hascheevos.sh -u YOUR_RA_USERNAME -p YOUR_RA_PASSWORD /path/to/the/ROM/file
```

Run it with `--help` to see more options.


---

**What's the point of creating this tool?!**

Links to the answer:

- https://retropie.org.uk/forum/topic/11859/what-about-adding-a-cheevos-flag-in-gamelist-xml

- http://retroachievements.org/viewtopic.php?t=5025

