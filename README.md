# dunspars
A command-line interface for Pokémon information pulled from the PokéAPI.

## Usage
### Pokemon
View a Pokémon's basic information. 
```
dunspars pokemon pikachu --evolution --moves
```
The `--evolution` and `--moves` option includes its evolutionary line and learnable moves respectively.

### Match
View match-up information such as stats and move weaknesses between 1-6 vs 1 Pokémon. The last Pokémon specified will be considered the attacker. 
```
dunspars match blaziken flygon goodra
```
In this example, it will display match-up information for `Blaziken vs Goodra` and `Flygon vs Goodra`.

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
Install [rustup](https://www.rust-lang.org/tools/install)
```
cd dunspars
cargo build --release
```
