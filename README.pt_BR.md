# hascheevos

Uma maneira de checar se sua ROM está boa para RetroAchievements.

## instalação

0. **Dependências**: `jq`, `curl`, `unzip`, `gzip` e `p7zip-full`. Em uma distribuição Linux típica você provavelmente já terá a maioria destes pacotes instalados e a única novidade será o `jq` (uma ferramenta para analisar dados JSON). Em um sistema baseado em Debian o comando abaixo deve instalar tudo que você precisa:
```
sudo apt-get install jq unzip gzip p7zip-full curl
```

Se você está usando outra distribuição (ou até mesmo Cygwin no Windows), o script ainda pode ser útil pra você. Apenas certifique-se de instalar os pacotes equivalentes.

1. Vá para o diretório que você quer instalar a ferramento (se não tem certeza, o seu diretório *home* pode ser a escolha mais fácil):

```
cd # /path/to/the/chosen/directory
```

2. Clone o repositório e vá para o diretório criado:

```
git clone --depth 1 https://github.com/meleu/hascheevos
cd hascheevos
```

3. Compile o "cheevos hash calculator":
```
make
```
(sim, o comando está correto: apenas `make` e nada mais! Isto compila o `src/cheevoshash.c` e cria o executável `bin/cheevoshash`.)

4. **[OPCIONAL]** Inclua o diretório da ferramento no seu `PATH`:

```
# adapte o caminho abaixo para a configuração do seu sistema.
# provavelmente você também vai querer colocar esse comando no final do seu ~/.bashrc
export PATH="$PATH:/path/to/hascheevos/bin"
```

4. Feito! A ferramenta está pronta para ser usada!


## como usar

**A** ferramenta deste repositório é o script [`hascheevos.sh`](https://github.com/meleu/hascheevos/blob/master/bin/hascheevos.sh). Execute-o com `--help` para ver as opções disponíveis.

### Checando se uma ROM está OK para cheevos

Esta é a maneira mais simples de usar o script:

```
hascheevos.sh /path/to/the/ROM
```

#### Exemplo 1 - a ROM está OK para cheevos

```
$ hascheevos.sh /path/to/megadrive/Sonic\ the\ Hedgehog\ \(USA\,\ Europe\).zip 
Checking "/path/to/megadrive/Sonic the Hedgehog (USA, Europe).zip"...
--- hash:    2e912d4a3164b529bbe82295970169c6
--- game ID: 1
--- "/path/to/megadrive/Sonic the Hedgehog (USA, Europe).zip" HAS CHEEVOS!
```

#### Exemplo 2: não há cheevos para sua ROM

```
$ hascheevos.sh /path/to/nes/Qix\ \(USA\).zip 
Checking "/path/to/nes/Qix (USA).zip"...
--- hash:    40089153660f092b5cbb6e204efce1b7
--- game ID: 1892
--- "/path/to/nes/Qix (USA).zip" has no cheevos. :(
```


### Copiar todas as ROMs que possuem cheevos para um diretório.

Se você tem uma grande coleção de ROMs e quer copiar apenas aquelas que possuem cheevos para um outro diretório, você pode usar a opção `--copy-roms-to`.

No exemplo abaixo nós copiaremos todas as roms que possuem cheevos de `/path/to/megadrive/roms/` para `folder/for/cheevos/with/roms/megadrive`.

```
hascheevos.sh --copy-roms-to folder/for/cheevos/with/roms /path/to/megadrive/roms/*
```

**Observações**

- se o diretório de destino não existir, ele será criado.

- o script automaticamente cria um subdiretório abaixo do diretório passado como argumento para `--copy-roms-to` com o nome do console (megadrive, snes, etc.) da respectiva ROM. Exemplo: se você passar o diretório `cheevos_roms`, o script cria subdiretórios como `cheevos_roms/megadrive` ou `cheevos_roms/nes`, de acordo com o nome do console da ROM.

- Não se preocupe com os arquivos que não são ROMs (como `gamelist.xml` ou `.srm`), o script ignora arquivos com extensões inválidas. ;-)


### [APENAS PARA RETROPIE] Checar se cada ROM de um determinado console possui cheevos.

***Observação:** Este recurso só é utilizável em um sistema RetroPie.*

No RetroPie as ROMs ficam localizadas em `$HOME/RetroPie/roms/CONSOLE_NAME`. Quando usando o script em um sistema RetroPie, você pode checar todas as ROMs de um determinado console usando a opção `--system`. Exemplo:  

```
hascheevos.sh --system nes
```

**Observação**: Se você passar `all` para a opção `--system`, o script procurará no diretório de todos os sistemas suportados. A saber: `megadrive`, `nes`, `snes`, `gb`, `gbc`, `gba`, `pcengine`, `mastersystem` e `n64`


### [APENAS PARA RETROPIE] Criar um "custom collections" para EmulationStation (para cada console) com todos os jogos que possuem cheevos.

***Observações:***

- *Este recurso só é utilizável em um sistema RetroPie.*
- *O recurso "custom collection" foi implementado no EmulationStation 2.6.0.*
- *Informações sobre como usar "custom collections" no ES podem ser encontradas [aqui](https://github.com/retropie/retropie-setup/wiki/EmulationStation#custom-collections).*

O comando abaixo cria "custom collections" para todos os sistemas suportados, populando-os com os seus jogos que possuem cheevos.

```
hascheevos.sh --collection --system all
```
Dependendo de quantas ROMs você possui este commando pode levar alguns minutos.

Depois que o script terminar, reinicie o EmulationStation, pressione `Start` para acessar o **MAIN MENU** e então vá em **GAME COLLECTIONS SETTING** -> **CUSTOM GAME COLLECTIONS** e habilite os "achievements collections" que você vê.

**Agora você tem um custom collection para cada sistema que suporta RetroAchievements e populado apenas com os seus jogos que possuem achievements.**

---

**What's the point of creating this tool?!**

Links to the answer:

- https://retropie.org.uk/forum/topic/11859/what-about-adding-a-cheevos-flag-in-gamelist-xml

- http://retroachievements.org/viewtopic.php?t=5025
