# Continent

Rust/Python world simulation

## Rust

- main.rs puts a world map and century indicators into the terminal, then generates a foo.json file with the information the python project needs to generate a report.
- magic.rs handles storing data about and generating the magic system
- jsonize.rs converts between json save data and workable sim data
- mkv.rs has the functionality for creating markov data and generating words from it.
- worldgen.rs randomly generates the world and all related structs
- build.rs converts csv strings to condensed .mkv format.

### Command Line Tool

#### continent.exe list

list all available save and gen files in the local directory

#### continent.exe run \[OPTIONS] \<PATH>

|                            |                                           |
| :------------------------- | :---------------------------------------- |
| -d, --duration \<DURATION> | Duration in years [default: 1000]         |
| -s, --save \<SAVE>         | File to save to (doesn't save by default) |

### Markov Format

The .mkv format is designed to efficiently store data that can be used to generate words, using a markov algorithm. The format contains two types of bytes: Terminators (T) of all 0s, and Letter-Counts (LC), of three bits representing a number followed by 5 bits representing a character. The three-bit segment is offset by one, so 000 represents 1 and 111 represents 8. The 5-bit segment goes from a=1 to z=26, and 27 is used to signify the end of the word (';').

The first section of the file is pairs of LCs, followed by a Terminator. Each letter-count is one possible way for a word to begin.

Each following section is a pair of LCs followed by a set of LCs, followed by a Terminator. The initial pair is the coda of a partial word, and each following LC is a weight for how likely that letter is to come after the coda pair.

## Python

- sim-template.py is a Jinja2 template file
- sim.py is the file with all of the game logic and the pygame window with the dynamic world map
- compile.py runs the Jinja2 to generate sim.py
- main.py sets parameters for sim.py and runs the code within
- launcher.py looks at the available world.json files and runs a pygame window that allows the user to choose arguments for main.py
- languages.py is an experiment to create languages that interact with the world (unimplemented)
- names.py uses a markov algorithm to generate names
- magic.py generates a magic system (deprecated)

## Templates

- report is a Jinja2 / Html template file that creates the page that describes a world
- map is a Jinja2 / Html template file that creates a map of a region of the world
