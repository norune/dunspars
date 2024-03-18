# dunspars
A command-line interface for Pokémon information pulled from the [PokéAPI](https://pokeapi.co/) via [Rustemon](https://github.com/mlemesle/rustemon).

## Usage
### Setup
Before using the program, run the one-time setup. 
```
dunspars setup
```
This action requires an internet connection. Once it is finished, the program should be available for use offline.

### Pokemon
View a Pokémon's basic information. 
```
dunspars pokemon pikachu --evolution --moves
```
The `--evolution` and `--moves` option includes its evolutionary line and learnable moves respectively.

### Game Version
You can specify a game via the `--game` option in any relevant subcommand.
```
dunspars pokemon clefairy --game emerald
```

### Match
View match-up information such as stats and move weaknesses between 1-6 vs 1 Pokémon. The last Pokémon specified will be considered the attacker. 
```
dunspars match blaziken flygon goodra
```
In this example, it will display match-up information for `Blaziken vs Goodra` and `Flygon vs Goodra`.

### Coverage
View your type coverage based on the types of the provided Pokémon.
```
dunspars coverage flamigo cramorant ribombee
```
This will list which of the provided Pokémon will offer offensive and defensive advantage for each type.

### Type
View a Pokémon Type's strengths and weaknesses.
```
dunspars type fairy
```

### Move
View the combat information of a Pokémon move.
```
dunspars move quick-attack
```

### Ability
View the effects of a Pokémon ability.
```
dunspars ability intimidate
```

### Help
```
dunspars --help
```
For help within a subcommand, the `--help` option should still apply.
```
dunspars pokemon --help
```

## Installation

### Supported Operating Systems
Ubuntu 23.10

### Releases
Binaries for each version are included in the [releases](https://github.com/norune/dunspars/releases) section of this repo.

### Compile from Source
Prerequisites
- [rustup](https://www.rust-lang.org/tools/install)
- [libsqlite3-dev](https://packages.ubuntu.com/mantic/libsqlite3-dev)

```
cd dunspars
cargo build --release
```
