# hascheevos

Tells whether a ROM has RetroAchievements achievements, by identifying it the same way RetroAchievements does.

## Language

**Console**:
A gaming platform as RetroAchievements defines it, identified by a numeric console ID.
_Avoid_: System, platform

**ROM**:
The file a user wants identified. It may be a cartridge dump, a disc image, or an archive containing one.
_Avoid_: Game file, image

**Hash**:
The 32-hex-character identifier RetroAchievements assigns to a ROM for a given console, computed by that console's official method.
_Avoid_: Checksum, MD5

**Console Hash**:
A hash together with the console whose method produced it. A ROM whose console is unknown yields one console hash per plausible console.
_Avoid_: Candidate, fingerprint

**Game**:
A RetroAchievements entry with a game ID. Several hashes can point to the same game.
_Avoid_: Title

**Hash Library**:
A console's published mapping from hashes to games.
_Avoid_: Hashlib, hash list
