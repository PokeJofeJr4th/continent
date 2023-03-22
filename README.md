# continent
Rust/Python world simulation
## Rust
- main.rs puts a world map and century indicators into the terminal, then generates a foo.json file with most of the information the python project needs to generate a report.
- lib.rs has the functionality for generating and dumping markov data.
- build.rs converts csv strings to condensed .mkv format.
## Python
- sim-template.py is a Jinja2 template file
- sim.py is the file with all of the game logic and the pygame window with the dynamic world map
- compile.py runs the Jinja2 to generate sim.py
- main.py sets parameters for sim.py and runs the code within
- launcher.py looks at the available world.json files and runs a pygame window that allows the user to choose arguments for main.py
- languages.py is an experiment to create languages that interact with the world
- names.py uses a markov algorithm to generate names
- magic.py generates a magic system
## Templates
- report is a Jinja2 / Html template file that creates the page that describes a world
- map is a Jinja2 / Html template file that creates a map of a region of the world
